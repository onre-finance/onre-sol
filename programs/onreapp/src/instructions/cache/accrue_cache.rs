use crate::constants::seeds;
use crate::instructions::cache::{
    CacheAccruedEvent, CacheErrorCode, CacheState, SECONDS_PER_YEAR, YIELD_SCALE,
};
use crate::state::State;
use crate::utils::token_utils::mint_tokens;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct AccrueCache<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = onyc_mint,
    )]
    pub state: Account<'info, State>,

    #[account(
        mut,
        seeds = [seeds::CACHE_STATE],
        bump = cache_state.bump,
        has_one = cache_admin,
        has_one = onyc_mint,
    )]
    pub cache_state: Account<'info, CacheState>,

    pub cache_admin: Signer<'info>,

    #[account(mut)]
    pub onyc_mint: InterfaceAccount<'info, Mint>,

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
    pub cache_vault_onyc_account: InterfaceAccount<'info, TokenAccount>,

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
    let current_supply_before_mint = ctx.accounts.onyc_mint.supply;

    let mint_amount = if spread == 0 || previous_lowest_supply == 0 || seconds_elapsed == 0 {
        0
    } else {
        let mint_amount_u128 = (previous_lowest_supply as u128)
            .checked_mul(spread as u128)
            .and_then(|v| v.checked_mul(seconds_elapsed as u128))
            .ok_or(CacheErrorCode::MathOverflow)?
            .checked_div(SECONDS_PER_YEAR)
            .and_then(|v| v.checked_div(YIELD_SCALE))
            .ok_or(CacheErrorCode::MathOverflow)?;

        require!(
            mint_amount_u128 <= u64::MAX as u128,
            CacheErrorCode::ResultOverflow
        );
        mint_amount_u128 as u64
    };

    if mint_amount > 0 {
        let mint_authority_seeds = &[seeds::MINT_AUTHORITY, &[ctx.bumps.mint_authority]];
        let mint_authority_signer_seeds = &[mint_authority_seeds.as_slice()];

        mint_tokens(
            &ctx.accounts.token_program,
            &ctx.accounts.onyc_mint,
            &ctx.accounts.cache_vault_onyc_account,
            &ctx.accounts.mint_authority.to_account_info(),
            mint_authority_signer_seeds,
            mint_amount,
            ctx.accounts.state.max_supply,
        )?;
    }

    cache_state.lowest_supply = current_supply_before_mint;
    cache_state.last_accrual_timestamp = now;

    emit!(CacheAccruedEvent {
        seconds_elapsed,
        spread,
        mint_amount,
        previous_lowest_supply,
        new_lowest_supply: cache_state.lowest_supply,
        timestamp: now,
    });

    Ok(())
}
