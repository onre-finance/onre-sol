use anchor_lang::prelude::*;

/// Worker-flow input escrow claim. Price and fees are resolved at fulfillment.
#[account]
#[derive(InitSpace)]
pub struct FulfillmentRequest {
    pub offer_config: Pubkey,
    pub user: Pubkey,
    pub request_id: u64,
    pub input_amount: u64,
    pub fulfilled_input_amount: u64,
    pub bump: u8,
}
