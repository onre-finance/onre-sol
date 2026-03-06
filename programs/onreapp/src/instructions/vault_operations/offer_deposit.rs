use crate::constants::seeds;
use crate::utils::transfer_tokens;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

/// Event emitted when tokens are successfully deposited to the offer vault
///
/// Provides transparency for tracking vault funding and token availability.
#[event]
pub struct OfferVaultDepositEvent {
    /// The token mint that was deposited
    pub mint: Pubkey,
    /// Amount of tokens deposited to the vault
    pub amount: u64,
    /// The depositor account that made the deposit
    pub depositor: Pubkey,
}

/// Account structure for depositing tokens to the offer vault
///
/// This struct defines the accounts required to fund the offer vault
/// with tokens that can be distributed during offer executions when the program
/// lacks mint authority and must transfer from pre-funded reserves.
#[derive(Accounts)]
pub struct OfferVaultDeposit<'info> {
    /// Program-derived authority that controls vault token accounts
    ///
    /// This PDA manages the vault token accounts and enables the program
    /// to distribute tokens during offer executions.
    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::OFFER_VAULT_AUTHORITY], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    /// The token mint for the deposit operation
    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Depositor's token account serving as the source of deposited tokens
    ///
    /// Must have sufficient balance to cover the requested deposit amount.
    #[account(
        mut,
        associated_token::mint = token_mint,
        associated_token::authority = depositor,
        associated_token::token_program = token_program
    )]
    pub depositor_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Vault's token account serving as the destination for deposited tokens
    ///
    /// Created automatically if it doesn't exist. Stores tokens that can be
    /// distributed during offer executions when minting is not available.
    #[account(
        init_if_needed,
        payer = depositor,
        associated_token::mint = token_mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = token_program
    )]
    pub vault_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The depositor account paying for account creation and providing tokens
    #[account(mut)]
    pub depositor: Signer<'info>,

    /// Token program interface for transfer operations
    pub token_program: Interface<'info, TokenInterface>,

    /// Associated Token Program for automatic token account creation
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// System program for account creation and rent payment
    pub system_program: Program<'info, System>,
}

/// Deposits tokens into the offer vault for distribution during offer executions
///
/// This instruction allows any wallet to fund the offer vault with tokens that can be
/// distributed to users when offers are executed and the program lacks mint authority.
/// This supports the transfer-based token distribution mechanism as an alternative
/// to the burn/mint architecture.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `amount` - Amount of tokens to deposit into the vault
///
/// # Returns
/// * `Ok(())` - If the deposit completes successfully
/// * `Err(_)` - If transfer fails or insufficient balance
///
/// # Effects
/// - Transfers tokens from depositor account to vault account
/// - Creates vault token account if it doesn't exist
/// - Increases available tokens for offer distributions
///
/// # Events
/// * `OfferVaultDepositEvent` - Emitted with mint, amount, and depositor details
pub fn offer_vault_deposit(ctx: Context<OfferVaultDeposit>, amount: u64) -> Result<()> {
    // Transfer tokens from depositor to vault
    transfer_tokens(
        &ctx.accounts.token_mint,
        &ctx.accounts.token_program,
        &ctx.accounts.depositor_token_account,
        &ctx.accounts.vault_token_account,
        &ctx.accounts.depositor,
        None,
        amount,
    )?;

    emit!(OfferVaultDepositEvent {
        mint: ctx.accounts.token_mint.key(),
        amount,
        depositor: ctx.accounts.depositor.key(),
    });

    msg!("Offer vault deposit successful: {} tokens", amount);
    Ok(())
}
