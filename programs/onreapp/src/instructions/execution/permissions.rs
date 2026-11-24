use super::{Execution, Route};
use crate::{
    model::{OfferAccess, OfferPermission},
    state::State,
    utils::{approver::approver_utils::verify_approval_message_generic, ApprovalMessage},
    OnreError,
};
use anchor_lang::prelude::*;

/// The offer's stored access mode selects authorization: permissioned offers need
/// an approver-signed approval for `user`, permissionless offers reject one,
/// allowlisted offers need the user's validated `OfferPermission`. Used by
/// `take_offer` and by `create_fulfillment_request`.
pub(crate) fn authorize(
    program_id: &Pubkey,
    access: OfferAccess,
    user: &Pubkey,
    state: &State,
    instructions_sysvar: &UncheckedAccount,
    approval: Option<&ApprovalMessage>,
    has_permission: bool,
) -> Result<()> {
    match access {
        // An approval is not bound to an offer, so it never opens an allowlisted one.
        OfferAccess::Allowlisted => {
            require!(has_permission, OnreError::PermissionRequired);
            Ok(())
        }
        OfferAccess::Permissioned => verify_approval_message_generic(
            program_id,
            user,
            &state.approver1,
            &state.approver2,
            instructions_sysvar,
            approval.ok_or(OnreError::ApprovalRequired)?,
        ),
        OfferAccess::Permissionless => {
            require!(approval.is_none(), OnreError::UnexpectedApproval);
            Ok(())
        }
    }
}

/// Whether `account` is `user`'s live `OfferPermission` for `offer_config`. Only the
/// boss-only grant creates program-owned accounts of this type, with these fields
/// taken from its PDA seeds; a revoked one is closed and no longer program-owned.
/// Passed as the first remaining account, so the `take_offer` account list is unchanged.
pub(super) fn holds_permission(
    account: Option<&AccountInfo>,
    offer_config: &Pubkey,
    user: &Pubkey,
) -> bool {
    account.is_some_and(|info| {
        info.owner == &crate::ID
            && OfferPermission::try_deserialize(&mut &info.data.borrow()[..])
                .is_ok_and(|p| p.offer_config == *offer_config && p.user == *user)
    })
}

/// A permissionless route's intermediary is the seed-validated canonical PDA, and
/// Anchor validates its canonical ATA addresses and mint-specific token programs;
/// also check stored token authorities, since init_if_needed's existing-account
/// path must not accept an ATA whose SPL owner was changed after creation.
pub(super) fn validate_intermediary(a: &Execution) -> Result<()> {
    let Route::Intermediary {
        authority,
        input,
        output,
        ..
    } = a.route
    else {
        return Ok(());
    };
    let expected = authority.key();
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
