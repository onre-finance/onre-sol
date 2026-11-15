//! Boss-only update of a VaultConfig after creation, matching EVM
//! `updateConfigurableVault`. Re-applies the creation rules; kind and instance are
//! unchanged.
use super::create_vault_config::validate_vault_settings;
use crate::{constants::seeds::STATE, model::VaultConfig, state::State, OnreError};
use anchor_lang::prelude::*;

#[event]
pub struct VaultConfigUpdatedEvent {
    pub boss: Pubkey,
    pub vault: Pubkey,
    pub previous: VaultConfig,
    pub configuration: VaultConfig,
    pub timestamp: i64,
}

/// Any program-owned VaultConfig, generic or BUFFER-derived; `Account` enforces the
/// owner and discriminator, and only the program creates these accounts.
#[derive(Accounts)]
pub struct UpdateVaultConfig<'info> {
    #[account(mut)]
    pub vault: Box<Account<'info, VaultConfig>>,

    #[account(seeds = [STATE], bump = state.bump, has_one = boss @ OnreError::InvalidBoss)]
    pub state: Box<Account<'info, State>>,

    pub boss: Signer<'info>,
}

/// Replaces the withdrawal destination and refill target with the creation rules;
/// kind and instance are unchanged. Also repoints BUFFER vaults, whose destination
/// is fixed to the boss at BUFFER initialization.
pub fn update_vault_config(
    ctx: Context<UpdateVaultConfig>,
    withdrawal_destination: Pubkey,
    refill_target_bps: u16,
) -> Result<()> {
    let a = ctx.accounts;
    validate_vault_settings(a.vault.kind, withdrawal_destination, refill_target_bps)?;
    let previous = (**a.vault).clone();
    require!(
        (withdrawal_destination, refill_target_bps)
            != (previous.withdrawal_destination, previous.refill_target_bps),
        OnreError::NoChange
    );
    a.vault.withdrawal_destination = withdrawal_destination;
    a.vault.refill_target_bps = refill_target_bps;
    emit!(VaultConfigUpdatedEvent {
        boss: a.boss.key(),
        vault: a.vault.key(),
        previous,
        configuration: (**a.vault).clone(),
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
