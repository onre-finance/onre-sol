//! Boss-only removal of a managed-token registration, so a wrongly registered mint
//! (e.g. an asset such as USDC) can be used as an offer asset again.
use crate::{
    constants::seeds::STATE,
    model::{
        seeds::{BUFFER, MANAGED_TOKEN},
        ManagedTokenConfig,
    },
    state::State,
    OnreError,
};
use anchor_lang::prelude::*;

#[event]
pub struct ManagedTokenUnregisteredEvent {
    pub boss: Pubkey,
    pub managed_token: Pubkey,
    pub managed_mint: Pubkey,
    pub timestamp: i64,
}

/// Closes `["managed_token", mint]` and refunds its rent to the boss. Only a token
/// that never went live can be removed: it must be disabled, have no per-mint BUFFER
/// and no market stats selected, and it cannot be ONyc. Offers that still reference
/// the token stop executing; its Pricers, Quoters and supply exclusions are left as is.
#[derive(Accounts)]
pub struct UnregisterManagedToken<'info> {
    #[account(
        mut,
        close = boss,
        seeds = [MANAGED_TOKEN, managed_token.managed_mint.as_ref()],
        bump = managed_token.bump,
        constraint = managed_token.managed_mint != state.onyc_mint @ OnreError::CannotUnregisterOnyc,
        constraint = !managed_token.enabled @ OnreError::ManagedTokenStillInUse,
        constraint = managed_token.market_stats().is_none() @ OnreError::ManagedTokenStillInUse,
    )]
    pub managed_token: Box<Account<'info, ManagedTokenConfig>>,

    /// CHECK: The token's canonical `["buffer", mint]` address; must be empty.
    #[account(
        seeds = [BUFFER, managed_token.managed_mint.as_ref()],
        bump,
        constraint = buffer_state.data_is_empty() @ OnreError::ManagedTokenStillInUse,
    )]
    pub buffer_state: UncheckedAccount<'info>,

    #[account(
        seeds = [STATE],
        bump = state.bump,
        has_one = boss @ OnreError::InvalidBoss
    )]
    pub state: Box<Account<'info, State>>,

    #[account(mut)]
    pub boss: Signer<'info>,
}

pub fn unregister_managed_token(ctx: Context<UnregisterManagedToken>) -> Result<()> {
    emit!(ManagedTokenUnregisteredEvent {
        boss: ctx.accounts.boss.key(),
        managed_token: ctx.accounts.managed_token.key(),
        managed_mint: ctx.accounts.managed_token.managed_mint,
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
