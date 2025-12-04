use anchor_lang::prelude::*;

/// Redemption offer for converting ONyc tokens back to stable tokens
///
/// Manages the redemption process where users can exchange ONyc (in-token)
/// for stable tokens like USDC (out-token) at the current NAV price.
/// This is the inverse of the standard Offer which exchanges stable tokens for ONyc.
#[account]
#[derive(InitSpace)]
pub struct RedemptionOffer {
    /// Reference to the original Offer PDA that this redemption offer is associated with
    pub offer: Pubkey,
    /// Input token mint for redemptions (must be ONyc mint from State)
    pub token_in_mint: Pubkey,
    /// Output token mint for redemptions (e.g., USDC)
    pub token_out_mint: Pubkey,
    /// Cumulative total of all executed redemptions over the contract's lifetime
    ///
    /// This tracks the total amount of ONyc that has been redeemed and burned.
    /// Uses u128 because cumulative redemptions can exceed the current total supply.
    pub executed_redemptions: u128,
    /// Total amount of pending redemption requests
    ///
    /// This tracks ONyc tokens that are locked in pending redemption requests.
    /// Uses u64 because pending redemptions cannot exceed the token's total supply.
    pub requested_redemptions: u64,
    /// Fee in basis points (10000 = 100%) charged when fulfilling redemption requests
    pub fee_basis_points: u16,
    /// PDA bump seed for account derivation
    pub bump: u8,
    /// Reserved space for future fields
    pub reserved: [u8; 117],
}

#[account]
#[derive(InitSpace)]
pub struct RedemptionRequest {
    /// Reference to the RedemptionOffer PDA
    pub offer: Pubkey,
    /// User requesting the redemption
    pub redeemer: Pubkey,
    /// Amount of token_in tokens requested for redemption
    pub amount: u64,
    /// Status of the redemption request
    /// 0: Pending, 1: Executed, 2: Cancelled
    pub status: u8,
    /// PDA bump seed for account derivation
    pub bump: u8,
    /// Reserved space for future fields
    pub reserved: [u8; 126],
}

/// Status of a redemption request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RedemptionRequestStatus {
    Pending = 0,
    Executed = 1,
    Cancelled = 2,
}

impl RedemptionRequestStatus {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// User nonce account for preventing replay attacks.
///
/// Each user has a unique nonce account that is incremented with each successful transaction.
#[account]
#[derive(InitSpace)]
pub struct UserNonceAccount {
    pub nonce: u64,
}
