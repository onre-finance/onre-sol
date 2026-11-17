//! Boss-only enable/disable of a FeeConfig. While disabled, execution and creation
//! checks that require it fail with `ConfigurationDisabled`.
use super::enabled_flag::set_enabled_flag;
use crate::{
    constants::seeds::STATE,
    model::{seeds::FEE_CONFIG, FeeConfig},
    state::State,
    OnreError,
};
use anchor_lang::prelude::*;

#[event]
pub struct FeeConfigEnabledSetEvent {
    pub boss: Pubkey,
    pub fee_config: Pubkey,
    pub enabled: bool,
    pub timestamp: i64,
}

#[derive(Accounts)]
pub struct SetFeeConfigEnabled<'info> {
    #[account(
        mut,
        seeds = [FEE_CONFIG, &fee_config.instance_id.to_le_bytes()],
        bump = fee_config.bump,
    )]
    pub fee_config: Box<Account<'info, FeeConfig>>,

    #[account(seeds = [STATE], bump = state.bump, has_one = boss @ OnreError::InvalidBoss)]
    pub state: Box<Account<'info, State>>,

    pub boss: Signer<'info>,
}

pub fn set_fee_config_enabled(ctx: Context<SetFeeConfigEnabled>, enabled: bool) -> Result<()> {
    let a = ctx.accounts;
    let timestamp = set_enabled_flag(&mut a.fee_config.enabled, enabled)?;
    emit!(FeeConfigEnabledSetEvent {
        boss: a.boss.key(),
        fee_config: a.fee_config.key(),
        enabled,
        timestamp,
    });
    Ok(())
}
