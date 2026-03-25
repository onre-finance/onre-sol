use crate::instructions::offer::offer_utils::{calculate_step_price_at, find_active_vector_at};
use crate::instructions::{Offer, OfferVector};
use crate::OfferCoreError;
use anchor_lang::prelude::*;

pub struct OfferValuationSnapshot {
    pub active_vector: OfferVector,
    pub current_price: u64,
    pub next_price_change_timestamp: u64,
}

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

pub fn get_offer_valuation_snapshot(
    offer: &Offer,
    current_time: u64,
) -> Result<OfferValuationSnapshot> {
    let (active_vector, current_price) = get_active_vector_and_current_price(offer, current_time)?;
    let next_price_change_timestamp =
        compute_next_price_change_timestamp(offer, &active_vector, current_time)?;

    Ok(OfferValuationSnapshot {
        active_vector,
        current_price,
        next_price_change_timestamp,
    })
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

fn compute_next_price_change_timestamp(
    offer: &Offer,
    active_vector: &OfferVector,
    current_time: u64,
) -> Result<u64> {
    let elapsed_since_base = current_time.saturating_sub(active_vector.base_time);
    let current_step = elapsed_since_base / active_vector.price_fix_duration;
    let next_interval_timestamp = active_vector
        .base_time
        .checked_add(
            (current_step + 1)
                .checked_mul(active_vector.price_fix_duration)
                .ok_or(OfferCoreError::OverflowError)?,
        )
        .ok_or(OfferCoreError::OverflowError)?;

    Ok(match find_next_vector_after(offer, current_time) {
        Some(vector) => next_interval_timestamp.min(vector.start_time),
        None => next_interval_timestamp,
    })
}

fn find_next_vector_after(offer: &Offer, current_time: u64) -> Option<OfferVector> {
    offer
        .vectors
        .iter()
        .filter(|vector| vector.start_time > current_time)
        .min_by_key(|vector| vector.start_time)
        .copied()
}
