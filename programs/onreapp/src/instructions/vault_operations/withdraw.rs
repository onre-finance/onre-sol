use crate::state::{State, VaultAuthority};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

#[event]
pub struct VaultWithdrawEvent {
    pub mint: Pubkey,
    pub amount: u64,
    pub boss: Pubkey,
}

/// Account structure for withdrawing tokens from the vault.
///
/// This struct defines the accounts required for the boss to withdraw tokens
/// from the vault authority's token accounts.
#[derive(Accounts)]
pub struct VaultWithdraw<'info> {
    /// The vault authority account that controls the vault token accounts.
    #[account(seeds = [b"vault_authority"], bump)]
    pub vault_authority: Account<'info, VaultAuthority>,

    /// The token mint for the withdrawal.
    pub token_mint: Box<Account<'info, Mint>>,

    /// Boss's token account for the specific mint (destination of tokens).
    /// Must already exist - will fail if it doesn't.
    #[account(
        mut,
        associated_token::mint = token_mint,
        associated_token::authority = boss
    )]
    pub boss_token_account: Box<Account<'info, TokenAccount>>,

    /// Vault's token account for the specific mint (source of tokens).
    /// Must already exist - will fail if it doesn't.
    #[account(
        mut,
        associated_token::mint = token_mint,
        associated_token::authority = vault_authority
    )]
    pub vault_token_account: Box<Account<'info, TokenAccount>>,

    /// The signer authorizing the withdrawal, must be the boss.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// SPL Token program.
    pub token_program: Program<'info, Token>,
}

/// Withdraws tokens from the vault.
///
/// Transfers tokens from the vault's token account to the boss's token account
/// for the specified mint. Both token accounts must already exist.
/// Only the boss can call this instruction.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the withdrawal.
/// - `amount`: Amount of tokens to withdraw.
///
/// # Returns
/// A `Result` indicating success or failure.
pub fn vault_withdraw(ctx: Context<VaultWithdraw>, amount: u64) -> Result<()> {
    // Get vault authority bump for signing
    let vault_authority_bump = ctx.bumps.vault_authority;
    let vault_authority_seeds = &[
        b"vault_authority".as_ref(),
        &[vault_authority_bump],
    ];
    let signer_seeds = &[vault_authority_seeds.as_slice()];

    // Transfer tokens from vault to boss using vault authority as signer
    let cpi_accounts = Transfer {
        from: ctx.accounts.vault_token_account.to_account_info(),
        to: ctx.accounts.boss_token_account.to_account_info(),
        authority: ctx.accounts.vault_authority.to_account_info(),
    };
    
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
    
    token::transfer(cpi_ctx, amount)?;

    emit!(VaultWithdrawEvent {
        mint: ctx.accounts.token_mint.key(),
        amount,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}