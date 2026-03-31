use crate::instructions::buffer::{BufferErrorCode, SECONDS_PER_YEAR, YIELD_SCALE};
use anchor_lang::prelude::*;

pub struct BufferAccrualBreakdown {
    pub buffer_mint_amount: u64,
    pub reserve_mint_amount: u64,
    pub management_fee_mint_amount: u64,
    pub performance_fee_mint_amount: u64,
    pub new_performance_fee_high_watermark: u64,
}

fn calculate_accrual_from_apr_delta(
    previous_supply: u64,
    apr_delta: u64,
    seconds_elapsed: u64,
) -> Result<u64> {
    if apr_delta == 0 || previous_supply == 0 || seconds_elapsed == 0 {
        return Ok(0);
    }

    let mint_amount_u128 = (previous_supply as u128)
        .checked_mul(apr_delta as u128)
        .and_then(|v| v.checked_mul(seconds_elapsed as u128))
        .ok_or(BufferErrorCode::MathOverflow)?
        .checked_div(SECONDS_PER_YEAR)
        .and_then(|v| v.checked_div(YIELD_SCALE))
        .ok_or(BufferErrorCode::MathOverflow)?;

    require!(
        mint_amount_u128 <= u64::MAX as u128,
        BufferErrorCode::ResultOverflow
    );

    Ok(mint_amount_u128 as u64)
}

pub fn calculate_gross_buffer_accrual(
    previous_supply: u64,
    gross_yield: u64,
    current_yield: u64,
    seconds_elapsed: u64,
) -> Result<u64> {
    let apr_delta = gross_yield.saturating_sub(current_yield);
    calculate_accrual_from_apr_delta(previous_supply, apr_delta, seconds_elapsed)
}

pub fn calculate_buffer_fee_split(
    buffer_mint_amount: u64,
    apr_delta: u64,
    management_fee_basis_points: u16,
    performance_fee_basis_points: u16,
    current_nav: u64,
    performance_fee_high_watermark: u64,
) -> Result<BufferAccrualBreakdown> {
    let management_fee_mint_amount = if apr_delta == 0 || buffer_mint_amount == 0 {
        0
    } else {
        let management_fee_apr = (management_fee_basis_points as u64)
            .checked_mul(100)
            .ok_or(BufferErrorCode::MathOverflow)?
        .min(apr_delta);

        let fee_u128 = (buffer_mint_amount as u128)
            .checked_mul(management_fee_apr as u128)
            .ok_or(BufferErrorCode::MathOverflow)?
            .checked_div(apr_delta as u128)
            .ok_or(BufferErrorCode::MathOverflow)?;

        require!(
            fee_u128 <= u64::MAX as u128,
            BufferErrorCode::ResultOverflow
        );
        fee_u128 as u64
    };

    let buffer_mint_amount_after_management = buffer_mint_amount
        .checked_sub(management_fee_mint_amount)
        .ok_or(BufferErrorCode::MathOverflow)?;

    let performance_fee_mint_amount = if current_nav > performance_fee_high_watermark {
        let fee_u128 = (buffer_mint_amount_after_management as u128)
            .checked_mul(performance_fee_basis_points as u128)
            .ok_or(BufferErrorCode::MathOverflow)?
            .checked_div(10_000)
            .ok_or(BufferErrorCode::MathOverflow)?;
        require!(
            fee_u128 <= u64::MAX as u128,
            BufferErrorCode::ResultOverflow
        );
        fee_u128 as u64
    } else {
        0
    };

    let reserve_mint_amount = buffer_mint_amount_after_management
        .checked_sub(performance_fee_mint_amount)
        .ok_or(BufferErrorCode::MathOverflow)?;

    let new_performance_fee_high_watermark = performance_fee_high_watermark.max(current_nav);

    Ok(BufferAccrualBreakdown {
        buffer_mint_amount,
        reserve_mint_amount,
        management_fee_mint_amount,
        performance_fee_mint_amount,
        new_performance_fee_high_watermark,
    })
}
