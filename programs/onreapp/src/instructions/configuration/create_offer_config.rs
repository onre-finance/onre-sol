use crate::model::{self, seeds::*};
use anchor_lang::prelude::*;

/// Creates an offer from existing compatible configuration; does not enable new settlement paths.
#[derive(Accounts)]
#[instruction(flow: model::OfferFlow)]
pub struct CreateOfferConfig<'info> {
    pub token_in_mint: Box<InterfaceAccount<'info, anchor_spl::token_interface::Mint>>,

    pub token_out_mint: Box<InterfaceAccount<'info, anchor_spl::token_interface::Mint>>,

    #[account(
        constraint = managed_token.enabled @ crate::OnreError::ConfigurationDisabled
    )]
    pub managed_token: Box<Account<'info, model::ManagedTokenConfig>>,

    #[account(
        constraint = asset.enabled @ crate::OnreError::ConfigurationDisabled
    )]
    pub asset: Box<Account<'info, model::AssetConfig>>,

    #[account(
        constraint = pricer.enabled @ crate::OnreError::ConfigurationDisabled
    )]
    pub pricer: Box<Account<'info, model::Pricer>>,

    #[account(
        constraint = quoter.enabled @ crate::OnreError::ConfigurationDisabled
    )]
    pub quoter: Box<Account<'info, model::Quoter>>,

    pub prop_amm_state: Option<Box<Account<'info, model::PropAmmState>>>,

    #[account(
        constraint = fee_config.enabled @ crate::OnreError::ConfigurationDisabled
    )]
    pub fee_config: Box<Account<'info, model::FeeConfig>>,

    #[account(
        constraint = proceeds_vault.kind == model::VaultKind::Proceeds @ crate::OnreError::InvalidConfigurableVaultKind
    )]
    pub proceeds_vault: Box<Account<'info, model::VaultConfig>>,

    pub liquidity_vault: Option<Box<Account<'info, model::VaultConfig>>>,

    #[account(
        init,
        payer = boss,
        space = 8 + model::OfferConfig::INIT_SPACE,
        seeds = [
            OFFER_CONFIG,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref(),
            &[flow as u8]
        ],
        bump
    )]
    pub offer_config: Box<Account<'info, model::OfferConfig>>,

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

pub fn create_offer_config(ctx: Context<CreateOfferConfig>, flow: model::OfferFlow) -> Result<()> {
    use model::{OfferDirection::*, OfferFlow::*, QuoterKind};
    let a = &ctx.accounts;
    let direction = if a.token_in_mint.key() == a.asset.asset_mint
        && a.token_out_mint.key() == a.managed_token.managed_mint
    {
        AssetToManaged
    } else if a.token_in_mint.key() == a.managed_token.managed_mint
        && a.token_out_mint.key() == a.asset.asset_mint
    {
        ManagedToAsset
    } else {
        return err!(crate::OnreError::InvalidOfferDirection);
    };
    require!(
        a.pricer.managed_mint == a.managed_token.managed_mint
            && a.pricer.denomination == a.asset.denomination,
        crate::OnreError::IncompatiblePricer
    );
    let compatible = match flow {
        Permissioned => a.quoter.kind == QuoterKind::Nav,
        Permissionless => matches!(
            a.quoter.kind,
            QuoterKind::NavPermissionless | QuoterKind::PropAmm
        ),
        Worker => direction == ManagedToAsset && a.quoter.kind == QuoterKind::Nav,
    };
    require!(compatible, crate::OnreError::IncompatibleQuoter);
    if a.quoter.kind == QuoterKind::PropAmm {
        let pair = a
            .prop_amm_state
            .as_ref()
            .ok_or(crate::OnreError::IncompatibleQuoter)?;
        require!(
            pair.quoter == a.quoter.key()
                && pair.managed_mint == a.managed_token.managed_mint
                && pair.asset_mint == a.asset.asset_mint,
            crate::OnreError::IncompatibleQuoter
        );
    } else {
        require!(
            a.prop_amm_state.is_none(),
            crate::OnreError::IncompatibleQuoter
        );
    }
    if let Some(vault) = &a.liquidity_vault {
        require!(
            vault.kind == model::VaultKind::Liquidity,
            crate::OnreError::InvalidConfigurableVaultKind
        );
    } else {
        require!(
            direction == AssetToManaged,
            crate::OnreError::LiquidityVaultRequired
        );
    }
    let offer = model::OfferConfig {
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
    Ok(())
}
