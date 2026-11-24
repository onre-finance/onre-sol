//! Boss-only allowlist of an offer's users: grant creates the user's
//! `OfferPermission`, revoke closes it.
use crate::{
    constants::seeds::STATE,
    model::{seeds::OFFER_PERMISSION, OfferConfig, OfferPermission},
    state::State,
    OnreError,
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(user: Pubkey)]
pub struct GrantOfferPermission<'info> {
    pub offer_config: Box<Account<'info, OfferConfig>>,

    #[account(
        init,
        payer = boss,
        space = 8 + OfferPermission::INIT_SPACE,
        seeds = [OFFER_PERMISSION, offer_config.key().as_ref(), user.as_ref()],
        bump
    )]
    pub permission: Account<'info, OfferPermission>,

    #[account(seeds = [STATE], bump = state.bump, has_one = boss @ OnreError::InvalidBoss)]
    pub state: Box<Account<'info, State>>,

    #[account(mut)]
    pub boss: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn grant_offer_permission(ctx: Context<GrantOfferPermission>, user: Pubkey) -> Result<()> {
    ctx.accounts.permission.set_inner(OfferPermission {
        offer_config: ctx.accounts.offer_config.key(),
        user,
        bump: ctx.bumps.permission,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct RevokeOfferPermission<'info> {
    #[account(mut, close = boss)]
    pub permission: Account<'info, OfferPermission>,

    #[account(seeds = [STATE], bump = state.bump, has_one = boss @ OnreError::InvalidBoss)]
    pub state: Box<Account<'info, State>>,

    #[account(mut)]
    pub boss: Signer<'info>,
}

pub fn revoke_offer_permission(_ctx: Context<RevokeOfferPermission>) -> Result<()> {
    Ok(())
}
