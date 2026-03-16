use crate::constants::seeds;
use crate::instructions::buffer::{BufferAdminUpdatedEvent, BufferErrorCode, BufferState};
use crate::state::State;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SetBufferAdmin<'info> {
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

pub fn set_buffer_admin(ctx: Context<SetBufferAdmin>, new_buffer_admin: Pubkey) -> Result<()> {
    let buffer_state = &mut ctx.accounts.buffer_state;
    require!(
        new_buffer_admin != buffer_state.buffer_admin,
        BufferErrorCode::NoChange
    );

    let old_buffer_admin = buffer_state.buffer_admin;
    buffer_state.buffer_admin = new_buffer_admin;

    emit!(BufferAdminUpdatedEvent {
        old_buffer_admin,
        new_buffer_admin,
    });

    Ok(())
}
