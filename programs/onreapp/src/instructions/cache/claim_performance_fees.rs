use crate::constants::seeds;
use crate::instructions::cache::{CacheErrorCode, CacheState, PerformanceFeesClaimedEvent};
use crate::state::State;
use crate::utils::transfer_tokens;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct ClaimPerformanceFees<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss,
        has_one = onyc_mint,
    )]
    pub state: Box<Account<'info, State>>,

    #[account(
        mut,
        seeds = [seeds::CACHE_STATE],
        bump = cache_state.bump,
        has_one = onyc_mint,
    )]
    pub cache_state: Box<Account<'info, CacheState>>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::PERFORMANCE_FEE_VAULT_AUTHORITY], bump)]
    pub performance_fee_vault_authority: UncheckedAccount<'info>,

    #[account(mut)]
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

pub fn claim_performance_fees(ctx: Context<ClaimPerformanceFees>, amount: u64) -> Result<()> {
    require!(
        amount <= ctx.accounts.performance_fee_vault_onyc_account.amount,
        CacheErrorCode::InsufficientFeeBalance
    );

    let authority_seeds = &[
        seeds::PERFORMANCE_FEE_VAULT_AUTHORITY,
        &[ctx.bumps.performance_fee_vault_authority],
    ];
    let signer_seeds = &[&authority_seeds[..]];

    transfer_tokens(
        &ctx.accounts.onyc_mint,
        &ctx.accounts.token_program,
        &ctx.accounts.performance_fee_vault_onyc_account,
        &ctx.accounts.boss_onyc_account,
        &ctx.accounts
            .performance_fee_vault_authority
            .to_account_info(),
        Some(signer_seeds),
        amount,
    )?;

    let cache_state = &mut ctx.accounts.cache_state;
    cache_state.total_performance_fees_claimed = cache_state
        .total_performance_fees_claimed
        .checked_add(amount)
        .ok_or(CacheErrorCode::MathOverflow)?;

    emit!(PerformanceFeesClaimedEvent {
        amount,
        total_claimed: cache_state.total_performance_fees_claimed,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}
