use super::offer_state::{Offer, OfferVector};
use crate::constants::seeds;
use crate::instructions::{find_active_vector_at, find_vector_index_by_start_time};
use crate::state::State;
use crate::OfferCoreError;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

/// Event emitted when a pricing vector is successfully deleted from an offer
///
/// Provides transparency for tracking pricing vector removals and offer configuration changes.
#[event]
pub struct OfferVectorDeletedEvent {
    /// The PDA address of the offer from which the vector was deleted
    pub offer_pda: Pubkey,
    /// Start time of the deleted pricing vector
    pub vector_start_time: u64,
}

/// Account structure for deleting a pricing vector from an offer
///
/// This struct defines the accounts required to remove a time-based pricing vector
/// from an existing offer. Only the boss can delete pricing vectors to control offer dynamics.
#[derive(Accounts)]
pub struct DeleteOfferVector<'info> {
    /// The offer account from which the pricing vector will be deleted
    ///
    /// This account is validated as a PDA derived from token mint addresses
    /// and contains the array of pricing vectors for the offer.
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
    #[account(seeds = [seeds::STATE], bump = state.bump, has_one = boss)]
    pub state: Account<'info, State>,

    /// The boss account authorized to delete pricing vectors from offers
    #[account(mut)]
    pub boss: Signer<'info>,
}

/// Deletes a pricing vector from an existing offer
///
/// This instruction removes a time-based pricing vector from an offer by setting it to
/// default values. The vector is identified by its start_time within the offer's vector array.
/// Deleting a vector immediately stops its price evolution and removes its configuration.
///
/// To maintain pricing continuity, the instruction prevents deletion of the previously
/// active vector to ensure smooth price transitions between remaining vectors.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `vector_start_time` - Start time of the pricing vector to delete
///
/// # Returns
/// * `Ok(())` - If the vector is successfully deleted
/// * `Err(DeleteOfferVectorErrorCode::VectorNotFound)` - If start_time is zero or vector doesn't exist
/// * `Err(DeleteOfferVectorErrorCode::CannotDeletePreviousVector)` - If attempting to delete previous active vector
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss account must match the one stored in program state
///
/// # Effects
/// - Pricing vector is set to default (empty) values
/// - Vector slot becomes available for future additions
/// - Price evolution for that timeframe is removed
///
/// # Events
/// * `OfferVectorDeletedEvent` - Emitted with offer PDA and deleted vector start time
pub fn delete_offer_vector(ctx: Context<DeleteOfferVector>, vector_start_time: u64) -> Result<()> {
    let offer = &mut ctx.accounts.offer.load_mut()?;

    // Validate inputs
    require!(
        vector_start_time != 0,
        DeleteOfferVectorErrorCode::VectorNotFound
    );

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

/// Error codes for delete offer vector operations
#[error_code]
pub enum DeleteOfferVectorErrorCode {
    /// The specified start_time is zero or no vector exists with that start_time
    #[msg("Vector not found")]
    VectorNotFound,

    /// Cannot delete the previously active vector to maintain pricing continuity
    #[msg("Cannot delete previously active vector")]
    CannotDeletePreviousVector,
}
