//! Permissionless refresh of a managed token's cached excluded supply.
use super::supply_exclusions::{record, sum_balances};
use crate::{
    constants::seeds::{CIRCULATING_SUPPLY_EXCLUDED_BALANCE, STATE},
    model::{seeds::SUPPLY_EXCLUSIONS, ManagedTokenSupplyExclusions},
    state::State,
    OnreError,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

#[derive(Accounts)]
pub struct RefreshManagedTokenSupplyExclusions<'info> {
    #[account(seeds = [STATE], bump = state.bump)]
    pub state: Box<Account<'info, State>>,

    #[account(
        constraint = managed_mint.key() != state.onyc_mint
            || legacy_excluded_balance.is_some() @ OnreError::InvalidCirculatingSupplyExcludedBalance
    )]
    pub managed_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        seeds = [SUPPLY_EXCLUSIONS, managed_mint.key().as_ref()],
        bump = supply_exclusions.bump,
    )]
    pub supply_exclusions: Box<Account<'info, ManagedTokenSupplyExclusions>>,

    /// CHECK: Legacy ONyc `["circ_supply_excl_balance"]` cache, required for ONyc and
    /// written with the same amount; ignored for other mints. Created on first use.
    #[account(mut, seeds = [CIRCULATING_SUPPLY_EXCLUDED_BALANCE], bump)]
    pub legacy_excluded_balance: Option<UncheckedAccount<'info>>,

    /// Any signer; pays only if the ONyc legacy cache must be created.
    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Re-sums the stored token accounts, which must be supplied as remaining accounts
/// exactly and in stored order, so a caller cannot omit one to raise circulating
/// supply. A closed account counts as zero. Not blocked by the kill switch.
pub fn refresh_managed_token_supply_exclusions(
    ctx: Context<RefreshManagedTokenSupplyExclusions>,
) -> Result<()> {
    let accounts = ctx.remaining_accounts;
    let a = ctx.accounts;
    let stored = a.supply_exclusions.active();
    require!(
        accounts.len() <= stored.len(),
        OnreError::TooManyExcludedTokenAccounts
    );
    require!(
        accounts.len() == stored.len(),
        OnreError::MissingExcludedTokenAccount
    );
    for (account, expected) in accounts.iter().zip(stored) {
        require_keys_eq!(
            account.key(),
            *expected,
            OnreError::InvalidExcludedTokenAccount
        );
    }
    let excluded_amount = sum_balances(&a.managed_mint, accounts, true)?;
    let legacy = a
        .legacy_excluded_balance
        .as_ref()
        .map(|account| account.to_account_info())
        .zip(ctx.bumps.legacy_excluded_balance);
    record(
        ctx.program_id,
        &a.state,
        &mut a.supply_exclusions,
        excluded_amount,
        legacy,
        &a.payer.to_account_info(),
        &a.system_program.to_account_info(),
        a.payer.key(),
    )
}
