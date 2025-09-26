use anchor_lang::prelude::*;
use anchor_lang::prelude::borsh::BorshDeserialize;
use anchor_lang::solana_program::{sysvar, ed25519_program};
use crate::utils::ed25519_parser::parse_ed25519_ix;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct ApprovalMessage {
    pub program_id: Pubkey,
    pub user_pubkey: Pubkey,
    pub amount: u64,
    pub expiry_unix: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The approval message has expired.")]
    Expired,
    #[msg("The approval message is for the wrong program.")]
    WrongProgram,
    #[msg("The approval message is for the wrong user.")]
    WrongUser,
    #[msg("Missing Ed25519 instruction.")]
    MissingEd25519Ix,
    #[msg("The instruction is for the wrong program.")]
    WrongIxProgram,
    #[msg("Malformed Ed25519 instruction.")]
    MalformedEd25519Ix,
    #[msg("Multiple signatures found in Ed25519 instruction.")]
    MultipleSigs,
    #[msg("The authority public key does not match.")]
    WrongAuthority,
    #[msg("The message in the Ed25519 instruction does not match the approval message.")]
    MsgMismatch,
    #[msg("Failed to deserialize the approval message.")]
    MsgDeserialize,
}


pub fn verify_approval_message_generic(
    program_id: &Pubkey,
    user_pubkey: &Pubkey,
    trusted_pubkey: &Pubkey,
    instructions_sysvar: &UncheckedAccount,
    msg: &ApprovalMessage,
    _offer_id: u64,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp as u64;
    require!(now <= msg.expiry_unix, ErrorCode::Expired);
    require!(msg.program_id == *program_id, ErrorCode::WrongProgram);
    require!(msg.user_pubkey.key().to_bytes() == user_pubkey.key().to_bytes(), ErrorCode::WrongUser);

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
    require!(parsed.pubkey == trusted_pubkey.to_bytes(), ErrorCode::WrongAuthority);
    let signed_msg = ApprovalMessage::try_from_slice(&parsed.message)
        .map_err(|_| ErrorCode::MsgDeserialize)?;
    require!(signed_msg.program_id == *program_id, ErrorCode::WrongProgram);
    require!(signed_msg.user_pubkey == *user_pubkey, ErrorCode::WrongUser);
    require!(signed_msg.expiry_unix >= Clock::get()?.unix_timestamp as u64, ErrorCode::Expired);

    Ok(())
}