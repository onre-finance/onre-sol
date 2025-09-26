use super::offer_state::{Offer, OfferVector};
use crate::constants::seeds;
use crate::instructions::{find_active_vector_at, find_vector_index_by_start_time};
use crate::state::State;
use crate::OfferCoreError;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

/// Event emitted when a time vector is deleted from a offer.
#[event]
pub struct OfferVectorDeletedEvent {
    pub offer_pda: Pubkey,
    pub vector_start_time: u64,
}

/// Account structure for deleting a time vector from an offer.
///
/// This struct defines the accounts required to delete a time vector from an existing offer.
/// Only the boss can delete time vectors from offers.
#[derive(Accounts)]
pub struct DeleteOfferVector<'info> {
    /// The individual offer account
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

    /// The signer authorizing the time vector deletion (must be boss).
    #[account(mut)]
    pub boss: Signer<'info>,
}

/// Deletes a time vector from an existing offer.
///
/// Removes the specified time vector by setting it to default values.
/// The vector is identified by vector_id within the specific offer.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the operation.
/// - `vector_id`: ID of the vector to delete.
///
/// # Errors
/// - [`DeleteOfferVectorErrorCode::VectorNotFound`] if vector_id is 0 or doesn't exist in the offer.
pub fn delete_offer_vector(ctx: Context<DeleteOfferVector>, vector_start_time: u64) -> Result<()> {
    let offer = &mut ctx.accounts.offer.load_mut()?;

    // Validate inputs
    require!(vector_start_time != 0, DeleteOfferVectorErrorCode::VectorNotFound);

    // Find and delete the vector by vector_start_time
    let vector_index = find_vector_index_by_start_time(&offer, vector_start_time)
        .ok_or_else(|| error!(DeleteOfferVectorErrorCode::VectorNotFound))?;

    let current_vector = find_active_vector_at(offer, Clock::get()?.unix_timestamp as u64);

    if current_vector.is_ok() {
        let prev_vector = find_active_vector_at(offer, current_vector?.start_time - 1);

        if prev_vector.is_ok() {
            require!(
                prev_vector?.start_time != vector_start_time,
                DeleteOfferVectorErrorCode::CannotDeletePreviousVector
            );
        }
    }

    // Delete the vector by setting it to default
    offer.vectors[vector_index] = OfferVector::default();

    msg!(
        "Time vector deleted from offer: {}, vector start_time: {}",
        ctx.accounts.offer.key(),
        vector_start_time
    );

    emit!(OfferVectorDeletedEvent {
        offer_pda: ctx.accounts.offer.key(),
        vector_start_time,
    });

    Ok(())
}

/// Error codes for delete offer vector operations.
#[error_code]
pub enum DeleteOfferVectorErrorCode {
    /// Triggered when the specified vector start_time is 0 or not found in the offer.
    #[msg("Vector not found")]
    VectorNotFound,

    #[msg("Cannot delete previously active vector")]
    CannotDeletePreviousVector,
}
