use super::offer_state::OfferAccount;
use crate::constants::seeds;
use crate::instructions::find_offer_mut;
use crate::state::State;
use crate::utils::MAX_BASIS_POINTS;
use anchor_lang::prelude::*;

/// Event emitted when a offer's fee is updated.
#[event]
pub struct OfferFeeUpdatedEvent {
    pub offer_id: u64,
    pub old_fee_basis_points: u64,
    pub new_fee_basis_points: u64,
    pub boss: Pubkey,
}

/// Account structure for updating a offer's fee.
///
/// This struct defines the accounts required to update the fee basis points of an existing offer.
/// Only the boss can update offer fees.
#[derive(Accounts)]
pub struct UpdateOfferFee<'info> {
    /// The offer account containing all offers
    #[account(mut, seeds = [seeds::OFFERS], bump)]
    pub offer_account: AccountLoader<'info, OfferAccount>,

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
/// - `offer_id`: ID of the offer to update.
/// - `new_fee_basis_points`: New fee in basis points (0-10000).
///
/// # Errors
/// - [`UpdateOfferFeeErrorCode::OfferNotFound`] if offer_id doesn't exist.
/// - [`UpdateOfferFeeErrorCode::InvalidFee`] if fee_basis_points > 10000.
pub fn update_offer_fee(
    ctx: Context<UpdateOfferFee>,
    offer_id: u64,
    new_fee_basis_points: u64,
) -> Result<()> {
    // Validate fee is within valid range (0-10000 basis points = 0-100%)
    require!(
        new_fee_basis_points <= MAX_BASIS_POINTS,
        UpdateOfferFeeErrorCode::InvalidFee
    );

    let offer_account = &mut ctx.accounts.offer_account.load_mut()?;

    // Find the offer by offer_id
    let offer = find_offer_mut(offer_account, offer_id)
        .map_err(|_| UpdateOfferFeeErrorCode::OfferNotFound)?;

    // Store old fee for event
    let old_fee_basis_points = offer.fee_basis_points;

    // Update the fee
    offer.fee_basis_points = new_fee_basis_points;

    msg!(
        "Offer fee updated for ID: {}, old fee: {}, new fee: {}",
        offer_id,
        old_fee_basis_points,
        new_fee_basis_points
    );

    emit!(OfferFeeUpdatedEvent {
        offer_id,
        old_fee_basis_points,
        new_fee_basis_points,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}

/// Error codes for update offer fee operations.
#[error_code]
pub enum UpdateOfferFeeErrorCode {
    /// Triggered when the offer_id doesn't exist.
    #[msg("Offer not found")]
    OfferNotFound,

    /// Triggered when fee_basis_points is greater than 10000.
    #[msg("Invalid fee: fee_basis_points must be <= 10000")]
    InvalidFee,
}
