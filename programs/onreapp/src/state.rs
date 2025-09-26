use anchor_lang::prelude::*;

/// Represents the program state in the Onre App program.
///
/// Stores the current boss's public key, kill switch state, and admin list used for authorization across instructions.
///
/// # Fields
/// - `boss`: Public key of the current boss, set via `initialize` and updated via `set_boss`.
/// - `is_killed`: Kill switch state - when true, certain operations are disabled for emergency purposes.
/// - `admins`: Array of admin public keys who can enable the kill switch.
#[account]
#[derive(InitSpace)]
pub struct State {
    pub boss: Pubkey,
    pub is_killed: bool,
    pub onyc_mint: Pubkey,
    pub admins: [Pubkey; MAX_ADMINS],
    pub approver: Pubkey, // A trusted entity
    pub bump: u8,
    pub reserved: [u8; 128],
}

#[account]
#[derive(InitSpace)]
pub struct OfferVaultAuthority {
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct MintAuthority {
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct PermissionlessAccount {
    #[max_len(50)]
    pub name: String,
}

pub const MAX_ADMINS: usize = 20;
