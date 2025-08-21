use anchor_lang::prelude::*;

/// Represents the program state in the Onre App program.
///
/// Stores the current boss's public key, used for authorization across instructions.
///
/// # Fields
/// - `boss`: Public key of the current boss, set via `initialize` and updated via `set_boss`.
#[account]
#[derive(InitSpace)]
pub struct State {
    pub boss: Pubkey,
}


#[account]
#[derive(InitSpace)]
pub struct VaultAuthority{}