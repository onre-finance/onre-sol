//! Boss-only enable/disable of a Quoter. While disabled, execution and creation
//! checks that require it fail with `ConfigurationDisabled`.
use super::enabled_flag::set_enabled_flag;
use crate::{
    constants::seeds::STATE,
    model::{seeds::QUOTER, Quoter},
    state::State,
    OnreError,
};
use anchor_lang::prelude::*;

#[event]
pub struct QuoterEnabledSetEvent {
    pub boss: Pubkey,
    pub quoter: Pubkey,
    pub enabled: bool,
    pub timestamp: i64,
}

#[derive(Accounts)]
pub struct SetQuoterEnabled<'info> {
    #[account(
        mut,
        seeds = [QUOTER, &[quoter.kind as u8], &quoter.instance_id.to_le_bytes()],
        bump = quoter.bump,
    )]
    pub quoter: Box<Account<'info, Quoter>>,

    #[account(seeds = [STATE], bump = state.bump, has_one = boss @ OnreError::InvalidBoss)]
    pub state: Box<Account<'info, State>>,

    pub boss: Signer<'info>,
}

pub fn set_quoter_enabled(ctx: Context<SetQuoterEnabled>, enabled: bool) -> Result<()> {
    let a = ctx.accounts;
    let timestamp = set_enabled_flag(&mut a.quoter.enabled, enabled)?;
    emit!(QuoterEnabledSetEvent {
        boss: a.boss.key(),
        quoter: a.quoter.key(),
        enabled,
        timestamp,
    });
    Ok(())
}
