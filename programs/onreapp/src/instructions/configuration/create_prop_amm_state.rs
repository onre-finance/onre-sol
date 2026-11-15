use crate::{
    constants::{seeds::STATE, MAX_BASIS_POINTS},
    instructions::prop_amm::config::{
        CADENCE_WAVE_STEP, CURVE_EXPONENT_SCALE, CURVE_EXPONENT_STEP, MAX_CADENCE_WAVE_SCALED,
    },
    model::{seeds::PROP_AMM, ManagedTokenConfig, PropAmmConfig, PropAmmState, Quoter, QuoterKind},
    state::State,
    OnreError,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

/// Records the created account and its complete initial configuration.
#[event]
pub struct PropAmmStateCreatedEvent {
    pub boss: Pubkey,
    pub prop_amm_state: Pubkey,
    pub configuration: PropAmmState,
    pub timestamp: i64,
}

/// Binds a Prop AMM quoter to a boss-selected external mint and registered managed mint.
/// Initializes zero pressure; the external asset needs no registration.
#[derive(Accounts)]
pub struct CreatePropAmmState<'info> {
    #[account(
        constraint = quoter.enabled @ OnreError::ConfigurationDisabled,
        constraint = quoter.kind == QuoterKind::PropAmm @ OnreError::IncompatibleQuoter
    )]
    pub quoter: Box<Account<'info, Quoter>>,

    #[account(
        constraint = managed_token.enabled @ OnreError::ConfigurationDisabled
    )]
    pub managed_token: Box<Account<'info, ManagedTokenConfig>>,

    pub asset_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init,
        payer = boss,
        space = 8 + PropAmmState::INIT_SPACE,
        seeds = [PROP_AMM, quoter.key().as_ref()],
        bump
    )]
    pub prop_amm_state: Box<Account<'info, PropAmmState>>,

    #[account(
        seeds = [STATE],
        bump = state.bump,
        has_one = boss @ OnreError::InvalidBoss
    )]
    pub state: Box<Account<'info, State>>,

    #[account(mut)]
    pub boss: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_prop_amm_state(
    ctx: Context<CreatePropAmmState>,
    config: PropAmmConfig,
) -> Result<()> {
    require!(
        config.curve_peg_haircut_bps <= MAX_BASIS_POINTS
            && (CURVE_EXPONENT_STEP..=CURVE_EXPONENT_SCALE * 10)
                .contains(&config.curve_exponent_scaled)
            && config
                .curve_exponent_scaled
                .is_multiple_of(CURVE_EXPONENT_STEP)
            && config.cadence_threshold > 0
            && config.cadence_wave_scaled <= MAX_CADENCE_WAVE_SCALED
            && config.cadence_wave_scaled.is_multiple_of(CADENCE_WAVE_STEP)
            && config.epoch_duration_seconds > 0
            && config.epoch_duration_seconds <= i64::MAX as u64
            && config.wall_sensitivity_scaled > 0,
        OnreError::InvalidAmount
    );
    require_keys_neq!(
        ctx.accounts.managed_token.managed_mint,
        ctx.accounts.asset_mint.key(),
        OnreError::InvalidSwapPair
    );
    ctx.accounts.prop_amm_state.set_inner(PropAmmState {
        quoter: ctx.accounts.quoter.key(),
        asset_mint: ctx.accounts.asset_mint.key(),
        managed_mint: ctx.accounts.managed_token.managed_mint,
        config,
        current_sell_value_stable: 0,
        current_buy_value_stable: 0,
        previous_net_sell_value_stable: 0,
        epoch_start: Clock::get()?.unix_timestamp,
        current_sell_trade_count: 0,
        bump: ctx.bumps.prop_amm_state,
    });
    emit!(PropAmmStateCreatedEvent {
        boss: ctx.accounts.boss.key(),
        prop_amm_state: ctx.accounts.prop_amm_state.key(),
        configuration: (**ctx.accounts.prop_amm_state).clone(),
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
