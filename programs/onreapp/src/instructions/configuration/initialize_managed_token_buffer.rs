use crate::model::{self, seeds::*};
use anchor_lang::prelude::*;

/// Initializes independent per-mint BUFFER accounting and three dedicated vault configurations.
/// Does not accrue, mint, create ATAs, migrate custody, or alter the legacy singleton BUFFER.
#[derive(Accounts)]
pub struct InitializeManagedTokenBuffer<'info> {
    #[account(
        constraint = managed_token.enabled @ crate::OnreError::ConfigurationDisabled,
        constraint = managed_token.managed_mint == managed_mint.key() @ crate::OnreError::InvalidMint
    )]
    pub managed_token: Box<Account<'info, model::ManagedTokenConfig>>,

    pub managed_mint: Box<InterfaceAccount<'info, anchor_spl::token_interface::Mint>>,

    #[account(
        seeds = [
            PRICER,
            managed_mint.key().as_ref(),
            &[model::PricingDenomination::Usd as u8],
            &0_u64.to_le_bytes()
        ],
        bump = pricer.bump,
        constraint = pricer.enabled @ crate::OnreError::ConfigurationDisabled,
        constraint = pricer.managed_mint == managed_mint.key() @ crate::OnreError::IncompatiblePricer
    )]
    pub pricer: Box<Account<'info, model::Pricer>>,

    #[account(
        init,
        payer = boss,
        space = 8 + model::ManagedTokenBufferState::INIT_SPACE,
        seeds = [BUFFER, managed_mint.key().as_ref()],
        bump
    )]
    pub buffer_state: Box<Account<'info, model::ManagedTokenBufferState>>,

    #[account(
        init,
        payer = boss,
        space = 8 + model::VaultConfig::INIT_SPACE,
        seeds = [BUFFER_RESERVE, managed_mint.key().as_ref()],
        bump
    )]
    pub reserve_vault: Box<Account<'info, model::VaultConfig>>,

    #[account(
        init,
        payer = boss,
        space = 8 + model::VaultConfig::INIT_SPACE,
        seeds = [BUFFER_MANAGEMENT, managed_mint.key().as_ref()],
        bump
    )]
    pub management_fee_vault: Box<Account<'info, model::VaultConfig>>,

    #[account(
        init,
        payer = boss,
        space = 8 + model::VaultConfig::INIT_SPACE,
        seeds = [BUFFER_PERFORMANCE, managed_mint.key().as_ref()],
        bump
    )]
    pub performance_fee_vault: Box<Account<'info, model::VaultConfig>>,

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

pub fn initialize_managed_token_buffer(ctx: Context<InitializeManagedTokenBuffer>) -> Result<()> {
    let destination = ctx.accounts.boss.key();
    for (vault, kind, instance_id, bump) in [
        (
            &mut ctx.accounts.reserve_vault,
            model::VaultKind::BufferReserve,
            0,
            ctx.bumps.reserve_vault,
        ),
        (
            &mut ctx.accounts.management_fee_vault,
            model::VaultKind::Fee,
            1,
            ctx.bumps.management_fee_vault,
        ),
        (
            &mut ctx.accounts.performance_fee_vault,
            model::VaultKind::Fee,
            2,
            ctx.bumps.performance_fee_vault,
        ),
    ] {
        vault.set_inner(model::VaultConfig {
            kind,
            instance_id,
            withdrawal_destination: destination,
            refill_target_bps: 0,
            bump,
        });
    }
    ctx.accounts
        .buffer_state
        .set_inner(model::ManagedTokenBufferState {
            managed_mint: ctx.accounts.managed_mint.key(),
            reserve_vault: ctx.accounts.reserve_vault.key(),
            management_fee_vault: ctx.accounts.management_fee_vault.key(),
            performance_fee_vault: ctx.accounts.performance_fee_vault.key(),
            previous_supply: ctx.accounts.managed_mint.supply,
            performance_fee_high_watermark: 0,
            last_accrual_timestamp: Clock::get()?.unix_timestamp,
            gross_apr: 0,
            management_fee_basis_points: 0,
            performance_fee_basis_points: 0,
            performance_fee_high_watermark_enabled: true,
            bump: ctx.bumps.buffer_state,
        });
    Ok(())
}
