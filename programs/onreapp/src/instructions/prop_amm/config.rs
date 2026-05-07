use crate::constants::{seeds, MAX_BASIS_POINTS};
use crate::state::State;
use anchor_lang::prelude::*;

pub const DEFAULT_POOL_TARGET_BPS: u16 = 1_500;
pub const DEFAULT_MIN_LIQUIDATION_HAIRCUT_BPS: u16 = 50;
pub const DEFAULT_CURVE_PEG_HAIRCUT_BPS: u16 = 700;
pub const CURVE_EXPONENT_SCALE: u32 = 10_000;
pub const CURVE_EXPONENT_STEP: u32 = 1_000;
pub const DEFAULT_CURVE_EXPONENT_SCALED: u32 = 25_000;
pub const DEFAULT_MIN_CADENCE_EXPONENT_SCALED: u32 = 1_000;
pub const DEFAULT_CADENCE_THRESHOLD: u32 = 20;
pub const DEFAULT_CADENCE_SENSITIVITY_SCALED: u32 = 10_000;
pub const DEFAULT_EPOCH_DURATION_SECONDS: i64 = 86_400;
pub const WALL_SENSITIVITY_SCALE: u128 = 10_000;
pub const DEFAULT_WALL_SENSITIVITY_SCALED: u32 = 20_000;

#[account]
#[derive(InitSpace)]
pub struct PropAmmState {
    pub pool_target_bps: u16,
    pub min_liquidation_haircut_bps: u16,
    pub curve_peg_haircut_bps: u16,
    pub curve_exponent_scaled: u32,
    pub min_cadence_exponent_scaled: u32,
    pub cadence_threshold: u32,
    pub cadence_sensitivity_scaled: u32,
    pub epoch_duration_seconds: i64,
    pub wall_sensitivity_scaled: u32,
    pub curr_sell_value_stable: u64,
    pub curr_buy_value_stable: u64,
    pub prev_net_sell_value_stable: u64,
    pub curr_sell_trade_count: u32,
    pub epoch_start: i64,
    pub bump: u8,
}

#[event]
pub struct PropAmmConfiguredEvent {
    pub old_pool_target_bps: u16,
    pub new_pool_target_bps: u16,
    pub old_min_liquidation_haircut_bps: u16,
    pub new_min_liquidation_haircut_bps: u16,
    pub old_curve_peg_haircut_bps: u16,
    pub new_curve_peg_haircut_bps: u16,
    pub old_curve_exponent_scaled: u32,
    pub new_curve_exponent_scaled: u32,
    pub old_min_cadence_exponent_scaled: u32,
    pub new_min_cadence_exponent_scaled: u32,
    pub old_cadence_threshold: u32,
    pub new_cadence_threshold: u32,
    pub old_cadence_sensitivity_scaled: u32,
    pub new_cadence_sensitivity_scaled: u32,
    pub old_epoch_duration_seconds: i64,
    pub new_epoch_duration_seconds: i64,
    pub old_wall_sensitivity_scaled: u32,
    pub new_wall_sensitivity_scaled: u32,
}

