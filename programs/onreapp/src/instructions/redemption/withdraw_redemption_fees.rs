use crate::constants::seeds;
use crate::instructions::redemption::RedemptionFeeVaultAuthority;
use crate::state::State;
use crate::utils::transfer_tokens;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

/// Event emitted when redemption fees are withdrawn from the vault
#[event]
pub struct RedemptionFeesWithdrawnEvent {
    /// Destination that received the fees
    pub destination: Pubkey,
    /// Amount of token_in withdrawn
    pub amount: u64,
}

/// Account structure for withdrawing redemption fees
#[derive(Accounts)]
pub struct WithdrawRedemptionFees<'info> {
    /// Program state account — boss access control
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss @ crate::OnreError::Unauthorized,
    )]
    pub state: Box<Account<'info, State>>,

    /// Boss must sign; also pays for any new ATA creation
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Global fee vault authority PDA
    #[account(
        seeds = [seeds::REDEMPTION_FEE_VAULT_AUTHORITY],
        bump = redemption_fee_vault_authority.bump,
    )]
    pub redemption_fee_vault_authority: Account<'info, RedemptionFeeVaultAuthority>,

    /// ATA of the fee vault — source of the withdrawal
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = redemption_fee_vault_authority,
        associated_token::token_program = token_in_program,
    )]
    pub fee_vault_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// ATA of the destination — created if needed
    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = token_in_mint,
        associated_token::authority = destination,
        associated_token::token_program = token_in_program,
    )]
    pub destination_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Destination wallet; boss decides where fees go
    /// CHECK: boss-controlled destination, no additional validation required
    pub destination: UncheckedAccount<'info>,

    /// The token mint whose fees are being withdrawn
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token program for token_in
    pub token_in_program: Interface<'info, TokenInterface>,

    /// Associated Token Program for ATA creation
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// System program required for account creation
    pub system_program: Program<'info, System>,
}

/// Withdraws accumulated redemption fees from the vault to a destination chosen by the boss.
///
/// # Arguments
/// * `ctx`    - Instruction context
/// * `amount` - Amount to withdraw. Pass `0` to withdraw the full vault balance.
pub fn withdraw_redemption_fees(ctx: Context<WithdrawRedemptionFees>, amount: u64) -> Result<()> {
    let vault_balance = ctx.accounts.fee_vault_token_in_account.amount;

    let effective_amount = if amount == 0 { vault_balance } else { amount };

    require!(effective_amount > 0, crate::OnreError::ZeroBalance);
    require!(
        effective_amount <= vault_balance,
        crate::OnreError::InsufficientBalance
    );

    let bump = ctx.accounts.redemption_fee_vault_authority.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[seeds::REDEMPTION_FEE_VAULT_AUTHORITY, &[bump]]];

    transfer_tokens(
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_in_program,
        &ctx.accounts.fee_vault_token_in_account,
        &ctx.accounts.destination_token_in_account,
        &ctx.accounts
            .redemption_fee_vault_authority
            .to_account_info(),
        Some(signer_seeds),
        effective_amount,
    )?;

    emit!(RedemptionFeesWithdrawnEvent {
        destination: ctx.accounts.destination.key(),
        amount: effective_amount,
    });

    Ok(())
}
