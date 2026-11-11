use anchor_lang::prelude::*;

use crate::constants::MAX_CIRCULATING_SUPPLY_EXCLUDED_ACCOUNTS;

/// Per-managed-mint supply exclusions; entries are SPL token account addresses.
#[account]
#[derive(InitSpace)]
pub struct ManagedTokenSupplyExclusions {
    pub managed_mint: Pubkey,
    pub token_accounts: [Pubkey; MAX_CIRCULATING_SUPPLY_EXCLUDED_ACCOUNTS],
    /// Only the first `count` entries are active; entries must be unique.
    pub count: u8,
    pub bump: u8,
}
