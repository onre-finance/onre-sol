use anchor_lang::prelude::*;

/// Derived reporting value, not a second authoritative account or cached global PDA.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarketStats {
    pub managed_mint: Pubkey,
    pub apy: u64,
    pub circulating_supply: u64,
    pub nav: u64,
    pub nav_adjustment: i64,
    /// USD value, scale 9; widened to hold large supplies after decimal normalization.
    pub tvl: u128,
    pub last_updated_at: i64,
    pub last_updated_slot: u64,
}
