use crate::constants::seeds;
use crate::instructions::SingleRedemptionOfferAccount;
use crate::state::State;
use crate::utils::MAX_BASIS_POINTS;
use anchor_lang::prelude::*;

/// Event emitted when a single redemption offer's fee is updated.
#[event]
pub struct SingleRedemptionOfferFeeUpdatedEvent {
    pub offer_id: u64,
    pub old_fee_basis_points: u64,
    pub new_fee_basis_points: u64,
    pub boss: Pubkey,
}

/// Account structure for updating a single redemption offer's fee.
///
/// This struct defines the accounts required to update the fee basis points of an existing single redemption offer.
/// Only the boss can update offer fees.
#[derive(Accounts)]
pub struct UpdateSingleRedemptionOfferFee<'info> {
    /// The single redemption offer account containing all single redemption offers
    #[account(mut, seeds = [seeds::SINGLE_REDEMPTION_OFFERS], bump)]
    pub single_redemption_offer_account: AccountLoader<'info, SingleRedemptionOfferAccount>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Account<'info, State>,

    /// The signer authorizing the fee update (must be boss).
    #[account(mut)]
    pub boss: Signer<'info>,
}

/// Updates the fee basis points for an existing single redemption offer.
///
/// Allows the boss to modify the fee charged when users take the single redemption offer.
/// The fee is specified in basis points (e.g., 500 = 5%).
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the operation.
/// - `offer_id`: ID of the single redemption offer to update.
/// - `new_fee_basis_points`: New fee in basis points (0-10000).
///
/// # Errors
/// - [`UpdateSingleRedemptionOfferFeeErrorCode::OfferNotFound`] if offer_id doesn't exist.
/// - [`UpdateSingleRedemptionOfferFeeErrorCode::InvalidFee`] if fee_basis_points > 10000.
pub fn update_single_redemption_offer_fee(
    ctx: Context<UpdateSingleRedemptionOfferFee>,
    offer_id: u64,
    new_fee_basis_points: u64,
) -> Result<()> {
    // Validate offer_id is not zero
    if offer_id == 0 {
        return Err(error!(
            UpdateSingleRedemptionOfferFeeErrorCode::OfferNotFound
        ));
    }

    // Validate fee is within valid range (0-10000 basis points = 0-100%)
    require!(
        new_fee_basis_points <= MAX_BASIS_POINTS,
        UpdateSingleRedemptionOfferFeeErrorCode::InvalidFee
    );

    let single_redemption_offer_account =
        &mut ctx.accounts.single_redemption_offer_account.load_mut()?;

    // Find the offer by offer_id
    let offer_index = single_redemption_offer_account
        .offers
        .iter()
        .position(|offer| offer.offer_id == offer_id)
        .ok_or(UpdateSingleRedemptionOfferFeeErrorCode::OfferNotFound)?;

    let offer = &mut single_redemption_offer_account.offers[offer_index];

    // Store old fee for event
    let old_fee_basis_points = offer.fee_basis_points;

    // Update the fee
    offer.fee_basis_points = new_fee_basis_points;

    msg!(
        "Single redemption offer fee updated for ID: {}, old fee: {}, new fee: {}",
        offer_id,
        old_fee_basis_points,
        new_fee_basis_points
    );

    emit!(SingleRedemptionOfferFeeUpdatedEvent {
        offer_id,
        old_fee_basis_points,
        new_fee_basis_points,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}

/// Error codes for update single redemption offer fee operations.
#[error_code]
pub enum UpdateSingleRedemptionOfferFeeErrorCode {
    /// Triggered when the offer_id doesn't exist.
    #[msg("Offer not found")]
    OfferNotFound,

    /// Triggered when fee_basis_points is greater than 10000.
    #[msg("Invalid fee: fee_basis_points must be <= 10000")]
    InvalidFee,
}
