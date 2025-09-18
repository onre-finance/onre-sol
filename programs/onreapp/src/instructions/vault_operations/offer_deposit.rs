use crate::constants::seeds;
use crate::state::{OfferVaultAuthority, State};
use crate::utils::transfer_tokens;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[event]
pub struct OfferVaultDepositEvent {
    pub mint: Pubkey,
    pub amount: u64,
    pub boss: Pubkey,
}

/// Account structure for depositing tokens to the offer vault.
///
/// This struct defines the accounts required for the boss to deposit tokens
/// into the offer vault authority's token accounts.
#[derive(Accounts)]
pub struct OfferVaultDeposit<'info> {
    /// The offer vault authority account that controls the vault token accounts.
    #[account(seeds = [seeds::OFFER_VAULT_AUTHORITY], bump)]
    pub vault_authority: Account<'info, OfferVaultAuthority>,

    /// The token mint for the deposit.
    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Boss's token account for the specific mint (source of tokens).
    #[account(
        mut,
        associated_token::mint = token_mint,
        associated_token::authority = boss,
        associated_token::token_program = token_program
    )]
    pub boss_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Vault's token account for the specific mint (destination of tokens).
    /// Uses init_if_needed to create the account if it doesn't exist.
    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = token_mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = token_program
    )]
    pub vault_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The signer authorizing the deposit, must be the boss.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// Token program.
    pub token_program: Interface<'info, TokenInterface>,

    /// Associated Token program.
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

/// Deposits tokens into the offer vault.
///
/// Transfers tokens from the boss's token account to the offer vault's token account
/// for the specified mint. Creates the vault token account if it doesn't exist.
/// Only the boss can call this instruction.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the deposit.
/// - `amount`: Amount of tokens to deposit.
///
/// # Returns
/// A `Result` indicating success or failure.
pub fn offer_vault_deposit(ctx: Context<OfferVaultDeposit>, amount: u64) -> Result<()> {
    // Transfer tokens from boss to vault
    transfer_tokens(
        &ctx.accounts.token_mint,
        &ctx.accounts.token_program,
        &ctx.accounts.boss_token_account,
        &ctx.accounts.vault_token_account,
        &ctx.accounts.boss,
        None,
        amount,
    )?;

    emit!(OfferVaultDepositEvent {
        mint: ctx.accounts.token_mint.key(),
        amount,
        boss: ctx.accounts.boss.key(),
    });

    msg!("Offer vault deposit successful: {} tokens", amount);
    Ok(())
}
