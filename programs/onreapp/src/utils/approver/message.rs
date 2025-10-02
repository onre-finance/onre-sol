use anchor_lang::{AnchorDeserialize, AnchorSerialize};
use anchor_lang::prelude::Pubkey;
use crate::borsh;

/// Message structure for approval verification
///
/// This structure contains the data required to verify that a user has received
/// approval from a trusted authority to perform a specific action within the program.
/// The message is signed by the trusted authority using Ed25519 signature.
///
/// # Fields
/// - `program_id`: The ID of the program for which this approval is valid
/// - `user_pubkey`: The public key of the user who is approved to perform the action
/// - `expiry_unix`: Unix timestamp when this approval expires
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct ApprovalMessage {
    /// The program ID this approval is valid for
    pub program_id: Pubkey,
    /// The user public key that is approved
    pub user_pubkey: Pubkey,
    /// Unix timestamp when this approval expires
    pub expiry_unix: u64,
}
