use crate::{
    constants::seeds::STATE,
    model::{seeds::PRICER, Pricer, PricingVector},
    state::State,
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

/// Append a scheduled vector to an independent Pricer; boss only.
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
        vector.price_fix_duration > 0,
        OnreError::InvalidPriceFixDuration
    );
    let pricer = &mut ctx.accounts.pricer;
    let empty_slot = pricer
        .vectors
        .iter()
        .position(|stored| stored.start_time == 0)
        .ok_or(OnreError::TooManyVectors)?;
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
    pricer.vectors[empty_slot] = vector;
    emit!(PricerVectorAddedEvent {
        boss: ctx.accounts.boss.key(),
        pricer: pricer.key(),
        vector,
        timestamp,
    });
    Ok(())
}
