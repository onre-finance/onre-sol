use crate::{
    model::{OfferDirection, OfferFlow, QuoterKind},
    utils::{has_transfer_fee, pressure_link::validate_pressure_link, program_controls_mint},
    OnreError,
};
use anchor_lang::prelude::*;

use super::{liquidity, permissions, TakeOffer};

/// Validated dispatch data, shared by quoting and settlement.
pub(super) struct ExecutionPlan {
    pub direction: OfferDirection,
    pub kind: QuoterKind,
    pub flow: OfferFlow,
    pub destination_balance: u64,
    pub destination_target_bps: u16,
    pub curve_balance: u64,
    pub curve_target_bps: u16,
}

impl ExecutionPlan {
    /// Enable paths only after their pricing, accounting and settlement exist.
    pub fn require_supported(&self) -> Result<()> {
        require!(
            matches!(
                (self.flow, self.kind),
                (OfferFlow::Permissioned, QuoterKind::Nav)
                    | (
                        OfferFlow::Permissionless,
                        QuoterKind::NavPermissionless | QuoterKind::PropAmm
                    )
            ),
            OnreError::UnsupportedOfferExecution
        );
        Ok(())
    }
}

pub(super) fn validate(program_id: &Pubkey, a: &TakeOffer) -> Result<ExecutionPlan> {
    require!(
        a.managed_mint().key() == a.managed_token.managed_mint
            && a.asset_mint().key() != a.managed_token.managed_mint,
        OnreError::InvalidOfferDirection
    );
    require!(
        a.token_in_mint.decimals == a.offer_config.token_in_decimals
            && a.token_out_mint.decimals == a.offer_config.token_out_decimals
            && a.managed_token.decimals == a.managed_mint().decimals,
        OnreError::InvalidMint
    );
    require!(
        program_controls_mint(a.managed_mint(), &a.mint_authority),
        OnreError::ProgramNotMintAuthority
    );
    require!(
        !has_transfer_fee(&a.token_in_mint)? && !has_transfer_fee(&a.token_out_mint)?,
        OnreError::TransferFeeNotSupported
    );
    let compatible = match a.offer_config.flow {
        OfferFlow::Permissioned => a.quoter.kind == QuoterKind::Nav,
        OfferFlow::Permissionless => matches!(
            a.quoter.kind,
            QuoterKind::NavPermissionless | QuoterKind::PropAmm
        ),
        OfferFlow::Worker => {
            a.offer_config.direction == OfferDirection::ManagedToAsset
                && a.quoter.kind == QuoterKind::Nav
        }
    };
    require!(compatible, OnreError::IncompatibleQuoter);
    permissions::validate_accounts(program_id, a)?;
    match a.offer_config.direction {
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
    Ok(ExecutionPlan {
        direction: a.offer_config.direction,
        kind: a.quoter.kind,
        flow: a.offer_config.flow,
        destination_balance: liquidity.destination.balance,
        destination_target_bps: liquidity.destination.refill_target_bps,
        curve_balance: match a.offer_config.direction {
            OfferDirection::AssetToManaged => liquidity.destination.balance,
            OfferDirection::ManagedToAsset => liquidity.source.balance,
        },
        curve_target_bps: match a.offer_config.direction {
            OfferDirection::AssetToManaged => liquidity.destination.refill_target_bps,
            OfferDirection::ManagedToAsset => liquidity.source.refill_target_bps,
        },
    })
}

fn validate_prop_amm(program_id: &Pubkey, a: &TakeOffer) -> Result<()> {
    let state = validate_pressure_link(
        program_id,
        a.quoter.key(),
        a.quoter.kind,
        a.offer_config.direction,
        a.offer_config.token_in_mint,
        a.offer_config.token_out_mint,
        a.prop_amm_state.as_deref(),
    )?;
    require_keys_eq!(
        state,
        a.offer_config.pressure_state,
        OnreError::IncompatibleQuoter
    );
    if a.quoter.kind == QuoterKind::PropAmm
        && a.offer_config.direction == OfferDirection::AssetToManaged
    {
        require!(
            a.offer_config.destination_vault != Pubkey::default(),
            OnreError::LiquidityVaultRequired
        );
    }
    Ok(())
}
