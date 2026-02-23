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
    /// Input token mint for redemptions (e.g., ONyc)
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
    pub requested_redemptions: u128,
    /// Fee in basis points (1000 = 10%) charged when fulfilling redemption requests
    pub fee_basis_points: u16,
    /// Counter for sequential redemption request numbering
    /// Increments with each new redemption request created
    pub request_counter: u64,
    /// PDA bump seed for account derivation
    pub bump: u8,
    /// Reserved space for future fields
    pub reserved: [u8; 109],
}

#[account]
#[derive(InitSpace)]
pub struct RedemptionRequest {
    /// Reference to the RedemptionOffer PDA
    pub offer: Pubkey,
    /// Unique sequential identifier for this request (counter value used for PDA derivation)
    pub request_id: u64,
    /// User requesting the redemption
    pub redeemer: Pubkey,
    /// Amount of token_in tokens requested for redemption
    pub amount: u64,
    /// Amount of token_in tokens that have already been fulfilled (partial fulfillment tracking)
    ///
    /// Starts at 0. Incremented by each partial or full fulfillment call.
    /// When fulfilled_amount == amount the request is fully settled and the account is closed.
    /// remaining = amount - fulfilled_amount is still locked in the redemption vault.
    pub fulfilled_amount: u64,
    /// PDA bump seed for account derivation
    pub bump: u8,
    /// Reserved space for future fields
    pub reserved: [u8; 119],
}
