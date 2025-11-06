use anchor_lang::prelude::*;

/// Global program state containing governance and configuration settings
///
/// Stores the core program authority structure, emergency controls, and trusted entities
/// used for authorization and approval verification across all program operations.
#[account]
#[derive(InitSpace)]
pub struct State {
    /// Primary program authority with full control over all operations
    pub boss: Pubkey,
    /// Proposed new boss for two-step ownership transfer
    pub proposed_boss: Pubkey,
    /// Emergency kill switch to halt critical operations when activated
    pub is_killed: bool,
    /// ONyc token mint used for market calculations and operations
    pub onyc_mint: Pubkey,
    /// Array of admin accounts authorized to enable the kill switch
    pub admins: [Pubkey; MAX_ADMINS],
    /// First trusted authority for cryptographic approval verification
    pub approver1: Pubkey,
    /// Second trusted authority for cryptographic approval verification
    pub approver2: Pubkey,
    /// PDA bump seed for account derivation
    pub bump: u8,
    /// Optional maximum supply cap for ONyc token minting (0 = no cap)
    pub max_supply: u64,
    /// Reserved space for future program state extensions
    pub reserved: [u8; 56],
}

/// Program-derived authority for controlling offer vault token accounts
///
/// This PDA manages token transfers and burning operations for the vault accounts
/// used in burn/mint token exchange architecture.
#[account]
#[derive(InitSpace)]
pub struct OfferVaultAuthority {
    /// PDA bump seed for account derivation
    pub bump: u8,
}

/// Program-derived authority for direct token minting operations
///
/// This PDA enables the program to mint tokens directly when it has mint authority,
/// supporting efficient burn/mint token exchange mechanisms.
#[account]
#[derive(InitSpace)]
pub struct MintAuthority {
    /// PDA bump seed for account derivation
    pub bump: u8,
}

/// Program-derived authority for permissionless token routing operations
///
/// This PDA manages intermediary accounts used for permissionless offer execution,
/// enabling secure token routing without direct user-boss relationships.
#[account]
#[derive(InitSpace)]
pub struct PermissionlessAuthority {
    /// Optional name identifier for the authority (max 50 characters)
    #[max_len(50)]
    pub name: String,
    /// PDA bump seed for account derivation
    pub bump: u8,
}

/// Maximum number of admin accounts that can be stored in program state
pub const MAX_ADMINS: usize = 20;
