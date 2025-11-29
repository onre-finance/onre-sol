use crate::constants::seeds;
use crate::instructions::redemption::Redemption;
use crate::state::{RedemptionState, State};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;


#[event]
pub struct CreateRedemptionEvent {
    /// The token mint being redeemed from
    pub token_in: Pubkey,
    /// The token mint being redeemed to
    pub token_out: Pubkey,
    /// The redemption account created for this pair
    pub redemption: Pubkey,
    /// The boss account that initiated the redemption creation
    pub boss: Pubkey,
}

#[derive(Accounts)]
pub struct CreateRedemptionContext<'info> {
    /// The user initiating the redemption process
    #[account(mut)]
    pub boss: Signer<'info>,

    /// The redemption state account holding global redemption parameters
    #[account(
        seeds = [seeds::REDEMPTION_STATE, token_in.key().as_ref(), token_out.key().as_ref()],
        bump = redemption_state.bump,
    )]
    pub redemption_state: Box<Account<'info, RedemptionState>>,

    /// The redemption account to be created for this specific redemption pair
    #[account(
        init,
        payer = boss,
        space = 8 + Redemption::INIT_SPACE,
        seeds = [seeds::REDEMPTION, token_in.key().as_ref(), token_out.key().as_ref()],
        bump,
    )]
    pub redemption: Box<Account<'info, Redemption>>,

    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss
    )]
    pub state: Box<Account<'info, State>>,

    /// The token mint for the input token being redeemed
    pub token_in: Box<InterfaceAccount<'info, Mint>>,

    /// The token mint for the output token being redeemed to
    pub token_out: Box<InterfaceAccount<'info, Mint>>,

    /// System program for account creation
    pub system_program: Program<'info, System>,
}

pub fn create_redemption(ctx: Context<CreateRedemptionContext>) -> Result<()> {
    let redemption = &mut ctx.accounts.redemption;
    redemption.token_in = ctx.accounts.token_in.key();
    redemption.token_out = ctx.accounts.token_out.key();
    redemption.bump = ctx.bumps.redemption;

    emit!(CreateRedemptionEvent {
        token_in: ctx.accounts.token_in.key(),
        token_out: ctx.accounts.token_out.key(),
        redemption: redemption.key(),
        boss: ctx.accounts.boss.key(),
    });
    Ok(())
}
