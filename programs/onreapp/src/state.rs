use anchor_lang::prelude::*;

const MAX_SEGMENTS: usize = 10;

/// Represents an offer data structure with time segments and typed structure.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct Offer {
    pub offer_id: u64,
    pub sell_token_mint: Pubkey,
    pub active_segments: u8,
    pub time_segments: [TimeSegment; MAX_SEGMENTS],
    pub offer_type: OfferType,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct TimeSegment {
    pub segment_id: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub start_price: u64,
    pub end_price: u64,
    pub price_fix_duration: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub enum OfferType {
    BuyOffer { 
        buy_token: OfferToken 
    },
    RedemptionOfferSingle { 
        buy_token: OfferToken 
    },
    RedemptionOfferDual { 
        buy_token_1: OfferToken,
        buy_token_2: OfferToken 
    },
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct OfferToken {
    pub mint: Pubkey,
    pub amount: u64,
}

/// Account holding 20 BuyOffer instances
#[account]
#[derive(InitSpace)]
pub struct BuyOfferAccount {
    pub offers: [Offer; 20],
    pub count: u8,
}

/// Account holding 20 RedemptionOfferSingle instances  
#[account]
#[derive(InitSpace)]
pub struct RedemptionOfferSingleAccount {
    pub offers: [Offer; 20],
    pub count: u8,
}

/// Account holding 20 RedemptionOfferDual instances
#[account]
#[derive(InitSpace)]
pub struct RedemptionOfferDualAccount {
    pub offers: [Offer; 20], 
    pub count: u8,
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
