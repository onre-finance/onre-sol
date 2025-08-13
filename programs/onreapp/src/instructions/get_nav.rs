use anchor_lang::prelude::*;

use crate::state::Offer;
use crate::instructions::pricing::calculate_current_sell_amount;

#[derive(Accounts)]
pub struct GetNav<'info> {
    /// The offer to read NAV data from
    pub offer: Account<'info, Offer>,
}

/// Read-only instruction that returns the current NAV for the offer.
/// NAV is defined as: current_sell_token_amount / buy_token_one_amount
pub fn get_nav(ctx: Context<GetNav>) -> Result<u64> {
    let offer = &ctx.accounts.offer;

    let current_sell_token_amount = calculate_current_sell_amount(&offer)? as u128;
    let buy_token_one_amount = offer.buy_token_1.amount as u128;

    // buy_token_one_amount is guaranteed > 0 by make_offer validation
    let nav = current_sell_token_amount
        .checked_div(buy_token_one_amount)
        .unwrap() as u64;

    Ok(nav)
}

