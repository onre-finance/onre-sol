use crate::constants::seeds;
use crate::state::{OfferVaultAuthority, State};
use crate::utils::transfer_tokens;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

/// Event emitted when tokens are successfully withdrawn from the offer vault
///
/// Provides transparency for tracking vault withdrawals and fund management.
#[event]
pub struct OfferVaultWithdrawEvent {
    /// The token mint that was withdrawn
    pub mint: Pubkey,
    /// Amount of tokens withdrawn from the vault
    pub amount: u64,
    /// The boss account that performed the withdrawal
    pub boss: Pubkey,
}

/// Account structure for withdrawing tokens from the offer vault
///
/// This struct defines the accounts required for the boss to recover tokens
/// from the offer vault, enabling fund management and reallocation of
/// vault reserves when needed.
#[derive(Accounts)]
pub struct OfferVaultWithdraw<'info> {
    /// Program-derived authority that controls vault token accounts
    ///
    /// This PDA manages the vault token accounts and signs the withdrawal
    /// transfer using program-derived signatures.
    #[account(seeds = [seeds::OFFER_VAULT_AUTHORITY], bump)]
    pub vault_authority: Account<'info, OfferVaultAuthority>,

    /// The token mint for the withdrawal operation
    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Boss's token account serving as the destination for withdrawn tokens
    ///
    /// Created automatically if it doesn't exist. Receives tokens withdrawn
    /// from the vault for boss fund management.
    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = token_mint,
        associated_token::authority = boss,
        associated_token::token_program = token_program
    )]
    pub boss_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Vault's token account serving as the source of withdrawn tokens
    ///
    /// Must have sufficient balance to cover the requested withdrawal amount.
    /// Controlled by the vault authority PDA.
    #[account(
        mut,
        associated_token::mint = token_mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = token_program
    )]
    pub vault_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The boss account authorized to withdraw tokens and pay for account creation
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Program state account containing boss authorization
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss
    )]
    pub state: Box<Account<'info, State>>,

    /// Token program interface for transfer operations
    pub token_program: Interface<'info, TokenInterface>,

    /// Associated Token Program for automatic token account creation
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// System program for account creation and rent payment
    pub system_program: Program<'info, System>,
}

/// Withdraws tokens from the offer vault for fund management
///
/// This instruction allows the boss to recover tokens from the offer vault,
/// enabling reallocation of vault reserves, emergency fund recovery, or
/// redistribution of unused vault tokens. Uses program-derived signatures
/// to authorize the transfer from vault to boss accounts.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `amount` - Amount of tokens to withdraw from the vault
///
/// # Returns
/// * `Ok(())` - If the withdrawal completes successfully
/// * `Err(_)` - If transfer fails or insufficient vault balance
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss account must match the one stored in program state
///
/// # Effects
/// - Transfers tokens from vault account to boss account
/// - Creates boss token account if it doesn't exist
/// - Reduces available tokens in vault reserves
///
/// # Events
/// * `OfferVaultWithdrawEvent` - Emitted with mint, amount, and withdrawer details
pub fn offer_vault_withdraw(ctx: Context<OfferVaultWithdraw>, amount: u64) -> Result<()> {
    // Create signer seeds for vault authority
    let vault_authority_seeds = &[seeds::OFFER_VAULT_AUTHORITY, &[ctx.bumps.vault_authority]];
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

    emit!(OfferVaultWithdrawEvent {
        mint: ctx.accounts.token_mint.key(),
        amount,
        boss: ctx.accounts.boss.key(),
    });

    msg!("Offer vault withdraw successful: {} tokens", amount);
    Ok(())
}
