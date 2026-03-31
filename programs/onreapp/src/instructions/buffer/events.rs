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
    pub previous_lowest_supply: u64,
    pub new_lowest_supply: u64,
    pub current_supply: u64,
    pub updated: bool,
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
pub struct ManagementFeesWithdrawnEvent {
    pub amount: u64,
    pub boss: Pubkey,
}

#[event]
pub struct PerformanceFeesWithdrawnEvent {
    pub amount: u64,
    pub boss: Pubkey,
}
