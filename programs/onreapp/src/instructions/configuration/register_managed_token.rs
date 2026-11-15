use crate::{
    constants::seeds::STATE,
    model::{seeds::MANAGED_TOKEN, ManagedTokenConfig},
    state::State,
    utils::MAX_TOKEN_DECIMALS,
    OnreError,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

/// Records the created account and its complete initial configuration.
#[event]
pub struct ManagedTokenRegisteredEvent {
    pub boss: Pubkey,
    pub managed_token: Pubkey,
    pub configuration: ManagedTokenConfig,
    pub timestamp: i64,
}

/// Registers an existing SPL mint for the new model. Does not change mint authority.
#[derive(Accounts)]
pub struct RegisterManagedToken<'info> {
    pub managed_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init,
        payer = boss,
        space = 8 + ManagedTokenConfig::INIT_SPACE,
        seeds = [MANAGED_TOKEN, managed_mint.key().as_ref()],
        bump
    )]
    pub managed_token: Box<Account<'info, ManagedTokenConfig>>,

    #[account(
        seeds = [STATE],
        bump = state.bump,
        has_one = boss @ OnreError::InvalidBoss
    )]
    pub state: Box<Account<'info, State>>,

    #[account(mut)]
    pub boss: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn register_managed_token(ctx: Context<RegisterManagedToken>) -> Result<()> {
    require!(
        ctx.accounts.managed_mint.decimals <= MAX_TOKEN_DECIMALS,
        OnreError::DecimalsExceedMax
    );
    ctx.accounts.managed_token.set_inner(ManagedTokenConfig {
        managed_mint: ctx.accounts.managed_mint.key(),
        decimals: ctx.accounts.managed_mint.decimals,
        enabled: true,
        max_supply: 0,
        max_mint_amount: 0,
        bump: ctx.bumps.managed_token,
    });
    emit!(ManagedTokenRegisteredEvent {
        boss: ctx.accounts.boss.key(),
        managed_token: ctx.accounts.managed_token.key(),
        configuration: (**ctx.accounts.managed_token).clone(),
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
