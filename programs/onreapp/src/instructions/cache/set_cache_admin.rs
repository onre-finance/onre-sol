use crate::constants::seeds;
use crate::instructions::cache::{CacheAdminUpdatedEvent, CacheErrorCode, CacheState};
use crate::state::State;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SetCacheAdmin<'info> {
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
}

pub fn set_cache_admin(ctx: Context<SetCacheAdmin>, new_cache_admin: Pubkey) -> Result<()> {
    let cache_state = &mut ctx.accounts.cache_state;
    require!(
        new_cache_admin != cache_state.cache_admin,
        CacheErrorCode::NoChange
    );

    let old_cache_admin = cache_state.cache_admin;
    cache_state.cache_admin = new_cache_admin;

    emit!(CacheAdminUpdatedEvent {
        old_cache_admin,
        new_cache_admin,
    });

    Ok(())
}
