use crate::constants::PRICE_DECIMALS;
use crate::instructions::market_info::get_apy::calculate_apy_from_apr;
use crate::instructions::market_info::get_nav_adjustment::find_previous_vector;
use crate::instructions::offer::offer_utils::{
    calculate_current_step_price, calculate_step_price_at, find_active_vector_at,
};
use crate::instructions::Offer;
use crate::state::MarketStats;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

/// Error codes for shared market-stats recomputation.
#[error_code]
pub enum MarketStatsErrorCode {
    /// The provided ONyc mint does not match the offer's token_out mint.
    #[msg("Invalid ONyc mint for market stats recomputation")]
    InvalidOnycMint,
    /// The shared TVL computation overflowed.
    #[msg("Math overflow")]
    Overflow,
}

/// Canonical in-memory representation of the derived market stats values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketStatsSnapshot {
    pub apy: u64,
    pub circulating_supply: u64,
    pub nav: u64,
    pub nav_adjustment: i64,
    pub tvl: u64,
}

/// Recomputes the protocol's canonical market stats from the current on-chain state.
///
/// This helper is intended to be shared by multiple instructions so the PDA write path
/// always uses identical business logic for price, supply, and TVL calculations.
pub fn recompute_market_stats(
    offer: &Offer,
    onyc_mint: &InterfaceAccount<Mint>,
    onyc_vault_account: &AccountInfo,
    token_program: &Interface<TokenInterface>,
) -> Result<MarketStatsSnapshot> {
    require_keys_eq!(
        offer.token_out_mint,
        onyc_mint.key(),
        MarketStatsErrorCode::InvalidOnycMint
    );

    let current_time = Clock::get()?.unix_timestamp as u64;
    let active_vector = find_active_vector_at(offer, current_time)?;

    let apy = calculate_apy_from_apr(active_vector.apr)?;
    let nav = calculate_current_step_price(
        active_vector.apr,
        active_vector.base_price,
        active_vector.base_time,
        active_vector.price_fix_duration,
    )?;
    let nav_adjustment = calculate_nav_adjustment(offer, active_vector)?;

    let vault_amount = read_optional_token_account_amount(onyc_vault_account, token_program)?;
    let circulating_supply = calculate_circulating_supply(onyc_mint.supply, vault_amount);
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

pub fn calculate_nav_adjustment(
    offer: &Offer,
    active_vector: crate::instructions::OfferVector,
) -> Result<i64> {
    let current_price = calculate_step_price_at(
        active_vector.apr,
        active_vector.base_price,
        active_vector.base_time,
        active_vector.price_fix_duration,
        active_vector.start_time,
    )?;

    let adjustment = if let Some(previous_vector) =
        find_previous_vector(offer, active_vector.start_time)
    {
        let previous_price = calculate_step_price_at(
            previous_vector.apr,
            previous_vector.base_price,
            previous_vector.base_time,
            previous_vector.price_fix_duration,
            active_vector.start_time,
        )?;

        i64::try_from(current_price)
            .and_then(|current_price| {
                i64::try_from(previous_price).map(|previous_price| current_price - previous_price)
            })
            .map_err(|_| error!(MarketStatsErrorCode::Overflow))?
    } else {
        i64::try_from(current_price).map_err(|_| error!(MarketStatsErrorCode::Overflow))?
    };

    Ok(adjustment)
}

pub fn calculate_tvl(circulating_supply: u64, nav: u64) -> Result<u64> {
    (circulating_supply as u128)
        .checked_mul(nav as u128)
        .and_then(|result| result.checked_div(10_u128.pow(PRICE_DECIMALS as u32)))
        .and_then(|result| u64::try_from(result).ok())
        .ok_or_else(|| error!(MarketStatsErrorCode::Overflow))
}

pub fn calculate_circulating_supply(total_supply: u64, vault_amount: u64) -> u64 {
    total_supply - vault_amount
}

pub fn read_optional_token_account_amount(
    vault_account: &AccountInfo,
    token_program: &Interface<TokenInterface>,
) -> Result<u64> {
    if vault_account.owner != token_program.key {
        return Ok(0);
    }

    if vault_account.data_is_empty() {
        return Ok(0);
    }

    let data_ref = vault_account.data.borrow();
    match TokenAccount::try_deserialize(&mut &data_ref[..]) {
        Ok(parsed) => Ok(parsed.amount),
        Err(_) => Ok(0),
    }
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
        assert_eq!(err, error!(MarketStatsErrorCode::Overflow));
    }

    #[test]
    fn circulating_supply_matches_programv4_subtraction() {
        let circulating_supply = calculate_circulating_supply(1_000_000_000, 250_000_000);
        assert_eq!(circulating_supply, 750_000_000);
    }

    #[test]
    #[should_panic]
    fn circulating_supply_does_not_saturate_underflow() {
        let _ = calculate_circulating_supply(1, 2);
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
