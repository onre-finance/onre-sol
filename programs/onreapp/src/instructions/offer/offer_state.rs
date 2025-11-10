use anchor_lang::prelude::*;

pub const MAX_VECTORS: usize = 10;

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
    /// Fee in basis points (10000 = 100%) charged when taking the offer
    pub fee_basis_points: u16,
    /// PDA bump seed for account derivation
    pub bump: u8,
    /// Whether the offer requires boss approval for taking (0 = false, 1 = true)
    needs_approval: u8,
    /// Whether the offer allows permissionless operations (0 = false, 1 = true)
    allow_permissionless: u8,
    /// Reserved space for future fields
    reserved: [u8; 131],
}

impl Offer {
    /// Returns whether the offer requires boss approval for taking
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
    /// Calculated activation time: max(base_time, current_time) when vector was added
    pub start_time: u64,
    /// Original requested activation time before current_time adjustment
    pub base_time: u64,
    /// Initial price with scale=9 (1_000_000_000 = 1.0) at vector start
    pub base_price: u64,
    /// Annual Percentage Rate scaled by 1_000_000 (1_000_000 = 1% APR)
    ///
    /// Determines compound interest rate for price growth over time.
    /// Scale=6 where 1_000_000 = 1% annual rate.
    pub apr: u64,
    /// Duration in seconds for each discrete pricing step
    pub price_fix_duration: u64,
}
