use anchor_lang::prelude::*;

#[event]
pub struct BufferInitializedEvent {
    pub buffer_state: Pubkey,
    pub onyc_mint: Pubkey,
    pub main_offer: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct BufferMainOfferUpdatedEvent {
    pub old_main_offer: Pubkey,
    pub new_main_offer: Pubkey,
}

#[event]
pub struct BufferGrossYieldUpdatedEvent {
    pub gross_yield: u64,
}

#[event]
pub struct BufferFeeConfigUpdatedEvent {
    pub old_management_fee_basis_points: u16,
    pub new_management_fee_basis_points: u16,
    pub old_management_fee_wallet: Pubkey,
    pub new_management_fee_wallet: Pubkey,
    pub old_performance_fee_basis_points: u16,
    pub new_performance_fee_basis_points: u16,
    pub old_performance_fee_wallet: Pubkey,
    pub new_performance_fee_wallet: Pubkey,
}

#[event]
pub struct BufferLowestSupplyUpdatedEvent {
    pub old_previous_supply: u64,
    pub new_previous_supply: u64,
    pub current_supply: u64,
    pub updated: bool,
    pub timestamp: i64,
}

#[event]
pub struct BufferAccruedEvent {
    pub token_in_mint: Pubkey,
    pub onyc_mint: Pubkey,
    pub seconds_elapsed: u64,
    pub apr_delta: u64,
    pub buffer_mint_amount: u64,
    pub reserve_mint_amount: u64,
    pub management_fee_mint_amount: u64,
    pub performance_fee_mint_amount: u64,
    pub old_previous_supply: u64,
    pub new_previous_supply: u64,
    pub old_previous_performance_fee_high_watermark: u64,
    pub new_performance_fee_high_watermark: u64,
    pub current_nav: u64,
    pub post_accrual_supply: u64,
    pub timestamp: i64,
}

#[event]
pub struct BufferBurnedForNavEvent {
    pub burn_amount: u64,
    pub asset_adjustment_amount: u64,
    pub total_assets: u64,
    pub target_nav: u64,
}

#[event]
pub struct ReserveVaultDepositedEvent {
    pub amount: u64,
    pub mint: Pubkey,
    pub depositor: Pubkey,
}

#[event]
pub struct ReserveVaultWithdrawnEvent {
    pub amount: u64,
    pub mint: Pubkey,
    pub boss: Pubkey,
}

#[event]
pub struct ManagementFeesWithdrawnEvent {
    pub amount: u64,
    pub boss: Pubkey,
}

#[event]
pub struct PerformanceFeesWithdrawnEvent {
    pub amount: u64,
    pub boss: Pubkey,
}
