//! Offer adapter over the shared modular BUFFER engine.
use super::TakeOffer;
use crate::instructions::buffer::modular_accrual::{self, BufferContext};
use anchor_lang::prelude::*;

pub use modular_accrual::ManagedTokenBufferAccruedEvent;

fn context<'a, 'info>(a: &'a TakeOffer<'info>) -> BufferContext<'a, 'info> {
    BufferContext {
        state: &a.state,
        managed_mint: a.managed_mint(),
        managed_program: a.managed_program(),
        buffer_state: &a.buffer_state,
        buffer_pricer: a.buffer_pricer.as_deref(),
        main_offer: a.main_offer.as_ref(),
        reserve_token: &a.buffer_reserve_token,
        management_token: &a.buffer_management_token,
        performance_token: &a.buffer_performance_token,
        mint_authority: &a.mint_authority,
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

pub(super) fn store_post_execution_supply(a: &TakeOffer, now: i64) -> Result<()> {
    modular_accrual::store_post_supply(&context(a), now)
}

pub(super) fn canonical_market_offer(
    program_id: &Pubkey,
    a: &TakeOffer,
) -> Result<crate::instructions::Offer> {
    modular_accrual::canonical_market_offer(program_id, &context(a))
}
