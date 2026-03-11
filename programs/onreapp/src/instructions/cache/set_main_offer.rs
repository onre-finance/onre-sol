use crate::constants::seeds;
use crate::instructions::cache::{
    CacheErrorCode, CacheMainOfferUpdatedEvent, CacheState,
};
use crate::instructions::Offer;
use crate::state::State;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SetMainOffer<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss,
    )]
    pub state: Account<'info, State>,

    #[account(
        mut,
        seeds = [seeds::CACHE_STATE],
        bump = cache_state.bump,
    )]
    pub cache_state: Account<'info, CacheState>,

    pub boss: Signer<'info>,

    pub offer: AccountLoader<'info, Offer>,
}

pub fn set_main_offer(ctx: Context<SetMainOffer>) -> Result<()> {
    let cache_state = &mut ctx.accounts.cache_state;
    let new_main_offer = ctx.accounts.offer.key();

    // The cache does not persist a caller-provided current yield. Instead, accrue_cache
    // reads the active vector APR from this stored offer and treats that APR as current_yield.
    require!(
        cache_state.main_offer != new_main_offer,
        CacheErrorCode::NoChange
    );

    let old_main_offer = cache_state.main_offer;
    cache_state.main_offer = new_main_offer;

    emit!(CacheMainOfferUpdatedEvent {
        old_main_offer,
        new_main_offer,
    });

    Ok(())
}
