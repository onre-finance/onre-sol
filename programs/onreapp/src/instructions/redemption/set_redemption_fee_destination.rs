use crate::constants::seeds;
use crate::instructions::redemption::RedemptionFeeVaultAuthority;
use crate::state::State;
use crate::utils::transfer_tokens;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

/// Event emitted when the redemption fee destination is updated
#[event]
pub struct RedemptionFeeDestinationUpdatedEvent {
    /// Previous fee destination (Pubkey::default() means the vault PDA)
    pub old_destination: Pubkey,
    /// New fee destination (Pubkey::default() means the vault PDA)
    pub new_destination: Pubkey,
}

/// Account structure for setting the redemption fee destination
#[derive(Accounts)]
#[instruction(fee_destination: Pubkey)]
pub struct SetRedemptionFeeDestination<'info> {
    /// Program state account — boss access control
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss @ SetRedemptionFeeDestinationErrorCode::Unauthorized,
    )]
    pub state: Box<Account<'info, State>>,

    /// Boss must sign; also pays for any new account creation
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Global fee vault authority PDA — created on first call
    #[account(
        init_if_needed,
        payer = boss,
        space = 8 + RedemptionFeeVaultAuthority::INIT_SPACE,
        seeds = [seeds::REDEMPTION_FEE_VAULT_AUTHORITY],
        bump,
    )]
    pub redemption_fee_vault_authority: Account<'info, RedemptionFeeVaultAuthority>,

    /// ATA of the old fee — sweep source
    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = token_in_mint,
        associated_token::authority = redemption_fee_vault_authority,
        associated_token::token_program = token_in_program,
    )]
    pub fee_vault_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// ATA of the new fee destination for token_in — sweep target
    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = token_in_mint,
        associated_token::authority = new_fee_destination,
        associated_token::token_program = token_in_program,
    )]
    pub new_destination_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The new fee destination account; key must equal the `fee_destination` argument
    /// CHECK: validated via `address` constraint against the instruction argument
    #[account(address = fee_destination)]
    pub new_fee_destination: UncheckedAccount<'info>,

    /// The token mint whose fees are being rerouted
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token program for token_in
    pub token_in_program: Interface<'info, TokenInterface>,

    /// Associated Token Program for ATA creation
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// System program required for account creation
    pub system_program: Program<'info, System>,
}

/// Sets (or updates) the redemption fee destination
///
/// If the fee vault ATA has a non-zero balance it is swept to the new destination
/// before the stored address is updated.
///
/// # Arguments
/// * `ctx`             - Instruction context
/// * `fee_destination` - New destination for redemption fees.
///                       Pass `Pubkey::default()` to revert to vault accumulation.
pub fn set_redemption_fee_destination(
    ctx: Context<SetRedemptionFeeDestination>,
    fee_destination: Pubkey,
) -> Result<()> {
    let old_destination = ctx.accounts.redemption_fee_vault_authority.fee_destination;

    require!(
        old_destination != fee_destination,
        SetRedemptionFeeDestinationErrorCode::NoChange
    );

    // Sweep any accumulated fees to the new destination
    let vault_balance = ctx.accounts.fee_vault_token_in_account.amount;
    if vault_balance > 0 {
        let bump = ctx.bumps.redemption_fee_vault_authority;
        let signer_seeds: &[&[&[u8]]] = &[&[seeds::REDEMPTION_FEE_VAULT_AUTHORITY, &[bump]]];

        transfer_tokens(
            &ctx.accounts.token_in_mint,
            &ctx.accounts.token_in_program,
            &ctx.accounts.fee_vault_token_in_account,
            &ctx.accounts.new_destination_token_in_account,
            &ctx.accounts
                .redemption_fee_vault_authority
                .to_account_info(),
            Some(signer_seeds),
            vault_balance,
        )?;
    }

    let vault_authority = &mut ctx.accounts.redemption_fee_vault_authority;
    vault_authority.fee_destination = fee_destination;
    vault_authority.bump = ctx.bumps.redemption_fee_vault_authority;

    emit!(RedemptionFeeDestinationUpdatedEvent {
        old_destination,
        new_destination: fee_destination,
    });

    Ok(())
}

/// Error codes for set_redemption_fee_destination
#[error_code]
pub enum SetRedemptionFeeDestinationErrorCode {
    /// Caller is not the boss
    #[msg("Unauthorized: boss signature required")]
    Unauthorized,

    /// The new destination is identical to the current one
    #[msg("Fee destination is already set to this value")]
    NoChange,
}
