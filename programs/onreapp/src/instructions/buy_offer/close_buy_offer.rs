use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use crate::instructions::buy_offer::BuyOfferAccount;
use crate::state::State;

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
    #[account(mut, seeds = [b"buy_offers"], bump)]
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


pub fn close_buy_offer(
    ctx: Context<CloseBuyOffer>,
    offer_id: u64,
) -> Result<()> {
    let buy_offer_account = &mut ctx.accounts.buy_offer_account.load_mut()?;

    let offer_index = buy_offer_account.offers.iter().position(|offer| offer.offer_id == offer_id);

    if let Some(index) = offer_index {
        let last_active_index = (buy_offer_account.count as usize).saturating_sub(1);

        if index != last_active_index {
            buy_offer_account.offers[index] = buy_offer_account.offers[last_active_index];
        }

        buy_offer_account.offers[last_active_index] = Default::default();

        buy_offer_account.count = buy_offer_account.count.saturating_sub(1);

        emit!(CloseBuyOfferEvent {
            offer_id,
            boss: ctx.accounts.boss.key(),
        });

        Ok(())
    } else {
        Err(error!(CloseBuyOfferErrorCode::OfferNotFound))
    }
}