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
    pub mint_amount: u64,
    pub previous_lowest_supply: u64,
    pub new_lowest_supply: u64,
    pub timestamp: i64,
}

#[event]
pub struct CacheBurnedForNavEvent {
    pub burn_amount: u64,
    pub asset_adjustment_amount: u64,
    pub total_assets: u64,
    pub target_nav: u64,
}
