use crate::constants::PRICE_DECIMALS;
use crate::instructions::offer::offer_utils::{
    calculate_current_step_price, calculate_step_price_at, find_active_vector_at,
};
use crate::instructions::{Offer, OfferVector};
use crate::OfferCoreError;
use anchor_lang::prelude::*;

pub fn get_active_vector_and_current_price(
    offer: &Offer,
    current_time: u64,
) -> Result<(OfferVector, u64)> {
    let active_vector = find_active_vector_at(offer, current_time)?;
    let current_price = calculate_current_step_price(
        active_vector.apr,
        active_vector.base_price,
        active_vector.base_time,
        active_vector.price_fix_duration,
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

pub fn compute_tvl_from_supply_and_price(token_supply: u64, current_price: u64) -> Option<u64> {
    (token_supply as u128)
        .checked_mul(current_price as u128)
        .and_then(|result| result.checked_div(10_u128.pow(PRICE_DECIMALS as u32)))
        .and_then(|result| (result <= u64::MAX as u128).then_some(result as u64))
}
