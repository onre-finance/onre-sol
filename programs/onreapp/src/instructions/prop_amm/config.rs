use crate::constants::{seeds, MAX_BASIS_POINTS};
use crate::state::State;
use anchor_lang::prelude::*;

pub const DEFAULT_POOL_TARGET_BPS: u16 = 1_500;
pub const DEFAULT_LINEAR_WEIGHT_BPS: u16 = 2_000;
pub const DEFAULT_BASE_EXPONENT: u8 = 3;

#[account]
#[derive(InitSpace)]
pub struct PropAmmState {
    pub pool_target_bps: u16,
    pub linear_weight_bps: u16,
    pub base_exponent: u8,
    pub bump: u8,
    pub reserved: [u8; 61],
}

#[event]
pub struct PropAmmConfiguredEvent {
    pub old_pool_target_bps: u16,
    pub new_pool_target_bps: u16,
    pub old_linear_weight_bps: u16,
    pub new_linear_weight_bps: u16,
    pub old_base_exponent: u8,
    pub new_base_exponent: u8,
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
    linear_weight_bps: u16,
    base_exponent: u8,
) -> Result<()> {
    require!(
        pool_target_bps <= MAX_BASIS_POINTS,
        crate::OnreError::InvalidAmount
    );
    require!(
        linear_weight_bps <= MAX_BASIS_POINTS,
        crate::OnreError::InvalidAmount
    );
    require!(
        (1..=10).contains(&base_exponent),
        crate::OnreError::InvalidAmount
    );

    let prop_amm_state = &mut ctx.accounts.prop_amm_state;
    let old_pool_target_bps = prop_amm_state.pool_target_bps;
    let old_linear_weight_bps = prop_amm_state.linear_weight_bps;
    let old_base_exponent = prop_amm_state.base_exponent;

    prop_amm_state.pool_target_bps = pool_target_bps;
    prop_amm_state.linear_weight_bps = linear_weight_bps;
    prop_amm_state.base_exponent = base_exponent;
    prop_amm_state.bump = ctx.bumps.prop_amm_state;

    emit!(PropAmmConfiguredEvent {
        old_pool_target_bps,
        new_pool_target_bps: pool_target_bps,
        old_linear_weight_bps,
        new_linear_weight_bps: linear_weight_bps,
        old_base_exponent,
        new_base_exponent: base_exponent,
    });

    Ok(())
}

impl Default for PropAmmState {
    fn default() -> Self {
        Self {
            pool_target_bps: DEFAULT_POOL_TARGET_BPS,
            linear_weight_bps: DEFAULT_LINEAR_WEIGHT_BPS,
            base_exponent: DEFAULT_BASE_EXPONENT,
            bump: 0,
            reserved: [0; 61],
        }
    }
}
