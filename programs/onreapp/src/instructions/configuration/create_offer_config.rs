use crate::{
    constants::seeds::STATE,
    model::{
        quoter_supported,
        seeds::{MANAGED_TOKEN, OFFER_CONFIG},
        FeeConfig, ManagedTokenConfig, OfferAccess, OfferConfig, OfferDirection, OfferExecution,
        Pricer, PropAmmState, Quoter, QuoterKind, VaultConfig, VaultKind,
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
#[instruction(instance_id: u64)]
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

    /// CHECK: Handler validates output custody; required for managed-to-asset.
    pub source_vault: Option<UncheckedAccount<'info>>,

    /// CHECK: Handler validates input custody; required for PropAmm buys.
    pub destination_vault: Option<UncheckedAccount<'info>>,

    /// Haircut-kind vault; required exactly for Prop AMM sells without a destination.
    pub haircut_vault: Option<Box<Account<'info, VaultConfig>>>,

    #[account(
        init,
        payer = boss,
        space = 8 + OfferConfig::INIT_SPACE,
        seeds = [
            OFFER_CONFIG,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref(),
            &instance_id.to_le_bytes()
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

pub fn create_offer_config(
    ctx: Context<CreateOfferConfig>,
    instance_id: u64,
    access: OfferAccess,
    execution: OfferExecution,
) -> Result<()> {
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
    let pressure_state = validate_offer_components(
        ctx.program_id,
        access,
        execution,
        direction,
        a.token_in_mint.key(),
        a.token_out_mint.key(),
        &OfferComponents {
            pricer: &a.pricer,
            quoter: &a.quoter,
            prop_amm_state: a.prop_amm_state.as_deref(),
            source_vault: a.source_vault.as_ref().map(|v| v.to_account_info()),
            destination_vault: a.destination_vault.as_ref().map(|v| v.to_account_info()),
            haircut_vault: a.haircut_vault.as_deref(),
        },
    )?;
    let offer = OfferConfig {
        token_in_mint: a.token_in_mint.key(),
        token_out_mint: a.token_out_mint.key(),
        instance_id,
        access,
        execution,
        direction,
        pricer: a.pricer.key(),
        quoter: a.quoter.key(),
        pressure_state,
        fee_config: a.fee_config.key(),
        proceeds_vault: a.proceeds_vault.key(),
        source_vault: a.source_vault.as_ref().map(|v| v.key()).unwrap_or_default(),
        destination_vault: a
            .destination_vault
            .as_ref()
            .map(|v| v.key())
            .unwrap_or_default(),
        haircut_vault: a
            .haircut_vault
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

/// Selected components of an offer, validated together on creation and update.
pub(crate) struct OfferComponents<'a, 'info> {
    pub pricer: &'a Account<'info, Pricer>,
    pub quoter: &'a Account<'info, Quoter>,
    pub prop_amm_state: Option<&'a Account<'info, PropAmmState>>,
    pub source_vault: Option<AccountInfo<'info>>,
    pub destination_vault: Option<AccountInfo<'info>>,
    pub haircut_vault: Option<&'a Account<'info, VaultConfig>>,
}

/// Rules shared by `create_offer_config` and `update_offer_config`: the Pricer
/// belongs to the managed mint, the quoter suits the access and execution modes, the pressure link is
/// valid, custody roles are supported vaults, managed-to-asset offers have a
/// source, PropAmm buys have a destination, and exactly the PropAmm sells without
/// a destination have a Haircut-kind vault. Returns the pressure link.
pub(crate) fn validate_offer_components(
    program_id: &Pubkey,
    access: OfferAccess,
    execution: OfferExecution,
    direction: OfferDirection,
    token_in_mint: Pubkey,
    token_out_mint: Pubkey,
    c: &OfferComponents,
) -> Result<Pubkey> {
    let managed_mint = match direction {
        OfferDirection::AssetToManaged => token_out_mint,
        OfferDirection::ManagedToAsset => token_in_mint,
    };
    require!(
        c.pricer.managed_mint == managed_mint,
        OnreError::IncompatiblePricer
    );
    require!(
        quoter_supported(access, execution, direction, c.quoter.kind),
        OnreError::IncompatibleQuoter
    );
    let pressure_state = validate_pressure_link(
        program_id,
        c.quoter.key(),
        c.quoter.kind,
        direction,
        token_in_mint,
        token_out_mint,
        c.prop_amm_state,
    )?;
    for vault in [&c.source_vault, &c.destination_vault]
        .into_iter()
        .flatten()
    {
        validate_liquidity_vault(vault, program_id)?;
    }
    require!(
        direction != OfferDirection::ManagedToAsset || c.source_vault.is_some(),
        OnreError::LiquidityVaultRequired
    );
    require!(
        direction != OfferDirection::AssetToManaged
            || c.quoter.kind != QuoterKind::PropAmm
            || c.destination_vault.is_some(),
        OnreError::LiquidityVaultRequired
    );
    let retains_haircut = direction == OfferDirection::ManagedToAsset
        && c.quoter.kind == QuoterKind::PropAmm
        && c.destination_vault.is_none();
    match (retains_haircut, c.haircut_vault) {
        (true, None) => return err!(OnreError::HaircutVaultRequired),
        (false, Some(_)) => return err!(OnreError::UnexpectedHaircutVault),
        (true, Some(vault)) => require!(
            vault.kind == VaultKind::Haircut,
            OnreError::InvalidConfigurableVaultKind
        ),
        (false, None) => {}
    }
    Ok(pressure_state)
}
