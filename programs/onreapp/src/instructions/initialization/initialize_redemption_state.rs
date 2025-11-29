use crate::constants::seeds;
use crate::state::{RedemptionState, State};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct InitializeRedemptionState<'info> {
    #[account(
        init,
        payer = boss,
        space = 8 + RedemptionState::INIT_SPACE,
        seeds = [seeds::REDEMPTION_STATE],
        bump,
    )]
    pub redemption_state: Box<Account<'info, RedemptionState>>,

    #[account(mut)]
    pub boss: Signer<'info>,

    #[account(
        has_one = boss
    )]
    pub state: Box<Account<'info, State>>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_redemption_state(
    ctx: Context<InitializeRedemptionState>,
    executor: Pubkey,
    initial_redemption_limit: u128,
    initial_fee_basis_points: u16,
    initial_user_basis_points: u16,
) -> Result<()> {
    let redemption_state = &mut ctx.accounts.redemption_state;
    redemption_state.executor = executor;
    redemption_state.redemption_limit = initial_redemption_limit;
    redemption_state.fee_basis_points = initial_fee_basis_points;
    redemption_state.user_basis_points = initial_user_basis_points;
    redemption_state.bump = ctx.bumps.redemption_state;
    Ok(())
}
