//! Standalone per-mint modular BUFFER operations, independent of the legacy singleton.
mod accounts;
mod burn_for_nav;
mod set_settings;
mod settle;
pub use accounts::*;
pub use burn_for_nav::*;
pub use set_settings::*;
pub use settle::*;

use crate::{
    instructions::{buffer::modular_accrual, market_info::refresh_modular_market_stats},
    utils::{has_transfer_fee, managed_token::effective_mint_limits},
    OnreError,
};
use anchor_lang::prelude::*;

/// Validates and accrues an initialized per-mint BUFFER up to `now`, then reloads
/// the mint so callers see the post-accrual supply.
fn accrue(program_id: &Pubkey, a: &mut ManagedBufferAccounts, bump: u8, now: i64) -> Result<()> {
    // Standalone calls require an initialized authoritative state, even if its
    // current rate or supply is zero. They cannot silently report no BUFFER.
    require!(
        !has_transfer_fee(&a.managed_mint)?,
        OnreError::TransferFeeNotSupported
    );
    require!(
        !a.buffer_state.data_is_empty(),
        OnreError::InvalidBufferStateAccount
    );
    // Validate configured mint control even when this accrual mints nothing.
    a.context().minter(bump)?;
    let (max_supply, max_mint) = effective_mint_limits(&a.state, &a.managed_token);
    modular_accrual::accrue(program_id, &a.context(), bump, now, max_supply, max_mint)?;
    a.managed_mint.reload()?;
    Ok(())
}

/// Refreshes the mint's market stats after a standalone BUFFER operation.
fn refresh_market_stats<'info>(
    program_id: &Pubkey,
    a: &ManagedBufferAccounts<'info>,
    market_stats: AccountInfo<'info>,
    legacy_market_stats: Option<AccountInfo<'info>>,
    excluded: AccountInfo<'info>,
) -> Result<()> {
    // No payer here: an ONyc legacy singleton not yet created is skipped.
    refresh_modular_market_stats(
        program_id,
        &a.state,
        &a.managed_token,
        &a.managed_mint,
        &excluded,
        &market_stats,
        legacy_market_stats.as_ref(),
        None,
        || modular_accrual::canonical_market_valuation(program_id, &a.context()),
    )
}

fn accrue_and_refresh<'info>(
    program_id: &Pubkey,
    a: &mut ManagedBufferAccounts<'info>,
    bump: u8,
    now: i64,
    market_stats: AccountInfo<'info>,
    legacy_market_stats: Option<AccountInfo<'info>>,
    excluded: AccountInfo<'info>,
) -> Result<()> {
    accrue(program_id, a, bump, now)?;
    refresh_market_stats(program_id, a, market_stats, legacy_market_stats, excluded)
}
