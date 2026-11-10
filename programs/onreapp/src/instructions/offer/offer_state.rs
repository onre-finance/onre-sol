use crate::constants::MAX_VECTORS;
use crate::model::PricingVector;
use anchor_lang::prelude::*;

/// Token exchange offer with dynamic APR-based pricing
///
/// Stores configuration for token pair exchanges with time-based pricing vectors
/// that implement compound interest growth using Annual Percentage Rate (APR).
/// Each offer is unique per token pair and supports up to 10 pricing vectors.
#[account(zero_copy)]
#[repr(C)]
#[derive(InitSpace)]
pub struct Offer {
    /// Input token mint for the exchange
    pub token_in_mint: Pubkey,
    /// Output token mint for the exchange
    pub token_out_mint: Pubkey,
    /// Array of pricing vectors defining price evolution over time
    pub vectors: [OfferVector; MAX_VECTORS],
    /// Fee in basis points, capped at 1000 bps (10%), charged when taking the offer.
    pub fee_basis_points: u16,
    /// PDA bump seed for account derivation
    pub bump: u8,
    /// Whether taking the offer requires a signature from state.approver1 or state.approver2.
    needs_approval: u8,
    /// Whether the offer allows permissionless operations (0 = false, 1 = true)
    allow_permissionless: u8,
    /// Whether the offer is disabled by emergency controls (0 = false, 1 = true)
    disabled: u8,
    /// Fee in basis points charged by permissionless offer execution.
    pub fee_basis_points_permissionless: u16,
    /// Reserved space for future fields
    reserved: [u8; 128],
}

impl Offer {
    /// Returns whether taking the offer requires a configured approver signature.
    pub fn needs_approval(&self) -> bool {
        self.needs_approval != 0
    }

    /// Sets the approval requirement for taking the offer
    pub fn set_approval(&mut self, needs_approval: bool) {
        self.needs_approval = if needs_approval { 1 } else { 0 };
    }

    /// Returns whether the offer allows permissionless operations
    pub fn allow_permissionless(&self) -> bool {
        self.allow_permissionless != 0
    }

    /// Sets whether the offer allows permissionless operations
    pub fn set_permissionless(&mut self, allow_permissionless: bool) {
        self.allow_permissionless = if allow_permissionless { 1 } else { 0 };
    }

    /// Returns whether the offer is currently disabled.
    fn is_disabled(&self) -> bool {
        self.disabled != 0
    }

    /// Sets the disabled state for the offer.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = if disabled { 1 } else { 0 };
    }

    /// Returns the fee basis points used by permissionless offer execution.
    pub fn permissionless_fee_basis_points(&self) -> u16 {
        self.fee_basis_points_permissionless
    }

    /// Sets the fee basis points used by permissionless offer execution.
    pub fn set_permissionless_fee_basis_points(&mut self, fee_basis_points: u16) {
        self.fee_basis_points_permissionless = fee_basis_points;
    }

    /// Ensures the offer is currently enabled.
    pub fn require_enabled(&self) -> Result<()> {
        require!(!self.is_disabled(), crate::OnreError::OfferDisabled);
        Ok(())
    }

    pub fn require_mints(&self, token_in_mint: Pubkey, token_out_mint: Pubkey) -> Result<()> {
        require_keys_eq!(
            self.token_in_mint,
            token_in_mint,
            crate::OnreError::InvalidTokenInMint
        );
        require_keys_eq!(
            self.token_out_mint,
            token_out_mint,
            crate::OnreError::InvalidTokenOutMint
        );
        Ok(())
    }
}

/// Time-based pricing vector with APR-driven compound growth
///
/// Defines price evolution over time using Annual Percentage Rate (APR) with
/// discrete pricing steps. Each vector becomes active at start_time and
/// implements compound interest pricing until the next vector activates.
#[zero_copy]
#[repr(C)]
#[derive(Default, InitSpace)]
pub struct OfferVector {
    /// Activation time. If omitted during insertion, defaults to max(base_time, current_time).
    pub start_time: u64,
    /// Base timestamp used for elapsed-time price growth.
    pub base_time: u64,
    /// Initial price with scale=9 (1_000_000_000 = 1.0) at vector start
    pub base_price: u64,
    /// Annual Percentage Rate scaled by 1_000_000 (1_000_000 = 100% APR; 10_000 = 1%)
    ///
    /// Determines compound interest rate for price growth over time.
    /// Scale=6 where 10_000 = 1% annual rate and 1_000_000 = 100%.
    pub apr: u64,
    /// Duration in seconds for each discrete pricing step
    pub price_fix_duration: u64,
}

// Adapt vector values without changing the legacy zero-copy account layout.
impl From<&OfferVector> for PricingVector {
    fn from(vector: &OfferVector) -> Self {
        Self {
            start_time: vector.start_time,
            base_time: vector.base_time,
            base_price: vector.base_price,
            apr: vector.apr,
            price_fix_duration: vector.price_fix_duration,
        }
    }
}

impl From<PricingVector> for OfferVector {
    fn from(vector: PricingVector) -> Self {
        Self {
            start_time: vector.start_time,
            base_time: vector.base_time,
            base_price: vector.base_price,
            apr: vector.apr,
            price_fix_duration: vector.price_fix_duration,
        }
    }
}
