use anchor_lang::prelude::*;

/// One registration per managed SPL mint, replacing ONyc-specific global configuration.
#[account]
#[derive(InitSpace)]
pub struct ManagedTokenConfig {
    pub managed_mint: Pubkey,
    pub decimals: u8,
    pub enabled: bool,
    /// Program-controlled issuance limits in mint base units; zero disables a limit.
    /// SPL mints cannot enforce these limits on other mint authorities.
    pub max_supply: u64,
    pub max_mint_amount: u64,
    pub bump: u8,
}
