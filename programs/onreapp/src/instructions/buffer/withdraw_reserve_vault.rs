use crate::constants::seeds;
use crate::instructions::buffer::{BufferState, ReserveVaultWithdrawnEvent};
use crate::state::State;
use crate::utils::transfer_tokens;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct WithdrawReserveVault<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss,
        has_one = onyc_mint,
    )]
    pub state: Box<Account<'info, State>>,

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
        init_if_needed,
        payer = boss,
        associated_token::mint = onyc_mint,
        associated_token::authority = boss,
        associated_token::token_program = token_program
    )]
    pub boss_onyc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = onyc_mint,
        associated_token::authority = reserve_vault_authority,
        associated_token::token_program = token_program
    )]
    pub reserve_vault_onyc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut)]
    pub boss: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn withdraw_reserve_vault(ctx: Context<WithdrawReserveVault>, amount: u64) -> Result<()> {
    let reserve_vault_authority_seeds = &[
        seeds::RESERVE_VAULT_AUTHORITY,
        &[ctx.bumps.reserve_vault_authority],
    ];
    let signer_seeds = &[reserve_vault_authority_seeds.as_slice()];

    transfer_tokens(
        &ctx.accounts.onyc_mint,
        &ctx.accounts.token_program,
        &ctx.accounts.reserve_vault_onyc_account,
        &ctx.accounts.boss_onyc_account,
        &ctx.accounts.reserve_vault_authority.to_account_info(),
        Some(signer_seeds),
        amount,
    )?;

    emit!(ReserveVaultWithdrawnEvent {
        amount,
        mint: ctx.accounts.onyc_mint.key(),
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}
