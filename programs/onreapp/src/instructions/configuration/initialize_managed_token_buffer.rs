use super::VaultConfigCreatedEvent;
use crate::{
    constants::seeds::STATE,
    model::{
        seeds::{BUFFER, BUFFER_MANAGEMENT, BUFFER_PERFORMANCE, BUFFER_RESERVE, PRICER},
        ManagedTokenBufferState, ManagedTokenConfig, Pricer, PricingDenomination, VaultConfig,
        VaultKind,
    },
    state::State,
    OnreError,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

/// Records the created account and its complete initial configuration.
#[event]
pub struct ManagedTokenBufferInitializedEvent {
    pub boss: Pubkey,
    pub buffer_state: Pubkey,
    pub configuration: ManagedTokenBufferState,
    pub timestamp: i64,
}

/// Initializes independent per-mint BUFFER accounting and three dedicated vault configurations.
/// Does not accrue, mint, create ATAs, migrate custody, or alter the legacy singleton BUFFER.
#[derive(Accounts)]
pub struct InitializeManagedTokenBuffer<'info> {
    #[account(
        constraint = managed_token.enabled @ OnreError::ConfigurationDisabled,
        constraint = managed_token.managed_mint == managed_mint.key() @ OnreError::InvalidMint
    )]
    pub managed_token: Box<Account<'info, ManagedTokenConfig>>,

    pub managed_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        seeds = [
            PRICER,
            managed_mint.key().as_ref(),
            &[PricingDenomination::Usd as u8],
            &0_u64.to_le_bytes()
        ],
        bump = pricer.bump,
        constraint = pricer.enabled @ OnreError::ConfigurationDisabled,
        constraint = pricer.managed_mint == managed_mint.key() @ OnreError::IncompatiblePricer
    )]
    pub pricer: Box<Account<'info, Pricer>>,

    #[account(
        init,
        payer = boss,
        space = 8 + ManagedTokenBufferState::INIT_SPACE,
        seeds = [BUFFER, managed_mint.key().as_ref()],
        bump
    )]
    pub buffer_state: Box<Account<'info, ManagedTokenBufferState>>,

    #[account(
        init,
        payer = boss,
        space = 8 + VaultConfig::INIT_SPACE,
        seeds = [BUFFER_RESERVE, managed_mint.key().as_ref()],
        bump
    )]
    pub reserve_vault: Box<Account<'info, VaultConfig>>,

    #[account(
        init,
        payer = boss,
        space = 8 + VaultConfig::INIT_SPACE,
        seeds = [BUFFER_MANAGEMENT, managed_mint.key().as_ref()],
        bump
    )]
    pub management_fee_vault: Box<Account<'info, VaultConfig>>,

    #[account(
        init,
        payer = boss,
        space = 8 + VaultConfig::INIT_SPACE,
        seeds = [BUFFER_PERFORMANCE, managed_mint.key().as_ref()],
        bump
    )]
    pub performance_fee_vault: Box<Account<'info, VaultConfig>>,

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

pub fn initialize_managed_token_buffer(ctx: Context<InitializeManagedTokenBuffer>) -> Result<()> {
    let destination = ctx.accounts.boss.key();
    let timestamp = Clock::get()?.unix_timestamp;
    for (vault, kind, instance_id, bump) in [
        (
            &mut ctx.accounts.reserve_vault,
            VaultKind::BufferReserve,
            0,
            ctx.bumps.reserve_vault,
        ),
        (
            &mut ctx.accounts.management_fee_vault,
            VaultKind::Fee,
            1,
            ctx.bumps.management_fee_vault,
        ),
        (
            &mut ctx.accounts.performance_fee_vault,
            VaultKind::Fee,
            2,
            ctx.bumps.performance_fee_vault,
        ),
    ] {
        vault.set_inner(VaultConfig {
            kind,
            instance_id,
            withdrawal_destination: destination,
            refill_target_bps: 0,
            bump,
        });
        emit!(VaultConfigCreatedEvent {
            boss: destination,
            vault: vault.key(),
            configuration: (***vault).clone(),
            timestamp,
        });
    }
    ctx.accounts
        .buffer_state
        .set_inner(ManagedTokenBufferState {
            managed_mint: ctx.accounts.managed_mint.key(),
            reserve_vault: ctx.accounts.reserve_vault.key(),
            management_fee_vault: ctx.accounts.management_fee_vault.key(),
            performance_fee_vault: ctx.accounts.performance_fee_vault.key(),
            previous_supply: ctx.accounts.managed_mint.supply,
            performance_fee_high_watermark: 0,
            last_accrual_timestamp: timestamp,
            gross_apr: 0,
            management_fee_basis_points: 0,
            performance_fee_basis_points: 0,
            performance_fee_high_watermark_enabled: true,
            bump: ctx.bumps.buffer_state,
        });
    emit!(ManagedTokenBufferInitializedEvent {
        boss: ctx.accounts.boss.key(),
        buffer_state: ctx.accounts.buffer_state.key(),
        configuration: (**ctx.accounts.buffer_state).clone(),
        timestamp,
    });
    Ok(())
}
