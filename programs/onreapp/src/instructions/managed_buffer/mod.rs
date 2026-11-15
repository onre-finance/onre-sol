//! Standalone per-mint modular BUFFER operations, independent of the legacy singleton.
mod accounts;
mod set_settings;
mod settle;
pub use accounts::*;
pub use set_settings::*;
pub use settle::*;

use crate::{
    instructions::{buffer::modular_accrual, market_info::refresh_managed_token_market_stats},
    utils::{has_transfer_fee, managed_token::effective_mint_limits},
    OnreError,
};
use anchor_lang::prelude::*;

fn accrue_and_refresh<'info>(
    program_id: &Pubkey,
    a: &mut ManagedBufferAccounts<'info>,
    bump: u8,
    now: i64,
    market_stats: AccountInfo<'info>,
    excluded: AccountInfo<'info>,
) -> Result<()> {
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
    let (max_supply, max_mint) = effective_mint_limits(&a.state, &a.managed_token);
    modular_accrual::accrue(program_id, &a.context(), bump, now, max_supply, max_mint)?;
    a.managed_mint.reload()?;
    refresh_managed_token_market_stats(
        program_id,
        &a.state,
        &a.managed_token,
        &a.managed_mint,
        &excluded,
        &market_stats,
        || modular_accrual::canonical_market_valuation(program_id, &a.context()),
    )
}
