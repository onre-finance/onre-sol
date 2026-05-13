use crate::constants::seeds;
use crate::state::{ConfigurableVault, ConfigurableVaultKind, State};
use crate::utils::transfer_tokens;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

#[event]
pub struct ConfigurableVaultWithdrawnEvent {
    pub kind: u8,
    pub mint: Pubkey,
    pub destination: Pubkey,
    pub amount: u64,
}

#[derive(Accounts)]
#[instruction(kind: ConfigurableVaultKind)]
pub struct WithdrawConfigurableVault<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss @ crate::OnreError::Unauthorized,
    )]
    pub state: Box<Account<'info, State>>,

    #[account(mut)]
    pub boss: Signer<'info>,

    #[account(
        seeds = [seeds::CONFIGURABLE_VAULT, kind.seed()],
        bump = configurable_vault.bump,
        constraint = configurable_vault.kind == kind.as_u8() @ crate::OnreError::InvalidConfigurableVaultKind,
    )]
    pub configurable_vault: Account<'info, ConfigurableVault>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = configurable_vault,
        associated_token::token_program = token_program,
    )]
    pub vault_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: Validated against configurable_vault.withdrawal_destination.
    #[account(address = configurable_vault.withdrawal_destination @ crate::OnreError::InvalidFeeRecipient)]
    pub destination: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = mint,
        associated_token::authority = destination,
        associated_token::token_program = token_program,
    )]
    pub destination_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn withdraw_configurable_vault(
    ctx: Context<WithdrawConfigurableVault>,
    kind: ConfigurableVaultKind,
    amount: u64,
) -> Result<()> {
    let destination = ctx.accounts.configurable_vault.withdrawal_destination;
    require!(
        destination != Pubkey::default(),
        crate::OnreError::MissingConfigurableVaultDestination
    );

    let vault_balance = ctx.accounts.vault_token_account.amount;
    let effective_amount = if amount == 0 { vault_balance } else { amount };

    require!(effective_amount > 0, crate::OnreError::ZeroBalance);
    require!(
        effective_amount <= vault_balance,
        crate::OnreError::InsufficientBalance
    );

    let bump = ctx.accounts.configurable_vault.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[seeds::CONFIGURABLE_VAULT, kind.seed(), &[bump]]];

    transfer_tokens(
        &ctx.accounts.mint,
        &ctx.accounts.token_program,
        &ctx.accounts.vault_token_account,
        &ctx.accounts.destination_token_account,
        &ctx.accounts.configurable_vault.to_account_info(),
        Some(signer_seeds),
        effective_amount,
    )?;

    emit!(ConfigurableVaultWithdrawnEvent {
        kind: kind.as_u8(),
        mint: ctx.accounts.mint.key(),
        destination,
        amount: effective_amount,
    });

    Ok(())
}
