use crate::{
    constants::seeds,
    model::OfferFlow,
    utils::{approver::approver_utils::verify_approval_message_generic, ApprovalMessage},
    OnreError,
};
use anchor_lang::prelude::*;

use super::TakeOffer;

/// The stored flow selects authorization; caller-supplied accounts cannot switch it.
pub(super) fn authorize(
    program_id: &Pubkey,
    a: &TakeOffer,
    approval: Option<&ApprovalMessage>,
) -> Result<()> {
    match a.offer_config.flow {
        OfferFlow::Permissioned => verify_approval_message_generic(
            program_id,
            &a.user.key(),
            &a.state.approver1,
            &a.state.approver2,
            &a.instructions_sysvar,
            approval.ok_or(OnreError::ApprovalRequired)?,
        ),
        OfferFlow::Permissionless => {
            require!(approval.is_none(), OnreError::UnexpectedApproval);
            Ok(())
        }
        OfferFlow::Worker => err!(OnreError::UnsupportedOfferExecution),
    }
}

pub(super) fn validate_accounts(program_id: &Pubkey, a: &TakeOffer) -> Result<()> {
    if a.offer_config.flow != OfferFlow::Permissionless {
        require!(
            a.permissionless_authority.is_none()
                && a.permissionless_token_in.is_none()
                && a.permissionless_token_out.is_none(),
            OnreError::PermissionlessNotAllowed
        );
        return Ok(());
    }
    let authority = a
        .permissionless_authority
        .as_ref()
        .ok_or(OnreError::WrongAuthority)?;
    let expected = Pubkey::find_program_address(&[seeds::PERMISSIONLESS_AUTHORITY], program_id).0;
    require_keys_eq!(authority.key(), expected, OnreError::WrongAuthority);
    let input = a
        .permissionless_token_in
        .as_ref()
        .ok_or(OnreError::InvalidVaultTokenInAccount)?;
    let output = a
        .permissionless_token_out
        .as_ref()
        .ok_or(OnreError::InvalidPermissionlessTokenOutAccount)?;
    // Anchor validates canonical ATA addresses and mint-specific token programs.
    // Also check stored token authorities: init_if_needed's existing-account path
    // must not accept an ATA whose SPL owner was changed after creation.
    require!(
        input.owner == expected && input.mint == a.token_in_mint.key(),
        OnreError::InvalidVaultTokenInAccount
    );
    require!(
        output.owner == expected && output.mint == a.token_out_mint.key(),
        OnreError::InvalidPermissionlessTokenOutAccount
    );
    Ok(())
}
