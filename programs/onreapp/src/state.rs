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
