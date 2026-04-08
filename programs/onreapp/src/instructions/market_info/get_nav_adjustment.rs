use crate::constants::seeds;
use crate::instructions::market_info::offer_valuation_utils::get_nav_adjustment_snapshot;
use crate::instructions::offer::offer_utils::find_active_vector_at;
use crate::instructions::Offer;
use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use anchor_spl::token_interface::Mint;

/// Event emitted when NAV adjustment calculation is completed
///
/// Provides transparency for tracking price changes between pricing vectors.
#[event]
pub struct GetNavAdjustmentEvent {
    /// The PDA address of the offer for which adjustment was calculated
    pub offer_pda: Pubkey,
    /// Current price from the active vector with scale=9
    pub current_price: u64,
    /// Previous price from the previous vector with scale=9 (None if first vector)
    pub previous_price: Option<u64>,
    /// Price adjustment (current - previous) as signed value with scale=9
    pub adjustment: i64,
    /// Unix timestamp when the adjustment calculation was performed
    pub timestamp: u64,
}

/// Account structure for querying NAV adjustment information
///
/// This struct defines the accounts required to calculate the price adjustment
/// between the current and previous pricing vectors. The calculation is read-only
/// and validates all accounts belong to the same offer.
#[derive(Accounts)]
pub struct GetNavAdjustment<'info> {
    /// The offer account containing pricing vectors for adjustment calculation
    ///
    /// This account is validated as a PDA derived from token mint addresses
    /// and contains multiple time-based pricing vectors for comparison.
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
            @ crate::OnreError::InvalidTokenInMint
    )]
    pub token_in_mint: InterfaceAccount<'info, Mint>,

    /// The output token mint account for offer validation
    #[account(
        constraint =
            token_out_mint.key() == offer.load()?.token_out_mint
            @ crate::OnreError::InvalidTokenOutMint
    )]
    pub token_out_mint: InterfaceAccount<'info, Mint>,
}

/// Calculates and returns the NAV adjustment between current and previous pricing vectors
///
/// This read-only instruction computes the price difference between the current
/// active vector's starting price and the previous vector's ending price. The
/// adjustment represents the price jump when transitioning between vectors.
///
/// The calculation compares prices at vector transition points:
/// - Current price: calculated at the start of the active vector
/// - Previous price: calculated at the end of the previous vector (same timestamp)
/// - Adjustment: signed difference (current - previous)
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
///
/// # Returns
/// * `Ok(adjustment)` - Signed price adjustment with scale=9 (positive = price increase)
/// * `Err(crate::OnreError::NoActiveVector)` - If no pricing vector is currently active
///
/// # Events
/// * `GetNavAdjustmentEvent` - Emitted with prices, adjustment, and timestamp
pub fn get_nav_adjustment(ctx: Context<GetNavAdjustment>) -> Result<i64> {
    let offer = ctx.accounts.offer.load()?;
    let current_time = Clock::get()?.unix_timestamp as u64;

    // Find the currently active pricing vector
    let active_vector = find_active_vector_at(&offer, current_time)?;

    let snapshot = get_nav_adjustment_snapshot(&offer, &active_vector)?;
    let current_price = snapshot.current_price;
    let previous_price_opt = snapshot.previous_price;
    let adjustment = snapshot.adjustment;

    msg!(
        "NAV Adjustment Info - Offer PDA: {}, Current Price: {}, Previous Price: {:?}, Adjustment: {}, Timestamp: {}",
        ctx.accounts.offer.key(),
        current_price,
        previous_price_opt,
        adjustment,
        current_time
    );

    emit!(GetNavAdjustmentEvent {
        offer_pda: ctx.accounts.offer.key(),
        current_price,
        previous_price: previous_price_opt,
        adjustment,
        timestamp: current_time,
    });

    Ok(adjustment)
}
