use crate::model::{self, seeds::*};
use anchor_lang::prelude::*;

/// Creates an independent, initially empty pricing-vector source for a registered mint.
#[derive(Accounts)]
#[instruction(instance_id: u64, denomination: model::PricingDenomination)]
pub struct CreatePricer<'info> {
    #[account(
        constraint = managed_token.enabled @ crate::OnreError::ConfigurationDisabled
    )]
    pub managed_token: Box<Account<'info, model::ManagedTokenConfig>>,

    #[account(
        init,
        payer = boss,
        space = 8 + model::Pricer::INIT_SPACE,
        seeds = [
            PRICER,
            managed_token.managed_mint.as_ref(),
            &[denomination as u8],
            &instance_id.to_le_bytes()
        ],
        bump
    )]
    pub pricer: Box<Account<'info, model::Pricer>>,

    #[account(
        seeds = [crate::constants::seeds::STATE],
        bump = state.bump,
        has_one = boss @ crate::OnreError::InvalidBoss
    )]
    pub state: Box<Account<'info, crate::state::State>>,

    #[account(mut)]
    pub boss: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_pricer(
    ctx: Context<CreatePricer>,
    instance_id: u64,
    denomination: model::PricingDenomination,
) -> Result<()> {
    ctx.accounts.pricer.set_inner(model::Pricer {
        instance_id,
        managed_mint: ctx.accounts.managed_token.managed_mint,
        denomination,
        vectors: [model::PricingVector::default(); crate::constants::MAX_VECTORS],
        vector_count: 0,
        enabled: true,
        bump: ctx.bumps.pricer,
    });
    Ok(())
}
