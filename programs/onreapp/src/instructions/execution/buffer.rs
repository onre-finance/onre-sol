//! Offer adapter over the shared modular BUFFER engine.
use super::Execution;
use crate::{
    instructions::buffer::modular_accrual::{self, BufferContext},
    OnreError,
};
use anchor_lang::prelude::*;

pub use modular_accrual::ManagedTokenBufferAccruedEvent;

/// The active BUFFER's accounts: the legacy singleton set for ONyc before the
/// one-way switch, the per-mint set otherwise.
fn context<'a, 'info>(a: &Execution<'a, 'info>) -> Result<BufferContext<'a, 'info>> {
    let b = a.buffer;
    let onyc_legacy =
        a.managed_mint().key() == a.state.onyc_mint && a.state.onyc_legacy_buffer_active();
    let (buffer_state, reserve_token, management_token, performance_token) = if onyc_legacy {
        let required = |account: &'a Option<UncheckedAccount<'info>>| {
            account
                .as_ref()
                .ok_or_else(|| error!(OnreError::LegacyBufferAccountsRequired))
        };
        (
            required(&b.legacy_buffer_state)?,
            required(&b.legacy_buffer_reserve_token)?,
            required(&b.legacy_buffer_management_token)?,
            required(&b.legacy_buffer_performance_token)?,
        )
    } else {
        (
            &b.buffer_state,
            &b.buffer_reserve_token,
            &b.buffer_management_token,
            &b.buffer_performance_token,
        )
    };
    Ok(BufferContext {
        state: a.state,
        managed_mint: a.managed_mint(),
        managed_program: a.managed_program(),
        buffer_state,
        buffer_pricer: b.buffer_pricer.as_deref(),
        reserve_token,
        management_token,
        performance_token,
        mint_authority: a.mint_authority,
        configured_mint_multisig: a.config.managed_token.mint_multisig,
        mint_multisig: b.mint_multisig.as_ref(),
    })
}

pub(super) fn accrue_before_execution(
    program_id: &Pubkey,
    a: &Execution,
    authority_bump: u8,
    now: i64,
    max_supply: u64,
    max_mint: u64,
) -> Result<bool> {
    modular_accrual::accrue(
        program_id,
        &context(a)?,
        authority_bump,
        now,
        max_supply,
        max_mint,
    )
}

pub(super) fn store_post_execution_supply(
    program_id: &Pubkey,
    a: &Execution,
    now: i64,
) -> Result<()> {
    modular_accrual::store_post_supply(program_id, &context(a)?, now)
}
