/// PDA seeds used throughout the program for account derivation
pub mod seeds {
    /// Seed for the program state account
    pub const STATE: &[u8] = b"state";
    
    /// Seed for the buy offers account
    pub const BUY_OFFERS: &[u8] = b"buy_offers";
    
    /// Seed for the single redemption offers account
    pub const SINGLE_REDEMPTION_OFFERS: &[u8] = b"single_redemption_offers";
    
    /// Seed for the dual redemption offers account
    pub const DUAL_REDEMPTION_OFFERS: &[u8] = b"dual_redemption_offers";
    
    /// Seed for the vault authority account
    pub const VAULT_AUTHORITY: &[u8] = b"vault_authority";
    
    /// Seed for the permissionless intermediary authority account
    pub const PERMISSIONLESS_1: &[u8] = b"permissionless-1";
}