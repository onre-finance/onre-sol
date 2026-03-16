use crate::constants::seeds;
use crate::instructions::Offer;
use crate::state::State;
use anchor_lang::prelude::*;

#[event]
pub struct MainOfferUpdatedEvent {
    pub old_main_offer: Pubkey,
    pub new_main_offer: Pubkey,
}

#[derive(Accounts)]
pub struct SetMainOffer<'info> {
    #[account(
        mut,
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss
    )]
    pub state: Account<'info, State>,

    pub boss: Signer<'info>,

    pub offer: AccountLoader<'info, Offer>,
}

pub fn set_main_offer(ctx: Context<SetMainOffer>) -> Result<()> {
    let state = &mut ctx.accounts.state;
    let new_main_offer = ctx.accounts.offer.key();

    require!(new_main_offer != state.main_offer, SetMainOfferErrorCode::NoChange);

    let old_main_offer = state.main_offer;
    state.main_offer = new_main_offer;

    emit!(MainOfferUpdatedEvent {
        old_main_offer,
        new_main_offer,
    });

    Ok(())
}

#[error_code]
pub enum SetMainOfferErrorCode {
    #[msg("No change: new main offer is the same as current")]
    NoChange,
}