#[derive(Accounts)]
pub struct ConfigurePropAmm<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss @ crate::OnreError::InvalidBoss
    )]
    pub state: Account<'info, State>,

    #[account(
        init_if_needed,
        payer = boss,
        space = 8 + PropAmmState::INIT_SPACE,
        seeds = [seeds::PROP_AMM_STATE],
        bump
    )]
    pub prop_amm_state: Account<'info, PropAmmState>,

    #[account(mut)]
    pub boss: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn configure_prop_amm(
    ctx: Context<ConfigurePropAmm>,
    pool_target_bps: u16,
    min_liquidation_haircut_bps: u16,
    curve_peg_haircut_bps: u16,
    curve_exponent_scaled: u32,
    min_cadence_exponent_scaled: u32,
    cadence_threshold: u32,
    cadence_sensitivity_scaled: u32,
    epoch_duration_seconds: i64,
    wall_sensitivity_scaled: u32,
) -> Result<()> {
    require!(
        pool_target_bps <= MAX_BASIS_POINTS,
        crate::OnreError::InvalidAmount
    );
    require!(
        min_liquidation_haircut_bps <= MAX_BASIS_POINTS,
        crate::OnreError::InvalidAmount
    );
    require!(
        curve_peg_haircut_bps <= MAX_BASIS_POINTS,
        crate::OnreError::InvalidAmount
    );
    require!(
        (CURVE_EXPONENT_STEP..=CURVE_EXPONENT_SCALE.saturating_mul(10))
            .contains(&curve_exponent_scaled),
        crate::OnreError::InvalidAmount
    );
    require!(
        curve_exponent_scaled % CURVE_EXPONENT_STEP == 0,
        crate::OnreError::InvalidAmount
    );
    require!(
        (CURVE_EXPONENT_STEP..=CURVE_EXPONENT_SCALE).contains(&min_cadence_exponent_scaled),
        crate::OnreError::InvalidAmount
    );
    require!(
        min_cadence_exponent_scaled % CURVE_EXPONENT_STEP == 0,
        crate::OnreError::InvalidAmount
    );
    require!(cadence_threshold > 0, crate::OnreError::InvalidAmount);
    require!(
        cadence_sensitivity_scaled <= CURVE_EXPONENT_SCALE.saturating_mul(10),
        crate::OnreError::InvalidAmount
    );
    require!(epoch_duration_seconds > 0, crate::OnreError::InvalidAmount);
    require!(wall_sensitivity_scaled > 0, crate::OnreError::InvalidAmount);

    let prop_amm_state = &mut ctx.accounts.prop_amm_state;
    let old_pool_target_bps = prop_amm_state.pool_target_bps;
    let old_min_liquidation_haircut_bps = prop_amm_state.min_liquidation_haircut_bps;
    let old_curve_peg_haircut_bps = prop_amm_state.curve_peg_haircut_bps;
    let old_curve_exponent_scaled = prop_amm_state.curve_exponent_scaled;
    let old_min_cadence_exponent_scaled = prop_amm_state.min_cadence_exponent_scaled;
    let old_cadence_threshold = prop_amm_state.cadence_threshold;
    let old_cadence_sensitivity_scaled = prop_amm_state.cadence_sensitivity_scaled;
    let old_epoch_duration_seconds = prop_amm_state.epoch_duration_seconds;
    let old_wall_sensitivity_scaled = prop_amm_state.wall_sensitivity_scaled;

    prop_amm_state.pool_target_bps = pool_target_bps;
    prop_amm_state.min_liquidation_haircut_bps = min_liquidation_haircut_bps;
    prop_amm_state.curve_peg_haircut_bps = curve_peg_haircut_bps;
    prop_amm_state.curve_exponent_scaled = curve_exponent_scaled;
    prop_amm_state.min_cadence_exponent_scaled = min_cadence_exponent_scaled;
    prop_amm_state.cadence_threshold = cadence_threshold;
    prop_amm_state.cadence_sensitivity_scaled = cadence_sensitivity_scaled;
    prop_amm_state.epoch_duration_seconds = epoch_duration_seconds;
    prop_amm_state.wall_sensitivity_scaled = wall_sensitivity_scaled;
    if prop_amm_state.epoch_start == 0 {
        prop_amm_state.epoch_start = Clock::get()?.unix_timestamp;
    }
    prop_amm_state.bump = ctx.bumps.prop_amm_state;

    emit!(PropAmmConfiguredEvent {
        old_pool_target_bps,
        new_pool_target_bps: pool_target_bps,
        old_min_liquidation_haircut_bps,
        new_min_liquidation_haircut_bps: min_liquidation_haircut_bps,
        old_curve_peg_haircut_bps,
        new_curve_peg_haircut_bps: curve_peg_haircut_bps,
        old_curve_exponent_scaled,
        new_curve_exponent_scaled: curve_exponent_scaled,
        old_min_cadence_exponent_scaled,
        new_min_cadence_exponent_scaled: min_cadence_exponent_scaled,
        old_cadence_threshold,
        new_cadence_threshold: cadence_threshold,
        old_cadence_sensitivity_scaled,
        new_cadence_sensitivity_scaled: cadence_sensitivity_scaled,
        old_epoch_duration_seconds,
        new_epoch_duration_seconds: epoch_duration_seconds,
        old_wall_sensitivity_scaled,
        new_wall_sensitivity_scaled: wall_sensitivity_scaled,
    });

    Ok(())
}

impl Default for PropAmmState {
    fn default() -> Self {
        Self {
            pool_target_bps: DEFAULT_POOL_TARGET_BPS,
            min_liquidation_haircut_bps: DEFAULT_MIN_LIQUIDATION_HAIRCUT_BPS,
            curve_peg_haircut_bps: DEFAULT_CURVE_PEG_HAIRCUT_BPS,
            curve_exponent_scaled: DEFAULT_CURVE_EXPONENT_SCALED,
            min_cadence_exponent_scaled: DEFAULT_MIN_CADENCE_EXPONENT_SCALED,
            cadence_threshold: DEFAULT_CADENCE_THRESHOLD,
            cadence_sensitivity_scaled: DEFAULT_CADENCE_SENSITIVITY_SCALED,
            epoch_duration_seconds: DEFAULT_EPOCH_DURATION_SECONDS,
            wall_sensitivity_scaled: DEFAULT_WALL_SENSITIVITY_SCALED,
            curr_sell_value_stable: 0,
            curr_buy_value_stable: 0,
            prev_net_sell_value_stable: 0,
            curr_sell_trade_count: 0,
            epoch_start: 0,
            bump: 0,
        }
    }
}
