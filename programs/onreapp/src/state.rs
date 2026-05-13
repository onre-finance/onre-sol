use crate::constants::{MAX_ADMINS, MAX_CIRCULATING_SUPPLY_EXCLUDED_ACCOUNTS};
use anchor_lang::prelude::*;

/// Global program state containing governance and configuration settings
///
/// Stores the core program authority structure, emergency controls, and trusted entities
/// used for authorization and approval verification across all program operations.
#[account]
#[derive(InitSpace)]
pub struct State {
    /// Primary program authority with full control over all operations
    pub boss: Pubkey,
    /// Proposed new boss for two-step ownership transfer
    pub proposed_boss: Pubkey,
    /// Emergency kill switch to halt critical operations when activated
    pub is_killed: bool,
    /// ONyc token mint used for market calculations and operations
    pub onyc_mint: Pubkey,
    /// Array of admin accounts authorized to enable the kill switch
    pub admins: [Pubkey; MAX_ADMINS],
    /// First trusted authority for cryptographic approval verification
    pub approver1: Pubkey,
    /// Second trusted authority for cryptographic approval verification
    pub approver2: Pubkey,
    /// PDA bump seed for account derivation
    pub bump: u8,
    /// Optional maximum supply cap for ONyc token minting (0 = no cap)
    pub max_supply: u64,
    /// Admin account authorized to manage ONr token mints and redemptions
    pub redemption_admin: Pubkey,
    /// Main offer account used for market operations and price discovery
    pub main_offer: Pubkey,
    /// Reserved space for future program state extensions
    pub reserved: [u8; 64],
}

/// Program-derived authority for permissionless token routing operations
///
/// This PDA manages intermediary accounts used for permissionless offer execution,
/// enabling secure token routing without direct user-boss relationships.
#[account]
#[derive(InitSpace)]
pub struct PermissionlessAuthority {
    /// Optional name identifier for the authority (max 50 characters)
    #[max_len(50)]
    pub name: String,
}

/// Global market statistics PDA holding the canonical protocol-wide metrics.
///
/// This account is intended to be updated by purchase and refresh instructions so
/// off-chain clients can fetch the latest derived market values from one PDA.
#[account]
#[derive(InitSpace)]
pub struct MarketStats {
    /// Latest APY scaled with the program's existing market-info precision.
    pub apy: u64,
    /// Total circulating ONyc supply at the most recent refresh.
    pub circulating_supply: u64,
    /// Latest NAV value using the market-info precision.
    pub nav: u64,
    /// Latest signed NAV adjustment value using the market-info precision.
    pub nav_adjustment: i64,
    /// Latest total value locked across tracked vaults.
    pub tvl: u64,
    /// Unix timestamp of the most recent successful recomputation.
    pub last_updated_at: i64,
    /// Slot of the most recent successful recomputation.
    pub last_updated_slot: u64,
    /// PDA bump seed for account derivation.
    pub bump: u8,
    /// Reserved bytes for forward-compatible layout expansion.
    pub reserved: [u8; 95],
}

/// Boss-managed owner list whose ONyc ATAs are excluded from circulating supply.
#[account]
#[derive(InitSpace)]
pub struct CirculatingSupplyExcludedAccounts {
    /// Owners whose ONyc ATAs must be included in excluded-balance updates.
    pub owners: [Pubkey; MAX_CIRCULATING_SUPPLY_EXCLUDED_ACCOUNTS],
    /// PDA bump seed.
    pub bump: u8,
    /// Reserved bytes for forward-compatible layout expansion.
    pub reserved: [u8; 31],
}

/// Cached ONyc balance excluded from circulating supply.
#[account]
#[derive(InitSpace)]
pub struct CirculatingSupplyExcludedBalance {
    /// Sum of all configured excluded-owner ONyc ATA balances.
    pub amount: u64,
    /// Unix timestamp of the most recent successful update.
    pub last_updated_at: i64,
    /// Slot of the most recent successful update.
    pub last_updated_slot: u64,
    /// PDA bump seed.
    pub bump: u8,
    /// Reserved bytes for forward-compatible layout expansion.
    pub reserved: [u8; 31],
}

/// Fee vault selector used for deriving shared configurable vault PDAs.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurableVaultKind {
    TakeOfferFee,
    RedemptionFee,
    ManagementFee,
    PerformanceFee,
}

impl ConfigurableVaultKind {
    pub fn seed(self) -> &'static [u8] {
        match self {
            Self::TakeOfferFee => crate::constants::seeds::TAKE_OFFER_FEE_VAULT,
            Self::RedemptionFee => crate::constants::seeds::REDEMPTION_FEE_VAULT,
            Self::ManagementFee => crate::constants::seeds::MANAGEMENT_FEE_VAULT,
            Self::PerformanceFee => crate::constants::seeds::PERFORMANCE_FEE_VAULT,
        }
    }

    pub fn as_u8(self) -> u8 {
        match self {
            Self::TakeOfferFee => 0,
            Self::RedemptionFee => 1,
            Self::ManagementFee => 2,
            Self::PerformanceFee => 3,
        }
    }
}

/// Program-owned configurable fee vault authority.
#[account]
#[derive(InitSpace)]
pub struct ConfigurableVault {
    /// Vault kind as `ConfigurableVaultKind::as_u8()`.
    pub kind: u8,
    /// Boss-configured withdrawal destination. Default means unset.
    pub withdrawal_destination: Pubkey,
    /// PDA bump seed.
    pub bump: u8,
    /// Reserved space for future fields.
    pub reserved: [u8; 31],
}
