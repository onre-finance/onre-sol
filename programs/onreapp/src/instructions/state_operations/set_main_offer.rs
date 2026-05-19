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
    let offer = ctx.accounts.offer.load()?;

    require_keys_eq!(
        offer.token_out_mint,
        state.onyc_mint,
        crate::OnreError::InvalidTokenOutMint
    );

    require!(
        new_main_offer != state.main_offer,
        crate::OnreError::NoChange
    );

    let old_main_offer = state.main_offer;
    state.main_offer = new_main_offer;

    emit!(MainOfferUpdatedEvent {
        old_main_offer,
        new_main_offer,
    });

    Ok(())
}
