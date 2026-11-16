use crate::{
    model::{OfferConfig, OfferDirection, QuoterKind},
    utils::{has_transfer_fee, pressure_link::validate_pressure_link},
    OnreError,
};
use anchor_lang::prelude::*;

use super::{liquidity, permissions, Execution};

/// Validated dispatch data, shared by quoting and settlement.
pub(super) struct ExecutionPlan {
    pub direction: OfferDirection,
    pub kind: QuoterKind,
    pub destination_balance: u64,
    pub destination_target_bps: u16,
    pub curve_balance: u64,
    pub curve_target_bps: u16,
}

/// The route must suit the offer's stored access and execution modes. Checked
/// before quoting and again at settlement. Any quoter suits any mode.
pub(super) fn require_supported(a: &Execution) -> Result<()> {
    let offer: &OfferConfig = &a.config.offer_config;
    require!(
        a.route.supports(offer),
        OnreError::UnsupportedOfferExecution
    );
    Ok(())
}

pub(super) fn validate(
    program_id: &Pubkey,
    a: &Execution,
    authority_bump: u8,
) -> Result<ExecutionPlan> {
    require_supported(a)?;
    require!(
        a.managed_mint().key() == a.config.managed_token.managed_mint
            && a.asset_mint().key() != a.config.managed_token.managed_mint,
        OnreError::InvalidOfferDirection
    );
    require!(
        a.token_in_mint.decimals == a.config.offer_config.token_in_decimals
            && a.token_out_mint.decimals == a.config.offer_config.token_out_decimals
            && a.config.managed_token.decimals == a.managed_mint().decimals,
        OnreError::InvalidMint
    );
    // Direct or multisig control, exactly as configured, even when no mint occurs.
    a.minter(authority_bump)?;
    require!(
        !has_transfer_fee(a.token_in_mint)? && !has_transfer_fee(a.token_out_mint)?,
        OnreError::TransferFeeNotSupported
    );
    permissions::validate_intermediary(a)?;
    match a.config.offer_config.direction {
        OfferDirection::AssetToManaged => require!(
            a.proceeds_token_in.is_some(),
            OnreError::InvalidConfigurableVaultTokenAccount
        ),
        OfferDirection::ManagedToAsset => require!(
            a.proceeds_token_in.is_none(),
            OnreError::InvalidConfigurableVaultTokenAccount
        ),
    }
    validate_prop_amm(program_id, a)?;
    let liquidity = liquidity::validate(program_id, a)?;
    liquidity::validate_haircut(program_id, a)?;
    Ok(ExecutionPlan {
        direction: a.config.offer_config.direction,
        kind: a.config.quoter.kind,
        destination_balance: liquidity.destination.balance,
        destination_target_bps: liquidity.destination.refill_target_bps,
        curve_balance: match a.config.offer_config.direction {
            OfferDirection::AssetToManaged => liquidity.destination.balance,
            OfferDirection::ManagedToAsset => liquidity.source.balance,
        },
        curve_target_bps: match a.config.offer_config.direction {
            OfferDirection::AssetToManaged => liquidity.destination.refill_target_bps,
            OfferDirection::ManagedToAsset => liquidity.source.refill_target_bps,
        },
    })
}

fn validate_prop_amm(program_id: &Pubkey, a: &Execution) -> Result<()> {
    let state = validate_pressure_link(
        program_id,
        a.config.quoter.key(),
        a.config.quoter.kind,
        a.config.offer_config.direction,
        a.config.offer_config.token_in_mint,
        a.config.offer_config.token_out_mint,
        a.custody.prop_amm_state.as_deref(),
    )?;
    require_keys_eq!(
        state,
        a.config.offer_config.pressure_state,
        OnreError::IncompatibleQuoter
    );
    if a.config.quoter.kind == QuoterKind::PropAmm
        && a.config.offer_config.direction == OfferDirection::AssetToManaged
    {
        require!(
            a.config.offer_config.destination_vault != Pubkey::default(),
            OnreError::LiquidityVaultRequired
        );
    }
    Ok(())
}
