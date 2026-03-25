use crate::instructions::offer::offer_utils::{calculate_step_price_at, find_active_vector_at};
use crate::instructions::{Offer, OfferVector};
use crate::OfferCoreError;
use anchor_lang::prelude::*;

pub fn get_active_vector_and_current_price(
    offer: &Offer,
    current_time: u64,
) -> Result<(OfferVector, u64)> {
    let active_vector = find_active_vector_at(offer, current_time)?;
    let current_price = calculate_step_price_at(
        active_vector.apr,
        active_vector.base_price,
        active_vector.base_time,
        active_vector.price_fix_duration,
        current_time,
    )?;

    Ok((active_vector, current_price))
}

pub fn compute_offer_current_price(offer: &Offer, current_time: u64) -> Result<u64> {
    let (_, current_price) = get_active_vector_and_current_price(offer, current_time)?;
    Ok(current_price)
}

pub fn compute_vector_price_at_time(vector: &OfferVector, at_time: u64) -> Result<u64> {
    calculate_step_price_at(
        vector.apr,
        vector.base_price,
        vector.base_time,
        vector.price_fix_duration,
        at_time,
    )
}

pub fn compute_signed_price_delta(current_price: u64, previous_price: u64) -> Result<i64> {
    let current = current_price as i128;
    let previous = previous_price as i128;
    let delta = current
        .checked_sub(previous)
        .ok_or(OfferCoreError::OverflowError)?;

    i64::try_from(delta).map_err(|_| error!(OfferCoreError::OverflowError))
}
