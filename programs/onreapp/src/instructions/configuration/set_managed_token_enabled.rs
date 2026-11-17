//! Boss-only enable/disable of a managed token. While disabled, execution and creation
//! checks that require it fail with `ConfigurationDisabled`.
use super::enabled_flag::set_enabled_flag;
use crate::{
    constants::seeds::STATE,
    model::{seeds::MANAGED_TOKEN, ManagedTokenConfig},
    state::State,
    OnreError,
};
use anchor_lang::prelude::*;

#[event]
pub struct ManagedTokenEnabledSetEvent {
    pub boss: Pubkey,
    pub managed_token: Pubkey,
    pub enabled: bool,
    pub timestamp: i64,
}

#[derive(Accounts)]
pub struct SetManagedTokenEnabled<'info> {
    #[account(
        mut,
        seeds = [MANAGED_TOKEN, managed_token.managed_mint.as_ref()],
        bump = managed_token.bump,
    )]
    pub managed_token: Box<Account<'info, ManagedTokenConfig>>,

    #[account(seeds = [STATE], bump = state.bump, has_one = boss @ OnreError::InvalidBoss)]
    pub state: Box<Account<'info, State>>,

    pub boss: Signer<'info>,
}

pub fn set_managed_token_enabled(
    ctx: Context<SetManagedTokenEnabled>,
    enabled: bool,
) -> Result<()> {
    let a = ctx.accounts;
    let timestamp = set_enabled_flag(&mut a.managed_token.enabled, enabled)?;
    emit!(ManagedTokenEnabledSetEvent {
        boss: a.boss.key(),
        managed_token: a.managed_token.key(),
        enabled,
        timestamp,
    });
    Ok(())
}
