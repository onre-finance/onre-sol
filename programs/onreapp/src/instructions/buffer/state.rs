use anchor_lang::prelude::*;

pub const SECONDS_PER_YEAR: u128 = 31_536_000;
pub const YIELD_SCALE: u128 = 1_000_000;

#[account]
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

impl BufferState {
    pub const INIT_SPACE: usize = 32 + 8 + 8 + 2 + 32 + 2 + 32 + 8 + 8 + 1 + 72;
}
