//! Price selection and arithmetic over vector data, independent of account layouts.

use crate::{
    model::PricingVector,
    utils::{mul_div_round_u128, pow_fixed},
    OnreError,
};
use anchor_lang::prelude::*;

const INT_SCALE: u128 = 1_000_000_000_000_000_000;
pub(crate) const SECONDS_IN_DAY: u64 = 86_400;
const APR_SCALE: u128 = 1_000_000;

/// Select the latest started vector. Zero start time denotes an empty slot;
/// vectors may contain gaps and need not be sorted. No allocation or count is needed.
pub fn find_active_vector_at(
    vectors: impl IntoIterator<Item = PricingVector>,
    time: u64,
) -> Result<PricingVector> {
    vectors
        .into_iter()
        .filter(|vector| vector.start_time != 0 && vector.start_time <= time)
        .max_by_key(|vector| vector.start_time)
        .ok_or_else(|| error!(OnreError::NoActiveVector))
}

/// Select a vector and calculate its stepped price at the supplied timestamp.
/// Callers supply vector values directly, adapting their storage representation
/// at the boundary; pricing does not load or inspect Offer or Pricer accounts.
pub fn get_active_vector_and_current_price(
    vectors: impl IntoIterator<Item = PricingVector>,
    time: u64,
) -> Result<(PricingVector, u64)> {
    let vector = find_active_vector_at(vectors, time)?;
    let price = calculate_step_price_at(
        vector.apr,
        vector.base_price,
        vector.base_time,
        vector.price_fix_duration,
        time,
    )?;
    require!(price > 0, OnreError::ZeroPriceNotAllowed);
    Ok((vector, price))
}

/// Calculates price growth using daily compounding with linear intra-day interpolation
///
/// The annual percentage rate is compounded daily, matching the APY semantics
/// used by market info. For elapsed times that are not an exact number of days,
/// the remaining fractional day is linearly interpolated between the current
/// daily-compounded price and the next daily-compounded price.
///
/// Formula:
///   daily_factor = 1 + apr / (APR_SCALE * 365)
///   full_day_price = base_price * daily_factor^(elapsed_seconds / 86400)
///   price = full_day_price + daily_delta * (elapsed_seconds % 86400) / 86400
///
/// # Arguments
/// * `apr` - Annual Percentage Rate with scale=6 (10_000 = 1%, 1_000_000 = 100%)
/// * `base_price` - Starting price with scale=9
/// * `elapsed_time` - Time elapsed since base_time in seconds
///
/// # Returns
/// * `Ok(u64)` - Calculated price with same scale as base_price
/// * `Err(crate::OnreError::OverflowError)` - If arithmetic overflow occurs
pub fn calculate_vector_price(apr: u64, base_price: u64, elapsed_time: u64) -> Result<u64> {
    if apr == 0 || elapsed_time == 0 {
        return Ok(base_price);
    }

    let daily_increment = INT_SCALE
        .checked_mul(apr as u128)
        .ok_or(crate::OnreError::OverflowError)?
        .checked_add((APR_SCALE * 365) / 2)
        .ok_or(crate::OnreError::OverflowError)?
        .checked_div(APR_SCALE * 365)
        .ok_or(crate::OnreError::OverflowError)?;

    let daily_factor = INT_SCALE
        .checked_add(daily_increment)
        .ok_or(crate::OnreError::OverflowError)?;

    let full_days = elapsed_time / SECONDS_IN_DAY;
    let remaining_seconds = elapsed_time % SECONDS_IN_DAY;

    let full_day_factor =
        pow_fixed(daily_factor, full_days, INT_SCALE).ok_or(crate::OnreError::OverflowError)?;
    let full_day_price = mul_div_round_u128(base_price as u128, full_day_factor, INT_SCALE)
        .ok_or(crate::OnreError::OverflowError)?;

    if remaining_seconds == 0 {
        if full_day_price > u64::MAX as u128 {
            return Err(error!(crate::OnreError::OverflowError));
        }

        return Ok(full_day_price as u64);
    }

    let next_day_price = mul_div_round_u128(full_day_price, daily_factor, INT_SCALE)
        .ok_or(crate::OnreError::OverflowError)?;
    let daily_delta = next_day_price
        .checked_sub(full_day_price)
        .ok_or(crate::OnreError::OverflowError)?;
    let partial_day_delta = mul_div_round_u128(
        daily_delta,
        remaining_seconds as u128,
        SECONDS_IN_DAY as u128,
    )
    .ok_or(crate::OnreError::OverflowError)?;
    let price = full_day_price
        .checked_add(partial_day_delta)
        .ok_or(crate::OnreError::OverflowError)?;

    if price > u64::MAX as u128 {
        return Err(error!(crate::OnreError::OverflowError));
    }

    Ok(price as u64)
}

