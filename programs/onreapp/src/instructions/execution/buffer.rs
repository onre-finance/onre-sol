//! Offer adapter over the shared modular BUFFER engine.
use super::TakeOffer;
use crate::instructions::buffer::modular_accrual::{self, BufferContext};
use anchor_lang::prelude::*;

pub use modular_accrual::ManagedTokenBufferAccruedEvent;

fn context<'a, 'info>(a: &'a TakeOffer<'info>) -> BufferContext<'a, 'info> {
    BufferContext {
        managed_mint: a.managed_mint(),
        managed_program: a.managed_program(),
        buffer_state: &a.buffer.buffer_state,
        buffer_pricer: a.buffer.buffer_pricer.as_deref(),
        reserve_token: &a.buffer.buffer_reserve_token,
        management_token: &a.buffer.buffer_management_token,
        performance_token: &a.buffer.buffer_performance_token,
        mint_authority: &a.mint_authority,
        configured_mint_multisig: a.config.managed_token.mint_multisig,
        mint_multisig: a.buffer.mint_multisig.as_ref(),
    }
}

pub(super) fn accrue_before_execution(
    program_id: &Pubkey,
    a: &TakeOffer,
    authority_bump: u8,
    now: i64,
    max_supply: u64,
    max_mint: u64,
) -> Result<bool> {
    modular_accrual::accrue(
        program_id,
        &context(a),
        authority_bump,
        now,
        max_supply,
        max_mint,
    )
}

pub(super) fn store_post_execution_supply(
    program_id: &Pubkey,
    a: &TakeOffer,
    now: i64,
) -> Result<()> {
    modular_accrual::store_post_supply(program_id, &context(a), now)
}
