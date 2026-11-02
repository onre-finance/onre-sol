use crate::constants::seeds;
use crate::constants::PRICE_DECIMALS;
use crate::instructions::market_info::offer_valuation_utils::{
    get_active_vector_and_current_price, get_nav_adjustment_snapshot,
};
use crate::instructions::Offer;
use crate::state::MarketStats;
use crate::utils::{
    load_or_init_pda_account, load_pda_account, mul_div_round_u128, pow_fixed, store_pda_account,
    PdaAccountInit,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

/// External scale factor used for APR/APY representation (scale=6)
/// 1_000_000 represents 100%, so 10_000 = 1%, 100_000 = 10%
const EXT_SCALE: u128 = 1_000_000;

/// Internal scale factor for high-precision fixed-point arithmetic (scale=18)
/// Used during intermediate calculations to maintain precision
const INT_SCALE: u128 = 1_000_000_000_000_000_000;

/// Number of compounding periods per year for daily compounding
/// Standard financial calculation uses 365 days per year
const N: u128 = 365;

/// Canonical in-memory representation of the derived market stats values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketStatsSnapshot {
    pub apy: u64,
    pub circulating_supply: u64,
    pub nav: u64,
    pub nav_adjustment: i64,
    pub tvl: u64,
}

impl PdaAccountInit for MarketStats {
    fn pda_seed_prefixes() -> &'static [&'static [u8]] {
        &[seeds::MARKET_STATS]
    }

    fn init_space() -> usize {
        8 + MarketStats::INIT_SPACE
    }

    fn init_value(bump: u8) -> Self {
        Self {
            apy: 0,
            circulating_supply: 0,
            nav: 0,
            nav_adjustment: 0,
            tvl: 0,
            last_updated_at: 0,
            last_updated_slot: 0,
            bump,
            reserved: [0; 95],
        }
    }

    fn invalid_owner_error() -> Error {
        error!(crate::OnreError::InvalidMarketStatsOwner)
    }

    fn invalid_data_error() -> Error {
        error!(crate::OnreError::InvalidMarketStatsData)
    }
}

/// Recomputes the protocol's canonical market stats from the current on-chain state.
///
/// This helper is intended to be shared by multiple instructions so the PDA write path
/// always uses identical business logic for price, supply, and TVL calculations.
pub fn recompute_market_stats(
    offer: &Offer,
    onyc_mint: &InterfaceAccount<Mint>,
    excluded_balance_amount: u64,
) -> Result<MarketStatsSnapshot> {
    require_keys_eq!(
        *onyc_mint.to_account_info().owner,
        anchor_spl::token::ID,
        crate::OnreError::InvalidTokenProgram
    );
    require_keys_eq!(
        offer.token_out_mint,
        onyc_mint.key(),
        crate::OnreError::InvalidOnycMint
    );

    let current_time = Clock::get()?.unix_timestamp as u64;
    let (active_vector, nav) = get_active_vector_and_current_price(offer, current_time)?;
    let apy = calculate_apy_from_apr(active_vector.apr)?;
    let nav_adjustment = calculate_nav_adjustment(offer, active_vector)?;

    let circulating_supply =
        calculate_circulating_supply(onyc_mint.supply, excluded_balance_amount)?;
    let tvl = calculate_tvl(circulating_supply, nav)?;

    Ok(MarketStatsSnapshot {
        apy,
        circulating_supply,
        nav,
        nav_adjustment,
        tvl,
    })
}

/// Writes the recomputed snapshot into the market-stats PDA and stamps refresh metadata.
pub fn update_market_stats_account(
    market_stats: &mut MarketStats,
    snapshot: MarketStatsSnapshot,
) -> Result<()> {
    let clock = Clock::get()?;
    apply_market_stats_snapshot(market_stats, snapshot, &clock);
    Ok(())
}

pub fn refresh_market_stats_typed(
    offer: &Offer,
    onyc_mint: &InterfaceAccount<Mint>,
    excluded_balance_amount: u64,
    market_stats: &mut MarketStats,
    bump: u8,
) -> Result<()> {
    let snapshot = recompute_market_stats(offer, onyc_mint, excluded_balance_amount)?;
    market_stats.bump = bump;
    update_market_stats_account(market_stats, snapshot)
}

