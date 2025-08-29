use anchor_lang::prelude::*;

/// Enum to specify which vault authority type to use for vault operations
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum VaultAuthorityType {
    BuyOffer,
    SingleRedemption,
    DualRedemption,
}