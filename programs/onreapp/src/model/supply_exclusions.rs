use anchor_lang::prelude::*;

use crate::constants::MAX_CIRCULATING_SUPPLY_EXCLUDED_ACCOUNTS;

/// Per-managed-mint supply exclusions at `["supply_exclusions", managed_mint]`.
///
/// Solana cannot read the excluded balances when circulating supply is needed
/// (every account would have to be supplied), so their summed balance is cached
/// here: computed when the boss sets the list and on each permissionless refresh.
/// Readers use only `excluded_amount`.
#[account]
#[derive(InitSpace)]
pub struct ManagedTokenSupplyExclusions {
    pub managed_mint: Pubkey,
    /// Token accounts of `managed_mint`; only the first `count` entries are active
    /// and they are unique. Refreshes must supply exactly these, in order.
    pub token_accounts: [Pubkey; MAX_CIRCULATING_SUPPLY_EXCLUDED_ACCOUNTS],
    pub count: u8,
    /// Summed balance of the active token accounts at the last set or refresh.
    pub excluded_amount: u64,
    pub last_updated_at: i64,
    pub last_updated_slot: u64,
    pub bump: u8,
}

impl ManagedTokenSupplyExclusions {
    pub fn active(&self) -> &[Pubkey] {
        &self.token_accounts[..self.count as usize]
    }
}
