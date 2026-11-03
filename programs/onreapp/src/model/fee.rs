use anchor_lang::prelude::*;

/// Reusable input-token fee policy, with a reference to a Fee-kind VaultConfig.
#[account]
#[derive(InitSpace)]
pub struct FeeConfig {
    pub instance_id: u64,
    pub fee_vault: Pubkey,
    pub basis_points: u16,
    /// Absolute fee floor in the consuming offer's input mint base units.
    pub minimum_fee_amount: u64,
    pub enabled: bool,
    pub bump: u8,
}
