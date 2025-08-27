use crate::constants::seeds;
use crate::instructions::SingleRedemptionOfferAccount;
use crate::state::State;
use anchor_lang::prelude::*;

#[error_code]
pub enum CloseSingleRedemptionOfferErrorCode {
    #[msg("Offer not found")]
    OfferNotFound,
}

#[event]
pub struct CloseSingleRedemptionOfferEvent {
    pub offer_id: u64,
    pub boss: Pubkey,
}

#[derive(Accounts)]
pub struct CloseSingleRedemptionOffer<'info> {
    /// The single redemption offer account within the SingleRedemptionOfferAccount, rent paid by `boss`.
    #[account(mut, seeds = [seeds::SINGLE_REDEMPTION_OFFERS], bump)]
    pub single_redemption_offer_account: AccountLoader<'info, SingleRedemptionOfferAccount>,

    /// The signer funding and authorizing the offer closure.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

/// Closes a single redemption offer.
///
/// Removes the specified redemption offer from the account by clearing it (setting to default values).
/// Only the boss can close redemption offers.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the closure operation.
/// - `offer_id`: ID of the offer to close.
///
/// # Errors
/// - [`CloseSingleRedemptionOfferErrorCode::OfferNotFound`] if the offer doesn't exist or ID is 0.
///
/// # Returns
/// A `Result` indicating success or failure.
pub fn close_single_redemption_offer(ctx: Context<CloseSingleRedemptionOffer>, offer_id: u64) -> Result<()> {
    if offer_id == 0 {
        return Err(error!(CloseSingleRedemptionOfferErrorCode::OfferNotFound));
    }
    let single_redemption_offer_account = &mut ctx.accounts.single_redemption_offer_account.load_mut()?;

    let offer_index = single_redemption_offer_account
        .offers
        .iter()
        .position(|offer| offer.offer_id == offer_id)
        .ok_or(CloseSingleRedemptionOfferErrorCode::OfferNotFound)?;

    single_redemption_offer_account.offers[offer_index] = Default::default();

    msg!("Redemption offer closed with ID: {}", offer_id);

    emit!(CloseSingleRedemptionOfferEvent {
        offer_id,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}