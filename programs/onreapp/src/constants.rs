/// PDA seeds used throughout the program for account derivation
pub mod seeds {
    /// Seed for the program state account
    pub const STATE: &[u8] = b"state";

    /// Seed for the kill switch account
    pub const KILL_SWITCH_STATE: &[u8] = b"kill_switch";

    /// Seed for the offers account
    pub const OFFERS: &[u8] = b"offers";

    /// Seed for the offer vault authority account
    pub const OFFER_VAULT_AUTHORITY: &[u8] = b"offer_vault_authority";

    /// Seed for the permissionless intermediary authority account
    pub const PERMISSIONLESS_1: &[u8] = b"permissionless-1";

    /// Seed for the admin state account
    pub const ADMIN_STATE: &[u8] = b"admin_state";

    /// Seed for mint authority PDA accounts
    pub const MINT_AUTHORITY: &[u8] = b"mint_authority";
}
