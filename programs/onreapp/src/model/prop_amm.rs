use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PropAmmConfig {
    pub epoch_duration_seconds: u64,
    pub curve_exponent_scaled: u32,
    pub cadence_threshold: u32,
    pub cadence_wave_scaled: u32,
    pub wall_sensitivity_scaled: u32,
    pub curve_peg_haircut_bps: u16,
}

/// One immutable pair per Prop AMM quoter; both directed offers may share its pressure.
#[account]
#[derive(InitSpace)]
pub struct PropAmmState {
    pub quoter: Pubkey,
    pub asset_mint: Pubkey,
    pub managed_mint: Pubkey,
    pub config: PropAmmConfig,
    /// Aggregate values in asset base units; widened beyond individual SPL transfers.
    pub current_sell_value_stable: u128,
    pub current_buy_value_stable: u128,
    pub previous_net_sell_value_stable: u128,
    pub epoch_start: i64,
    pub current_sell_trade_count: u32,
    pub bump: u8,
}
