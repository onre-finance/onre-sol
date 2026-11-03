use crate::model::{self, seeds::*};
use anchor_lang::prelude::*;

/// Records the created account and its complete initial configuration.
#[event]
pub struct ManagedTokenRegisteredEvent {
    pub boss: Pubkey,
    pub managed_token: Pubkey,
    pub configuration: model::ManagedTokenConfig,
    pub timestamp: i64,
}

/// Registers an existing SPL mint for the new model. Does not change mint authority.
#[derive(Accounts)]
pub struct RegisterManagedToken<'info> {
    pub managed_mint: Box<InterfaceAccount<'info, anchor_spl::token_interface::Mint>>,

    /// CHECK: Its derived address must have no asset registration data.
    #[account(
        seeds = [ASSET, managed_mint.key().as_ref()],
        bump,
        constraint = asset_registration.data_is_empty() @ crate::OnreError::TokenRoleConflict
    )]
    pub asset_registration: UncheckedAccount<'info>,

    #[account(
        init,
        payer = boss,
        space = 8 + model::ManagedTokenConfig::INIT_SPACE,
        seeds = [MANAGED_TOKEN, managed_mint.key().as_ref()],
        bump
    )]
    pub managed_token: Box<Account<'info, model::ManagedTokenConfig>>,

    #[account(
        seeds = [crate::constants::seeds::STATE],
        bump = state.bump,
        has_one = boss @ crate::OnreError::InvalidBoss
    )]
    pub state: Box<Account<'info, crate::state::State>>,

    #[account(mut)]
    pub boss: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn register_managed_token(ctx: Context<RegisterManagedToken>) -> Result<()> {
    require!(
        ctx.accounts.managed_mint.decimals <= crate::utils::MAX_TOKEN_DECIMALS,
        crate::OnreError::DecimalsExceedMax
    );
    ctx.accounts
        .managed_token
        .set_inner(model::ManagedTokenConfig {
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
