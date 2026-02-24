use crate::constants::seeds;
use crate::instructions::cache::{CacheLowestSupplyUpdatedEvent, CacheState};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

#[derive(Accounts)]
pub struct UpdateLowestSupply<'info> {
    #[account(
        mut,
        seeds = [seeds::CACHE_STATE],
        bump = cache_state.bump,
        has_one = onyc_mint,
    )]
    pub cache_state: Account<'info, CacheState>,
    pub onyc_mint: InterfaceAccount<'info, Mint>,
}

pub fn update_lowest_supply(ctx: Context<UpdateLowestSupply>) -> Result<()> {
    let cache_state = &mut ctx.accounts.cache_state;
    let current_supply = ctx.accounts.onyc_mint.supply;
    let previous_lowest_supply = cache_state.lowest_supply;
    let mut updated = false;

    if current_supply < cache_state.lowest_supply {
        cache_state.lowest_supply = current_supply;
        updated = true;
    }
    let timestamp = Clock::get()?.unix_timestamp;

    emit!(CacheLowestSupplyUpdatedEvent {
        previous_lowest_supply,
        new_lowest_supply: cache_state.lowest_supply,
        current_supply,
        updated,
        timestamp,
    });

    Ok(())
}
