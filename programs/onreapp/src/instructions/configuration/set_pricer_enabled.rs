//! Boss-only enable/disable of a Pricer. While disabled, execution and creation
//! checks that require it fail with `ConfigurationDisabled`.
use super::enabled_flag::set_enabled_flag;
use crate::{
    constants::seeds::STATE,
    model::{seeds::PRICER, Pricer},
    state::State,
    OnreError,
};
use anchor_lang::prelude::*;

#[event]
pub struct PricerEnabledSetEvent {
    pub boss: Pubkey,
    pub pricer: Pubkey,
    pub enabled: bool,
    pub timestamp: i64,
}

#[derive(Accounts)]
pub struct SetPricerEnabled<'info> {
    #[account(
        mut,
        seeds = [
            PRICER,
            pricer.managed_mint.as_ref(),
            &[pricer.denomination as u8],
            &pricer.instance_id.to_le_bytes()
        ],
        bump = pricer.bump,
    )]
    pub pricer: Box<Account<'info, Pricer>>,

    #[account(seeds = [STATE], bump = state.bump, has_one = boss @ OnreError::InvalidBoss)]
    pub state: Box<Account<'info, State>>,

    pub boss: Signer<'info>,
}

pub fn set_pricer_enabled(ctx: Context<SetPricerEnabled>, enabled: bool) -> Result<()> {
    let a = ctx.accounts;
    let timestamp = set_enabled_flag(&mut a.pricer.enabled, enabled)?;
    emit!(PricerEnabledSetEvent {
        boss: a.boss.key(),
        pricer: a.pricer.key(),
        enabled,
        timestamp,
    });
    Ok(())
}
