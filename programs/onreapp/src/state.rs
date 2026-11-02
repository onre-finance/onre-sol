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
    /// Emergency kill switch to halt guarded value-moving operations when activated
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
    /// Optional supply cap for program-controlled minting paths (0 = no cap)
    pub max_supply: u64,
    /// Worker authorized to fulfill or cancel redemptions and settle BUFFER
    pub worker: Pubkey,
    /// Optional maximum amount allowed in one logical program mint operation (0 = no cap).
    /// BUFFER accrual applies this to the total gross accrual before fee splitting.
    pub max_mint_amount: u64,
    /// Main offer account used for market operations and price discovery
    pub main_offer: Pubkey,
    /// Reserved space for future program state extensions
    pub reserved: [u8; 56],
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
/// Derived from `["market_stats"]` and recomputed from `State::main_offer` by
/// purchase and refresh instructions. RPC reads return the stored snapshot without
/// recalculation; clients must check the timestamp and slot for freshness. Supply
/// uses the separately cached excluded balance, which may need its own refresh.
#[account]
#[derive(InitSpace)]
pub struct MarketStats {
    /// Latest APY, scale 6 (1_000_000 = 100%).
    pub apy: u64,
    /// Total circulating ONyc supply at the most recent refresh.
    pub circulating_supply: u64,
    /// Latest main-offer NAV, scale 9 (1_000_000_000 = 1).
    pub nav: u64,
    /// Latest signed NAV adjustment, scale 9.
    pub nav_adjustment: i64,
    /// Latest TVL computed as circulating ONyc supply multiplied by NAV, divided by the price scale.
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

/// Configurable accounting vault selector used for deriving shared vault PDAs.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurableVaultKind {
    OfferFee,
    ManagementFee,
    PerformanceFee,
    PropAmmBuyFee,
    OfferProceeds,
    PropAmmProceeds,
    PermissionlessOfferFee,
    RedemptionFee,
    PropAmmSellFee,
}

impl ConfigurableVaultKind {
    pub fn seed(self) -> &'static [u8] {
        match self {
            Self::OfferFee => crate::constants::seeds::OFFER_FEE_VAULT,
            Self::ManagementFee => crate::constants::seeds::MANAGEMENT_FEE_VAULT,
            Self::PerformanceFee => crate::constants::seeds::PERFORMANCE_FEE_VAULT,
            Self::PropAmmBuyFee => crate::constants::seeds::PROP_AMM_BUY_FEE_VAULT,
            Self::OfferProceeds => crate::constants::seeds::OFFER_PROCEEDS_VAULT,
            Self::PropAmmProceeds => crate::constants::seeds::PROP_AMM_PROCEEDS_VAULT,
            Self::PermissionlessOfferFee => crate::constants::seeds::PERMISSIONLESS_OFFER_FEE_VAULT,
            Self::RedemptionFee => crate::constants::seeds::REDEMPTION_FEE_VAULT,
            Self::PropAmmSellFee => crate::constants::seeds::PROP_AMM_SELL_FEE_VAULT,
        }
    }

    pub const fn as_u8(self) -> u8 {
        match self {
            Self::OfferFee => 0,
            Self::ManagementFee => 1,
            Self::PerformanceFee => 2,
            Self::PropAmmBuyFee => 3,
            Self::OfferProceeds => 4,
            Self::PropAmmProceeds => 5,
            Self::PermissionlessOfferFee => 6,
            Self::RedemptionFee => 7,
            Self::PropAmmSellFee => 8,
        }
    }
}

/// Program-owned configurable accounting vault authority.
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
