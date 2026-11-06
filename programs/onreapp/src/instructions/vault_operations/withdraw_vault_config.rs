use crate::{
    constants::seeds::STATE,
    model::{VaultConfig, VaultKind},
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
pub struct VaultConfigWithdrawnEvent {
    pub vault: Pubkey,
    pub mint: Pubkey,
    pub caller: Pubkey,
    pub destination: Pubkey,
    pub amount: u64,
}

/// Withdraw only to the configured destination. Fee/Proceeds allow any caller;
/// Liquidity/BufferReserve additionally require State::boss. Zero withdraws all.
/// BUFFER vaults use the managed mint's role-specific seeds, not generic vault seeds.
#[derive(Accounts)]
pub struct WithdrawVaultConfig<'info> {
    pub vault: Box<Account<'info, VaultConfig>>,
    #[account(mint::token_program = token_program)]
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = vault,
        associated_token::token_program = token_program
    )]
    pub vault_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    /// CHECK: Fixed recipient, never a caller-selected withdrawal authority.
    #[account(address = vault.withdrawal_destination @ OnreError::InvalidFeeRecipient)]
    pub destination: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = caller,
        associated_token::mint = mint,
        associated_token::authority = destination,
        associated_token::token_program = token_program
    )]
    pub destination_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut)]
    pub caller: Signer<'info>,
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

pub fn withdraw_vault_config(ctx: Context<WithdrawVaultConfig>, amount: u64) -> Result<()> {
    let a = ctx.accounts;
    let authority = validate_vault_config(&a.vault.key(), &a.vault, &a.mint.key(), ctx.program_id)?;
    if matches!(
        a.vault.kind,
        VaultKind::Liquidity | VaultKind::BufferReserve
    ) {
        require_keys_eq!(a.caller.key(), a.state.boss, OnreError::InvalidBoss);
    }
    require!(
        a.vault.withdrawal_destination != Pubkey::default(),
        OnreError::MissingConfigurableVaultDestination
    );
    require!(
        !has_transfer_fee(&a.mint)?,
        OnreError::TransferFeeNotSupported
    );
    let amount = if amount == 0 {
        a.vault_token_account.amount
    } else {
        amount
    };
    require!(amount > 0, OnreError::ZeroBalance);
    require!(
        amount <= a.vault_token_account.amount,
        OnreError::InsufficientBalance
    );
    require!(
        a.destination_token_account.owner == a.destination.key()
            && a.destination_token_account.mint == a.mint.key(),
        OnreError::InvalidFeeRecipient
    );
    let seeds = authority.signer_seeds();
    transfer_tokens(
        &a.mint,
        &a.token_program,
        &a.vault_token_account,
        &a.destination_token_account,
        &a.vault.to_account_info(),
        Some(&[&seeds]),
        amount,
    )?;
    emit!(VaultConfigWithdrawnEvent {
        vault: a.vault.key(),
        mint: a.mint.key(),
        caller: a.caller.key(),
        destination: a.destination.key(),
        amount,
    });
    Ok(())
}
