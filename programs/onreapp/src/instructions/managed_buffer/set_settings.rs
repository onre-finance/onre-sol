use super::accounts::{
    __client_accounts_managed_buffer_accounts, __cpi_client_accounts_managed_buffer_accounts,
    ManagedBufferAccountsBumps,
};
use super::{accrue_and_refresh, ManagedBufferAccounts};
use crate::{
    constants::MAX_BASIS_POINTS, instructions::buffer::modular_accrual,
    instructions::buffer::MAX_BUFFER_GROSS_APR, model::BufferSettings, OnreError,
};
use anchor_lang::prelude::*;

#[event]
pub struct ManagedTokenBufferSettingsSetEvent {
    pub boss: Pubkey,
    pub managed_mint: Pubkey,
    pub buffer_state: Pubkey,
    pub previous: BufferSettings,
    pub settings: BufferSettings,
    pub timestamp: i64,
}

/// Boss-only policy update; first settle under existing settings. ONyc updates
/// legacy settings during coexistence and per-mint settings after cutover.
#[derive(Accounts)]
pub struct SetManagedTokenBufferSettings<'info> {
    pub buffer_accounts: ManagedBufferAccounts<'info>,
    #[account(mut,address=buffer_accounts.state.boss @ OnreError::InvalidBoss)]
    pub boss: Signer<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: ONyc refresh validates the canonical MarketStats PDA; other mints ignore it.
    #[account(mut)]
    pub market_stats: UncheckedAccount<'info>,
    /// CHECK: ONyc refresh validates the canonical excluded-balance cache.
    pub circulating_supply_excluded_balance: UncheckedAccount<'info>,
}

pub fn set_managed_token_buffer_settings(
    ctx: Context<SetManagedTokenBufferSettings>,
    settings: BufferSettings,
) -> Result<()> {
    require!(
        settings.gross_apr <= MAX_BUFFER_GROSS_APR,
        OnreError::InvalidAPR
    );
    require!(
        settings.management_fee_basis_points <= MAX_BASIS_POINTS
            && settings.performance_fee_basis_points <= MAX_BASIS_POINTS,
        OnreError::InvalidFee
    );
    let a = ctx.accounts;
    let previous = modular_accrual::read_settings(ctx.program_id, &a.buffer_accounts.context())?;
    require!(previous != settings, OnreError::NoChange);
    let now = Clock::get()?.unix_timestamp;
    accrue_and_refresh(
        ctx.program_id,
        &mut a.buffer_accounts,
        ctx.bumps.buffer_accounts.mint_authority,
        now,
        a.boss.to_account_info(),
        a.system_program.to_account_info(),
        a.market_stats.to_account_info(),
        a.circulating_supply_excluded_balance.to_account_info(),
    )?;
    modular_accrual::write_settings(ctx.program_id, &a.buffer_accounts.context(), settings)?;
    emit!(ManagedTokenBufferSettingsSetEvent {
        boss: a.boss.key(),
        managed_mint: a.buffer_accounts.managed_mint.key(),
        buffer_state: a.buffer_accounts.buffer_state.key(),
        previous,
        settings,
        timestamp: now
    });
    Ok(())
}
