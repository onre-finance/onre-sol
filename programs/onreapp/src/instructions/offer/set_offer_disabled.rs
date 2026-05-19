use crate::constants::seeds;
use crate::instructions::Offer;
use crate::state::State;
use anchor_lang::prelude::*;

#[event]
pub struct OfferDisabledSetEvent {
    pub offer_pda: Pubkey,
    pub disabled: bool,
    pub signer: Pubkey,
}

#[derive(Accounts)]
pub struct SetOfferDisabled<'info> {
    #[account(mut)]
    pub offer: AccountLoader<'info, Offer>,

    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
    )]
    pub state: Box<Account<'info, State>>,

    pub signer: Signer<'info>,
}

pub fn set_offer_disabled(ctx: Context<SetOfferDisabled>, disabled: bool) -> Result<()> {
    let state = &ctx.accounts.state;
    let signer = &ctx.accounts.signer;
    let boss_signed = state.boss == signer.key() && signer.is_signer;
    let admin_signed = state.admins.contains(&signer.key()) && signer.is_signer;

    if disabled {
        require!(
            boss_signed || admin_signed,
            crate::OnreError::UnauthorizedToDisableOffer
        );
    } else {
        require!(boss_signed, crate::OnreError::OnlyBossCanEnableOffer);
    }

    let mut offer = ctx.accounts.offer.load_mut()?;
    offer.set_disabled(disabled);

    emit!(OfferDisabledSetEvent {
        offer_pda: ctx.accounts.offer.key(),
        disabled,
        signer: signer.key(),
    });

    Ok(())
}
