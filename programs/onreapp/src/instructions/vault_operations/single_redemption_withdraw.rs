use crate::constants::seeds;
use crate::state::{SingleRedemptionVaultAuthority, State};
use crate::utils::transfer_tokens;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[event]
pub struct SingleRedemptionVaultWithdrawEvent {
    pub mint: Pubkey,
    pub amount: u64,
    pub boss: Pubkey,
}

/// Account structure for withdrawing tokens from the single redemption vault.
///
/// This struct defines the accounts required for the boss to withdraw tokens
/// from the single redemption vault authority's token accounts.
#[derive(Accounts)]
pub struct SingleRedemptionVaultWithdraw<'info> {
    /// The single redemption vault authority account that controls the vault token accounts.
    #[account(seeds = [seeds::SINGLE_REDEMPTION_VAULT_AUTHORITY], bump)]
    pub vault_authority: Account<'info, SingleRedemptionVaultAuthority>,

    /// The token mint for the withdrawal.
    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Boss's token account for the specific mint (destination of tokens).
    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = token_mint,
        associated_token::authority = boss,
        associated_token::token_program = token_program
    )]
    pub boss_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Vault's token account for the specific mint (source of tokens).
    #[account(
        mut,
        associated_token::mint = token_mint,
        associated_token::authority = vault_authority,
    )]
    pub vault_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The signer authorizing the withdrawal, must be the boss.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// SPL Token program.
    pub token_program: Interface<'info, TokenInterface>,

    /// Associated Token program.
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

/// Withdraws tokens from the single redemption vault.
///
/// Transfers tokens from the single redemption vault's token account to the boss's token account
/// for the specified mint. Creates the boss token account if it doesn't exist.
/// Only the boss can call this instruction.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the withdrawal.
/// - `amount`: Amount of tokens to withdraw.
///
/// # Returns
/// A `Result` indicating success or failure.
pub fn single_redemption_vault_withdraw(
    ctx: Context<SingleRedemptionVaultWithdraw>,
    amount: u64,
) -> Result<()> {
    // Create signer seeds for vault authority
    let vault_authority_seeds = &[
        seeds::SINGLE_REDEMPTION_VAULT_AUTHORITY,
        &[ctx.bumps.vault_authority],
    ];
    let signer_seeds = &[&vault_authority_seeds[..]];

    // Transfer tokens from vault to boss
    transfer_tokens(
        &ctx.accounts.token_mint,
        &ctx.accounts.token_program,
        &ctx.accounts.vault_token_account,
        &ctx.accounts.boss_token_account,
        &ctx.accounts.vault_authority.to_account_info(),
        Some(signer_seeds),
        amount,
    )?;

    emit!(SingleRedemptionVaultWithdrawEvent {
        mint: ctx.accounts.token_mint.key(),
        amount,
        boss: ctx.accounts.boss.key(),
    });

    msg!(
        "Single redemption vault withdraw successful: {} tokens",
        amount
    );
    Ok(())
}
