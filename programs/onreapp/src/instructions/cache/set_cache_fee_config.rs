use crate::constants::{seeds, MAX_BASIS_POINTS};
use crate::instructions::cache::{CacheErrorCode, CacheFeeConfigUpdatedEvent, CacheState};
use crate::state::State;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SetCacheFeeConfig<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss,
    )]
    pub state: Account<'info, State>,

    #[account(
        mut,
        seeds = [seeds::CACHE_STATE],
        bump = cache_state.bump,
    )]
    pub cache_state: Account<'info, CacheState>,

    pub boss: Signer<'info>,
}

pub fn set_cache_fee_config(
    ctx: Context<SetCacheFeeConfig>,
    management_fee_basis_points: u16,
    performance_fee_basis_points: u16,
) -> Result<()> {
    require!(
        management_fee_basis_points <= MAX_BASIS_POINTS,
        CacheErrorCode::InvalidFee
    );
    require!(
        performance_fee_basis_points <= MAX_BASIS_POINTS,
        CacheErrorCode::InvalidFee
    );

    let cache_state = &mut ctx.accounts.cache_state;
    require!(
        cache_state.management_fee_basis_points != management_fee_basis_points
            || cache_state.performance_fee_basis_points != performance_fee_basis_points,
        CacheErrorCode::NoChange
    );

    let old_management_fee_basis_points = cache_state.management_fee_basis_points;
    let old_performance_fee_basis_points = cache_state.performance_fee_basis_points;

    cache_state.management_fee_basis_points = management_fee_basis_points;
    cache_state.performance_fee_basis_points = performance_fee_basis_points;

    emit!(CacheFeeConfigUpdatedEvent {
        old_management_fee_basis_points,
        new_management_fee_basis_points: management_fee_basis_points,
        old_performance_fee_basis_points,
        new_performance_fee_basis_points: performance_fee_basis_points,
    });

    Ok(())
}
