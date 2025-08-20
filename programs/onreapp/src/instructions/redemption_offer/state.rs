use anchor_lang::prelude::*;

pub const MAX_REDEMPTION_OFFERS: usize = 50;

/// Redemption offer struct for token exchange with static pricing
#[zero_copy]
#[repr(C)]
pub struct SingleRedemptionOffer {
    pub offer_id: u64,
    pub token_in_mint: Pubkey,
    pub token_out_mint: Pubkey,
    pub start_time: u64,
    pub end_time: u64,
    pub price: u64,
}


impl Default for SingleRedemptionOffer {
    fn default() -> Self {
        Self {
            offer_id: 0,
            token_in_mint: Pubkey::default(),
            token_out_mint: Pubkey::default(),
            start_time: 0,
            end_time: 0,
            price: 0
        }
    }
}

/// Account holding MAX_BUY_OFFERS RedemptionOfferSingle instances (should fit 10KB limit)
#[account(zero_copy)]
#[repr(C)]
pub struct SingleRedemptionOfferAccount {
    pub offers: [SingleRedemptionOffer; MAX_REDEMPTION_OFFERS],
    pub counter: u64
}
