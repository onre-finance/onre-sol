use super::accounts::{
    __client_accounts_managed_buffer_accounts, __cpi_client_accounts_managed_buffer_accounts,
    ManagedBufferAccountsBumps,
};
use super::{accrue_and_refresh, ManagedBufferAccounts};
use crate::OnreError;
use anchor_lang::prelude::*;

/// Worker-only settlement of a mint's per-mint BUFFER. No trade or custody movement.
#[derive(Accounts)]
pub struct SettleManagedTokenBuffer<'info> {
    pub buffer_accounts: ManagedBufferAccounts<'info>,

    #[account(
        address = buffer_accounts.state.worker @ OnreError::InvalidWorker,
    )]
    pub worker: Signer<'info>,

    /// CHECK: Must equal `ManagedTokenConfig::market_stats` when one is selected;
    /// ignored otherwise.
    #[account(mut)]
    pub market_stats: UncheckedAccount<'info>,

    /// CHECK: Legacy ONyc `["market_stats"]` singleton. Required when the managed
    /// mint is ONyc and refreshed from the canonical Pricer once it exists (address
    /// validated in the handler); ignored for other mints.
    #[account(mut)]
    pub legacy_market_stats: Option<UncheckedAccount<'info>>,

    /// CHECK: The mint's `["supply_exclusions", mint]` cache, validated where read;
    /// may be absent except for ONyc.
    pub supply_exclusions: UncheckedAccount<'info>,
}

pub fn settle_managed_token_buffer(ctx: Context<SettleManagedTokenBuffer>) -> Result<()> {
    let a = ctx.accounts;
    accrue_and_refresh(
        ctx.program_id,
        &mut a.buffer_accounts,
        ctx.bumps.buffer_accounts.mint_authority,
        Clock::get()?.unix_timestamp,
        a.market_stats.to_account_info(),
        a.legacy_market_stats
            .as_ref()
            .map(|account| account.to_account_info()),
        a.supply_exclusions.to_account_info(),
    )
}
