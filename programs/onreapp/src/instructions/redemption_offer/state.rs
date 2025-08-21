use anchor_lang::prelude::*;

pub const MAX_REDEMPTION_OFFERS: usize = 50;
pub const MAX_DUAL_REDEMPTION_OFFERS: usize = 50;

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

/// Account holding MAX_REDEMPTION_OFFERS SingleRedemptionOffer instances (should fit 10KB limit)
#[account(zero_copy)]
#[repr(C)]
pub struct SingleRedemptionOfferAccount {
    pub offers: [SingleRedemptionOffer; MAX_REDEMPTION_OFFERS],
    pub counter: u64
}

/// Dual redemption offer struct for token exchange with static pricing for two output tokens
#[zero_copy]
#[repr(C)]
pub struct DualRedemptionOffer {
    pub offer_id: u64,
    pub token_in_mint: Pubkey,
    pub token_out_mint_1: Pubkey,
    pub token_out_mint_2: Pubkey,
    pub start_time: u64,
    pub end_time: u64,
    pub price_1: u64, // Price for token_out_1 with 9 decimal precision
    pub price_2: u64, // Price for token_out_2 with 9 decimal precision
    pub ratio_basis_points: u16, // Basis points (e.g., 8000 = 80% for token_out_1, 20% for token_out_2)
    pub _padding: [u8; 6], // Padding to align struct size to 64 bytes
}

impl Default for DualRedemptionOffer {
    fn default() -> Self {
        Self {
            offer_id: 0,
            token_in_mint: Pubkey::default(),
            token_out_mint_1: Pubkey::default(),
            token_out_mint_2: Pubkey::default(),
            start_time: 0,
            end_time: 0,
            price_1: 0,
            price_2: 0,
            ratio_basis_points: 0,
            _padding: [0; 6], // Ensure struct size is 64 bytes
        }
    }
}

/// Account holding MAX_DUAL_REDEMPTION_OFFERS DualRedemptionOffer instances (should fit 10KB limit)
#[account(zero_copy)]
#[repr(C)]
pub struct DualRedemptionOfferAccount {
    pub offers: [DualRedemptionOffer; MAX_DUAL_REDEMPTION_OFFERS],
    pub counter: u64
}
