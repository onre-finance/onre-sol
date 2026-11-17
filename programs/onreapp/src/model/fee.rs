use anchor_lang::prelude::*;

/// Reusable input-token fee policy, with a reference to a Fee-kind VaultConfig.
///
/// Bound to input-token decimals rather than a mint: any offer whose input token
/// has `token_decimals` decimals may use it, so e.g. USDC and USDG offers (both 6
/// decimals) can share one fee policy with the same absolute minimum.
#[account]
#[derive(InitSpace)]
pub struct FeeConfig {
    pub instance_id: u64,
    pub fee_vault: Pubkey,
    /// Decimals of every consuming offer's input token; immutable.
    pub token_decimals: u8,
    pub basis_points: u16,
    /// Absolute fee floor in base units of a `token_decimals`-decimal input token.
    pub minimum_fee_amount: u64,
    pub enabled: bool,
    pub bump: u8,
}
