use anchor_lang::prelude::*;

/// Represents the program state in the Onre App program.
///
/// Stores the current boss's public key and kill switch state, used for authorization across instructions.
///
/// # Fields
/// - `boss`: Public key of the current boss, set via `initialize` and updated via `set_boss`.
/// - `is_killed`: Kill switch state - when true, certain operations are disabled for emergency purposes.
#[account]
#[derive(InitSpace)]
pub struct State {
    pub boss: Pubkey,
    pub is_killed: bool,
}

#[account]
#[derive(InitSpace)]
pub struct OfferVaultAuthority {}

#[account]
#[derive(InitSpace)]
pub struct PermissionlessAccount {
    #[max_len(50)]
    pub name: String,
}

#[account]
#[derive(InitSpace)]
pub struct AdminState {
    pub admins: [Pubkey; MAX_ADMINS],
}

pub const MAX_ADMINS: usize = 20;
