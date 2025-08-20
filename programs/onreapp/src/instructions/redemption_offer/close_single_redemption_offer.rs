use crate::instructions::SingleRedemptionOfferAccount;
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_spl::token::Token;

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
    #[account(mut, seeds = [b"single_redemption_offers"], bump)]
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

    emit!(CloseSingleRedemptionOfferEvent {
        offer_id,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}