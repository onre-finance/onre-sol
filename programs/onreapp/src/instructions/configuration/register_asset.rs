use crate::{
    constants::seeds::STATE,
    model::{
        seeds::{ASSET, MANAGED_TOKEN},
        AssetConfig, PricingDenomination,
    },
    state::State,
    utils::MAX_TOKEN_DECIMALS,
    OnreError,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

/// Records the created account and its complete initial configuration.
#[event]
pub struct AssetRegisteredEvent {
    pub boss: Pubkey,
    pub asset: Pubkey,
    pub configuration: AssetConfig,
    pub timestamp: i64,
}

/// Attests an external mint's pricing denomination. A mint cannot also be managed.
#[derive(Accounts)]
pub struct RegisterAsset<'info> {
    pub asset_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: Its derived address must have no managed-token registration data.
    #[account(
        seeds = [MANAGED_TOKEN, asset_mint.key().as_ref()],
        bump,
        constraint = managed_registration.data_is_empty() @ OnreError::TokenRoleConflict
    )]
    pub managed_registration: UncheckedAccount<'info>,

    #[account(
        init,
        payer = boss,
        space = 8 + AssetConfig::INIT_SPACE,
        seeds = [ASSET, asset_mint.key().as_ref()],
        bump
    )]
    pub asset: Box<Account<'info, AssetConfig>>,

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

pub fn register_asset(
    ctx: Context<RegisterAsset>,
    denomination: PricingDenomination,
) -> Result<()> {
    require!(
        ctx.accounts.asset_mint.decimals <= MAX_TOKEN_DECIMALS,
        OnreError::DecimalsExceedMax
    );
    ctx.accounts.asset.set_inner(AssetConfig {
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
