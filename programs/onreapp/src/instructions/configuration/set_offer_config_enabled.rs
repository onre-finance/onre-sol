//! Boss-only enable/disable of a OfferConfig. While disabled, execution and creation
//! checks that require it fail with `ConfigurationDisabled`.
use super::enabled_flag::set_enabled_flag;
use crate::{
    constants::seeds::STATE,
    model::{seeds::OFFER_CONFIG, OfferConfig},
    state::State,
    OnreError,
};
use anchor_lang::prelude::*;

#[event]
pub struct OfferConfigEnabledSetEvent {
    pub boss: Pubkey,
    pub offer_config: Pubkey,
    pub enabled: bool,
    pub timestamp: i64,
}

#[derive(Accounts)]
pub struct SetOfferConfigEnabled<'info> {
    #[account(
        mut,
        seeds = [
            OFFER_CONFIG,
            offer_config.token_in_mint.as_ref(),
            offer_config.token_out_mint.as_ref(),
            &offer_config.instance_id.to_le_bytes()
        ],
        bump = offer_config.bump,
    )]
    pub offer_config: Box<Account<'info, OfferConfig>>,

    #[account(seeds = [STATE], bump = state.bump, has_one = boss @ OnreError::InvalidBoss)]
    pub state: Box<Account<'info, State>>,

    pub boss: Signer<'info>,
}

pub fn set_offer_config_enabled(ctx: Context<SetOfferConfigEnabled>, enabled: bool) -> Result<()> {
    let a = ctx.accounts;
    let timestamp = set_enabled_flag(&mut a.offer_config.enabled, enabled)?;
    emit!(OfferConfigEnabledSetEvent {
        boss: a.boss.key(),
        offer_config: a.offer_config.key(),
        enabled,
        timestamp,
    });
    Ok(())
}
