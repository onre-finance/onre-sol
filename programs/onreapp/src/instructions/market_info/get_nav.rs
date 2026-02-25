use crate::constants::seeds;
use crate::instructions::market_info::offer_valuation_utils::get_active_vector_and_current_price;
use crate::instructions::{Offer, OfferVector};
use crate::OfferCoreError;
use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use anchor_spl::token_interface::Mint;

/// Event emitted when NAV (Net Asset Value) calculation is completed
///
/// Provides transparency for tracking current pricing information for offers.
#[event]
pub struct GetNAVEvent {
    /// The PDA address of the offer for which NAV was calculated
    pub offer_pda: Pubkey,
    /// Current price with 9 decimal precision (scale=9)
    pub current_price: u64,
    /// Unix timestamp when the price calculation was performed
    pub timestamp: u64,
    /// Unix timestamp when the next price change will occur
    pub next_price_change_timestamp: u64,
}

/// Account structure for querying NAV (Net Asset Value) information
///
/// This struct defines the accounts required to calculate the current price
/// for a specific offer. The calculation is read-only and validates all
/// accounts belong to the same offer.
#[derive(Accounts)]
pub struct GetNAV<'info> {
    /// The offer account containing pricing vectors and configuration
    ///
    /// This account is validated as a PDA derived from token mint addresses
    /// and contains time-based pricing vectors for price calculation.
    #[account(
        seeds = [
            seeds::OFFER,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref()
        ],
        bump = offer.load()?.bump
    )]
    pub offer: AccountLoader<'info, Offer>,

    /// The input token mint account for offer validation
    #[account(
        constraint =
            token_in_mint.key() == offer.load()?.token_in_mint
            @ OfferCoreError::InvalidTokenInMint
    )]
    pub token_in_mint: InterfaceAccount<'info, Mint>,

    /// The output token mint account for offer validation
    #[account(
        constraint =
            token_out_mint.key() == offer.load()?.token_out_mint
            @ OfferCoreError::InvalidTokenOutMint
    )]
    pub token_out_mint: InterfaceAccount<'info, Mint>,
}

/// Calculates and returns the current NAV (Net Asset Value) for a specific offer
///
/// This read-only instruction calculates the current price by finding the active
/// pricing vector and applying time-based price calculations with APR growth.
/// The price represents the current exchange rate with 9 decimal precision.
///
/// The calculation uses the currently active vector's base price, APR, and
/// time elapsed since the base time to determine the current stepped price.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
///
/// # Returns
/// * `Ok(current_price)` - The calculated price with scale=9 (1_000_000_000 = 1.0)
/// * `Err(OfferCoreError::NoActiveVector)` - If no pricing vector is currently active
///
/// # Events
/// * `GetNAVEvent` - Emitted with offer PDA, current price, and timestamp
pub fn get_nav(ctx: Context<GetNAV>) -> Result<u64> {
    let offer = ctx.accounts.offer.load()?;
    let current_time = Clock::get()?.unix_timestamp as u64;

    let (active_vector, current_price) = get_active_vector_and_current_price(&offer, current_time)?;

    // Calculate when the next NAV change will occur
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

    // Find the next vector that will become active
    let next_vector = find_next_vector_after(&offer, current_time);

    // Next NAV change is the minimum of next interval and next vector start time
    let next_price_change_timestamp = match next_vector {
        Some(vector) => next_interval_timestamp.min(vector.start_time),
        None => next_interval_timestamp,
    };

    msg!(
        "NAV Info - Offer PDA: {}, Current Timestamp: {}, Current Price: {}, Next Change: {}",
        ctx.accounts.offer.key(),
        current_time,
        current_price,
        next_price_change_timestamp,
    );

    emit!(GetNAVEvent {
        offer_pda: ctx.accounts.offer.key(),
        current_price,
        timestamp: current_time,
        next_price_change_timestamp,
    });

    Ok(current_price)
}

/// Finds the next vector that will become active after the current time
///
/// Searches through all vectors to find the one with the smallest start_time
/// that is still in the future (greater than current_time).
///
/// # Arguments
/// * `offer` - The offer containing pricing vectors to search
/// * `current_time` - The current Unix timestamp
///
/// # Returns
/// * `Some(OfferVector)` - The next future vector if one exists
/// * `None` - If no vectors are scheduled in the future
fn find_next_vector_after(offer: &Offer, current_time: u64) -> Option<OfferVector> {
    offer
        .vectors
        .iter()
        .filter(|vector| vector.start_time > current_time)
        .min_by_key(|vector| vector.start_time)
        .copied()
}
