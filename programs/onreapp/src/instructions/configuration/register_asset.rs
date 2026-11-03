use crate::model::{self, seeds::*};
use anchor_lang::prelude::*;

/// Records the created account and its complete initial configuration.
#[event]
pub struct AssetRegisteredEvent {
    pub boss: Pubkey,
    pub asset: Pubkey,
    pub configuration: model::AssetConfig,
    pub timestamp: i64,
}

/// Attests an external mint's pricing denomination. A mint cannot also be managed.
#[derive(Accounts)]
pub struct RegisterAsset<'info> {
    pub asset_mint: Box<InterfaceAccount<'info, anchor_spl::token_interface::Mint>>,

    /// CHECK: Its derived address must have no managed-token registration data.
    #[account(
        seeds = [MANAGED_TOKEN, asset_mint.key().as_ref()],
        bump,
        constraint = managed_registration.data_is_empty() @ crate::OnreError::TokenRoleConflict
    )]
    pub managed_registration: UncheckedAccount<'info>,

    #[account(
        init,
        payer = boss,
        space = 8 + model::AssetConfig::INIT_SPACE,
        seeds = [ASSET, asset_mint.key().as_ref()],
        bump
    )]
    pub asset: Box<Account<'info, model::AssetConfig>>,

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

pub fn register_asset(
    ctx: Context<RegisterAsset>,
    denomination: model::PricingDenomination,
) -> Result<()> {
    require!(
        ctx.accounts.asset_mint.decimals <= crate::utils::MAX_TOKEN_DECIMALS,
        crate::OnreError::DecimalsExceedMax
    );
    ctx.accounts.asset.set_inner(model::AssetConfig {
        asset_mint: ctx.accounts.asset_mint.key(),
        denomination,
        enabled: true,
        bump: ctx.bumps.asset,
    });
    emit!(AssetRegisteredEvent {
        boss: ctx.accounts.boss.key(),
        asset: ctx.accounts.asset.key(),
        configuration: (**ctx.accounts.asset).clone(),
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