/// Calculates discrete step price at a specific time
///
/// Internal helper function that calculates the step price at any given time
/// using the discrete interval pricing model.
///
/// # Arguments
/// * `apr` - Annual Percentage Rate scaled by 1_000_000
/// * `base_price` - Starting price with scale=9
/// * `base_time` - Unix timestamp when pricing vector starts
/// * `price_fix_duration` - Duration of each discrete price interval in seconds
/// * `time` - Specific time to calculate price for
///
/// # Returns
/// * `Ok(u64)` - Price at the specified time
/// * `Err(_)` - If calculation fails or time is invalid
pub fn calculate_step_price_at(
    apr: u64,
    base_price: u64,
    base_time: u64,
    price_fix_duration: u64,
    time: u64,
) -> Result<u64> {
    require!(base_time <= time, crate::OnreError::NoActiveVector);
    require!(
        price_fix_duration > 0,
        crate::OnreError::InvalidPriceFixDuration
    );

    let elapsed_since_start = time.saturating_sub(base_time);

    // Calculate which price interval we're in (discrete intervals)
    let current_step = elapsed_since_start / price_fix_duration;

    // elapsed_effective = (k + 1) * D  (end-of-current-interval snap)
    let step_end_time = current_step
        .checked_add(1)
        .ok_or(crate::OnreError::OverflowError)?
        .checked_mul(price_fix_duration)
        .ok_or(crate::OnreError::OverflowError)?;

    // Use the vector price calculation with the effective elapsed time
    calculate_vector_price(apr, base_price, step_end_time)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instructions::OfferVector;

    fn vector(start_time: u64, base_price: u64) -> PricingVector {
        PricingVector {
            start_time,
            base_time: start_time,
            base_price,
            apr: 0,
            price_fix_duration: SECONDS_IN_DAY,
        }
    }

    #[test]
    fn selects_latest_started_vector_across_unsorted_data_and_empty_slots() {
        let vectors = [
            vector(20, 2_000_000_000),
            PricingVector::default(),
            vector(10, 1_000_000_000),
            vector(30, 3_000_000_000),
        ];
        for (time, start, price) in [
            (19, 10, 1_000_000_000),
            (20, 20, 2_000_000_000),
            (29, 20, 2_000_000_000),
            (30, 30, 3_000_000_000),
        ] {
            let (active, actual_price) =
                get_active_vector_and_current_price(vectors, time).unwrap();
            assert_eq!(active.start_time, start);
            assert_eq!(actual_price, price);
        }
    }

    #[test]
    fn empty_and_future_only_data_have_no_active_vector() {
        for vectors in [
            [PricingVector::default(), PricingVector::default()],
            [PricingVector::default(), vector(20, 1_000_000_000)],
        ] {
            let error = get_active_vector_and_current_price(vectors, 10).unwrap_err();
            assert_eq!(error, error!(OnreError::NoActiveVector));
        }
    }

    #[test]
    fn legacy_and_model_vector_data_use_the_same_compounding_and_step_boundaries() {
        let model = PricingVector {
            apr: 97_600,
            ..vector(10, 1_000_000_000)
        };
        let legacy = OfferVector::from(model);
        assert_eq!(PricingVector::from(&legacy), model);
        for (time, expected_price) in [
            (10, 1_000_267_397),
            (10 + SECONDS_IN_DAY - 1, 1_000_267_397),
            (10 + SECONDS_IN_DAY, 1_000_534_866),
        ] {
            let result = get_active_vector_and_current_price([model], time).unwrap();
            let adapted = get_active_vector_and_current_price(
                [&legacy].into_iter().map(PricingVector::from),
                time,
            )
            .unwrap();
            assert_eq!(result, adapted);
            assert_eq!(result.1, expected_price);
        }
    }

    #[test]
    fn invalid_vector_data_returns_an_error_instead_of_dividing_by_zero() {
        let invalid = PricingVector {
            price_fix_duration: 0,
            ..vector(10, 1_000_000_000)
        };
        let error = get_active_vector_and_current_price([invalid], 10).unwrap_err();
        assert_eq!(error, error!(OnreError::InvalidPriceFixDuration));
    }
}
