use crate::constants::seeds;
use crate::instructions::Offer;
use crate::state::State;
use crate::OfferCoreError;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

#[event]
pub struct CloseOfferEvent {
    pub offer_pda: Pubkey,
    pub boss: Pubkey,
}

#[derive(Accounts)]
pub struct CloseOffer<'info> {
    /// The offer account within the OfferAccount, rent paid by `boss`.
    #[account(
        mut,
        seeds = [
            seeds::OFFERS,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref()
        ],
        bump,
        close = boss
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

    /// The signer funding and authorizing the offer closure.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Account<'info, State>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

pub fn close_offer(ctx: Context<CloseOffer>) -> Result<()> {
    emit!(CloseOfferEvent {
        offer_pda: ctx.accounts.offer.key(),
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}
