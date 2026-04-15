use crate::utils::approver::message::ApprovalMessage;
use crate::utils::ed25519_parser::parse_ed25519_ix;
use anchor_lang::prelude::*;
use solana_instructions_sysvar::{load_current_index_checked, load_instruction_at_checked};
use solana_program::ed25519_program;

/// Verifies cryptographic approval messages signed by trusted authorities
///
/// This function performs comprehensive validation of approval messages using Ed25519
/// signature verification. It ensures the approval was signed by one of the two correct
/// authorities, is intended for the current program and user, and has not expired.
///
/// The verification process validates both the approval message content and the
/// cryptographic signature by examining the Ed25519 instruction that must immediately
/// precede the current instruction in the transaction.
///
/// # Arguments
/// * `program_id` - The current program ID for validation context
/// * `user_pubkey` - The user requesting approval
/// * `approver1` - The first authorized signing authority
/// * `approver2` - The second authorized signing authority
/// * `instructions_sysvar` - Instructions sysvar for accessing previous instructions
/// * `msg` - The approval message to verify
///
/// # Returns
/// * `Ok(())` - If approval signature and content are valid with either approver
/// * `Err(_)` - If validation fails with both approvers
///
/// # Validation Steps
/// 1. Expiry time validation against current timestamp
/// 2. Program ID matching verification
/// 3. User public key matching verification
/// 4. Ed25519 signature instruction location and parsing
/// 5. Trusted authority signature verification (against either approver1 or approver2)
/// 6. Signed message content validation
pub fn verify_approval_message_generic(
    program_id: &Pubkey,
    user_pubkey: &Pubkey,
    approver1: &Pubkey,
    approver2: &Pubkey,
    instructions_sysvar: &UncheckedAccount,
    msg: &ApprovalMessage,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp as u64;
    require!(now <= msg.expiry_unix, crate::OnreError::Expired);
    require!(
        msg.program_id == *program_id,
        crate::OnreError::WrongProgram
    );
    require!(
        msg.user_pubkey.key() == user_pubkey.key(),
        crate::OnreError::WrongUser
    );

    // 2) Find the *previous* instruction and ensure it's Ed25519 verify
    let cur_idx = load_current_index_checked(&instructions_sysvar.to_account_info())
        .map_err(|_| crate::OnreError::MissingEd25519Ix)?;
    require!(cur_idx > 0, crate::OnreError::MissingEd25519Ix);

    let ix = load_instruction_at_checked(
        (cur_idx - 1) as usize,
        &instructions_sysvar.to_account_info(),
    )
    .map_err(|_| crate::OnreError::MissingEd25519Ix)?;

    require!(
        ix.program_id == ed25519_program::id(),
        crate::OnreError::WrongIxProgram
    );
    require!(ix.accounts.is_empty(), crate::OnreError::BadEd25519Accounts);

    let parsed = parse_ed25519_ix(&ix.data).ok_or(crate::OnreError::MalformedEd25519Ix)?;
    require!(parsed.sig_count == 1, crate::OnreError::MultipleSigs);

    // Check if the signature is from either approver1 or approver2
    let is_approver1 = *approver1 != Pubkey::default() && parsed.pubkey == approver1.to_bytes();
    let is_approver2 = *approver2 != Pubkey::default() && parsed.pubkey == approver2.to_bytes();
    require!(
        is_approver1 || is_approver2,
        crate::OnreError::WrongAuthority
    );

    let signed_msg = ApprovalMessage::try_from_slice(&parsed.message)
        .map_err(|_| crate::OnreError::MsgDeserialize)?;
    require!(
        signed_msg.program_id == *program_id,
        crate::OnreError::WrongProgram
    );
    require!(
        signed_msg.user_pubkey == *user_pubkey,
        crate::OnreError::WrongUser
    );
    require!(signed_msg.expiry_unix >= now, crate::OnreError::Expired);
    require!(signed_msg == *msg, crate::OnreError::MsgMismatch);

    Ok(())
}
