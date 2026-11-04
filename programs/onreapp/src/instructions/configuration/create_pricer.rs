use crate::{
    constants::{seeds::STATE, MAX_VECTORS},
    model::{seeds::PRICER, ManagedTokenConfig, Pricer, PricingDenomination, PricingVector},
    state::State,
    OnreError,
};
use anchor_lang::prelude::*;

/// Records the created account and its complete initial configuration.
#[event]
pub struct PricerCreatedEvent {
    pub boss: Pubkey,
    pub pricer: Pubkey,
    pub configuration: Pricer,
    pub timestamp: i64,
}

/// Creates an independent, initially empty pricing-vector source for a registered mint.
#[derive(Accounts)]
#[instruction(instance_id: u64, denomination: PricingDenomination)]
pub struct CreatePricer<'info> {
    #[account(
        constraint = managed_token.enabled @ OnreError::ConfigurationDisabled
    )]
    pub managed_token: Box<Account<'info, ManagedTokenConfig>>,

    #[account(
        init,
        payer = boss,
        space = 8 + Pricer::INIT_SPACE,
        seeds = [
            PRICER,
            managed_token.managed_mint.as_ref(),
            &[denomination as u8],
            &instance_id.to_le_bytes()
        ],
        bump
    )]
    pub pricer: Box<Account<'info, Pricer>>,

    #[account(
        seeds = [STATE],
        bump = state.bump,
        has_one = boss @ OnreError::InvalidBoss
    )]
    pub state: Box<Account<'info, State>>,

    #[account(mut)]
    pub boss: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_pricer(
    ctx: Context<CreatePricer>,
    instance_id: u64,
    denomination: PricingDenomination,
) -> Result<()> {
    ctx.accounts.pricer.set_inner(Pricer {
        instance_id,
        managed_mint: ctx.accounts.managed_token.managed_mint,
        denomination,
        vectors: [PricingVector::default(); MAX_VECTORS],
        enabled: true,
        bump: ctx.bumps.pricer,
    });
    emit!(PricerCreatedEvent {
        boss: ctx.accounts.boss.key(),
        pricer: ctx.accounts.pricer.key(),
        configuration: (**ctx.accounts.pricer).clone(),
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
