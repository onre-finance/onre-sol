//! Standalone modular BUFFER operations, sharing legacy ONyc accounting during coexistence.
mod accounts;
mod cutover;
mod set_settings;
mod settle;
pub use accounts::*;
pub use cutover::*;
pub use set_settings::*;
pub use settle::*;

use crate::instructions::{buffer::modular_accrual, market_info::refresh_market_stats_pda};
use anchor_lang::prelude::*;

fn accrue_and_refresh<'info>(
    program_id: &Pubkey,
    a: &mut ManagedBufferAccounts<'info>,
    bump: u8,
    now: i64,
    payer: AccountInfo<'info>,
    system_program: AccountInfo<'info>,
    market_stats: AccountInfo<'info>,
    excluded: AccountInfo<'info>,
) -> Result<()> {
    // Standalone calls require an initialized authoritative state, even if its
    // current rate or supply is zero. They cannot silently report no BUFFER.
    modular_accrual::read_settings(program_id, &a.context())?;
    let (max_supply, max_mint) = a.limits();
    modular_accrual::accrue(program_id, &a.context(), bump, now, max_supply, max_mint)?;
    a.managed_mint.reload()?;
    if a.managed_mint.key() == a.state.onyc_mint {
        let offer = modular_accrual::canonical_market_offer(program_id, &a.context())?;
        refresh_market_stats_pda(
            &offer,
            &a.managed_mint,
            &excluded,
            &market_stats,
            &payer,
            &system_program,
            program_id,
        )?;
    }
    Ok(())
}
