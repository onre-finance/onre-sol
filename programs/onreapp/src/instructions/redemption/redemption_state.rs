use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Redemption {
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub bump: u8,
}



#[account]
#[derive(InitSpace)]
pub struct UserRedemption {
    pub redemption_id: u128,
    pub to_redeem: u64,
    pub redeemed: u64,
    pub bump: u8,
}