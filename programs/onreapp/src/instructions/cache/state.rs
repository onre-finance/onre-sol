use anchor_lang::prelude::*;

pub const SECONDS_PER_YEAR: u128 = 31_536_000;
pub const YIELD_SCALE: u128 = 1_000_000;

#[account]
#[derive(InitSpace)]
pub struct CacheState {
    pub onyc_mint: Pubkey,
    pub cache_admin: Pubkey,
    pub gross_yield: u64,
    pub current_yield: u64,
    pub lowest_supply: u64,
    pub last_accrual_timestamp: i64,
    pub bump: u8,
    pub reserved: [u8; 95],
}
