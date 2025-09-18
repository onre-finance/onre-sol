use crate::constants::seeds;
use crate::instructions::find_offer_index;
use crate::instructions::offer::OfferAccount;
use crate::state::State;
use anchor_lang::prelude::*;

#[event]
pub struct CloseOfferEvent {
    pub offer_id: u64,
    pub boss: Pubkey,
}

#[derive(Accounts)]
pub struct CloseOffer<'info> {
    /// The offer account within the OfferAccount, rent paid by `boss`.
    #[account(mut, seeds = [seeds::OFFERS], bump)]
    pub offer_account: AccountLoader<'info, OfferAccount>,

    /// The signer funding and authorizing the offer closure.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Account<'info, State>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

pub fn close_offer(ctx: Context<CloseOffer>, offer_id: u64) -> Result<()> {
    let offer_account = &mut ctx.accounts.offer_account.load_mut()?;

    let offer_index = find_offer_index(offer_account, offer_id)?;

    offer_account.offers[offer_index] = Default::default();

    emit!(CloseOfferEvent {
        offer_id,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}
