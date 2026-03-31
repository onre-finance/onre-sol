use crate::constants::{seeds, MAX_BASIS_POINTS};
use crate::instructions::buffer::{BufferErrorCode, BufferFeeConfigUpdatedEvent, BufferState};
use crate::state::State;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SetBufferFeeConfig<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss,
    )]
    pub state: Account<'info, State>,

    #[account(
        mut,
        seeds = [seeds::BUFFER_STATE],
        bump = buffer_state.bump,
    )]
    pub buffer_state: Account<'info, BufferState>,

    pub boss: Signer<'info>,
}

pub fn set_buffer_fee_config(
    ctx: Context<SetBufferFeeConfig>,
    management_fee_basis_points: u16,
    management_fee_wallet: Pubkey,
    performance_fee_basis_points: u16,
    performance_fee_wallet: Pubkey,
) -> Result<()> {
    require!(
        management_fee_basis_points <= MAX_BASIS_POINTS,
        BufferErrorCode::InvalidFee
    );
    require!(
        performance_fee_basis_points <= MAX_BASIS_POINTS,
        BufferErrorCode::InvalidFee
    );
    require!(
        management_fee_basis_points == 0 || management_fee_wallet != Pubkey::default(),
        BufferErrorCode::InvalidFeeWallet
    );
    require!(
        performance_fee_basis_points == 0 || performance_fee_wallet != Pubkey::default(),
        BufferErrorCode::InvalidFeeWallet
    );

    let buffer_state = &mut ctx.accounts.buffer_state;
    require!(
        buffer_state.management_fee_basis_points != management_fee_basis_points
            || buffer_state.management_fee_wallet != management_fee_wallet
            || buffer_state.performance_fee_basis_points != performance_fee_basis_points
            || buffer_state.performance_fee_wallet != performance_fee_wallet,
        BufferErrorCode::NoChange
    );

    let old_management_fee_basis_points = buffer_state.management_fee_basis_points;
    let old_management_fee_wallet = buffer_state.management_fee_wallet;
    let old_performance_fee_basis_points = buffer_state.performance_fee_basis_points;
    let old_performance_fee_wallet = buffer_state.performance_fee_wallet;

    buffer_state.management_fee_basis_points = management_fee_basis_points;
    buffer_state.management_fee_wallet = management_fee_wallet;
    buffer_state.performance_fee_basis_points = performance_fee_basis_points;
    buffer_state.performance_fee_wallet = performance_fee_wallet;

    emit!(BufferFeeConfigUpdatedEvent {
        old_management_fee_basis_points,
        new_management_fee_basis_points: management_fee_basis_points,
        old_management_fee_wallet,
        new_management_fee_wallet: management_fee_wallet,
        old_performance_fee_basis_points,
        new_performance_fee_basis_points: performance_fee_basis_points,
        old_performance_fee_wallet,
        new_performance_fee_wallet: performance_fee_wallet,
    });

    Ok(())
}