pub fn refresh_market_stats_pda<'info>(
    offer: &Offer,
    onyc_mint: &InterfaceAccount<'info, Mint>,
    excluded_balance_account: &AccountInfo<'info>,
    market_stats_account: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    program_id: &Pubkey,
) -> Result<()> {
    let excluded_balance_amount = super::load_circulating_supply_excluded_balance_amount(
        program_id,
        excluded_balance_account,
    )?;
    let snapshot = recompute_market_stats(offer, onyc_mint, excluded_balance_amount)?;
    let (market_stats_pda, market_stats_bump) =
        Pubkey::find_program_address(&[seeds::MARKET_STATS], program_id);
    require_keys_eq!(
        market_stats_account.key(),
        market_stats_pda,
        crate::OnreError::InvalidMarketStatsOwner
    );

    let mut market_stats = load_or_init_pda_account::<MarketStats>(
        market_stats_account,
        payer,
        system_program,
        program_id,
        market_stats_bump,
    )?;
    market_stats.bump = market_stats_bump;
    update_market_stats_account(&mut market_stats, snapshot)?;
    store_pda_account(market_stats_account, &market_stats)
}

pub fn load_main_offer(
    program_id: &Pubkey,
    market_offer_account: &AccountInfo,
    state: &crate::state::State,
) -> Result<Offer> {
    require_keys_eq!(
        market_offer_account.key(),
        state.main_offer,
        crate::OnreError::InvalidMainOffer
    );

    load_pda_account(
        market_offer_account,
        program_id,
        crate::OnreError::InvalidMainOffer.into(),
        crate::OnreError::InvalidMainOffer.into(),
    )
}

pub fn apply_market_stats_snapshot(
    market_stats: &mut MarketStats,
    snapshot: MarketStatsSnapshot,
    clock: &Clock,
) {
    market_stats.apy = snapshot.apy;
    market_stats.circulating_supply = snapshot.circulating_supply;
    market_stats.nav = snapshot.nav;
    market_stats.nav_adjustment = snapshot.nav_adjustment;
    market_stats.tvl = snapshot.tvl;
    market_stats.last_updated_at = clock.unix_timestamp;
    market_stats.last_updated_slot = clock.slot;
}

pub fn read_market_stats_account(market_stats_account: &AccountInfo) -> Result<MarketStats> {
    load_pda_account(
        market_stats_account,
        &crate::ID,
        crate::OnreError::InvalidMarketStatsOwner.into(),
        crate::OnreError::InvalidMarketStatsData.into(),
    )
}

pub fn write_market_stats_account(
    market_stats_account: &AccountInfo,
    market_stats: &MarketStats,
) -> Result<()> {
    let mut data = market_stats_account.try_borrow_mut_data()?;
    let mut slice: &mut [u8] = &mut data;
    market_stats.try_serialize(&mut slice)
}

pub fn calculate_nav_adjustment(
    offer: &Offer,
    active_vector: crate::instructions::OfferVector,
) -> Result<i64> {
    Ok(get_nav_adjustment_snapshot(offer, &active_vector)
        .map_err(|_| error!(crate::OnreError::Overflow))?
        .adjustment)
}

pub fn calculate_tvl(circulating_supply: u64, nav: u64) -> Result<u64> {
    (circulating_supply as u128)
        .checked_mul(nav as u128)
        .and_then(|result| result.checked_div(10_u128.pow(PRICE_DECIMALS as u32)))
        .and_then(|result| u64::try_from(result).ok())
        .ok_or_else(|| error!(crate::OnreError::Overflow))
}

pub fn calculate_circulating_supply(total_supply: u64, excluded_supply: u64) -> Result<u64> {
    total_supply
        .checked_sub(excluded_supply)
        .ok_or_else(|| error!(crate::OnreError::ArithmeticUnderflow))
}

