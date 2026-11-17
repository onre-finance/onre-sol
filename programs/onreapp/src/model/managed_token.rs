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
    /// Program-owned MarketStats account refreshed by modular operations on this mint.
    /// Any MarketStats address is allowed (e.g. the legacy `["market_stats"]` singleton
    /// for ONyc); default means modular reporting is disabled.
    pub market_stats: Pubkey,
    /// 1-of-N SPL multisig used as mint authority for modular minting, with the program
    /// mint-authority PDA as one signer. Default means the PDA mints directly. Legacy
    /// instructions always mint directly. See `utils::program_minter`.
    pub mint_multisig: Pubkey,
}

/// Optional references, `None` when stored as `Pubkey::default()`.
impl ManagedTokenConfig {
    pub fn market_stats(&self) -> Option<Pubkey> {
        super::optional(self.market_stats)
    }
    pub fn mint_multisig(&self) -> Option<Pubkey> {
        super::optional(self.mint_multisig)
    }
}
