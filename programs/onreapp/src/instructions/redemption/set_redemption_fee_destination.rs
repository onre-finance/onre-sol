use crate::constants::seeds;
use crate::instructions::redemption::RedemptionFeeVaultAuthority;
use crate::state::State;
use anchor_lang::prelude::*;

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
        has_one = boss @ crate::OnreError::Unauthorized,
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

    /// System program required for account creation
    pub system_program: Program<'info, System>,
}

/// Sets (or updates) the redemption fee destination address.
///
/// Only updates the stored destination address; does not move any tokens.
/// Use `withdraw_redemption_fees` to sweep accumulated fees.
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
        crate::OnreError::NoChange
    );

    let vault_authority = &mut ctx.accounts.redemption_fee_vault_authority;
    vault_authority.fee_destination = fee_destination;
    vault_authority.bump = ctx.bumps.redemption_fee_vault_authority;

    emit!(RedemptionFeeDestinationUpdatedEvent {
        old_destination,
        new_destination: fee_destination,
    });

    Ok(())
}

// Error codes for set_redemption_fee_destination
