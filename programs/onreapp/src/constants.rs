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

    /// Seed for the global market stats PDA
    pub const MARKET_STATS: &[u8] = b"market_stats";

    /// Seed for the circulating supply excluded owner-list PDA
    pub const CIRCULATING_SUPPLY_EXCLUDED_ACCOUNTS: &[u8] = b"circ_supply_excl_accounts";

    /// Seed for the cached circulating supply excluded-balance PDA
    pub const CIRCULATING_SUPPLY_EXCLUDED_BALANCE: &[u8] = b"circ_supply_excl_balance";

    /// Seed for the redemption offer account
    pub const REDEMPTION_OFFER: &[u8] = b"redemption_offer";

    /// Seed for the redemption offer vault authority account
    pub const REDEMPTION_OFFER_VAULT_AUTHORITY: &[u8] = b"redemption_offer_vault_authority";

    /// Seed for the redemption request account
    pub const REDEMPTION_REQUEST: &[u8] = b"redemption_request";

    /// Seed for the user nonce account
    pub const NONCE_ACCOUNT: &[u8] = b"nonce_account";

    /// Seed for the BUFFER pool state account
    pub const BUFFER_STATE: &[u8] = b"buffer_state";

    /// Seed for the reserve vault authority account
    pub const RESERVE_VAULT_AUTHORITY: &[u8] = b"reserve_vault_authority";

    /// Seed for the redemption fee vault authority PDA

    /// Seed prefix for configurable fee vault authority accounts
    pub const CONFIGURABLE_VAULT: &[u8] = b"configurable_vault";

    /// Seed suffix for offer fee vault authority
    pub const OFFER_FEE_VAULT: &[u8] = b"offer_fee";

    /// Seed suffix for management fee vault authority
    pub const MANAGEMENT_FEE_VAULT: &[u8] = b"management_fee";

    /// Seed suffix for performance fee vault authority
    pub const PERFORMANCE_FEE_VAULT: &[u8] = b"performance_fee";

    /// Seed suffix for prop AMM fee vault authority
    pub const PROP_AMM_FEE_VAULT: &[u8] = b"prop_amm_fee";

    /// Seed suffix for net proceeds vault authority
    pub const OFFER_PROCEEDS_VAULT: &[u8] = b"offer_proceeds";

    /// Seed suffix for prop AMM net proceeds vault authority
    pub const PROP_AMM_PROCEEDS_VAULT: &[u8] = b"prop_amm_proceeds";

    /// Seed for the Prop AMM configuration state PDA
    pub const PROP_AMM_STATE: &[u8] = b"prop_amm_state";
}

/// Maximum number of pricing vectors allowed per offer
pub const MAX_VECTORS: usize = 10;

/// Maximum number of admin accounts that can be stored in program state
pub const MAX_ADMINS: usize = 20;

/// Maximum number of token account owners excluded from circulating supply.
pub const MAX_CIRCULATING_SUPPLY_EXCLUDED_ACCOUNTS: usize = 20;

/// Number of decimals used for price representation
pub const PRICE_DECIMALS: u8 = 9;

/// Maximum possible value of basis points (100%)
pub const MAX_BASIS_POINTS: u16 = 10000;

/// Maximum allowed fee in basis points (10% = 1000 basis points)
pub const MAX_ALLOWED_FEE_BPS: u16 = 1000;

/// Maximum lifetime allowed for swap quotes.
pub const MAX_QUOTE_LIFETIME_SECONDS: i64 = 60;
