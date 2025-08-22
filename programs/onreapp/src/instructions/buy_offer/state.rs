use anchor_lang::prelude::*;

const MAX_SEGMENTS: usize = 10;
pub const MAX_BUY_OFFERS: usize = 10;

/// Buy offer struct for token exchange with dynamic pricing
#[zero_copy]
#[repr(C)]
pub struct BuyOffer {
    pub offer_id: u64,
    pub token_in_mint: Pubkey,
    pub token_out_mint: Pubkey,
    pub time_segments: [BuyOfferTimeSegment; MAX_SEGMENTS],
}

/// Time segment for buy offers with pricing information
#[zero_copy]
#[repr(C)]
pub struct BuyOfferTimeSegment {
    pub segment_id: u64,
    pub valid_from: u64,
    pub start_time: u64,
    pub start_price: u64,
    // Price yield percentage * 10000 (with 4 decimal places)
    pub price_yield: u64,
    pub price_fix_duration: u64,
}

impl Default for BuyOfferTimeSegment {
    fn default() -> Self {
        Self {
            segment_id: 0,
            valid_from: 0,
            start_time: 0,
            start_price: 0,
            price_yield: 0,
            price_fix_duration: 0,
        }
    }
}

impl Default for BuyOffer {
    fn default() -> Self {
        Self {
            offer_id: 0,
            token_in_mint: Pubkey::default(),
            token_out_mint: Pubkey::default(),
            time_segments: [BuyOfferTimeSegment::default(); MAX_SEGMENTS],
        }
    }
}

/// Account holding MAX_BUY_OFFERS BuyOffer instances
#[account(zero_copy)]
#[repr(C)]
pub struct BuyOfferAccount {
    pub offers: [BuyOffer; MAX_BUY_OFFERS],
    pub counter: u64
}
