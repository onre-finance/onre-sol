use crate::{
    constants::{seeds::STATE, MAX_CIRCULATING_SUPPLY_EXCLUDED_ACCOUNTS},
    instructions::market_info::supply_exclusions::{record, sum_balances},
    model::{
        seeds::{MANAGED_TOKEN, SUPPLY_EXCLUSIONS},
        ManagedTokenConfig, ManagedTokenSupplyExclusions,
    },
    state::State,
    OnreError,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

/// Replaces a managed token's excluded token accounts with the remaining accounts,
/// in order, and caches their summed balance immediately.
#[derive(Accounts)]
pub struct SetManagedTokenSupplyExclusions<'info> {
    #[account(
        seeds = [MANAGED_TOKEN, managed_mint.key().as_ref()],
        bump = managed_token.bump,
    )]
    pub managed_token: Box<Account<'info, ManagedTokenConfig>>,

    pub managed_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = boss,
        space = 8 + ManagedTokenSupplyExclusions::INIT_SPACE,
        seeds = [SUPPLY_EXCLUSIONS, managed_mint.key().as_ref()],
        bump
    )]
    pub supply_exclusions: Box<Account<'info, ManagedTokenSupplyExclusions>>,

    /// CHECK: Legacy ONyc `["circ_supply_excl_balance"]` cache, required for ONyc and
    /// written with the same amount; ignored for other mints. Address validated in
    /// the handler; created on first use.
    #[account(mut)]
    pub legacy_excluded_balance: Option<UncheckedAccount<'info>>,

    #[account(
        seeds = [STATE],
        bump = state.bump,
        has_one = boss @ OnreError::InvalidBoss
    )]
    pub state: Box<Account<'info, State>>,

    #[account(mut)]
    pub boss: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Remaining accounts are the new list: at most 20 unique token accounts of the
/// managed mint under its token program. An empty list clears the exclusions.
pub fn set_managed_token_supply_exclusions(
    ctx: Context<SetManagedTokenSupplyExclusions>,
) -> Result<()> {
    let accounts = ctx.remaining_accounts;
    require!(
        accounts.len() <= MAX_CIRCULATING_SUPPLY_EXCLUDED_ACCOUNTS,
        OnreError::TooManyExcludedTokenAccounts
    );
    for (index, account) in accounts.iter().enumerate() {
        require!(
            accounts[..index]
                .iter()
                .all(|other| other.key != account.key),
            OnreError::DuplicateExcludedTokenAccount
        );
    }
    let a = ctx.accounts;
    let excluded_amount = sum_balances(&a.managed_mint, accounts, false)?;

    let exclusions = &mut a.supply_exclusions;
    exclusions.managed_mint = a.managed_mint.key();
    exclusions.token_accounts = [Pubkey::default(); MAX_CIRCULATING_SUPPLY_EXCLUDED_ACCOUNTS];
    for (slot, account) in exclusions.token_accounts.iter_mut().zip(accounts) {
        *slot = account.key();
    }
    exclusions.count = accounts.len() as u8;
    exclusions.bump = ctx.bumps.supply_exclusions;
    let legacy = a
        .legacy_excluded_balance
        .as_ref()
        .map(|account| account.to_account_info());
    record(
        ctx.program_id,
        &a.state,
        exclusions,
        excluded_amount,
        legacy.as_ref(),
        &a.boss.to_account_info(),
        &a.system_program.to_account_info(),
        a.boss.key(),
    )
}
