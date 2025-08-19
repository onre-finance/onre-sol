use std::mem::size_of;


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
    pub start_time: u64,
    pub end_time: u64,
    pub start_price: u64,
    pub end_price: u64,
    pub price_fix_duration: u64,
}

pub const MAX_BUY_OFFERS: usize = 100; // adjust to your value

#[repr(C)]
pub struct BuyOfferAccount {
    pub offers: [BuyOffer; MAX_BUY_OFFERS],
    pub count: u64,
}

fn main() {
    println!("Size of BuyOffer: {} bytes", size_of::<BuyOffer>());
    println!("Size of BuyOfferAccount (struct only): {} bytes", size_of::<BuyOfferAccount>());

    // If using Anchor, remember to add 8 bytes for the discriminator:
    println!("Total account size for Anchor init: {} bytes", 8 + size_of::<BuyOfferAccount>());
}
