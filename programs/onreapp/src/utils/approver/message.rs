use anchor_lang::{AnchorDeserialize, AnchorSerialize};
use anchor_lang::prelude::Pubkey;
use crate::borsh;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct ApprovalMessage {
    pub program_id: Pubkey,
    pub user_pubkey: Pubkey,
    pub expiry_unix: u64,
}