/// Converts Annual Percentage Rate (APR) to Annual Percentage Yield (APY) using daily compounding
///
/// This function implements the standard financial formula for converting APR to APY
/// with daily compounding (365 periods per year). The calculation uses high-precision
/// fixed-point arithmetic to maintain accuracy across the full range of input values.
///
/// # Mathematical Formula
/// ```text
/// APY = (1 + APR/365)^365 - 1
/// ```
///
/// # Implementation Details
/// - Uses 1e18 internal precision for intermediate calculations
/// - Applies proper rounding for the final result
/// - Employs exponentiation by squaring for efficient power calculation
/// - All operations are checked for overflow protection
///
/// # Arguments
/// * `apr_scaled` - Annual Percentage Rate with scale=6 (1_000_000 = 100%)
///
/// # Returns
/// * `Ok(apy)` - Annual Percentage Yield with scale=6 (same scaling as input)
/// * `Err(crate::OnreError::Overflow)` - If mathematical overflow occurs
/// * `Err(crate::OnreError::DivByZero)` - If division by zero occurs
///
/// # Scale Information
/// Both input and output use scale=6, where 1_000_000 represents 100%:
/// - 10_000 = 1%
/// - 100_000 = 10%
/// - 1_000_000 = 100%
pub fn calculate_apy_from_apr(apr_scaled: u64) -> Result<u64> {
    let apr = apr_scaled as u128;

    // incr = INT_SCALE * (apr / EXT_SCALE) / N
    let num = INT_SCALE
        .checked_mul(apr)
        .ok_or_else(|| error!(crate::OnreError::Overflow))?;
    let den = EXT_SCALE
        .checked_mul(N)
        .ok_or_else(|| error!(crate::OnreError::Overflow))?;
    let incr = num
        .checked_add(den / 2)
        .ok_or_else(|| error!(crate::OnreError::Overflow))?
        .checked_div(den)
        .ok_or_else(|| error!(crate::OnreError::DivByZero))?;

    let base = INT_SCALE
        .checked_add(incr)
        .ok_or_else(|| error!(crate::OnreError::Overflow))?;

    // (1 + r/n)^n at 1e18 precision
    let pow =
        pow_fixed(base, N as u64, INT_SCALE).ok_or_else(|| error!(crate::OnreError::Overflow))?;

    // APY_int = pow - 1.0
    let apy_int = pow
        .checked_sub(INT_SCALE)
        .ok_or_else(|| error!(crate::OnreError::Overflow))?;

    // Convert back to 1e6 scale with rounding: apy_scaled = round(apy_int * EXT_SCALE / INT_SCALE)
    let apy_scaled_u128 = mul_div_round_u128(apy_int, EXT_SCALE, INT_SCALE)
        .ok_or_else(|| error!(crate::OnreError::Overflow))?;

    if apy_scaled_u128 > u64::MAX as u128 {
        return Err(error!(crate::OnreError::Overflow));
    }

    Ok(apy_scaled_u128 as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instructions::OfferVector;

    fn offer_with_vectors(vectors: [OfferVector; crate::constants::MAX_VECTORS]) -> Offer {
        let mut offer: Offer = unsafe { std::mem::zeroed() };
        offer.token_in_mint = Pubkey::new_unique();
        offer.token_out_mint = Pubkey::new_unique();
        offer.vectors = vectors;
        offer.fee_basis_points = 0;
        offer.bump = 0;
        offer
    }

    #[test]
    fn nav_adjustment_negative_transition_matches_programv4() {
        let previous = OfferVector {
            start_time: 100,
            base_time: 100,
            base_price: 2_000_000_000,
            apr: 0,
            price_fix_duration: 60,
        };
        let current = OfferVector {
            start_time: 200,
            base_time: 200,
            base_price: 1_500_000_000,
            apr: 0,
            price_fix_duration: 60,
        };
        let mut vectors = [OfferVector::default(); crate::constants::MAX_VECTORS];
        vectors[0] = previous;
        vectors[1] = current;
        let offer = offer_with_vectors(vectors);

        let adjustment = calculate_nav_adjustment(&offer, current).unwrap();

        assert_eq!(adjustment, -500_000_000);
    }

    #[test]
    fn nav_adjustment_positive_transition_matches_programv4() {
        let previous = OfferVector {
            start_time: 100,
            base_time: 100,
            base_price: 1_000_000_000,
            apr: 0,
            price_fix_duration: 60,
        };
        let current = OfferVector {
            start_time: 200,
            base_time: 200,
            base_price: 1_500_000_000,
            apr: 0,
            price_fix_duration: 60,
        };
        let mut vectors = [OfferVector::default(); crate::constants::MAX_VECTORS];
        vectors[0] = previous;
        vectors[1] = current;
        let offer = offer_with_vectors(vectors);

        let adjustment = calculate_nav_adjustment(&offer, current).unwrap();

        assert_eq!(adjustment, 500_000_000);
    }

    #[test]
    fn nav_adjustment_uses_vector_transition_time() {
        let previous = OfferVector {
            start_time: 100,
            base_time: 100,
            base_price: 1_000_000_000,
            apr: 365_000,
            price_fix_duration: 86_400,
        };
        let current = OfferVector {
            start_time: 200,
            base_time: 200,
            base_price: 1_100_000_000,
            apr: 365_000,
            price_fix_duration: 86_400,
        };
        let mut vectors = [OfferVector::default(); crate::constants::MAX_VECTORS];
        vectors[0] = previous;
        vectors[1] = current;
        let offer = offer_with_vectors(vectors);

        let adjustment = calculate_nav_adjustment(&offer, current).unwrap();

        assert_eq!(adjustment, 100_100_000);
    }

    #[test]
    fn market_stats_update_preserves_positive_nav_adjustment_sign() {
        let mut market_stats = MarketStats {
            apy: 0,
            circulating_supply: 0,
            nav: 0,
            nav_adjustment: 0,
            tvl: 0,
            last_updated_at: 0,
            last_updated_slot: 0,
            bump: 7,
            reserved: [0; 95],
        };

        let snapshot = MarketStatsSnapshot {
            apy: 10,
            circulating_supply: 20,
            nav: 30,
            nav_adjustment: 40,
            tvl: 50,
        };

        let clock = Clock {
            slot: 42,
            epoch_start_timestamp: 0,
            epoch: 0,
            leader_schedule_epoch: 0,
            unix_timestamp: 1_700_000_000,
        };
        apply_market_stats_snapshot(&mut market_stats, snapshot, &clock);

        assert_eq!(market_stats.apy, 10);
        assert_eq!(market_stats.circulating_supply, 20);
        assert_eq!(market_stats.nav, 30);
        assert_eq!(market_stats.nav_adjustment, 40);
        assert_eq!(market_stats.tvl, 50);
        assert_eq!(market_stats.last_updated_slot, 42);
        assert_eq!(market_stats.last_updated_at, 1_700_000_000);
        assert_eq!(market_stats.bump, 7);
    }

    #[test]
    fn market_stats_update_preserves_negative_nav_adjustment_sign() {
        let mut market_stats = MarketStats {
            apy: 0,
            circulating_supply: 0,
            nav: 0,
            nav_adjustment: 0,
            tvl: 0,
            last_updated_at: 0,
            last_updated_slot: 0,
            bump: 7,
            reserved: [0; 95],
        };

        let snapshot = MarketStatsSnapshot {
            apy: 10,
            circulating_supply: 20,
            nav: 30,
            nav_adjustment: -40,
            tvl: 50,
        };

        let clock = Clock {
            slot: 42,
            epoch_start_timestamp: 0,
            epoch: 0,
            leader_schedule_epoch: 0,
            unix_timestamp: 1_700_000_000,
        };
        apply_market_stats_snapshot(&mut market_stats, snapshot, &clock);

        assert_eq!(market_stats.nav_adjustment, -40);
        assert!(market_stats.nav_adjustment.is_negative());
        assert_eq!(market_stats.last_updated_slot, 42);
        assert_eq!(market_stats.last_updated_at, 1_700_000_000);
        assert_eq!(market_stats.bump, 7);
    }

    #[test]
    fn tvl_uses_price_decimals_scale() {
        let tvl = calculate_tvl(2_000_000_000, 1_500_000_000).unwrap();
        assert_eq!(tvl, 3_000_000_000);
    }

    #[test]
    fn tvl_overflow_is_rejected() {
        let err = calculate_tvl(u64::MAX, u64::MAX).unwrap_err();
        assert_eq!(err, error!(crate::OnreError::Overflow));
    }

    #[test]
    fn circulating_supply_matches_programv4_subtraction() {
        let circulating_supply = calculate_circulating_supply(1_000_000_000, 250_000_000).unwrap();
        assert_eq!(circulating_supply, 750_000_000);
    }

    #[test]
    fn circulating_supply_subtracts_cached_excluded_balance() {
        let circulating_supply = calculate_circulating_supply(1_000_000_000, 350_000_000).unwrap();
        assert_eq!(circulating_supply, 650_000_000);
    }

    #[test]
    fn circulating_supply_subtracts_cached_excluded_balance_for_varied_values() {
        for (total_supply, excluded_amount, expected) in [
            (10_000, 0, 10_000),
            (10_000, 1_250, 8_750),
            (5_000_000_000, 2_000_000_001, 2_999_999_999),
            (u64::MAX, 20, u64::MAX - 20),
        ] {
            assert_eq!(
                calculate_circulating_supply(total_supply, excluded_amount).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn circulating_supply_rejects_excluded_supply_underflow() {
        let err = calculate_circulating_supply(1, 2).unwrap_err();
        assert_eq!(err, error!(crate::OnreError::ArithmeticUnderflow));
    }

    #[test]
    fn first_vector_adjustment_matches_current_nav() {
        let current = OfferVector {
            start_time: 100,
            base_time: 100,
            base_price: 1_000_000_000,
            apr: 36_500,
            price_fix_duration: 86_400,
        };
        let mut vectors = [OfferVector::default(); crate::constants::MAX_VECTORS];
        vectors[0] = current;
        let offer = offer_with_vectors(vectors);

        let adjustment = calculate_nav_adjustment(&offer, current).unwrap();

        assert_eq!(adjustment, 1_000_100_000);
    }
}
