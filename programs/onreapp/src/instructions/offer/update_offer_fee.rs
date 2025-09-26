use crate::constants::seeds;
use crate::instructions::Offer;
use crate::state::State;
use crate::utils::MAX_BASIS_POINTS;
use crate::OfferCoreError;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

/// Event emitted when a offer's fee is updated.
#[event]
pub struct OfferFeeUpdatedEvent {
    pub offer_pda: Pubkey,
    pub old_fee_basis_points: u32,
    pub new_fee_basis_points: u32,
    pub boss: Pubkey,
}

/// Account structure for updating a offer's fee.
///
/// This struct defines the accounts required to update the fee basis points of an existing offer.
/// Only the boss can update offer fees.
#[derive(Accounts)]
pub struct UpdateOfferFee<'info> {
    /// The offer account containing all offers
    #[account(
        mut,
        seeds = [
            seeds::OFFER,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref()
        ],
        bump
    )]
    pub offer: AccountLoader<'info, Offer>,

    #[account(
        constraint =
            token_in_mint.key() == offer.load()?.token_in_mint
            @ OfferCoreError::InvalidTokenInMint
    )]
    pub token_in_mint: InterfaceAccount<'info, Mint>,

    #[account(
        constraint =
            token_out_mint.key() == offer.load()?.token_out_mint
            @ OfferCoreError::InvalidTokenOutMint
    )]
    pub token_out_mint: InterfaceAccount<'info, Mint>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Account<'info, State>,

    /// The signer authorizing the fee update (must be boss).
    #[account(mut)]
    pub boss: Signer<'info>,
}

/// Updates the fee basis points for an existing offer.
///
/// Allows the boss to modify the fee charged when users take the offer.
/// The fee is specified in basis points (e.g., 500 = 5%).
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the operation.
/// - `new_fee_basis_points`: New fee in basis points (0-10000).
///
/// # Errors
/// - [`UpdateOfferFeeErrorCode::InvalidFee`] if fee_basis_points > 10000.
pub fn update_offer_fee(ctx: Context<UpdateOfferFee>, new_fee_basis_points: u32) -> Result<()> {
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

/// Error codes for update offer fee operations.
#[error_code]
pub enum UpdateOfferFeeErrorCode {
    /// Triggered when fee_basis_points is greater than 10000.
    #[msg("Invalid fee: fee_basis_points must be <= 10000")]
    InvalidFee,

    #[msg("Invalid token in mint for offer")]
    InvalidTokenInMint,

    #[msg("Invalid token out mint for offer")]
    InvalidTokenOutMint,
}
