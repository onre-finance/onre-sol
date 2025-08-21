use crate::constants::seeds;
use crate::state::{State, VaultAuthority};
use crate::utils::transfer_tokens;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use anchor_spl::associated_token::AssociatedToken;

#[event]
pub struct VaultDepositEvent {
    pub mint: Pubkey,
    pub amount: u64,
    pub boss: Pubkey,
}

/// Account structure for depositing tokens to the vault.
///
/// This struct defines the accounts required for the boss to deposit tokens
/// into the vault authority's token accounts.
#[derive(Accounts)]
pub struct VaultDeposit<'info> {
    /// The vault authority account that controls the vault token accounts.
    #[account(seeds = [seeds::VAULT_AUTHORITY], bump)]
    pub vault_authority: Account<'info, VaultAuthority>,

    /// The token mint for the deposit.
    pub token_mint: Box<Account<'info, Mint>>,

    /// Boss's token account for the specific mint (source of tokens).
    #[account(
        mut,
        associated_token::mint = token_mint,
        associated_token::authority = boss
    )]
    pub boss_token_account: Box<Account<'info, TokenAccount>>,

    /// Vault's token account for the specific mint (destination of tokens).
    /// Uses init_if_needed to create the account if it doesn't exist.
    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = token_mint,
        associated_token::authority = vault_authority
    )]
    pub vault_token_account: Box<Account<'info, TokenAccount>>,

    /// The signer authorizing the deposit, must be the boss.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// SPL Token program.
    pub token_program: Program<'info, Token>,

    /// Associated Token program.
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

/// Deposits tokens into the vault.
///
/// Transfers tokens from the boss's token account to the vault's token account
/// for the specified mint. Creates the vault token account if it doesn't exist.
/// Only the boss can call this instruction.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the deposit.
/// - `amount`: Amount of tokens to deposit.
///
/// # Returns
/// A `Result` indicating success or failure.
pub fn vault_deposit(ctx: Context<VaultDeposit>, amount: u64) -> Result<()> {
    // Transfer tokens from boss to vault
    transfer_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.boss_token_account,
        &ctx.accounts.vault_token_account,
        &ctx.accounts.boss,
        None,
        amount,
    )?;

    msg!(
        "Vault deposit - mint: {}, amount: {}",
        ctx.accounts.token_mint.key(),
        amount
    );

    emit!(VaultDepositEvent {
        mint: ctx.accounts.token_mint.key(),
        amount,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}