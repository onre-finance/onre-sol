use crate::instructions::cache::{CacheErrorCode, SECONDS_PER_YEAR, YIELD_SCALE};
use crate::utils::mul_basis_points_floor;
use anchor_lang::prelude::*;

pub struct CacheAccrualBreakdown {
    pub gross_mint_amount: u64,
    pub cache_mint_amount: u64,
    pub management_fee_mint_amount: u64,
    pub performance_fee_mint_amount: u64,
    pub new_performance_fee_high_watermark: u64,
}

pub fn calculate_gross_cache_accrual(
    lowest_supply: u64,
    gross_yield: u64,
    current_yield: u64,
    seconds_elapsed: u64,
) -> Result<u64> {
    let spread = gross_yield.saturating_sub(current_yield);

    if spread == 0 || lowest_supply == 0 || seconds_elapsed == 0 {
        return Ok(0);
    }

    let mint_amount_u128 = (lowest_supply as u128)
        .checked_mul(spread as u128)
        .and_then(|v| v.checked_mul(seconds_elapsed as u128))
        .ok_or(CacheErrorCode::MathOverflow)?
        .checked_div(SECONDS_PER_YEAR)
        .and_then(|v| v.checked_div(YIELD_SCALE))
        .ok_or(CacheErrorCode::MathOverflow)?;

    require!(
        mint_amount_u128 <= u64::MAX as u128,
        CacheErrorCode::ResultOverflow
    );

    Ok(mint_amount_u128 as u64)
}

pub fn calculate_cache_fee_split(
    gross_mint_amount: u64,
    management_fee_basis_points: u16,
    performance_fee_basis_points: u16,
    cache_balance_before_mint: u64,
    performance_fee_high_watermark: u64,
) -> Result<CacheAccrualBreakdown> {
    let management_fee_mint_amount =
        mul_basis_points_floor(gross_mint_amount, management_fee_basis_points)
            .ok_or(CacheErrorCode::MathOverflow)?;

    let gross_mint_after_management = gross_mint_amount
        .checked_sub(management_fee_mint_amount)
        .ok_or(CacheErrorCode::MathOverflow)?;

    let cache_balance_after_management = cache_balance_before_mint
        .checked_add(gross_mint_after_management)
        .ok_or(CacheErrorCode::MathOverflow)?;

    let performance_profit =
        cache_balance_after_management.saturating_sub(performance_fee_high_watermark);
    let performance_fee_mint_amount =
        mul_basis_points_floor(performance_profit, performance_fee_basis_points)
            .ok_or(CacheErrorCode::MathOverflow)?
            .min(gross_mint_after_management);

    let cache_mint_amount = gross_mint_after_management
        .checked_sub(performance_fee_mint_amount)
        .ok_or(CacheErrorCode::MathOverflow)?;

    let new_performance_fee_high_watermark = performance_fee_high_watermark.max(
        cache_balance_before_mint
            .checked_add(cache_mint_amount)
            .ok_or(CacheErrorCode::MathOverflow)?,
    );

    Ok(CacheAccrualBreakdown {
        gross_mint_amount,
        cache_mint_amount,
        management_fee_mint_amount,
        performance_fee_mint_amount,
        new_performance_fee_high_watermark,
    })
}
