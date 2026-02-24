/// PDA seeds used throughout the program for account derivation
pub mod seeds {
    /// Seed for the program state account
    pub const STATE: &[u8] = b"state";

    /// Seed for the offers account
    pub const OFFER: &[u8] = b"offer";

    /// Seed for the offer vault authority account
    pub const OFFER_VAULT_AUTHORITY: &[u8] = b"offer_vault_authority";

    /// Seed for the permissionless intermediary authority account
    pub const PERMISSIONLESS_AUTHORITY: &[u8] = b"permissionless-1";

    /// Seed for mint authority PDA accounts
    pub const MINT_AUTHORITY: &[u8] = b"mint_authority";

    /// Seed for the redemption offer account
    pub const REDEMPTION_OFFER: &[u8] = b"redemption_offer";

    /// Seed for the redemption offer vault authority account
    pub const REDEMPTION_OFFER_VAULT_AUTHORITY: &[u8] = b"redemption_offer_vault_authority";

    /// Seed for the redemption request account
    pub const REDEMPTION_REQUEST: &[u8] = b"redemption_request";

    /// Seed for the user nonce account
    pub const NONCE_ACCOUNT: &[u8] = b"nonce_account";

    /// Seed for the CACHE pool state account
    pub const CACHE_STATE: &[u8] = b"cache_state";

    /// Seed for the CACHE vault authority account
    pub const CACHE_VAULT_AUTHORITY: &[u8] = b"cache_vault_authority";
}

/// Maximum number of pricing vectors allowed per offer
pub const MAX_VECTORS: usize = 10;

/// Maximum number of admin accounts that can be stored in program state
pub const MAX_ADMINS: usize = 20;

/// Number of decimals used for price representation
pub const PRICE_DECIMALS: u8 = 9;

/// Maximum possible value of basis points (100%)
pub const MAX_BASIS_POINTS: u16 = 10000;

/// Maximum allowed fee in basis points (10% = 1000 basis points)
pub const MAX_ALLOWED_FEE_BPS: u16 = 1000;
