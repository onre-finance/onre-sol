use crate::constants::seeds;
use crate::instructions::buy_offer::BuyOfferAccount;
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_spl::token::Token;

#[error_code]
pub enum CloseBuyOfferErrorCode {
    #[msg("Offer not found")]
    OfferNotFound,
}

#[event]
pub struct CloseBuyOfferEvent {
    pub offer_id: u64,
    pub boss: Pubkey,
}

#[derive(Accounts)]
pub struct CloseBuyOffer<'info> {
    /// The buy offer account within the BuyOfferAccount, rent paid by `boss`.
    #[account(mut, seeds = [seeds::BUY_OFFERS], bump)]
    pub buy_offer_account: AccountLoader<'info, BuyOfferAccount>,

    /// The signer funding and authorizing the offer closure.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

pub fn close_buy_offer(ctx: Context<CloseBuyOffer>, offer_id: u64) -> Result<()> {
    if offer_id == 0 {
        return Err(error!(CloseBuyOfferErrorCode::OfferNotFound));
    }
    let buy_offer_account = &mut ctx.accounts.buy_offer_account.load_mut()?;

    let offer_index = buy_offer_account
        .offers
        .iter()
        .position(|offer| offer.offer_id == offer_id)
        .ok_or(CloseBuyOfferErrorCode::OfferNotFound)?;

    buy_offer_account.offers[offer_index] = Default::default();

    emit!(CloseBuyOfferEvent {
        offer_id,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}
