use crate::constants::MAX_BASIS_POINTS;
use anchor_lang::prelude::*;

pub const SECONDS_PER_YEAR: u128 = 31_536_000;
pub const YIELD_SCALE: u128 = 1_000_000;
pub const BASIS_POINTS_SCALE: u128 = MAX_BASIS_POINTS as u128;
pub const BASIS_POINT_TO_YIELD_SCALE: u128 = YIELD_SCALE / BASIS_POINTS_SCALE;

#[account]
#[derive(InitSpace)]
pub struct BufferState {
    pub onyc_mint: Pubkey,
    pub gross_apr: u64,
    pub previous_supply: u64,
    pub management_fee_basis_points: u16,
    pub management_fee_wallet: Pubkey,
    pub performance_fee_basis_points: u16,
    pub performance_fee_wallet: Pubkey,
    pub performance_fee_high_watermark: u64,
    pub last_accrual_timestamp: i64,
    pub bump: u8,
    pub reserved: [u8; 72],
}
