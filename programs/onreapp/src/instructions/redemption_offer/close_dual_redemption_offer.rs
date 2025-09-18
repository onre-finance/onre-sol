use crate::constants::seeds;
use crate::instructions::DualRedemptionOfferAccount;
use crate::state::State;
use anchor_lang::prelude::*;

#[error_code]
pub enum CloseDualRedemptionOfferErrorCode {
    #[msg("Offer not found")]
    OfferNotFound,
}

#[event]
pub struct CloseDualRedemptionOfferEvent {
    pub offer_id: u64,
    pub boss: Pubkey,
}

/// Account structure for closing a dual redemption offer.
///
/// This struct defines the accounts required for the boss to close a dual redemption offer.
/// Only the boss can close dual redemption offers.
#[derive(Accounts)]
pub struct CloseDualRedemptionOffer<'info> {
    /// The dual redemption offer account containing all offers.
    #[account(mut, seeds = [seeds::DUAL_REDEMPTION_OFFERS], bump)]
    pub dual_redemption_offer_account: AccountLoader<'info, DualRedemptionOfferAccount>,

    /// The signer funding and authorizing the offer closure.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Account<'info, State>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

/// Closes a dual redemption offer.
///
/// Removes the specified dual redemption offer from the account by clearing it (setting to default values).
/// Only the boss can close dual redemption offers.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the closure operation.
/// - `offer_id`: ID of the offer to close.
///
/// # Errors
/// - [`CloseDualRedemptionOfferErrorCode::OfferNotFound`] if the offer doesn't exist or ID is 0.
///
/// # Returns
/// A `Result` indicating success or failure.
pub fn close_dual_redemption_offer(
    ctx: Context<CloseDualRedemptionOffer>,
    offer_id: u64,
) -> Result<()> {
    if offer_id == 0 {
        return Err(error!(CloseDualRedemptionOfferErrorCode::OfferNotFound));
    }
    let dual_redemption_offer_account =
        &mut ctx.accounts.dual_redemption_offer_account.load_mut()?;

    let offer_index = dual_redemption_offer_account
        .offers
        .iter()
        .position(|offer| offer.offer_id == offer_id)
        .ok_or(CloseDualRedemptionOfferErrorCode::OfferNotFound)?;

    dual_redemption_offer_account.offers[offer_index] = Default::default();

    msg!("Dual redemption offer closed with ID: {}", offer_id);

    emit!(CloseDualRedemptionOfferEvent {
        offer_id,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}
