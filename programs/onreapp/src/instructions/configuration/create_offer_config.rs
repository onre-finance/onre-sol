use crate::{
    constants::seeds::STATE,
    model::{
        seeds::OFFER_CONFIG, AssetConfig, FeeConfig, ManagedTokenConfig, OfferConfig,
        OfferDirection, OfferFlow, Pricer, PropAmmState, Quoter, QuoterKind, VaultConfig,
        VaultKind,
    },
    state::State,
    OnreError,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

/// Records the created account and its complete initial configuration.
#[event]
pub struct OfferConfigCreatedEvent {
    pub boss: Pubkey,
    pub offer_config: Pubkey,
    pub configuration: OfferConfig,
    pub timestamp: i64,
}

/// Creates an offer from existing compatible configuration; does not enable new settlement paths.
#[derive(Accounts)]
#[instruction(flow: OfferFlow)]
pub struct CreateOfferConfig<'info> {
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,

    pub token_out_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        constraint = managed_token.enabled @ OnreError::ConfigurationDisabled
    )]
    pub managed_token: Box<Account<'info, ManagedTokenConfig>>,

    #[account(
        constraint = asset.enabled @ OnreError::ConfigurationDisabled
    )]
    pub asset: Box<Account<'info, AssetConfig>>,

    #[account(
        constraint = pricer.enabled @ OnreError::ConfigurationDisabled
    )]
    pub pricer: Box<Account<'info, Pricer>>,

    #[account(
        constraint = quoter.enabled @ OnreError::ConfigurationDisabled
    )]
    pub quoter: Box<Account<'info, Quoter>>,

    pub prop_amm_state: Option<Box<Account<'info, PropAmmState>>>,

    #[account(
        constraint = fee_config.enabled @ OnreError::ConfigurationDisabled
    )]
    pub fee_config: Box<Account<'info, FeeConfig>>,

    #[account(
        constraint = proceeds_vault.kind == VaultKind::Proceeds @ OnreError::InvalidConfigurableVaultKind
    )]
    pub proceeds_vault: Box<Account<'info, VaultConfig>>,

    pub liquidity_vault: Option<Box<Account<'info, VaultConfig>>>,

    #[account(
        init,
        payer = boss,
        space = 8 + OfferConfig::INIT_SPACE,
        seeds = [
            OFFER_CONFIG,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref(),
            &[flow as u8]
        ],
        bump
    )]
    pub offer_config: Box<Account<'info, OfferConfig>>,

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

pub fn create_offer_config(ctx: Context<CreateOfferConfig>, flow: OfferFlow) -> Result<()> {
    let a = &ctx.accounts;
    let direction = if a.token_in_mint.key() == a.asset.asset_mint
        && a.token_out_mint.key() == a.managed_token.managed_mint
    {
        OfferDirection::AssetToManaged
    } else if a.token_in_mint.key() == a.managed_token.managed_mint
        && a.token_out_mint.key() == a.asset.asset_mint
    {
        OfferDirection::ManagedToAsset
    } else {
        return err!(OnreError::InvalidOfferDirection);
    };
    require!(
        a.pricer.managed_mint == a.managed_token.managed_mint
            && a.pricer.denomination == a.asset.denomination,
        OnreError::IncompatiblePricer
    );
    let compatible = match flow {
        OfferFlow::Permissioned => a.quoter.kind == QuoterKind::Nav,
        OfferFlow::Permissionless => matches!(
            a.quoter.kind,
            QuoterKind::NavPermissionless | QuoterKind::PropAmm
        ),
        OfferFlow::Worker => {
            direction == OfferDirection::ManagedToAsset && a.quoter.kind == QuoterKind::Nav
        }
    };
    require!(compatible, OnreError::IncompatibleQuoter);
    if a.quoter.kind == QuoterKind::PropAmm {
        let pair = a
            .prop_amm_state
            .as_ref()
            .ok_or(OnreError::IncompatibleQuoter)?;
        require!(
            pair.quoter == a.quoter.key()
                && pair.managed_mint == a.managed_token.managed_mint
                && pair.asset_mint == a.asset.asset_mint,
            OnreError::IncompatibleQuoter
        );
    } else {
        require!(a.prop_amm_state.is_none(), OnreError::IncompatibleQuoter);
    }
    if let Some(vault) = &a.liquidity_vault {
        require!(
            vault.kind == VaultKind::Liquidity,
            OnreError::InvalidConfigurableVaultKind
        );
    } else {
        require!(
            direction == OfferDirection::AssetToManaged,
            OnreError::LiquidityVaultRequired
        );
    }
    let offer = OfferConfig {
        token_in_mint: a.token_in_mint.key(),
        token_out_mint: a.token_out_mint.key(),
        flow,
        direction,
        pricer: a.pricer.key(),
        quoter: a.quoter.key(),
        fee_config: a.fee_config.key(),
        proceeds_vault: a.proceeds_vault.key(),
        liquidity_vault: a
            .liquidity_vault
            .as_ref()
            .map(|v| v.key())
            .unwrap_or_default(),
        token_in_decimals: a.token_in_mint.decimals,
        token_out_decimals: a.token_out_mint.decimals,
        enabled: true,
        bump: ctx.bumps.offer_config,
    };
    ctx.accounts.offer_config.set_inner(offer);
    emit!(OfferConfigCreatedEvent {
        boss: ctx.accounts.boss.key(),
        offer_config: ctx.accounts.offer_config.key(),
        configuration: (**ctx.accounts.offer_config).clone(),
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
