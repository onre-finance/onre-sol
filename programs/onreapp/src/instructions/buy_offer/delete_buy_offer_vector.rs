use super::state::{BuyOfferAccount, BuyOfferVector};
use crate::instructions::find_offer_mut;
use crate::state::State;
use anchor_lang::prelude::*;

/// Event emitted when a time vector is deleted from a buy offer.
#[event]
pub struct BuyOfferVectorDeletedEvent {
    pub offer_id: u64,
    pub vector_id: u64,
}

/// Account structure for deleting a time vector from a buy offer.
///
/// This struct defines the accounts required to delete a time vector from an existing buy offer.
/// Only the boss can delete time vectors from offers.
#[derive(Accounts)]
pub struct DeleteBuyOfferVector<'info> {
    /// The buy offer account containing all buy offers
    #[account(mut, seeds = [b"buy_offers"], bump)]
    pub buy_offer_account: AccountLoader<'info, BuyOfferAccount>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// The signer authorizing the time vector deletion (must be boss).
    #[account(mut)]
    pub boss: Signer<'info>,
}

/// Deletes a time vector from an existing buy offer.
///
/// Removes the specified time vector by setting it to default values.
/// The vector is identified by both offer_id and vector_id.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the operation.
/// - `offer_id`: ID of the buy offer containing the vector to delete.
/// - `vector_id`: ID of the vector to delete.
///
/// # Errors
/// - [`DeleteBuyOfferVectorErrorCode::OfferNotFound`] if offer_id is 0 or doesn't exist.
/// - [`DeleteBuyOfferVectorErrorCode::VectorNotFound`] if vector_id is 0 or doesn't exist in the offer.
pub fn delete_buy_offer_vector(
    ctx: Context<DeleteBuyOfferVector>,
    offer_id: u64,
    vector_id: u64,
) -> Result<()> {
    let buy_offer_account = &mut ctx.accounts.buy_offer_account.load_mut()?;

    // Validate inputs
    require!(offer_id != 0, DeleteBuyOfferVectorErrorCode::OfferNotFound);
    require!(
        vector_id != 0,
        DeleteBuyOfferVectorErrorCode::VectorNotFound
    );

    // Find the offer by offer_id
    let offer = find_offer_mut(buy_offer_account, offer_id)?;

    // Find and delete the vector by vector_id
    let vector_index = find_vector_index_by_id(&offer.vectors, vector_id)?;

    // Delete the vector by setting it to default
    offer.vectors[vector_index] = BuyOfferVector::default();

    msg!(
        "Time vector deleted from buy offer ID: {}, vector ID: {}",
        offer_id,
        vector_id
    );

    emit!(BuyOfferVectorDeletedEvent {
        offer_id,
        vector_id,
    });

    Ok(())
}

/// Finds the index of a vector by its ID in the vectors array.
fn find_vector_index_by_id(vectors: &[BuyOfferVector; 10], vector_id: u64) -> Result<usize> {
    vectors
        .iter()
        .position(|vector| vector.vector_id == vector_id && vector.vector_id != 0)
        .ok_or(DeleteBuyOfferVectorErrorCode::VectorNotFound.into())
}

/// Error codes for delete buy offer vector operations.
#[error_code]
pub enum DeleteBuyOfferVectorErrorCode {
    /// Triggered when the specified offer_id is 0 or not found.
    #[msg("Buy offer with the specified ID was not found")]
    OfferNotFound,

    /// Triggered when the specified vector_id is 0 or not found in the offer.
    #[msg("Vector with the specified ID was not found in the offer")]
    VectorNotFound,
}
