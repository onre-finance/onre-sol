use crate::{
    constants::seeds::STATE,
    model::VaultConfig,
    state::State,
    utils::{has_transfer_fee, transfer_tokens, vault_authority::validate_vault_config},
    OnreError,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

#[event]
pub struct VaultConfigDepositedEvent {
    pub vault: Pubkey,
    pub mint: Pubkey,
    pub depositor: Pubkey,
    pub amount: u64,
}

/// Anyone may fund a canonical VaultConfig's ATA; the depositor pays ATA rent.
/// Supports generic vaults and token-specific BUFFER vaults. Creates no configuration.
#[derive(Accounts)]
pub struct DepositVaultConfig<'info> {
    pub vault: Box<Account<'info, VaultConfig>>,
    #[account(mint::token_program = token_program)]
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = depositor,
        associated_token::token_program = token_program
    )]
    pub depositor_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = depositor,
        associated_token::mint = mint,
        associated_token::authority = vault,
        associated_token::token_program = token_program
    )]
    pub vault_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut)]
    pub depositor: Signer<'info>,
    #[account(
        seeds = [STATE],
        bump = state.bump,
        constraint = !state.is_killed @ OnreError::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn deposit_vault_config(ctx: Context<DepositVaultConfig>, amount: u64) -> Result<()> {
    let a = ctx.accounts;
    validate_vault_config(&a.vault.key(), &a.vault, &a.mint.key(), ctx.program_id)?;
    require!(
        a.vault_token_account.owner == a.vault.key() && a.vault_token_account.mint == a.mint.key(),
        OnreError::InvalidConfigurableVaultTokenAccount
    );
    require!(amount > 0, OnreError::ZeroValue);
    require!(
        !has_transfer_fee(&a.mint)?,
        OnreError::TransferFeeNotSupported
    );
    transfer_tokens(
        &a.mint,
        &a.token_program,
        &a.depositor_token_account,
        &a.vault_token_account,
        &a.depositor.to_account_info(),
        None,
        amount,
    )?;
    emit!(VaultConfigDepositedEvent {
        vault: a.vault.key(),
        mint: a.mint.key(),
        depositor: a.depositor.key(),
        amount,
    });
    Ok(())
}
