//! Boss-only removal of modular Pricer vectors, matching legacy offers and EVM:
//! single deletion is limited to future vectors; deleting all is unrestricted.
use crate::{
    constants::seeds::STATE,
    model::{seeds::PRICER, Pricer, PricingVector},
    state::State,
    OnreError,
};
use anchor_lang::prelude::*;

#[event]
pub struct PricerVectorDeletedEvent {
    pub boss: Pubkey,
    pub pricer: Pubkey,
    pub vector_start_time: u64,
    pub timestamp: i64,
}

#[event]
pub struct AllPricerVectorsDeletedEvent {
    pub boss: Pubkey,
    pub pricer: Pubkey,
    pub vectors_deleted_count: u8,
    pub timestamp: i64,
}

#[derive(Accounts)]
pub struct DeletePricerVector<'info> {
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

/// Removes one scheduled vector that has not started yet.
pub fn delete_pricer_vector(
    ctx: Context<DeletePricerVector>,
    vector_start_time: u64,
) -> Result<()> {
    let timestamp = Clock::get()?.unix_timestamp;
    let now = u64::try_from(timestamp).map_err(|_| OnreError::InvalidTimestamp)?;
    require!(vector_start_time > now, OnreError::StartTimeInPast);
    let pricer = &mut ctx.accounts.pricer;
    let stored = pricer
        .vectors
        .iter_mut()
        .find(|stored| stored.start_time == vector_start_time)
        .ok_or(OnreError::VectorNotFound)?;
    *stored = PricingVector::default();
    emit!(PricerVectorDeletedEvent {
        boss: ctx.accounts.boss.key(),
        pricer: pricer.key(),
        vector_start_time,
        timestamp,
    });
    Ok(())
}

/// Removes every vector, including the active one; pricing then fails with
/// `NoActiveVector` until a new vector is added.
pub fn delete_all_pricer_vectors(ctx: Context<DeletePricerVector>) -> Result<()> {
    let pricer = &mut ctx.accounts.pricer;
    let mut vectors_deleted_count = 0;
    for stored in pricer
        .vectors
        .iter_mut()
        .filter(|stored| stored.start_time != 0)
    {
        *stored = PricingVector::default();
        vectors_deleted_count += 1;
    }
    emit!(AllPricerVectorsDeletedEvent {
        boss: ctx.accounts.boss.key(),
        pricer: pricer.key(),
        vectors_deleted_count,
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
