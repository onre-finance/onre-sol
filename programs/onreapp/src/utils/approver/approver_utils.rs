use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar;
use solana_program::ed25519_program;
use crate::utils::approver::message::ApprovalMessage;
use crate::utils::ed25519_parser::parse_ed25519_ix;


/// Error codes for approval verification operations
#[error_code]
pub enum ErrorCode {
    /// The approval message timestamp has passed the current time
    #[msg("The approval message has expired.")]
    Expired,
    /// The approval message was signed for a different program ID
    #[msg("The approval message is for the wrong program.")]
    WrongProgram,
    /// The approval message was signed for a different user
    #[msg("The approval message is for the wrong user.")]
    WrongUser,
    /// No Ed25519 instruction found before the current instruction
    #[msg("Missing Ed25519 instruction.")]
    MissingEd25519Ix,
    /// The previous instruction is not an Ed25519 instruction
    #[msg("The instruction is for the wrong program.")]
    WrongIxProgram,
    /// The Ed25519 instruction data is malformed or invalid
    #[msg("Malformed Ed25519 instruction.")]
    MalformedEd25519Ix,
    /// The Ed25519 instruction contains more than one signature
    #[msg("Multiple signatures found in Ed25519 instruction.")]
    MultipleSigs,
    /// The signing authority does not match the trusted authority
    #[msg("The authority public key does not match.")]
    WrongAuthority,
    /// The signed message does not match the provided approval message
    #[msg("The message in the Ed25519 instruction does not match the approval message.")]
    MsgMismatch,
    /// Failed to deserialize the approval message from the signature
    #[msg("Failed to deserialize the approval message.")]
    MsgDeserialize,
}


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
    require!(now <= msg.expiry_unix, ErrorCode::Expired);
    require!(msg.program_id == *program_id, ErrorCode::WrongProgram);
    require!(msg.user_pubkey.key() == user_pubkey.key(), ErrorCode::WrongUser);

    // 2) Find the *previous* instruction and ensure it's Ed25519 verify
    let cur_idx = sysvar::instructions::load_current_index_checked(&instructions_sysvar.to_account_info())
        .map_err(|_| ErrorCode::MissingEd25519Ix)?;
    require!(cur_idx > 0, ErrorCode::MissingEd25519Ix);

    let ix = sysvar::instructions::load_instruction_at_checked(
        (cur_idx - 1) as usize,
        &instructions_sysvar.to_account_info(),
    ).map_err(|_| ErrorCode::MissingEd25519Ix)?;

    require!(ix.program_id == ed25519_program::id(), ErrorCode::WrongIxProgram);

    let parsed = parse_ed25519_ix(&ix.data).ok_or(ErrorCode::MalformedEd25519Ix)?;
    require!(parsed.sig_count == 1, ErrorCode::MultipleSigs);

    // Check if the signature is from either approver1 or approver2
    let is_approver1 = *approver1 != Pubkey::default() && parsed.pubkey == approver1.to_bytes();
    let is_approver2 = *approver2 != Pubkey::default() && parsed.pubkey == approver2.to_bytes();
    require!(is_approver1 || is_approver2, ErrorCode::WrongAuthority);

    let signed_msg = ApprovalMessage::try_from_slice(&parsed.message)
        .map_err(|_| ErrorCode::MsgDeserialize)?;
    require!(signed_msg.program_id == *program_id, ErrorCode::WrongProgram);
    require!(signed_msg.user_pubkey == *user_pubkey, ErrorCode::WrongUser);
    require!(signed_msg.expiry_unix >= Clock::get()?.unix_timestamp as u64, ErrorCode::Expired);

    Ok(())
}