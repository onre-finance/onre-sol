use crate::constants::seeds;
use crate::instructions::offer::offer_utils::{
    calculate_current_step_price, find_active_vector_at,
};
use crate::instructions::Offer;
use crate::OfferCoreError;
use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use anchor_spl::token_interface::Mint;

/// Event emitted when get_NAV is called
#[event]
pub struct GetNAVEvent {
    /// The PDA of the offer
    pub offer_pda: Pubkey,
    /// Current price for the offer
    pub current_price: u64,
    /// Unix timestamp when the price was calculated
    pub timestamp: u64,
}

/// Accounts required for getting NAV information
#[derive(Accounts)]
pub struct GetNAV<'info> {
    /// The individual offer account
    #[account(
        seeds = [
            seeds::OFFER,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref()
        ],
        bump = offer.load()?.bump
    )]
    pub offer: AccountLoader<'info, Offer>,

    #[account(
        constraint =
            token_in_mint.key() == offer.load()?.token_in_mint
            @ OfferCoreError::InvalidTokenInMint
    )]
    pub token_in_mint: InterfaceAccount<'info, Mint>,

    #[account(
        constraint =
            token_out_mint.key() == offer.load()?.token_out_mint
            @ OfferCoreError::InvalidTokenOutMint
    )]
    pub token_out_mint: InterfaceAccount<'info, Mint>,
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
///
/// # Returns
///
/// * `Ok(u64)` - The current price if successfully calculated
/// * `Err(_)` - If the offer doesn't exist or price calculation fails
///
/// # Emits
///
/// * `GetNAVEvent` - Contains offer_pda, current_price, and timestamp
pub fn get_nav(ctx: Context<GetNAV>) -> Result<u64> {
    let offer = ctx.accounts.offer.load()?;
    let current_time = Clock::get()?.unix_timestamp as u64;

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
        "NAV Info - Offer PDA: {}, Current Price: {}, Timestamp: {}",
        ctx.accounts.offer.key(),
        current_price,
        current_time
    );

    emit!(GetNAVEvent {
        offer_pda: ctx.accounts.offer.key(),
        current_price,
        timestamp: current_time,
    });

    Ok(current_price)
}
