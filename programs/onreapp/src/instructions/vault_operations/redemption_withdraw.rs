use crate::constants::seeds;
use crate::state::State;
use crate::utils::transfer_tokens;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

/// Event emitted when tokens are successfully withdrawn from the redemption vault
///
/// Provides transparency for tracking redemption vault withdrawals and fund management.
#[event]
pub struct RedemptionVaultWithdrawEvent {
    /// The token mint that was withdrawn
    pub mint: Pubkey,
    /// Amount of tokens withdrawn from the vault
    pub amount: u64,
    /// The boss account that performed the withdrawal
    pub boss: Pubkey,
}

/// Account structure for withdrawing tokens from the redemption vault
///
/// This struct defines the accounts required for the boss to recover tokens
/// from the redemption vault, enabling fund management and reallocation of
/// vault reserves when needed.
#[derive(Accounts)]
pub struct RedemptionVaultWithdraw<'info> {
    /// Program-derived authority that controls redemption vault token accounts
    ///
    /// This PDA manages the redemption vault token accounts and signs the withdrawal
    /// transfer using program-derived signatures.
    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::REDEMPTION_OFFER_VAULT_AUTHORITY], bump)]
    pub redemption_vault_authority: AccountInfo<'info>,

    /// The token mint for the withdrawal operation
    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Boss's token account serving as the destination for withdrawn tokens
    ///
    /// Created automatically if it doesn't exist. Receives tokens withdrawn
    /// from the redemption vault for boss fund management.
    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = token_mint,
        associated_token::authority = boss,
        associated_token::token_program = token_program
    )]
    pub boss_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Redemption vault's token account serving as the source of withdrawn tokens
    ///
    /// Must have sufficient balance to cover the requested withdrawal amount.
    /// Controlled by the redemption vault authority PDA.
    #[account(
        mut,
        associated_token::mint = token_mint,
        associated_token::authority = redemption_vault_authority,
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

/// Withdraws tokens from the redemption vault for fund management
///
/// This instruction allows the boss to recover tokens from the redemption vault,
/// enabling reallocation of vault reserves, emergency fund recovery, or
/// redistribution of unused vault tokens. Uses program-derived signatures
/// to authorize the transfer from redemption vault to boss accounts.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `amount` - Amount of tokens to withdraw from the redemption vault
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
/// - Transfers tokens from redemption vault account to boss account
/// - Creates boss token account if it doesn't exist
/// - Reduces available tokens in redemption vault reserves
///
/// # Events
/// * `RedemptionVaultWithdrawEvent` - Emitted with mint, amount, and withdrawer details
pub fn redemption_vault_withdraw(ctx: Context<RedemptionVaultWithdraw>, amount: u64) -> Result<()> {
    // Create signer seeds for redemption vault authority
    let redemption_vault_authority_seeds = &[
        seeds::REDEMPTION_OFFER_VAULT_AUTHORITY,
        &[ctx.bumps.redemption_vault_authority],
    ];
    let signer_seeds = &[&redemption_vault_authority_seeds[..]];

    // Transfer tokens from redemption vault to boss
    transfer_tokens(
        &ctx.accounts.token_mint,
        &ctx.accounts.token_program,
        &ctx.accounts.vault_token_account,
        &ctx.accounts.boss_token_account,
        &ctx.accounts.redemption_vault_authority.to_account_info(),
        Some(signer_seeds),
        amount,
    )?;

    emit!(RedemptionVaultWithdrawEvent {
        mint: ctx.accounts.token_mint.key(),
        amount,
        boss: ctx.accounts.boss.key(),
    });

    msg!("Redemption vault withdraw successful: {} tokens", amount);
    Ok(())
}
