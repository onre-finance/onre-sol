use crate::constants::seeds;
use crate::instructions::DualRedemptionOfferAccount;
use crate::state::State;
use crate::utils::MAX_BASIS_POINTS;
use anchor_lang::prelude::*;

/// Event emitted when a dual redemption offer's fee is updated.
#[event]
pub struct DualRedemptionOfferFeeUpdatedEvent {
    pub offer_id: u64,
    pub old_fee_basis_points: u64,
    pub new_fee_basis_points: u64,
    pub boss: Pubkey,
}

/// Account structure for updating a dual redemption offer's fee.
///
/// This struct defines the accounts required to update the fee basis points of an existing dual redemption offer.
/// Only the boss can update offer fees.
#[derive(Accounts)]
pub struct UpdateDualRedemptionOfferFee<'info> {
    /// The dual redemption offer account containing all dual redemption offers
    #[account(mut, seeds = [seeds::DUAL_REDEMPTION_OFFERS], bump)]
    pub dual_redemption_offer_account: AccountLoader<'info, DualRedemptionOfferAccount>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Account<'info, State>,

    /// The signer authorizing the fee update (must be boss).
    #[account(mut)]
    pub boss: Signer<'info>,
}

/// Updates the fee basis points for an existing dual redemption offer.
///
/// Allows the boss to modify the fee charged when users take the dual redemption offer.
/// The fee is specified in basis points (e.g., 500 = 5%).
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the operation.
/// - `offer_id`: ID of the dual redemption offer to update.
/// - `new_fee_basis_points`: New fee in basis points (0-10000).
///
/// # Errors
/// - [`UpdateDualRedemptionOfferFeeErrorCode::OfferNotFound`] if offer_id doesn't exist.
/// - [`UpdateDualRedemptionOfferFeeErrorCode::InvalidFee`] if fee_basis_points > 10000.
pub fn update_dual_redemption_offer_fee(
    ctx: Context<UpdateDualRedemptionOfferFee>,
    offer_id: u64,
    new_fee_basis_points: u64,
) -> Result<()> {
    // Validate offer_id is not zero
    if offer_id == 0 {
        return Err(error!(UpdateDualRedemptionOfferFeeErrorCode::OfferNotFound));
    }

    // Validate fee is within valid range (0-10000 basis points = 0-100%)
    require!(
        new_fee_basis_points <= MAX_BASIS_POINTS,
        UpdateDualRedemptionOfferFeeErrorCode::InvalidFee
    );

    let dual_redemption_offer_account =
        &mut ctx.accounts.dual_redemption_offer_account.load_mut()?;

    // Find the offer by offer_id
    let offer_index = dual_redemption_offer_account
        .offers
        .iter()
        .position(|offer| offer.offer_id == offer_id)
        .ok_or(UpdateDualRedemptionOfferFeeErrorCode::OfferNotFound)?;

    let offer = &mut dual_redemption_offer_account.offers[offer_index];

    // Store old fee for event
    let old_fee_basis_points = offer.fee_basis_points;

    // Update the fee
    offer.fee_basis_points = new_fee_basis_points;

    msg!(
        "Dual redemption offer fee updated for ID: {}, old fee: {}, new fee: {}",
        offer_id,
        old_fee_basis_points,
        new_fee_basis_points
    );

    emit!(DualRedemptionOfferFeeUpdatedEvent {
        offer_id,
        old_fee_basis_points,
        new_fee_basis_points,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}

/// Error codes for update dual redemption offer fee operations.
#[error_code]
pub enum UpdateDualRedemptionOfferFeeErrorCode {
    /// Triggered when the offer_id doesn't exist.
    #[msg("Offer not found")]
    OfferNotFound,

    /// Triggered when fee_basis_points is greater than 10000.
    #[msg("Invalid fee: fee_basis_points must be <= 10000")]
    InvalidFee,
}
