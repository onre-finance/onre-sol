//! Boss-only replacement of a modular Prop AMM configuration, matching EVM
//! `configurePropAmm`. The bound pair and accumulated pressure are kept.
use super::create_prop_amm_state::validate_prop_amm_config;
use crate::{
    constants::seeds::STATE,
    model::{seeds::PROP_AMM, PropAmmConfig, PropAmmState},
    state::State,
    OnreError,
};
use anchor_lang::prelude::*;

#[event]
pub struct PropAmmConfigUpdatedEvent {
    pub boss: Pubkey,
    pub prop_amm_state: Pubkey,
    pub previous: PropAmmConfig,
    pub config: PropAmmConfig,
    pub timestamp: i64,
}

#[derive(Accounts)]
pub struct UpdatePropAmmConfig<'info> {
    #[account(
        mut,
        seeds = [PROP_AMM, prop_amm_state.quoter.as_ref()],
        bump = prop_amm_state.bump,
    )]
    pub prop_amm_state: Box<Account<'info, PropAmmState>>,

    #[account(seeds = [STATE], bump = state.bump, has_one = boss @ OnreError::InvalidBoss)]
    pub state: Box<Account<'info, State>>,

    pub boss: Signer<'info>,
}

/// Replaces the curve, cadence, epoch and wall configuration with the creation
/// limits, as EVM `configurePropAmm` does. The bound pair and accumulated pressure
/// are kept; a new epoch duration applies to the stored epoch anchor.
pub fn update_prop_amm_config(
    ctx: Context<UpdatePropAmmConfig>,
    config: PropAmmConfig,
) -> Result<()> {
    validate_prop_amm_config(&config)?;
    let a = ctx.accounts;
    let previous = a.prop_amm_state.config;
    require!(previous != config, OnreError::NoChange);
    a.prop_amm_state.config = config;
    emit!(PropAmmConfigUpdatedEvent {
        boss: a.boss.key(),
        prop_amm_state: a.prop_amm_state.key(),
        previous,
        config,
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
