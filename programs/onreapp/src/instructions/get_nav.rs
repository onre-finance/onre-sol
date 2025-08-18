use anchor_lang::prelude::*;
use crate::instructions::pricing;
use crate::state::Offer;

#[derive(Accounts)]
pub struct GetNav<'info> {
    /// The offer to read NAV data from
    pub offer: Account<'info, Offer>,
}

/// Read-only instruction that returns the current NAV for the offer.
/// NAV is defined as: current_sell_token_amount / buy_token_one_amount
pub fn get_nav(ctx: Context<GetNav>) -> Result<u64> {
    let offer = &ctx.accounts.offer;
    pricing::calculate_nav(offer)
}

