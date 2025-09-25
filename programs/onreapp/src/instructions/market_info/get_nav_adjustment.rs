use crate::constants::seeds;
use crate::instructions::offer::offer_utils::{
    find_active_vector_at, find_offer,
};
use crate::instructions::{calculate_step_price_at, OfferAccount};
use anchor_lang::prelude::*;
use anchor_lang::Accounts;

/// Event emitted when get_nav_adjustment is called
#[event]
pub struct GetNavAdjustmentEvent {
    /// The ID of the offer
    pub offer_id: u64,
    /// Current price from the active vector
    pub current_price: u64,
    /// Previous price from the previous vector (if any)
    pub previous_price: Option<u64>,
    /// Price adjustment (current - previous), signed value
    pub adjustment: i64,
    /// Unix timestamp when the adjustment was calculated
    pub timestamp: u64,
}

/// Accounts required for getting NAV adjustment information
#[derive(Accounts)]
pub struct GetNavAdjustment<'info> {
    /// The offer account containing all active offers
    #[account(seeds = [seeds::OFFERS], bump)]
    pub offer_account: AccountLoader<'info, OfferAccount>,
}

/// Finds the previous vector for an offer based on the currently active vector
///
/// # Arguments
/// * `offer` - The offer to search for the previous vector
/// * `current_vector_start_time` - Start time of the currently active vector
///
/// # Returns
/// The previous `OfferVector` or None if no previous vector exists
pub fn find_previous_vector(
    offer: &crate::instructions::Offer,
    current_vector_start_time: u64,
) -> Option<crate::instructions::OfferVector> {
    offer
        .vectors
        .iter()
        .filter(|vector| vector.vector_id != 0) // Only consider non-empty vectors
        .filter(|vector| vector.start_time < current_vector_start_time) // Only vectors before current
        .max_by_key(|vector| vector.start_time) // Find latest start_time before current
        .copied()
}

/// Gets the current NAV adjustment for a specific offer
///
/// This instruction calculates the price difference between the current vector
/// and the previous vector at the current time. It returns a signed integer
/// representing the price change (current - previous).
///
/// # Arguments
///
/// * `ctx` - The instruction context containing required accounts
/// * `offer_id` - The unique ID of the offer to get the adjustment for
///
/// # Returns
///
/// * `Ok(adjustment)` - The signed price adjustment value
/// * `Err(_)` - If the offer doesn't exist or calculation fails
///
/// # Emits
///
/// * `GetNavAdjustmentEvent` - Contains offer_id, prices, adjustment, and timestamp
pub fn get_nav_adjustment(ctx: Context<GetNavAdjustment>, offer_id: u64) -> Result<i64> {
    let offer_account = ctx.accounts.offer_account.load()?;
    let current_time = Clock::get()?.unix_timestamp as u64;

    // Find the offer
    let offer = find_offer(&offer_account, offer_id)?;

    // Find the currently active pricing vector
    let active_vector = find_active_vector_at(&offer, current_time)?;

    // Calculate price at the start of the active vector
    let current_price = calculate_step_price_at(
        active_vector.apr,
        active_vector.base_price,
        active_vector.base_time,
        active_vector.price_fix_duration,
        active_vector.start_time,
    )?;

    // Find the previous vector and calculate its price
    let (previous_price_opt, adjustment) =
        if let Some(previous_vector) = find_previous_vector(&offer, active_vector.start_time) {
            // Calculate the price of the previous vector at its end time (when current vector starts)
            let previous_price = calculate_step_price_at(
                previous_vector.apr,
                previous_vector.base_price,
                previous_vector.base_time,
                previous_vector.price_fix_duration,
                active_vector.start_time, // End time of previous vector
            )?;

            // Calculate adjustment: current - previous
            let adjustment = if current_price >= previous_price {
                (current_price - previous_price) as i64
            } else {
                -((previous_price - current_price) as i64)
            };

            (Some(previous_price), adjustment)
        } else {
            // No previous vector, so adjustment is the current price (compared to 0)
            (None, current_price as i64)
        };

    msg!(
        "NAV Adjustment Info - Offer ID: {}, Current Price: {}, Previous Price: {:?}, Adjustment: {}, Timestamp: {}",
        offer_id,
        current_price,
        previous_price_opt,
        adjustment,
        current_time
    );

    emit!(GetNavAdjustmentEvent {
        offer_id,
        current_price,
        previous_price: previous_price_opt,
        adjustment,
        timestamp: current_time,
    });

    Ok(adjustment)
}
