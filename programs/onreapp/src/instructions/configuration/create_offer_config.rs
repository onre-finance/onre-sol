use crate::{
    constants::seeds::STATE,
    model::{
        seeds::{MANAGED_TOKEN, OFFER_CONFIG},
        FeeConfig, ManagedTokenConfig, OfferConfig, OfferDirection, OfferFlow, Pricer,
        PropAmmState, Quoter, QuoterKind, VaultConfig, VaultKind,
    },
    state::State,
    utils::{
        pressure_link::validate_pressure_link, vault_authority::validate_liquidity_vault,
        MAX_TOKEN_DECIMALS,
    },
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

/// Creates an offer with a boss-selected external asset and compatible Pricer.
/// Records enabled configuration; execution is limited to implemented paths.
#[derive(Accounts)]
#[instruction(flow: OfferFlow)]
pub struct CreateOfferConfig<'info> {
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,

    pub token_out_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        constraint = managed_token.enabled @ OnreError::ConfigurationDisabled
    )]
    pub managed_token: Box<Account<'info, ManagedTokenConfig>>,

    /// CHECK: The handler validates the other mint's canonical managed-registration
    /// PDA and requires empty data, enforcing exactly one managed mint.
    pub asset_managed_registration: UncheckedAccount<'info>,

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

    /// CHECK: Handler validates modular Liquidity, legacy configurable, or redemption custody.
    pub liquidity_vault: Option<UncheckedAccount<'info>>,

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
    require!(
        a.token_in_mint.decimals <= MAX_TOKEN_DECIMALS
            && a.token_out_mint.decimals <= MAX_TOKEN_DECIMALS,
        OnreError::DecimalsExceedMax
    );
    require_keys_neq!(
        a.token_in_mint.key(),
        a.token_out_mint.key(),
        OnreError::InvalidOfferDirection
    );
    let (direction, asset_mint) = if a.token_in_mint.key() == a.managed_token.managed_mint {
        (OfferDirection::ManagedToAsset, a.token_out_mint.key())
    } else if a.token_out_mint.key() == a.managed_token.managed_mint {
        (OfferDirection::AssetToManaged, a.token_in_mint.key())
    } else {
        return err!(OnreError::InvalidOfferDirection);
    };
    let expected_managed_registration =
        Pubkey::find_program_address(&[MANAGED_TOKEN, asset_mint.as_ref()], ctx.program_id).0;
    require_keys_eq!(
        a.asset_managed_registration.key(),
        expected_managed_registration,
        OnreError::InvalidMint
    );
    require!(
        a.asset_managed_registration.data_is_empty(),
        OnreError::TokenRoleConflict
    );
    require!(
        a.pricer.managed_mint == a.managed_token.managed_mint,
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
    let pressure_state = validate_pressure_link(
        ctx.program_id,
        a.quoter.key(),
        a.quoter.kind,
        direction,
        a.token_in_mint.key(),
        a.token_out_mint.key(),
        a.prop_amm_state.as_deref(),
    )?;
    if let Some(vault) = &a.liquidity_vault {
        validate_liquidity_vault(&vault.to_account_info(), ctx.program_id)?;
    } else {
        require!(
            direction == OfferDirection::AssetToManaged && a.quoter.kind != QuoterKind::PropAmm,
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
        pressure_state,
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
