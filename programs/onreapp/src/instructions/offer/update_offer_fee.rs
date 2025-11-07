use crate::constants::seeds;
use crate::instructions::Offer;
use crate::state::State;
use crate::utils::MAX_BASIS_POINTS;
use crate::OfferCoreError;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

/// Event emitted when an offer's fee is successfully updated
///
/// Provides transparency for tracking fee changes and offer configuration modifications.
#[event]
pub struct OfferFeeUpdatedEvent {
    /// The PDA address of the offer whose fee was updated
    pub offer_pda: Pubkey,
    /// Previous fee in basis points (10000 = 100%)
    pub old_fee_basis_points: u16,
    /// New fee in basis points (10000 = 100%)
    pub new_fee_basis_points: u16,
    /// The boss account that authorized the fee update
    pub boss: Pubkey,
}

/// Account structure for updating an offer's fee configuration
///
/// This struct defines the accounts required to modify the fee basis points
/// charged when users execute offers. Only the boss can update offer fees.
#[derive(Accounts)]
pub struct UpdateOfferFee<'info> {
    /// The offer account whose fee will be updated
    ///
    /// This account is validated as a PDA derived from token mint addresses
    /// and contains the fee configuration to be modified.
    #[account(
        mut,
        seeds = [
            seeds::OFFER,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref()
        ],
        bump = offer.load()?.bump
    )]
    pub offer: AccountLoader<'info, Offer>,

    /// The input token mint account for offer validation
    #[account(
        constraint =
            token_in_mint.key() == offer.load()?.token_in_mint
            @ OfferCoreError::InvalidTokenInMint
    )]
    pub token_in_mint: InterfaceAccount<'info, Mint>,

    /// The output token mint account for offer validation
    #[account(
        constraint =
            token_out_mint.key() == offer.load()?.token_out_mint
            @ OfferCoreError::InvalidTokenOutMint
    )]
    pub token_out_mint: InterfaceAccount<'info, Mint>,

    /// Program state account containing boss authorization
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss)]
    pub state: Account<'info, State>,

    /// The boss account authorized to update offer fees
    pub boss: Signer<'info>,
}

/// Updates the fee configuration for an existing offer
///
/// This instruction allows the boss to modify the fee charged when users execute
/// offers. The fee applies to all future offer executions and is deducted from
/// the user's token_in payment before calculating exchange amounts.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `new_fee_basis_points` - New fee in basis points (10000 = 100%, 500 = 5%)
///
/// # Returns
/// * `Ok(())` - If the fee is successfully updated
/// * `Err(UpdateOfferFeeErrorCode::InvalidFee)` - If fee exceeds 10000 basis points
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss account must match the one stored in program state
///
/// # Effects
/// - Updates the offer's fee_basis_points field
/// - Affects all future offer executions
/// - Does not modify existing pricing vectors
///
/// # Events
/// * `OfferFeeUpdatedEvent` - Emitted with old and new fee values
pub fn update_offer_fee(ctx: Context<UpdateOfferFee>, new_fee_basis_points: u16) -> Result<()> {
    // Validate fee is within valid range (0-10000 basis points = 0-100%)
    require!(
        new_fee_basis_points <= MAX_BASIS_POINTS,
        UpdateOfferFeeErrorCode::InvalidFee
    );

    let offer = &mut ctx.accounts.offer.load_mut()?;

    // Store old fee for event
    let old_fee_basis_points = offer.fee_basis_points;

    // Update the fee
    offer.fee_basis_points = new_fee_basis_points;

    msg!(
        "Offer fee updated for offer: {}, old fee: {}, new fee: {}",
        ctx.accounts.offer.key(),
        old_fee_basis_points,
        new_fee_basis_points
    );

    emit!(OfferFeeUpdatedEvent {
        offer_pda: ctx.accounts.offer.key(),
        old_fee_basis_points,
        new_fee_basis_points,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}

/// Error codes for update offer fee operations
#[error_code]
pub enum UpdateOfferFeeErrorCode {
    /// Fee basis points exceeds maximum allowed value of 10000 (100%)
    #[msg("Invalid fee: fee_basis_points must be <= 10000")]
    InvalidFee,

    /// The provided token_in mint does not match the offer's expected mint
    #[msg("Invalid token in mint for offer")]
    InvalidTokenInMint,

    /// The provided token_out mint does not match the offer's expected mint
    #[msg("Invalid token out mint for offer")]
    InvalidTokenOutMint,
}
