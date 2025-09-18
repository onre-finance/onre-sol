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
    
    /// Seed for the buy offer vault authority account
    pub const BUY_OFFER_VAULT_AUTHORITY: &[u8] = b"buy_offer_vault_authority";
    
    /// Seed for the single redemption vault authority account
    pub const SINGLE_REDEMPTION_VAULT_AUTHORITY: &[u8] = b"single_redemption_vault_auth";
    
    /// Seed for the dual redemption vault authority account
    pub const DUAL_REDEMPTION_VAULT_AUTHORITY: &[u8] = b"dual_redemption_vault_auth";
    
    /// Seed for the permissionless intermediary authority account
    pub const PERMISSIONLESS_1: &[u8] = b"permissionless-1";

    /// Seed for the admin state account
    pub const ADMIN_STATE: &[u8] = b"admin_state";

    /// Seed for mint authority PDA accounts
    pub const MINT_AUTHORITY: &[u8] = b"mint_authority";
}