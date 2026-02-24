use crate::constants::seeds;
use crate::instructions::cache::{CacheErrorCode, CacheState, CacheYieldUpdatedEvent};
use crate::state::State;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SetCacheYields<'info> {
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

pub fn set_cache_yields(
    ctx: Context<SetCacheYields>,
    gross_yield: u64,
    current_yield: u64,
) -> Result<()> {
    let cache_state = &mut ctx.accounts.cache_state;
    require!(
        cache_state.gross_yield != gross_yield || cache_state.current_yield != current_yield,
        CacheErrorCode::NoChange
    );

    cache_state.gross_yield = gross_yield;
    cache_state.current_yield = current_yield;

    emit!(CacheYieldUpdatedEvent {
        gross_yield,
        current_yield,
    });

    Ok(())
}
