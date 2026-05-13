use crate::constants::seeds;
use crate::state::{ConfigurableVault, ConfigurableVaultKind, State};
use anchor_lang::prelude::*;

#[event]
pub struct ConfigurableVaultDestinationUpdatedEvent {
    pub kind: u8,
    pub old_destination: Pubkey,
    pub new_destination: Pubkey,
}

#[derive(Accounts)]
#[instruction(kind: ConfigurableVaultKind)]
pub struct SetConfigurableVaultDestination<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss @ crate::OnreError::Unauthorized,
    )]
    pub state: Box<Account<'info, State>>,

    #[account(mut)]
    pub boss: Signer<'info>,

    #[account(
        init_if_needed,
        payer = boss,
        space = 8 + ConfigurableVault::INIT_SPACE,
        seeds = [seeds::CONFIGURABLE_VAULT, kind.seed()],
        bump,
    )]
    pub configurable_vault: Account<'info, ConfigurableVault>,

    pub system_program: Program<'info, System>,
}

pub fn set_configurable_vault_destination(
    ctx: Context<SetConfigurableVaultDestination>,
    kind: ConfigurableVaultKind,
    withdrawal_destination: Pubkey,
) -> Result<()> {
    let expected_kind = kind.as_u8();
    let vault = &mut ctx.accounts.configurable_vault;

    if vault.kind == 0 && vault.withdrawal_destination == Pubkey::default() && vault.bump == 0 {
        vault.kind = expected_kind;
        vault.bump = ctx.bumps.configurable_vault;
    }

    require!(
        vault.kind == expected_kind,
        crate::OnreError::InvalidConfigurableVaultKind
    );
    require!(
        vault.withdrawal_destination != withdrawal_destination,
        crate::OnreError::NoChange
    );

    let old_destination = vault.withdrawal_destination;
    vault.withdrawal_destination = withdrawal_destination;

    emit!(ConfigurableVaultDestinationUpdatedEvent {
        kind: expected_kind,
        old_destination,
        new_destination: withdrawal_destination,
    });

    Ok(())
}
