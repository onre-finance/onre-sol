use anchor_lang::prelude::*;

pub const MAX_VECTORS: usize = 10;
pub const MAX_OFFERS: usize = 10;

/// Offer struct for token exchange with dynamic pricing
#[account(zero_copy)]
#[repr(C)]
#[derive(InitSpace)]
pub struct Offer {
    pub token_in_mint: Pubkey,
    pub token_out_mint: Pubkey,
    pub vectors: [OfferVector; MAX_VECTORS],
    pub fee_basis_points: u16,
    needs_approval: u8,
    allow_permissionless: u8,
    pub bump: u8,
    reserved: [u8; 131],
}

impl Offer {
    pub fn needs_approval(&self) -> bool {
        self.needs_approval != 0
    }

    pub fn set_approval(&mut self, needs_approval: bool) {
        self.needs_approval = if needs_approval { 1 } else { 0 };
    }

    pub fn allow_permissionless(&self) -> bool {
        self.allow_permissionless != 0
    }

    pub fn set_permissionless(&mut self, allow_permissionless: bool) {
        self.allow_permissionless = if allow_permissionless { 1 } else { 0 };
    }
}

/// Time vector for offers with pricing information
#[zero_copy]
#[repr(C)]
#[derive(Default, InitSpace)]
pub struct OfferVector {
    pub start_time: u64,
    pub base_time: u64,
    pub base_price: u64,
    /// Annual Percentage Rate (APR)
    ///
    /// APR represents the annualized rate of return for this offer.
    /// It is scaled by 1,000,000 for precision (6 decimal places).
    ///
    /// Examples:
    /// - 0 = 0% APR (fixed price, no yield over time)
    /// - 36_500 = 0.0365% APR (3.65% annual rate)
    /// - 1_000_000 = 1% APR
    /// - 10_000_000 = 10% APR
    ///
    /// The APR determines how the price increases over time intervals.
    pub apr: u64,
    pub price_fix_duration: u64,
}
