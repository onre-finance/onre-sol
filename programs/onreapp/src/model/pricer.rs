use anchor_lang::prelude::*;

use crate::constants::MAX_VECTORS;

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PricingDenomination {
    Usd,
}

/// Price evolution only; vectors belong to a Pricer, never to an offer.
#[derive(
    AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq,
)]
pub struct PricingVector {
    pub start_time: u64,
    pub base_time: u64,
    /// USD price per managed token, scale 9.
    pub base_price: u64,
    /// Annual rate, scale 6 (1_000_000 = 100%).
    pub apr: u64,
    pub price_fix_duration: u64,
}

/// Independent price source for an immutable managed mint and denomination.
///
/// Multiple instances may price the same mint in the same denomination.
/// Instance zero is the canonical USD source for token-wide BUFFER and reporting.
#[account]
#[derive(InitSpace)]
pub struct Pricer {
    pub instance_id: u64,
    pub managed_mint: Pubkey,
    pub denomination: PricingDenomination,
    /// Fixed slots; `start_time == 0` denotes an empty slot, as in legacy Offer.
    pub vectors: [PricingVector; MAX_VECTORS],
    pub enabled: bool,
    pub bump: u8,
}
