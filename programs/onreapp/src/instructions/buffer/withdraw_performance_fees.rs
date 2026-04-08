use crate::constants::seeds;
use crate::instructions::buffer::{
    withdraw_fee_tokens, BufferState, FeeKind, PerformanceFeesWithdrawnEvent,
};
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct WithdrawPerformanceFees<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss,
        has_one = onyc_mint,
    )]
    pub state: Box<Account<'info, State>>,

    #[account(
        mut,
        has_one = onyc_mint,
        seeds = [seeds::BUFFER_STATE],
        bump = buffer_state.bump,
    )]
    pub buffer_state: Box<Account<'info, BufferState>>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::PERFORMANCE_FEE_VAULT_AUTHORITY], bump)]
    pub performance_fee_vault_authority: UncheckedAccount<'info>,

    /// CHECK: must match configured performance fee wallet
    #[account(address = buffer_state.performance_fee_wallet @ crate::OnreError::InvalidFeeRecipient)]
    pub performance_fee_recipient: UncheckedAccount<'info>,

    #[account(mut)]
    pub onyc_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = onyc_mint,
        associated_token::authority = performance_fee_recipient,
        associated_token::token_program = token_program
    )]
    pub performance_fee_recipient_onyc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = onyc_mint,
        associated_token::authority = performance_fee_vault_authority,
        associated_token::token_program = token_program
    )]
    pub performance_fee_vault_onyc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut)]
    pub boss: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn withdraw_performance_fees(ctx: Context<WithdrawPerformanceFees>, amount: u64) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.performance_fee_recipient_onyc_account.owner,
        ctx.accounts.performance_fee_recipient.key(),
        crate::OnreError::InvalidFeeRecipient
    );

    withdraw_fee_tokens(
        FeeKind::Performance,
        &ctx.accounts.onyc_mint,
        &ctx.accounts.token_program,
        &ctx.accounts.performance_fee_vault_onyc_account,
        &ctx.accounts.performance_fee_recipient_onyc_account,
        &ctx.accounts
            .performance_fee_vault_authority
            .to_account_info(),
        ctx.bumps.performance_fee_vault_authority,
        amount,
    )?;

    emit!(PerformanceFeesWithdrawnEvent {
        amount,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}
