use crate::instructions::buffer::{
    BASIS_POINTS_SCALE, BASIS_POINT_TO_YIELD_SCALE, SECONDS_PER_YEAR, YIELD_SCALE,
};
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
    current_yield: u64,
    seconds_elapsed: u64,
) -> Result<u64> {
    if apr_delta == 0 || previous_supply == 0 || seconds_elapsed == 0 {
        return Ok(0);
    }

    let target_nav_growth_denominator = SECONDS_PER_YEAR
        .checked_mul(YIELD_SCALE)
        .and_then(|v| v.checked_add((current_yield as u128).checked_mul(seconds_elapsed as u128)?))
        .ok_or(crate::OnreError::MathOverflow)?;

    let mint_amount_u128 = (previous_supply as u128)
        .checked_mul(apr_delta as u128)
        .and_then(|v| v.checked_mul(seconds_elapsed as u128))
        .ok_or(crate::OnreError::MathOverflow)?
        .checked_div(target_nav_growth_denominator)
        .ok_or(crate::OnreError::MathOverflow)?;

    require!(
        mint_amount_u128 <= u64::MAX as u128,
        crate::OnreError::ResultOverflow
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
    calculate_accrual_from_apr_delta(previous_supply, apr_delta, current_yield, seconds_elapsed)
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
            .checked_mul(BASIS_POINT_TO_YIELD_SCALE as u64)
            .ok_or(crate::OnreError::MathOverflow)?
            .min(apr_delta);

        let fee_u128 = (buffer_mint_amount as u128)
            .checked_mul(management_fee_apr as u128)
            .ok_or(crate::OnreError::MathOverflow)?
            .checked_div(apr_delta as u128)
            .ok_or(crate::OnreError::MathOverflow)?;

        require!(
            fee_u128 <= u64::MAX as u128,
            crate::OnreError::ResultOverflow
        );
        fee_u128 as u64
    };

    let buffer_mint_amount_after_management = buffer_mint_amount
        .checked_sub(management_fee_mint_amount)
        .ok_or(crate::OnreError::MathOverflow)?;

    // NAV is stepwise-constant over each `price_fix_duration` interval. Using `>=`
    // ensures performance fees apply for an interval whose stepped NAV is exactly at
    // the stored watermark, rather than skipping that entire fixed-price window and
    // only starting once NAV jumps strictly above it in a later step.
    let performance_fee_mint_amount =
        if performance_fee_high_watermark != 0 && current_nav >= performance_fee_high_watermark {
            let fee_u128 = (buffer_mint_amount_after_management as u128)
                .checked_mul(performance_fee_basis_points as u128)
                .ok_or(crate::OnreError::MathOverflow)?
                .checked_div(BASIS_POINTS_SCALE)
                .ok_or(crate::OnreError::MathOverflow)?;
            require!(
                fee_u128 <= u64::MAX as u128,
                crate::OnreError::ResultOverflow
            );
            fee_u128 as u64
        } else {
            0
        };

    let reserve_mint_amount = buffer_mint_amount_after_management
        .checked_sub(performance_fee_mint_amount)
        .ok_or(crate::OnreError::MathOverflow)?;

    let new_performance_fee_high_watermark = performance_fee_high_watermark.max(current_nav);

    Ok(BufferAccrualBreakdown {
        buffer_mint_amount,
        reserve_mint_amount,
        management_fee_mint_amount,
        performance_fee_mint_amount,
        new_performance_fee_high_watermark,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const HALF_YEAR_SECONDS: u64 = SECONDS_PER_YEAR as u64 / 2;

    #[test]
    fn calculate_gross_buffer_accrual_returns_zero_for_zero_inputs() {
        assert_eq!(
            calculate_gross_buffer_accrual(0, 150_000, 50_000, HALF_YEAR_SECONDS).unwrap(),
            0
        );
        assert_eq!(
            calculate_gross_buffer_accrual(1_000_000_000, 50_000, 50_000, HALF_YEAR_SECONDS)
                .unwrap(),
            0
        );
        assert_eq!(
            calculate_gross_buffer_accrual(1_000_000_000, 150_000, 50_000, 0).unwrap(),
            0
        );
    }

    #[test]
    fn calculate_gross_buffer_accrual_preserves_zero_current_yield_behavior() {
        assert_eq!(
            calculate_gross_buffer_accrual(1_000_000_000, 100_000, 0, HALF_YEAR_SECONDS).unwrap(),
            50_000_000
        );
    }

    #[test]
    fn calculate_gross_buffer_accrual_discounts_by_target_nav_growth() {
        let cases = [
            (
                1_000_000_000,
                150_000,
                50_000,
                HALF_YEAR_SECONDS,
                50_000_000,
                48_780_487,
                1_219_513,
            ),
            (
                1_000_000_000,
                120_000,
                20_000,
                SECONDS_PER_YEAR as u64,
                100_000_000,
                98_039_215,
                1_960_785,
            ),
            (
                2_500_000_000,
                300_000,
                100_000,
                SECONDS_PER_YEAR as u64 / 4,
                125_000_000,
                121_951_219,
                3_048_781,
            ),
            (
                1_000_000_000,
                80_000,
                60_000,
                2_592_000,
                1_643_835,
                1_635_768,
                8_067,
            ),
        ];

        for (
            previous_supply,
            gross_yield,
            current_yield,
            seconds_elapsed,
            old_flat_mint_amount,
            expected_discounted_mint_amount,
            expected_overmint_amount,
        ) in cases
        {
            let actual = calculate_gross_buffer_accrual(
                previous_supply,
                gross_yield,
                current_yield,
                seconds_elapsed,
            )
            .unwrap();

            assert_eq!(actual, expected_discounted_mint_amount);
            assert_eq!(
                old_flat_mint_amount - actual,
                expected_overmint_amount,
                "old flat formula overmint should match the precise dilution amount"
            );
        }
    }

    #[test]
    fn calculate_buffer_fee_split_charges_performance_fee_at_high_watermark_boundary() {
        let split = calculate_buffer_fee_split(10_000, 100_000, 100, 2_000, 1_000, 1_000).unwrap();

        assert_eq!(split.buffer_mint_amount, 10_000);
        assert_eq!(split.management_fee_mint_amount, 1_000);
        assert_eq!(split.performance_fee_mint_amount, 1_800);
        assert_eq!(split.reserve_mint_amount, 7_200);
        assert_eq!(split.new_performance_fee_high_watermark, 1_000);
    }

    #[test]
    fn calculate_buffer_fee_split_seeds_uninitialized_high_watermark_without_performance_fee() {
        let split = calculate_buffer_fee_split(10_000, 100_000, 100, 2_000, 1_000, 0).unwrap();

        assert_eq!(split.buffer_mint_amount, 10_000);
        assert_eq!(split.management_fee_mint_amount, 1_000);
        assert_eq!(split.performance_fee_mint_amount, 0);
        assert_eq!(split.reserve_mint_amount, 9_000);
        assert_eq!(split.new_performance_fee_high_watermark, 1_000);
    }

    #[test]
    fn calculate_buffer_fee_split_skips_performance_fee_below_high_watermark() {
        let split = calculate_buffer_fee_split(10_000, 100_000, 0, 2_000, 999, 1_000).unwrap();

        assert_eq!(split.performance_fee_mint_amount, 0);
        assert_eq!(split.reserve_mint_amount, 10_000);
        assert_eq!(split.new_performance_fee_high_watermark, 1_000);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_gross_buffer_accrual_returns_zero_for_zero_inputs() {
        assert_eq!(calculate_gross_buffer_accrual(0, 100_000, 0, 31_536_000).unwrap(), 0);
        assert_eq!(
            calculate_gross_buffer_accrual(1_000_000_000, 100_000, 100_000, 31_536_000).unwrap(),
            0
        );
        assert_eq!(
            calculate_gross_buffer_accrual(1_000_000_000, 100_000, 0, 0).unwrap(),
            0
        );
    }

    #[test]
    fn calculate_buffer_fee_split_skips_performance_fee_below_high_watermark() {
        let breakdown = calculate_buffer_fee_split(
            100_000_000,
            100_000,
            100,
            1_000,
            900_000_000,
            1_000_000_000,
        )
        .unwrap();

        assert_eq!(breakdown.management_fee_mint_amount, 10_000_000);
        assert_eq!(breakdown.performance_fee_mint_amount, 0);
        assert_eq!(breakdown.reserve_mint_amount, 90_000_000);
        assert_eq!(breakdown.new_performance_fee_high_watermark, 1_000_000_000);
    }

    #[test]
    fn calculate_buffer_fee_split_charges_performance_fee_at_high_watermark_boundary() {
        let breakdown = calculate_buffer_fee_split(
            100_000_000,
            100_000,
            100,
            1_000,
            1_000_000_000,
            1_000_000_000,
        )
        .unwrap();

        assert_eq!(breakdown.management_fee_mint_amount, 10_000_000);
        assert_eq!(breakdown.performance_fee_mint_amount, 9_000_000);
        assert_eq!(breakdown.reserve_mint_amount, 81_000_000);
        assert_eq!(breakdown.new_performance_fee_high_watermark, 1_000_000_000);
    }
}
