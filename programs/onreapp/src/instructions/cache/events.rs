use anchor_lang::prelude::*;

#[event]
pub struct CacheInitializedEvent {
    pub cache_state: Pubkey,
    pub onyc_mint: Pubkey,
    pub cache_admin: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct CacheAdminUpdatedEvent {
    pub old_cache_admin: Pubkey,
    pub new_cache_admin: Pubkey,
}

#[event]
pub struct CacheYieldUpdatedEvent {
    pub gross_yield: u64,
    pub current_yield: u64,
}

#[event]
pub struct CacheFeeConfigUpdatedEvent {
    pub old_management_fee_basis_points: u16,
    pub new_management_fee_basis_points: u16,
    pub old_performance_fee_basis_points: u16,
    pub new_performance_fee_basis_points: u16,
}

#[event]
pub struct CacheLowestSupplyUpdatedEvent {
    pub previous_lowest_supply: u64,
    pub new_lowest_supply: u64,
    pub current_supply: u64,
    pub updated: bool,
    pub timestamp: i64,
}

#[event]
pub struct CacheAccruedEvent {
    pub seconds_elapsed: u64,
    pub spread: u64,
    pub gross_mint_amount: u64,
    pub cache_mint_amount: u64,
    pub management_fee_mint_amount: u64,
    pub performance_fee_mint_amount: u64,
    pub previous_lowest_supply: u64,
    pub new_lowest_supply: u64,
    pub previous_performance_fee_high_watermark: u64,
    pub new_performance_fee_high_watermark: u64,
    pub timestamp: i64,
}

#[event]
pub struct CacheBurnedForNavEvent {
    pub burn_amount: u64,
    pub asset_adjustment_amount: u64,
    pub total_assets: u64,
    pub target_nav: u64,
}

#[event]
pub struct ManagementFeesClaimedEvent {
    pub amount: u64,
    pub total_claimed: u64,
    pub boss: Pubkey,
}

#[event]
pub struct PerformanceFeesClaimedEvent {
    pub amount: u64,
    pub total_claimed: u64,
    pub boss: Pubkey,
}
