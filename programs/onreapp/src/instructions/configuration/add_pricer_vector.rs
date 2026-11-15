use crate::{
    constants::{seeds::STATE, MAX_PRICE_FIX_DURATION},
    model::{seeds::PRICER, Pricer, PricingVector},
    state::State,
    utils::pricing,
    OnreError,
};
use anchor_lang::prelude::*;

#[event]
pub struct PricerVectorAddedEvent {
    pub boss: Pubkey,
    pub pricer: Pubkey,
    pub vector: PricingVector,
    pub timestamp: i64,
}

#[event]
pub struct PricerVectorEvictedEvent {
    pub pricer: Pubkey,
    pub vector_start_time: u64,
}

/// Append a scheduled vector to an independent Pricer; boss only. Older vectors
/// are evicted first with the rule shared with legacy offers and EVM: the active
/// vector, the one before it, and all future vectors are kept.
#[derive(Accounts)]
pub struct AddPricerVector<'info> {
    #[account(
        mut,
        seeds = [
            PRICER,
            pricer.managed_mint.as_ref(),
            &[pricer.denomination as u8],
            &pricer.instance_id.to_le_bytes()
        ],
        bump = pricer.bump
    )]
    pub pricer: Box<Account<'info, Pricer>>,

    #[account(
        seeds = [STATE],
        bump = state.bump,
        has_one = boss @ OnreError::InvalidBoss
    )]
    pub state: Box<Account<'info, State>>,

    pub boss: Signer<'info>,
}

pub fn add_pricer_vector(ctx: Context<AddPricerVector>, vector: PricingVector) -> Result<()> {
    let timestamp = Clock::get()?.unix_timestamp;
    let now = u64::try_from(timestamp).map_err(|_| OnreError::InvalidTimestamp)?;
    require!(vector.start_time >= now, OnreError::StartTimeInPast);
    require!(
        vector.base_time > 0 && vector.base_price > 0,
        OnreError::ZeroValue
    );
    require!(
        vector.base_time <= vector.start_time,
        OnreError::InvalidTimeRange
    );
    require!(
        vector.price_fix_duration > 0 && vector.price_fix_duration <= MAX_PRICE_FIX_DURATION,
        OnreError::InvalidPriceFixDuration
    );
    let pricer_key = ctx.accounts.pricer.key();
    let pricer = &mut ctx.accounts.pricer;
    if let Some(latest) = pricer
        .vectors
        .iter()
        .filter(|stored| stored.start_time != 0)
        .map(|stored| stored.start_time)
        .max()
    {
        require!(vector.start_time != latest, OnreError::DuplicateStartTime);
        require!(vector.start_time > latest, OnreError::InvalidTimeRange);
    }
    if let Some(retention) =
        pricing::vector_retention(pricer.vectors.iter().copied(), vector.start_time, now)
    {
        for stored in pricer.vectors.iter_mut() {
            if pricing::is_evicted(stored.start_time, retention) {
                emit!(PricerVectorEvictedEvent {
                    pricer: pricer_key,
                    vector_start_time: stored.start_time,
                });
                *stored = PricingVector::default();
            }
        }
    }
    let empty_slot = pricer
        .vectors
        .iter()
        .position(|stored| stored.start_time == 0)
        .ok_or(OnreError::TooManyVectors)?;
    pricer.vectors[empty_slot] = vector;
    emit!(PricerVectorAddedEvent {
        boss: ctx.accounts.boss.key(),
        pricer: pricer_key,
        vector,
        timestamp,
    });
    Ok(())
}
