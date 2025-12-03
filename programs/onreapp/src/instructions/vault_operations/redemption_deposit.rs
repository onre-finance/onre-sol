use crate::constants::seeds;
use crate::state::State;
use crate::utils::transfer_tokens;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

/// Event emitted when tokens are successfully deposited to the redemption vault
///
/// Provides transparency for tracking redemption vault funding and token availability.
#[event]
pub struct RedemptionVaultDepositEvent {
    /// The token mint that was deposited
    pub mint: Pubkey,
    /// Amount of tokens deposited to the vault
    pub amount: u64,
    /// The boss account that made the deposit
    pub boss: Pubkey,
}

/// Account structure for depositing tokens to the redemption vault
///
/// This struct defines the accounts required for the boss to fund the redemption vault
/// with tokens that can be distributed during redemption executions when the program
/// lacks mint authority and must transfer from pre-funded reserves.
#[derive(Accounts)]
pub struct RedemptionVaultDeposit<'info> {
    /// Program-derived authority that controls redemption vault token accounts
    ///
    /// This PDA manages the redemption vault token accounts and enables the program
    /// to distribute tokens during redemption executions.
    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::REDEMPTION_OFFER_VAULT_AUTHORITY], bump)]
    pub redemption_vault_authority: AccountInfo<'info>,

    /// The token mint for the deposit operation
    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Boss's token account serving as the source of deposited tokens
    ///
    /// Must have sufficient balance to cover the requested deposit amount.
    #[account(
        mut,
        associated_token::mint = token_mint,
        associated_token::authority = boss,
        associated_token::token_program = token_program
    )]
    pub boss_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Redemption vault's token account serving as the destination for deposited tokens
    ///
    /// Created automatically if it doesn't exist. Stores tokens that can be
    /// distributed during redemption executions when minting is not available.
    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = token_mint,
        associated_token::authority = redemption_vault_authority,
        associated_token::token_program = token_program
    )]
    pub vault_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The boss account authorized to deposit tokens and pay for account creation
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

/// Deposits tokens into the redemption vault for distribution during redemption executions
///
/// This instruction allows the boss to fund the redemption vault with tokens that can be
/// distributed to users when redemptions are executed and the program lacks mint authority.
/// This supports the transfer-based token distribution mechanism as an alternative
/// to the burn/mint architecture.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `amount` - Amount of tokens to deposit into the redemption vault
///
/// # Returns
/// * `Ok(())` - If the deposit completes successfully
/// * `Err(_)` - If transfer fails or insufficient balance
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss account must match the one stored in program state
///
/// # Effects
/// - Transfers tokens from boss account to redemption vault account
/// - Creates redemption vault token account if it doesn't exist
/// - Increases available tokens for redemption distributions
///
/// # Events
/// * `RedemptionVaultDepositEvent` - Emitted with mint, amount, and depositor details
pub fn redemption_vault_deposit(ctx: Context<RedemptionVaultDeposit>, amount: u64) -> Result<()> {
    // Transfer tokens from boss to redemption vault
    transfer_tokens(
        &ctx.accounts.token_mint,
        &ctx.accounts.token_program,
        &ctx.accounts.boss_token_account,
        &ctx.accounts.vault_token_account,
        &ctx.accounts.boss,
        None,
        amount,
    )?;

    emit!(RedemptionVaultDepositEvent {
        mint: ctx.accounts.token_mint.key(),
        amount,
        boss: ctx.accounts.boss.key(),
    });

    msg!("Redemption vault deposit successful: {} tokens", amount);
    Ok(())
}
