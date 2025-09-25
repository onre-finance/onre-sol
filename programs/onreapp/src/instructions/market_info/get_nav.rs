use crate::constants::seeds;
use crate::instructions::offer::offer_utils::{
    calculate_current_step_price, find_active_vector_at, find_offer,
};
use crate::instructions::OfferAccount;
use anchor_lang::prelude::*;
use anchor_lang::Accounts;

/// Event emitted when get_NAV is called
#[event]
pub struct GetNAVEvent {
    /// The ID of the offer
    pub offer_id: u64,
    /// Current price for the offer
    pub current_price: u64,
    /// Unix timestamp when the price was calculated
    pub timestamp: u64,
}

/// Accounts required for getting NAV information
#[derive(Accounts)]
pub struct GetNAV<'info> {
    /// The offer account containing all active offers
    #[account(seeds = [seeds::OFFERS], bump)]
    pub offer_account: AccountLoader<'info, OfferAccount>,
}

/// Gets the current NAV (price) for a specific offer
///
/// This instruction allows anyone to query the current price for an offer
/// without making any state modifications. The price is calculated using
/// the existing offer_utils::calculate_current_step_price function.
///
/// # Arguments
///
/// * `ctx` - The instruction context containing required accounts
/// * `offer_id` - The unique ID of the offer to get the price for
///
/// # Returns
///
/// * `Ok(())` - If the price was successfully calculated and emitted
/// * `Err(_)` - If the offer doesn't exist or price calculation fails
///
/// # Emits
///
/// * `GetNAVEvent` - Contains offer_id, current_price, and timestamp
pub fn get_nav(ctx: Context<GetNAV>, offer_id: u64) -> Result<u64> {
    let offer_account = ctx.accounts.offer_account.load()?;
    let current_time = Clock::get()?.unix_timestamp as u64;

    // Find the offer
    let offer = find_offer(&offer_account, offer_id)?;

    // Find the currently active pricing vector
    let active_vector = find_active_vector_at(&offer, current_time)?;

    // Calculate current price with 9 decimals
    let current_price = calculate_current_step_price(
        active_vector.apr,
        active_vector.base_price,
        active_vector.base_time,
        active_vector.price_fix_duration,
    )?;

    msg!(
        "NAV Info - Offer ID: {}, Current Price: {}, Timestamp: {}",
        offer_id,
        current_price,
        current_time
    );

    emit!(GetNAVEvent {
        offer_id,
        current_price,
        timestamp: current_time,
    });

    Ok(current_price)
}
