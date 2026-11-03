use crate::model::{self, seeds::*};
use anchor_lang::prelude::*;

/// Creates a reusable vault authority/configuration. Does not create or fund token accounts.
#[derive(Accounts)]
#[instruction(kind: model::VaultKind, instance_id: u64)]
pub struct CreateVaultConfig<'info> {
    #[account(
        init,
        payer = boss,
        space = 8 + model::VaultConfig::INIT_SPACE,
        seeds = [
            VAULT,
            &[kind as u8],
            &instance_id.to_le_bytes()
        ],
        bump
    )]
    pub vault: Box<Account<'info, model::VaultConfig>>,

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

pub fn create_vault_config(
    ctx: Context<CreateVaultConfig>,
    kind: model::VaultKind,
    instance_id: u64,
    withdrawal_destination: Pubkey,
    refill_target_bps: u16,
) -> Result<()> {
    require!(
        withdrawal_destination != Pubkey::default(),
        crate::OnreError::MissingConfigurableVaultDestination
    );
    require!(
        refill_target_bps <= crate::constants::MAX_BASIS_POINTS,
        crate::OnreError::InvalidAmount
    );
    require!(
        kind == model::VaultKind::Liquidity || refill_target_bps == 0,
        crate::OnreError::InvalidAmount
    );
    ctx.accounts.vault.set_inner(model::VaultConfig {
        kind,
        instance_id,
        withdrawal_destination,
        refill_target_bps,
        bump: ctx.bumps.vault,
    });
    Ok(())
}
