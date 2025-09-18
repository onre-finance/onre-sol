use super::buy_offer_state::BuyOfferAccount;
use crate::constants::seeds;
use crate::instructions::find_offer_mut;
use crate::state::State;
use crate::utils::MAX_BASIS_POINTS;
use anchor_lang::prelude::*;

/// Event emitted when a buy offer's fee is updated.
#[event]
pub struct BuyOfferFeeUpdatedEvent {
    pub offer_id: u64,
    pub old_fee_basis_points: u64,
    pub new_fee_basis_points: u64,
    pub boss: Pubkey,
}

/// Account structure for updating a buy offer's fee.
///
/// This struct defines the accounts required to update the fee basis points of an existing buy offer.
/// Only the boss can update offer fees.
#[derive(Accounts)]
pub struct UpdateBuyOfferFee<'info> {
    /// The buy offer account containing all buy offers
    #[account(mut, seeds = [seeds::BUY_OFFERS], bump)]
    pub buy_offer_account: AccountLoader<'info, BuyOfferAccount>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Account<'info, State>,

    /// The signer authorizing the fee update (must be boss).
    #[account(mut)]
    pub boss: Signer<'info>,
}

/// Updates the fee basis points for an existing buy offer.
///
/// Allows the boss to modify the fee charged when users take the buy offer.
/// The fee is specified in basis points (e.g., 500 = 5%).
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the operation.
/// - `offer_id`: ID of the buy offer to update.
/// - `new_fee_basis_points`: New fee in basis points (0-10000).
///
/// # Errors
/// - [`UpdateBuyOfferFeeErrorCode::OfferNotFound`] if offer_id doesn't exist.
/// - [`UpdateBuyOfferFeeErrorCode::InvalidFee`] if fee_basis_points > 10000.
pub fn update_buy_offer_fee(
    ctx: Context<UpdateBuyOfferFee>,
    offer_id: u64,
    new_fee_basis_points: u64,
) -> Result<()> {
    // Validate fee is within valid range (0-10000 basis points = 0-100%)
    require!(
        new_fee_basis_points <= MAX_BASIS_POINTS,
        UpdateBuyOfferFeeErrorCode::InvalidFee
    );

    let buy_offer_account = &mut ctx.accounts.buy_offer_account.load_mut()?;

    // Find the offer by offer_id
    let offer = find_offer_mut(buy_offer_account, offer_id)
        .map_err(|_| UpdateBuyOfferFeeErrorCode::OfferNotFound)?;

    // Store old fee for event
    let old_fee_basis_points = offer.fee_basis_points;

    // Update the fee
    offer.fee_basis_points = new_fee_basis_points;

    msg!(
        "Buy offer fee updated for ID: {}, old fee: {}, new fee: {}",
        offer_id,
        old_fee_basis_points,
        new_fee_basis_points
    );

    emit!(BuyOfferFeeUpdatedEvent {
        offer_id,
        old_fee_basis_points,
        new_fee_basis_points,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}

/// Error codes for update buy offer fee operations.
#[error_code]
pub enum UpdateBuyOfferFeeErrorCode {
    /// Triggered when the offer_id doesn't exist.
    #[msg("Offer not found")]
    OfferNotFound,

    /// Triggered when fee_basis_points is greater than 10000.
    #[msg("Invalid fee: fee_basis_points must be <= 10000")]
    InvalidFee,
}
