//! Token-wide valuation for refill and symmetric curve hard-wall targets.
use super::TakeOffer;
use crate::{
    constants::{MAX_BASIS_POINTS, PRICE_DECIMALS},
    instructions::market_info::{
        calculate_circulating_supply, load_circulating_supply_excluded_balance_amount,
        load_main_offer, offer_valuation_utils::compute_offer_current_price,
    },
    model::{seeds::PRICER, PricingDenomination},
    utils::pricing::get_active_vector_and_current_price,
    OnreError, ID,
};
use anchor_lang::prelude::*;

/// Asset base units. Uses actual mint supply at the time of invocation, canonical
/// token-wide NAV, and ONyc's existing cached excluded balance. Other managed
/// mints currently have no executable exclusion configuration and use full supply.
pub(super) fn target(a: &TakeOffer, bps: u16, now: u64) -> Result<u128> {
    require!(bps <= MAX_BASIS_POINTS, OnreError::InvalidAmount);
    if bps == 0 {
        return Ok(0);
    }
    let (supply, nav) = if a.managed_mint().key() == a.state.onyc_mint {
        let offer = load_main_offer(
            &ID,
            a.main_offer.as_ref().ok_or(OnreError::InvalidMainOffer)?,
            &a.state,
        )?;
        let excluded = load_circulating_supply_excluded_balance_amount(
            &ID,
            &a.circulating_supply_excluded_balance.to_account_info(),
        )?;
        (
            calculate_circulating_supply(a.managed_mint().supply, excluded)?,
            compute_offer_current_price(&offer, now)?,
        )
    } else {
        let canonical = Pubkey::find_program_address(
            &[
                PRICER,
                a.managed_mint().key().as_ref(),
                &[PricingDenomination::Usd as u8],
                &0_u64.to_le_bytes(),
            ],
            &ID,
        )
        .0;
        let pricer = if a.pricer.key() == canonical {
            &a.pricer
        } else {
            a.buffer_pricer
                .as_ref()
                .ok_or(OnreError::IncompatiblePricer)?
        };
        require!(
            pricer.key() == canonical
                && pricer.enabled
                && pricer.managed_mint == a.managed_mint().key()
                && pricer.denomination == PricingDenomination::Usd,
            OnreError::IncompatiblePricer
        );
        (
            a.managed_mint().supply,
            get_active_vector_and_current_price(pricer.vectors.iter().copied(), now)?.1,
        )
    };
    asset_target(
        supply,
        nav,
        bps,
        a.managed_mint().decimals,
        a.asset_mint().decimals,
    )
}

fn asset_target(
    supply: u64,
    nav: u64,
    bps: u16,
    managed_decimals: u8,
    asset_decimals: u8,
) -> Result<u128> {
    let tvl = supply as u128 * nav as u128 / 10_u128.pow(PRICE_DECIMALS as u32);
    let mut numerator = tvl
        .checked_mul(bps as u128)
        .ok_or(OnreError::MathOverflow)?;
    let mut denominator = MAX_BASIS_POINTS as u128;
    if asset_decimals >= managed_decimals {
        numerator = numerator
            .checked_mul(10_u128.pow((asset_decimals - managed_decimals) as u32))
            .ok_or(OnreError::MathOverflow)?;
    } else {
        denominator = denominator
            .checked_mul(10_u128.pow((managed_decimals - asset_decimals) as u32))
            .ok_or(OnreError::MathOverflow)?;
    }
    Ok(numerator / denominator)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn targets_convert_decimals_and_preserve_evm_rounding() {
        assert_eq!(
            asset_target(1_000_000_000, 2_000_000_000, 1000, 9, 6).unwrap(),
            200_000
        );
        assert_eq!(
            asset_target(1_000_000, 2_000_000_000, 1000, 6, 9).unwrap(),
            200_000_000
        );
        assert_eq!(asset_target(1999, 1_000_000_000, 5000, 3, 0).unwrap(), 0);
    }
}
