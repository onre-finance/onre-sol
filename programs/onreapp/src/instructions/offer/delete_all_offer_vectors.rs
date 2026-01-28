use super::offer_state::{Offer, OfferVector};
use crate::constants::{seeds, MAX_VECTORS};
use crate::state::State;
use crate::OfferCoreError;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

/// Event emitted when all pricing vectors are deleted from an offer
///
/// Provides transparency for tracking bulk pricing vector removals.
#[event]
pub struct AllOfferVectorsDeletedEvent {
    /// The PDA address of the offer from which vectors were deleted
    pub offer_pda: Pubkey,
    /// Number of vectors that were deleted (non-empty vectors)
    pub vectors_deleted_count: u8,
}

/// Account structure for deleting all pricing vectors from an offer
///
/// This struct defines the accounts required to remove all time-based pricing vectors
/// from an existing offer. Only the boss can delete pricing vectors to control offer dynamics.
#[derive(Accounts)]
pub struct DeleteAllOfferVectors<'info> {
    /// The offer account from which all pricing vectors will be deleted
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
    pub boss: Signer<'info>,
}

/// Deletes all pricing vectors from an existing offer
///
/// This instruction removes ALL time-based pricing vectors from an offer,
/// regardless of whether they are in the past, currently active, or in the future.
/// All vectors are set to default values, effectively clearing the offer's pricing schedule.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
///
/// # Returns
/// * `Ok(())` - If all vectors are successfully deleted
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss account must match the one stored in program state
///
/// # Effects
/// - All pricing vectors are set to default (empty) values
/// - All vector slots become available for future additions
/// - Price evolution for all timeframes is removed
///
/// # Events
/// * `AllOfferVectorsDeletedEvent` - Emitted with offer PDA and count of deleted vectors
pub fn delete_all_offer_vectors(ctx: Context<DeleteAllOfferVectors>) -> Result<()> {
    let offer = &mut ctx.accounts.offer.load_mut()?;

    // Count non-empty vectors and delete them
    let mut deleted_count: u8 = 0;

    for i in 0..MAX_VECTORS {
        if offer.vectors[i].start_time != 0 {
            offer.vectors[i] = OfferVector::default();
            deleted_count += 1;
        }
    }

    msg!(
        "All vectors deleted from offer: {}, count: {}",
        ctx.accounts.offer.key(),
        deleted_count
    );

    emit!(AllOfferVectorsDeletedEvent {
        offer_pda: ctx.accounts.offer.key(),
        vectors_deleted_count: deleted_count,
    });

    Ok(())
}
