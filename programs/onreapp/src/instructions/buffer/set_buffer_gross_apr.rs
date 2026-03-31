use crate::constants::seeds;
use crate::instructions::buffer::{BufferErrorCode, BufferGrossYieldUpdatedEvent, BufferState};
use crate::state::State;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SetBufferGrossYield<'info> {
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

pub fn set_buffer_gross_apr(ctx: Context<SetBufferGrossYield>, gross_yield: u64) -> Result<()> {
    let buffer_state = &mut ctx.accounts.buffer_state;

    require!(
        buffer_state.gross_apr != gross_yield,
        BufferErrorCode::NoChange
    );

    buffer_state.gross_apr = gross_yield;

    emit!(BufferGrossYieldUpdatedEvent { gross_yield });

    Ok(())
}
