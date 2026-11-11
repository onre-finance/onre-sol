use super::accounts::{
    __client_accounts_managed_buffer_accounts, __cpi_client_accounts_managed_buffer_accounts,
    ManagedBufferAccountsBumps,
};
use super::{accrue_and_refresh, ManagedBufferAccounts};
use crate::OnreError;
use anchor_lang::prelude::*;

/// Worker-only settlement. ONyc uses the legacy singleton and vaults; other
/// managed mints use their own state. No trade, custody migration or second clock.
#[derive(Accounts)]
pub struct SettleManagedTokenBuffer<'info> {
    pub buffer_accounts: ManagedBufferAccounts<'info>,
    #[account(mut,address=buffer_accounts.state.worker @ OnreError::InvalidWorker)]
    pub worker: Signer<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: ONyc refresh validates the canonical MarketStats PDA; other mints ignore it.
    #[account(mut)]
    pub market_stats: UncheckedAccount<'info>,
    /// CHECK: ONyc refresh validates the canonical excluded-balance cache.
    pub circulating_supply_excluded_balance: UncheckedAccount<'info>,
}

pub fn settle_managed_token_buffer(ctx: Context<SettleManagedTokenBuffer>) -> Result<()> {
    let a = ctx.accounts;
    accrue_and_refresh(
        ctx.program_id,
        &mut a.buffer_accounts,
        ctx.bumps.buffer_accounts.mint_authority,
        Clock::get()?.unix_timestamp,
        a.worker.to_account_info(),
        a.system_program.to_account_info(),
        a.market_stats.to_account_info(),
        a.circulating_supply_excluded_balance.to_account_info(),
    )
}
