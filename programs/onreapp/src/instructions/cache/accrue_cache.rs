use crate::constants::seeds;
use crate::instructions::cache::{
    calculate_cache_fee_split, calculate_gross_cache_accrual, CacheAccruedEvent, CacheErrorCode,
    CacheState,
};
use crate::state::State;
use crate::utils::token_utils::{mint_tokens, TokenUtilsErrorCode};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct AccrueCache<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = onyc_mint,
    )]
    pub state: Box<Account<'info, State>>,

    #[account(
        mut,
        seeds = [seeds::CACHE_STATE],
        bump = cache_state.bump,
        has_one = cache_admin,
        has_one = onyc_mint,
    )]
    pub cache_state: Box<Account<'info, CacheState>>,

    pub cache_admin: Signer<'info>,

    #[account(mut)]
    pub onyc_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        seeds = [seeds::CACHE_VAULT_AUTHORITY],
        bump,
    )]
    pub cache_vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = cache_vault_onyc_account.owner == cache_vault_authority.key() @ CacheErrorCode::InvalidOnycMint,
        constraint = cache_vault_onyc_account.mint == onyc_mint.key() @ CacheErrorCode::InvalidOnycMint
    )]
    pub cache_vault_onyc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        seeds = [seeds::MANAGEMENT_FEE_VAULT_AUTHORITY],
        bump,
    )]
    pub management_fee_vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = management_fee_vault_onyc_account.owner == management_fee_vault_authority.key() @ CacheErrorCode::InvalidOnycMint,
        constraint = management_fee_vault_onyc_account.mint == onyc_mint.key() @ CacheErrorCode::InvalidOnycMint
    )]
    pub management_fee_vault_onyc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        seeds = [seeds::PERFORMANCE_FEE_VAULT_AUTHORITY],
        bump,
    )]
    pub performance_fee_vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = performance_fee_vault_onyc_account.owner == performance_fee_vault_authority.key() @ CacheErrorCode::InvalidOnycMint,
        constraint = performance_fee_vault_onyc_account.mint == onyc_mint.key() @ CacheErrorCode::InvalidOnycMint
    )]
    pub performance_fee_vault_onyc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        seeds = [seeds::MINT_AUTHORITY],
        constraint = onyc_mint.mint_authority.unwrap() == mint_authority.key() @ CacheErrorCode::NoMintAuthority,
        bump
    )]
    pub mint_authority: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn accrue_cache(ctx: Context<AccrueCache>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let cache_state = &mut ctx.accounts.cache_state;
    require!(
        now >= cache_state.last_accrual_timestamp,
        CacheErrorCode::InvalidTimestamp
    );

    let seconds_elapsed = (now - cache_state.last_accrual_timestamp) as u64;
    let spread = cache_state
        .gross_yield
        .saturating_sub(cache_state.current_yield);
    let previous_lowest_supply = cache_state.lowest_supply;
    let previous_performance_fee_high_watermark = cache_state.performance_fee_high_watermark;
    let current_supply_before_mint = ctx.accounts.onyc_mint.supply;
    let cache_balance_before_mint = ctx.accounts.cache_vault_onyc_account.amount;
    let gross_mint_amount = calculate_gross_cache_accrual(
        previous_lowest_supply,
        cache_state.gross_yield,
        cache_state.current_yield,
        seconds_elapsed,
    )?;
    let fee_split = calculate_cache_fee_split(
        gross_mint_amount,
        cache_state.management_fee_basis_points,
        cache_state.performance_fee_basis_points,
        cache_balance_before_mint,
        previous_performance_fee_high_watermark,
    )?;

    if gross_mint_amount > 0 {
        if ctx.accounts.state.max_supply > 0 {
            let new_supply = ctx
                .accounts
                .onyc_mint
                .supply
                .checked_add(gross_mint_amount)
                .ok_or(CacheErrorCode::MathOverflow)?;
            require!(
                new_supply <= ctx.accounts.state.max_supply,
                TokenUtilsErrorCode::MaxSupplyExceeded
            );
        }

        let mint_authority_seeds = &[seeds::MINT_AUTHORITY, &[ctx.bumps.mint_authority]];
        let mint_authority_signer_seeds = &[mint_authority_seeds.as_slice()];

        if fee_split.cache_mint_amount > 0 {
            mint_tokens(
                &ctx.accounts.token_program,
                &ctx.accounts.onyc_mint,
                &ctx.accounts.cache_vault_onyc_account,
                &ctx.accounts.mint_authority.to_account_info(),
                mint_authority_signer_seeds,
                fee_split.cache_mint_amount,
                0,
            )?;
        }

        if fee_split.management_fee_mint_amount > 0 {
            mint_tokens(
                &ctx.accounts.token_program,
                &ctx.accounts.onyc_mint,
                &ctx.accounts.management_fee_vault_onyc_account,
                &ctx.accounts.mint_authority.to_account_info(),
                mint_authority_signer_seeds,
                fee_split.management_fee_mint_amount,
                0,
            )?;
        }

        if fee_split.performance_fee_mint_amount > 0 {
            mint_tokens(
                &ctx.accounts.token_program,
                &ctx.accounts.onyc_mint,
                &ctx.accounts.performance_fee_vault_onyc_account,
                &ctx.accounts.mint_authority.to_account_info(),
                mint_authority_signer_seeds,
                fee_split.performance_fee_mint_amount,
                0,
            )?;
        }

        cache_state.total_management_fees_accrued = cache_state
            .total_management_fees_accrued
            .checked_add(fee_split.management_fee_mint_amount)
            .ok_or(CacheErrorCode::MathOverflow)?;
        cache_state.total_performance_fees_accrued = cache_state
            .total_performance_fees_accrued
            .checked_add(fee_split.performance_fee_mint_amount)
            .ok_or(CacheErrorCode::MathOverflow)?;
    }

    cache_state.lowest_supply = current_supply_before_mint;
    cache_state.performance_fee_high_watermark = fee_split.new_performance_fee_high_watermark;
    cache_state.last_accrual_timestamp = now;

    emit!(CacheAccruedEvent {
        seconds_elapsed,
        spread,
        gross_mint_amount: fee_split.gross_mint_amount,
        cache_mint_amount: fee_split.cache_mint_amount,
        management_fee_mint_amount: fee_split.management_fee_mint_amount,
        performance_fee_mint_amount: fee_split.performance_fee_mint_amount,
        previous_lowest_supply,
        new_lowest_supply: cache_state.lowest_supply,
        previous_performance_fee_high_watermark,
        new_performance_fee_high_watermark: cache_state.performance_fee_high_watermark,
        timestamp: now,
    });

    Ok(())
}
