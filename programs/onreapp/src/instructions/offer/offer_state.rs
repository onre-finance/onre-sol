use anchor_lang::prelude::*;

const MAX_VECTORS: usize = 10;
pub const MAX_OFFERS: usize = 10;

/// Offer struct for token exchange with dynamic pricing
#[zero_copy]
#[repr(C)]
#[derive(Default)]
pub struct Offer {
    pub offer_id: u64,
    pub token_in_mint: Pubkey,
    pub token_out_mint: Pubkey,
    pub vectors: [OfferVector; MAX_VECTORS],
    pub counter: u64,
    pub fee_basis_points: u64,
}

/// Time vector for offers with pricing information
#[zero_copy]
#[repr(C)]
#[derive(Default)]
pub struct OfferVector {
    pub vector_id: u64,
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

/// Account holding MAX_OFFERS Offer instances
#[account(zero_copy)]
#[repr(C)]
pub struct OfferAccount {
    pub offers: [Offer; MAX_OFFERS],
    pub counter: u64,
}
