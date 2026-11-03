use anchor_lang::prelude::*;

use super::PricingDenomination;

/// Boss-attested denomination of an external asset; mint metadata is not a price oracle.
#[account]
#[derive(InitSpace)]
pub struct AssetConfig {
    pub asset_mint: Pubkey,
    pub denomination: PricingDenomination,
    pub enabled: bool,
    pub bump: u8,
}
