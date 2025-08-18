use anchor_lang::prelude::*;

const MAX_SEGMENTS: usize = 10;

/// Represents an offer data structure with time segments and typed structure.
#[zero_copy]
#[repr(C)]
pub struct Offer {
    pub offer_id: u64,
    pub sell_token_mint: Pubkey,
    pub active_segments: u8,
    pub offer_type: u8, // 0 = BuyOffer, 1 = RedemptionSingle, 2 = RedemptionDual
    pub time_segments: [TimeSegment; MAX_SEGMENTS],
    pub buy_token_1: OfferToken,
    pub buy_token_2: OfferToken, // Only used for RedemptionDual (offer_type == 2)
    pub _padding: [u8; 6], // Padding to align to 8-byte boundary
}

#[zero_copy]
#[repr(C)]
pub struct TimeSegment {
    pub segment_id: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub start_price: u64,
    pub end_price: u64,
    pub price_fix_duration: u64,
}

#[zero_copy]
#[repr(C)]
pub struct OfferToken {
    pub mint: Pubkey,
    pub amount: u64,
}

/// Account holding 20 BuyOffer instances
#[account(zero_copy)]
#[repr(C)]
pub struct BuyOfferAccount {
    pub offers: [Offer; 20],
    pub count: u8,
    pub _padding: [u8; 7], // Padding to align to 8-byte boundary
}

/// Account holding 20 RedemptionOfferSingle instances  
#[account(zero_copy)]
#[repr(C)]
pub struct RedemptionOfferSingleAccount {
    pub offers: [Offer; 20],
    pub count: u8,
    pub _padding: [u8; 7], // Padding to align to 8-byte boundary
}

/// Account holding 20 RedemptionOfferDual instances
#[account(zero_copy)]
#[repr(C)]
pub struct RedemptionOfferDualAccount {
    pub offers: [Offer; 20],
    pub count: u8,
    pub _padding: [u8; 7], // Padding to align to 8-byte boundary
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
