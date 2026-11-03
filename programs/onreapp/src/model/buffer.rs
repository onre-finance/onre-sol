use anchor_lang::prelude::*;

/// EVM BufferState equivalent; distinct name preserves the legacy discriminator.
#[account]
#[derive(InitSpace)]
pub struct ManagedTokenBufferState {
    pub managed_mint: Pubkey,
    pub reserve_vault: Pubkey,
    pub management_fee_vault: Pubkey,
    pub performance_fee_vault: Pubkey,
    pub previous_supply: u64,
    pub performance_fee_high_watermark: u64,
    pub last_accrual_timestamp: i64,
    /// Annual gross rate, scale 6.
    pub gross_apr: u64,
    pub management_fee_basis_points: u16,
    pub performance_fee_basis_points: u16,
    pub performance_fee_high_watermark_enabled: bool,
    pub bump: u8,
}
