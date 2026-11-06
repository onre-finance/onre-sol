use crate::{
    model::{seeds as model_seeds, OfferDirection, OfferFlow, QuoterKind},
    utils::{has_transfer_fee, program_controls_mint},
    OnreError,
};
use anchor_lang::prelude::*;

use super::{liquidity, permissions, TakeOffer};

/// Validated dispatch data, shared by quoting and settlement.
pub(super) struct ExecutionPlan {
    pub direction: OfferDirection,
    pub kind: QuoterKind,
    pub flow: OfferFlow,
    pub liquidity_balance: u64,
    pub has_liquidity: bool,
}

impl ExecutionPlan {
    /// Enable paths only after their pricing, accounting and settlement exist.
    pub fn require_supported(&self) -> Result<()> {
        require!(
            matches!(
                (self.flow, self.kind),
                (OfferFlow::Permissioned, QuoterKind::Nav)
                    | (OfferFlow::Permissionless, QuoterKind::NavPermissionless)
            ),
            OnreError::UnsupportedOfferExecution
        );
        // Asset-to-managed refill is a later layer; never ignore a configured vault.
        require!(
            self.direction != OfferDirection::AssetToManaged || !self.has_liquidity,
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
    let balances = liquidity::validate(program_id, a)?;
    Ok(ExecutionPlan {
        direction: a.offer_config.direction,
        kind: a.quoter.kind,
        flow: a.offer_config.flow,
        liquidity_balance: balances,
        has_liquidity: a.liquidity_vault.is_some(),
    })
}

fn validate_prop_amm(program_id: &Pubkey, a: &TakeOffer) -> Result<()> {
    if a.quoter.kind != QuoterKind::PropAmm {
        require!(a.prop_amm_state.is_none(), OnreError::IncompatibleQuoter);
        return Ok(());
    }
    let pair = a
        .prop_amm_state
        .as_ref()
        .ok_or(OnreError::IncompatibleQuoter)?;
    let (expected, bump) = Pubkey::find_program_address(
        &[model_seeds::PROP_AMM, a.quoter.key().as_ref()],
        program_id,
    );
    require!(
        pair.key() == expected
            && pair.bump == bump
            && pair.quoter == a.quoter.key()
            && pair.managed_mint == a.managed_mint().key()
            && pair.asset_mint == a.asset_mint().key(),
        OnreError::IncompatibleQuoter
    );
    Ok(())
}
