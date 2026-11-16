//! Boss-only replacement of an offer's components after creation, matching EVM
//! `updateOfferConfigReferences`. Mints, instance ID, access, execution and direction stay
//! immutable; every component is revalidated with the creation rules.
use super::create_offer_config::{validate_offer_components, OfferComponents};
use crate::{
    constants::seeds::STATE,
    model::{
        seeds::OFFER_CONFIG, FeeConfig, OfferConfig, Pricer, PropAmmState, Quoter, VaultConfig,
        VaultKind,
    },
    state::State,
    OnreError,
};
use anchor_lang::prelude::*;

#[event]
pub struct OfferConfigUpdatedEvent {
    pub boss: Pubkey,
    pub offer_config: Pubkey,
    pub previous: OfferConfig,
    pub configuration: OfferConfig,
    pub timestamp: i64,
}

/// The full new component set, supplied as in `create_offer_config`. Omitting
/// `prop_amm_state` detaches a NAV offer's pressure link; omitting a vault role
/// clears it.
#[derive(Accounts)]
pub struct UpdateOfferConfig<'info> {
    #[account(
        mut,
        seeds = [
            OFFER_CONFIG,
            offer_config.token_in_mint.as_ref(),
            offer_config.token_out_mint.as_ref(),
            &offer_config.instance_id.to_le_bytes()
        ],
        bump = offer_config.bump,
    )]
    pub offer_config: Box<Account<'info, OfferConfig>>,

    /// Any enabled Pricer instance of the offer's managed mint.
    #[account(constraint = pricer.enabled @ OnreError::ConfigurationDisabled)]
    pub pricer: Box<Account<'info, Pricer>>,

    #[account(constraint = quoter.enabled @ OnreError::ConfigurationDisabled)]
    pub quoter: Box<Account<'info, Quoter>>,

    pub prop_amm_state: Option<Box<Account<'info, PropAmmState>>>,

    #[account(constraint = fee_config.enabled @ OnreError::ConfigurationDisabled)]
    pub fee_config: Box<Account<'info, FeeConfig>>,

    #[account(
        constraint = proceeds_vault.kind == VaultKind::Proceeds @ OnreError::InvalidConfigurableVaultKind
    )]
    pub proceeds_vault: Box<Account<'info, VaultConfig>>,

    /// CHECK: Validated as supported output custody; required for managed-to-asset.
    pub source_vault: Option<UncheckedAccount<'info>>,

    /// CHECK: Validated as supported input custody; required for PropAmm buys.
    pub destination_vault: Option<UncheckedAccount<'info>>,

    /// Haircut-kind vault; required exactly for Prop AMM sells without a destination.
    pub haircut_vault: Option<Box<Account<'info, VaultConfig>>>,

    #[account(seeds = [STATE], bump = state.bump, has_one = boss @ OnreError::InvalidBoss)]
    pub state: Box<Account<'info, State>>,

    pub boss: Signer<'info>,
}

pub fn update_offer_config(ctx: Context<UpdateOfferConfig>) -> Result<()> {
    let a = &ctx.accounts;
    let previous = (**a.offer_config).clone();
    let source_vault = a.source_vault.as_ref().map(|v| v.to_account_info());
    let destination_vault = a.destination_vault.as_ref().map(|v| v.to_account_info());
    let pressure_state = validate_offer_components(
        ctx.program_id,
        previous.access,
        previous.execution,
        previous.direction,
        previous.token_in_mint,
        previous.token_out_mint,
        &OfferComponents {
            pricer: &a.pricer,
            quoter: &a.quoter,
            prop_amm_state: a.prop_amm_state.as_deref(),
            source_vault: source_vault.clone(),
            destination_vault: destination_vault.clone(),
            haircut_vault: a.haircut_vault.as_deref(),
        },
    )?;
    let next = OfferConfig {
        pricer: a.pricer.key(),
        quoter: a.quoter.key(),
        pressure_state,
        fee_config: a.fee_config.key(),
        proceeds_vault: a.proceeds_vault.key(),
        source_vault: source_vault.map(|v| v.key()).unwrap_or_default(),
        destination_vault: destination_vault.map(|v| v.key()).unwrap_or_default(),
        haircut_vault: a
            .haircut_vault
            .as_ref()
            .map(|v| v.key())
            .unwrap_or_default(),
        ..previous.clone()
    };
    require!(
        (
            next.pricer,
            next.quoter,
            next.pressure_state,
            next.fee_config,
            next.proceeds_vault,
            next.source_vault,
            next.destination_vault,
            next.haircut_vault,
        ) != (
            previous.pricer,
            previous.quoter,
            previous.pressure_state,
            previous.fee_config,
            previous.proceeds_vault,
            previous.source_vault,
            previous.destination_vault,
            previous.haircut_vault,
        ),
        OnreError::NoChange
    );
    let boss = a.boss.key();
    let offer_config = a.offer_config.key();
    ctx.accounts.offer_config.set_inner(next.clone());
    emit!(OfferConfigUpdatedEvent {
        boss,
        offer_config,
        previous,
        configuration: next,
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
