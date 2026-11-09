use crate::{
    constants::seeds::STATE,
    model::{seeds::OFFER_CONFIG, OfferConfig, PropAmmState, Quoter},
    state::State,
    utils::pressure_link::validate_pressure_link,
    OnreError,
};
use anchor_lang::prelude::*;

#[event]
pub struct OfferPressureStateSetEvent {
    pub boss: Pubkey,
    pub offer_config: Pubkey,
    pub previous_state: Pubkey,
    pub pressure_state: Pubkey,
    pub timestamp: i64,
}

/// Configure which pair tracker receives this offer's successful trade volumes.
/// NAV can attach/detach; PropAmm must retain its own state. Does not reset pressure.
#[derive(Accounts)]
pub struct SetOfferPressureState<'info> {
    #[account(
        mut,
        seeds = [OFFER_CONFIG, offer_config.token_in_mint.as_ref(), offer_config.token_out_mint.as_ref(), &[offer_config.flow as u8]],
        bump = offer_config.bump,
        has_one = quoter @ OnreError::IncompatibleQuoter
    )]
    pub offer_config: Box<Account<'info, OfferConfig>>,
    pub quoter: Box<Account<'info, Quoter>>,
    pub prop_amm_state: Option<Box<Account<'info, PropAmmState>>>,
    #[account(seeds = [STATE], bump = state.bump, has_one = boss @ OnreError::InvalidBoss)]
    pub state: Box<Account<'info, State>>,
    pub boss: Signer<'info>,
}

pub fn set_offer_pressure_state(ctx: Context<SetOfferPressureState>) -> Result<()> {
    let a = ctx.accounts;
    let pressure_state = validate_pressure_link(
        ctx.program_id,
        a.quoter.key(),
        a.quoter.kind,
        a.offer_config.direction,
        a.offer_config.token_in_mint,
        a.offer_config.token_out_mint,
        a.prop_amm_state.as_deref(),
    )?;
    let previous_state = a.offer_config.pressure_state;
    require_keys_neq!(pressure_state, previous_state, OnreError::NoChange);
    a.offer_config.pressure_state = pressure_state;
    emit!(OfferPressureStateSetEvent {
        boss: a.boss.key(),
        offer_config: a.offer_config.key(),
        previous_state,
        pressure_state,
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
