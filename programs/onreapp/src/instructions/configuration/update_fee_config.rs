//! Boss-only update of a FeeConfig after creation, matching EVM `updateFeeConfig`.
//! Re-applies the creation rules; the instance, token decimals and enabled flag are
//! unchanged.
use crate::{
    constants::{seeds::STATE, MAX_ALLOWED_FEE_BPS},
    model::{seeds::FEE_CONFIG, FeeConfig, VaultConfig, VaultKind},
    state::State,
    OnreError,
};
use anchor_lang::prelude::*;

#[event]
pub struct FeeConfigUpdatedEvent {
    pub boss: Pubkey,
    pub fee_config: Pubkey,
    pub previous: FeeConfig,
    pub configuration: FeeConfig,
    pub timestamp: i64,
}

#[derive(Accounts)]
pub struct UpdateFeeConfig<'info> {
    #[account(
        mut,
        seeds = [FEE_CONFIG, &fee_config.instance_id.to_le_bytes()],
        bump = fee_config.bump,
    )]
    pub fee_config: Box<Account<'info, FeeConfig>>,

    #[account(
        constraint = fee_vault.kind == VaultKind::Fee @ OnreError::InvalidConfigurableVaultKind,
        // BUFFER fee vaults withdraw only their managed mint; other fees would be stuck.
        constraint = fee_vault.is_generic(&fee_vault.key(), &crate::ID) @ OnreError::InvalidConfigurableVault,
    )]
    pub fee_vault: Box<Account<'info, VaultConfig>>,

    #[account(seeds = [STATE], bump = state.bump, has_one = boss @ OnreError::InvalidBoss)]
    pub state: Box<Account<'info, State>>,

    pub boss: Signer<'info>,
}

/// Replaces the fee rate, minimum fee and Fee-kind vault; instance, token decimals
/// and enabled flag are unchanged.
pub fn update_fee_config(
    ctx: Context<UpdateFeeConfig>,
    basis_points: u16,
    minimum_fee_amount: u64,
) -> Result<()> {
    require!(basis_points <= MAX_ALLOWED_FEE_BPS, OnreError::InvalidFee);
    let a = ctx.accounts;
    let previous = (**a.fee_config).clone();
    let fee_vault = a.fee_vault.key();
    require!(
        (basis_points, minimum_fee_amount, fee_vault)
            != (
                previous.basis_points,
                previous.minimum_fee_amount,
                previous.fee_vault
            ),
        OnreError::NoChange
    );
    a.fee_config.basis_points = basis_points;
    a.fee_config.minimum_fee_amount = minimum_fee_amount;
    a.fee_config.fee_vault = fee_vault;
    emit!(FeeConfigUpdatedEvent {
        boss: a.boss.key(),
        fee_config: a.fee_config.key(),
        previous,
        configuration: (**a.fee_config).clone(),
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
