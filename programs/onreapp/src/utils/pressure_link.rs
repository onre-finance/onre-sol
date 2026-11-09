use crate::{
    model::{seeds::PROP_AMM, OfferDirection, PropAmmState, QuoterKind},
    OnreError,
};
use anchor_lang::prelude::*;

/// One optional tracker per offer. The selected quote algorithm and its accounting
/// link are separate; a PropAmm quoter must use its own state for both.
pub(crate) fn validate_pressure_link(
    program_id: &Pubkey,
    quoter: Pubkey,
    kind: QuoterKind,
    direction: OfferDirection,
    token_in: Pubkey,
    token_out: Pubkey,
    state: Option<&Account<PropAmmState>>,
) -> Result<Pubkey> {
    let Some(state) = state else {
        require!(kind != QuoterKind::PropAmm, OnreError::IncompatibleQuoter);
        return Ok(Pubkey::default());
    };
    let (expected, bump) =
        Pubkey::find_program_address(&[PROP_AMM, state.quoter.as_ref()], program_id);
    let (asset, managed) = match direction {
        OfferDirection::AssetToManaged => (token_in, token_out),
        OfferDirection::ManagedToAsset => (token_out, token_in),
    };
    require!(
        state.key() == expected
            && state.bump == bump
            && state.asset_mint == asset
            && state.managed_mint == managed,
        OnreError::IncompatibleQuoter
    );
    if kind == QuoterKind::PropAmm {
        require_keys_eq!(state.quoter, quoter, OnreError::IncompatibleQuoter);
    }
    Ok(state.key())
}
