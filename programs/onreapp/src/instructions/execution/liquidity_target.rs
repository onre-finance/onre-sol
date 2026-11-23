//! Token-wide valuation for refill and symmetric curve hard-wall targets.
use super::Execution;
use crate::{
    constants::{MAX_BASIS_POINTS, PRICE_DECIMALS},
    instructions::market_info::{calculate_circulating_supply, excluded_supply_amount},
    model::Pricer,
    utils::{
        managed_token::{canonical_usd_pricer_address, require_usd_pricer_for},
        pricing::get_active_vector_and_current_price,
    },
    OnreError, ID,
};
use anchor_lang::prelude::*;

/// The offer's Pricer when it is the canonical USD instance zero, otherwise the
/// separately supplied `buffer_pricer`. Derives the canonical address once.
pub(super) fn canonical_pricer<'a, 'info>(
    a: &Execution<'a, 'info>,
) -> Result<&'a Account<'info, Pricer>> {
    let mint = a.managed_mint().key();
    let canonical = canonical_usd_pricer_address(&ID, &mint);
    let pricer = if a.config.pricer.key() == canonical {
        &a.config.pricer
    } else {
        a.buffer
            .buffer_pricer
            .as_ref()
            .ok_or(OnreError::IncompatiblePricer)?
    };
    require_keys_eq!(pricer.key(), canonical, OnreError::IncompatiblePricer);
    require_usd_pricer_for(&mint, pricer)?;
    Ok(pricer)
}

/// Asset base units. Uses actual mint supply at the time of invocation and the
/// canonical token-wide NAV, minus the mint's cached `supply_exclusions` amount.
pub(super) fn target(a: &Execution, bps: u16, now: u64) -> Result<u128> {
    require!(bps <= MAX_BASIS_POINTS, OnreError::InvalidAmount);
    if bps == 0 {
        return Ok(0);
    }
    let nav =
        get_active_vector_and_current_price(canonical_pricer(a)?.vectors.iter().copied(), now)?.1;
    let excluded = excluded_supply_amount(
        &ID,
        a.state,
        a.managed_mint(),
        &a.buffer.supply_exclusions.to_account_info(),
    )?;
    let supply = calculate_circulating_supply(a.managed_mint().supply, excluded)?;
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
