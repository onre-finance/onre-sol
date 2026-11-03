use crate::model::{self, seeds::*};
use anchor_lang::prelude::*;

/// Records the created account and its complete initial configuration.
#[event]
pub struct FeeConfigCreatedEvent {
    pub boss: Pubkey,
    pub fee_config: Pubkey,
    pub configuration: model::FeeConfig,
    pub timestamp: i64,
}

/// Creates an input-token fee policy referencing an existing Fee-kind vault.
#[derive(Accounts)]
#[instruction(instance_id: u64)]
pub struct CreateFeeConfig<'info> {
    #[account(
        constraint = fee_vault.kind == model::VaultKind::Fee @ crate::OnreError::InvalidConfigurableVaultKind
    )]
    pub fee_vault: Box<Account<'info, model::VaultConfig>>,

    #[account(
        init,
        payer = boss,
        space = 8 + model::FeeConfig::INIT_SPACE,
        seeds = [FEE_CONFIG, &instance_id.to_le_bytes()],
        bump
    )]
    pub fee_config: Box<Account<'info, model::FeeConfig>>,

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

pub fn create_fee_config(
    ctx: Context<CreateFeeConfig>,
    instance_id: u64,
    basis_points: u16,
    minimum_fee_amount: u64,
) -> Result<()> {
    require!(
        basis_points <= crate::constants::MAX_ALLOWED_FEE_BPS,
        crate::OnreError::InvalidFee
    );
    ctx.accounts.fee_config.set_inner(model::FeeConfig {
        instance_id,
        fee_vault: ctx.accounts.fee_vault.key(),
        basis_points,
        minimum_fee_amount,
        enabled: true,
        bump: ctx.bumps.fee_config,
    });
    emit!(FeeConfigCreatedEvent {
        boss: ctx.accounts.boss.key(),
        fee_config: ctx.accounts.fee_config.key(),
        configuration: (**ctx.accounts.fee_config).clone(),
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
