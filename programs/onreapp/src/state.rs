use anchor_lang::prelude::*;

const MAX_SEGMENTS: usize = 10;

// Offer structs
// Represents an offer data structure with time segments and typed structure.

#[zero_copy]
#[repr(C)]
pub struct RedemptionSingleOffer {
    pub offer_id: u64,
    pub token_in_mint: Pubkey,
    pub token_out_mint: Pubkey,
    pub time_segments: [RedemptionSingleOfferTimeSegment; MAX_SEGMENTS],
}

/// Redemption offer that pays out in two different tokens based on a ratio
/// 
/// The ratio determines how much goes to token_out_one vs token_out_two:
/// - Uses basis points (10000 = 100%)
/// - Example: ratio = 8000 means 80% goes to token_out_one, 20% to token_out_two
/// - For 1e9 token_in with ratio 8000: 0.8e9 in token_out_one, 0.2e9 in token_out_two
#[zero_copy]
#[repr(C)]
pub struct RedemptionDualOffer {
    pub offer_id: u64,
    pub token_in_mint: Pubkey,
    pub token_out_one_mint: Pubkey,
    pub token_out_two_mint: Pubkey,
    pub time_segments: [RedemptionDualOfferTimeSegment; MAX_SEGMENTS],
}

#[zero_copy]
#[repr(C)]
pub struct RedemptionSingleOfferTimeSegment {
    pub segment_id: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub price: u64,
}

#[zero_copy]
#[repr(C)]
pub struct RedemptionDualOfferTimeSegment {
    pub segment_id: u64,
    pub start_time: u64,
    pub end_time: u64,
    /// Percentage of token_in that goes to token_out_one (in basis points, max 10000 = 100%)
    /// Remaining amount (10000 - token_out_one_ratio) goes to token_out_two
    pub token_out_one_ratio: u64,
    pub price_token_one: u64,
    pub price_token_two: u64,
}

/// Account holding 20 RedemptionDualOffer instances
#[account(zero_copy)]
#[repr(C)]
pub struct RedemptionOfferDualAccount {
    pub offers: [RedemptionDualOffer; 20],
    pub count: u64,
}

/// Represents the program state in the Onre App program.
///
/// Stores the current boss's public key, used for authorization across instructions.
///
/// # Fields
/// - `boss`: Public key of the current boss, set via `initialize` and updated via `set_boss`.
#[account]
#[derive(InitSpace)]
pub struct State {
    pub boss: Pubkey,
}
