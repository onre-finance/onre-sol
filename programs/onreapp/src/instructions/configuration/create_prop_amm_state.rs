use crate::model::{self, seeds::*};
use anchor_lang::prelude::*;

/// Records the created account and its complete initial configuration.
#[event]
pub struct PropAmmStateCreatedEvent {
    pub boss: Pubkey,
    pub prop_amm_state: Pubkey,
    pub configuration: model::PropAmmState,
    pub timestamp: i64,
}

/// Binds a Prop AMM quoter to one immutable registered pair and initializes zero pressure.
#[derive(Accounts)]
pub struct CreatePropAmmState<'info> {
    #[account(
        constraint = quoter.enabled @ crate::OnreError::ConfigurationDisabled,
        constraint = quoter.kind == model::QuoterKind::PropAmm @ crate::OnreError::IncompatibleQuoter
    )]
    pub quoter: Box<Account<'info, model::Quoter>>,

    #[account(
        constraint = managed_token.enabled @ crate::OnreError::ConfigurationDisabled
    )]
    pub managed_token: Box<Account<'info, model::ManagedTokenConfig>>,

    #[account(
        constraint = asset.enabled @ crate::OnreError::ConfigurationDisabled
    )]
    pub asset: Box<Account<'info, model::AssetConfig>>,

    #[account(
        init,
        payer = boss,
        space = 8 + model::PropAmmState::INIT_SPACE,
        seeds = [PROP_AMM, quoter.key().as_ref()],
        bump
    )]
    pub prop_amm_state: Box<Account<'info, model::PropAmmState>>,

    #[account(
        seeds = [crate::constants::seeds::STATE],
        bump = state.bump,
        has_one = boss @ crate::OnreError::InvalidBoss
    )]
    pub state: Box<Account<'info, crate::state::State>>,

    #[account(mut)]
    pub boss: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_prop_amm_state(
    ctx: Context<CreatePropAmmState>,
    config: model::PropAmmConfig,
) -> Result<()> {
    use crate::instructions::prop_amm::config::{
        CADENCE_WAVE_STEP, CURVE_EXPONENT_SCALE, CURVE_EXPONENT_STEP, MAX_CADENCE_WAVE_SCALED,
    };
    require!(
        config.curve_peg_haircut_bps <= crate::constants::MAX_BASIS_POINTS
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
        crate::OnreError::InvalidAmount
    );
    require_keys_neq!(
        ctx.accounts.managed_token.managed_mint,
        ctx.accounts.asset.asset_mint,
        crate::OnreError::InvalidSwapPair
    );
    ctx.accounts.prop_amm_state.set_inner(model::PropAmmState {
        quoter: ctx.accounts.quoter.key(),
        asset_mint: ctx.accounts.asset.asset_mint,
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
