use crate::constants::seeds;
use crate::instructions::buffer::{BufferState, ReserveVaultDepositedEvent};
use crate::utils::transfer_tokens;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct DepositReserveVault<'info> {
    #[account(
        has_one = onyc_mint,
        seeds = [seeds::BUFFER_STATE],
        bump = buffer_state.bump,
    )]
    pub buffer_state: Box<Account<'info, BufferState>>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::RESERVE_VAULT_AUTHORITY], bump)]
    pub reserve_vault_authority: UncheckedAccount<'info>,

    pub onyc_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = onyc_mint,
        associated_token::authority = depositor,
        associated_token::token_program = token_program
    )]
    pub depositor_onyc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = depositor,
        associated_token::mint = onyc_mint,
        associated_token::authority = reserve_vault_authority,
        associated_token::token_program = token_program
    )]
    pub reserve_vault_onyc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut)]
    pub depositor: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn deposit_reserve_vault(ctx: Context<DepositReserveVault>, amount: u64) -> Result<()> {
    transfer_tokens(
        &ctx.accounts.onyc_mint,
        &ctx.accounts.token_program,
        &ctx.accounts.depositor_onyc_account,
        &ctx.accounts.reserve_vault_onyc_account,
        &ctx.accounts.depositor.to_account_info(),
        None,
        amount,
    )?;

    emit!(ReserveVaultDepositedEvent {
        amount,
        depositor: ctx.accounts.depositor.key(),
    });

    Ok(())
}
