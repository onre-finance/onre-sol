use anchor_lang::prelude::*;

use crate::state::Offer;
use crate::instructions::take_offer::TakeOfferErrorCode;

/// Calculates the current sell token amount based on the offer's dynamic pricing model.
///
/// The offer is divided into intervals, each lasting `price_fix_duration` seconds.
/// The sell token amount starts at `sell_token_start_amount` + one interval increment at
/// the beginning of the first interval and progresses towards `sell_token_end_amount`
/// by the end of the last interval.
pub fn calculate_current_sell_amount(offer: &Offer) -> Result<u64> {
    let current_time = Clock::get()?.unix_timestamp as u64;

    require!(
        current_time >= offer.offer_start_time && current_time < offer.offer_end_time,
        TakeOfferErrorCode::InvalidCurrentTime
    );

    let total_duration = offer
        .offer_end_time
        .checked_sub(offer.offer_start_time)
        .unwrap();
    let number_of_intervals = total_duration
        .checked_div(offer.price_fix_duration)
        .unwrap();
    let current_interval = current_time
        .checked_sub(offer.offer_start_time)
        .unwrap()
        .checked_div(offer.price_fix_duration)
        .unwrap();

    let sell_token_amount_per_interval = offer
        .sell_token_end_amount
        .checked_sub(offer.sell_token_start_amount)
        .unwrap()
        .checked_div(number_of_intervals)
        .unwrap();

    let sell_token_current_amount = offer
        .sell_token_start_amount
        .checked_add(
            sell_token_amount_per_interval
                .checked_mul(current_interval + 1)
                .unwrap(),
        )
        .unwrap();

    Ok(sell_token_current_amount)
}

pub fn calculate_nav(offer: &Offer) -> Result<u64> {
    let current_sell_token_amount = calculate_current_sell_amount(offer)? as u128;
    let buy_token_one_amount = offer.buy_token_1.amount as u128;

    let nav = current_sell_token_amount
        .checked_div(buy_token_one_amount)
        .unwrap() as u64;

    Ok(nav)
}



