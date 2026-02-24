use crate::constants::{seeds, PRICE_DECIMALS};
use crate::instructions::cache::{CacheBurnedForNavEvent, CacheErrorCode, CacheState};
use crate::instructions::market_info::offer_valuation_utils::{
    compute_offer_current_price, compute_tvl_from_supply_and_price,
};
use crate::instructions::Offer;
use crate::state::State;
use crate::utils::math_utils::ceil_div_u128;
use crate::utils::token_utils::{burn_tokens, read_optional_token_account_amount};
use crate::OfferCoreError;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct BurnForNavIncrease<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss,
        has_one = onyc_mint,
    )]
    pub state: Account<'info, State>,

    #[account(
        mut,
        seeds = [seeds::CACHE_STATE],
        bump = cache_state.bump,
        has_one = onyc_mint,
    )]
    pub cache_state: Account<'info, CacheState>,

    pub boss: Signer<'info>,

    #[account(
        seeds = [
            seeds::OFFER,
            token_in_mint.key().as_ref(),
            onyc_mint.key().as_ref()
        ],
        bump
    )]
    pub offer: AccountLoader<'info, Offer>,

    pub token_in_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub onyc_mint: InterfaceAccount<'info, Mint>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::OFFER_VAULT_AUTHORITY], bump)]
    pub offer_vault_authority: UncheckedAccount<'info>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        seeds = [seeds::CACHE_VAULT_AUTHORITY],
        bump,
    )]
    pub cache_vault_authority: UncheckedAccount<'info>,

    /// CHECK: Account is validated in instruction logic to allow uninitialized vault account
    pub vault_token_out_account: UncheckedAccount<'info>,

    #[account(mut)]
    pub cache_vault_onyc_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn burn_for_nav_increase(
    ctx: Context<BurnForNavIncrease>,
    asset_adjustment_amount: u64,
    target_nav: u64,
) -> Result<()> {
    require!(target_nav > 0, CacheErrorCode::InvalidTargetNav);

    let offer = ctx.accounts.offer.load()?;
    require_keys_eq!(
        ctx.accounts.token_in_mint.key(),
        offer.token_in_mint,
        OfferCoreError::InvalidTokenInMint
    );
    let expected_vault_token_out_account = get_associated_token_address_with_program_id(
        &ctx.accounts.offer_vault_authority.key(),
        &ctx.accounts.onyc_mint.key(),
        &ctx.accounts.token_program.key(),
    );
    require_keys_eq!(
        ctx.accounts.vault_token_out_account.key(),
        expected_vault_token_out_account,
        CacheErrorCode::InvalidOnycMint
    );
    require_keys_eq!(
        ctx.accounts.cache_vault_onyc_account.owner,
        ctx.accounts.cache_vault_authority.key(),
        CacheErrorCode::InvalidOnycMint
    );
    require_keys_eq!(
        ctx.accounts.cache_vault_onyc_account.mint,
        ctx.accounts.onyc_mint.key(),
        CacheErrorCode::InvalidOnycMint
    );
    let current_time = Clock::get()?.unix_timestamp as u64;
    let current_price = compute_offer_current_price(&offer, current_time)?;

    let vault_token_out_amount = read_optional_token_account_amount(
        &ctx.accounts.vault_token_out_account,
        &ctx.accounts.token_program,
    )?;
    let token_supply = ctx
        .accounts
        .onyc_mint
        .supply
        .saturating_sub(vault_token_out_amount);
    let total_assets = compute_tvl_from_supply_and_price(token_supply, current_price)
        .ok_or(CacheErrorCode::MathOverflow)?;

    require!(
        total_assets >= asset_adjustment_amount,
        CacheErrorCode::InvalidAssetAdjustmentAmount
    );

    let nav_scale = 10u128
        .checked_pow(PRICE_DECIMALS as u32)
        .ok_or(CacheErrorCode::MathOverflow)?;
    let assets_after = (total_assets - asset_adjustment_amount) as u128;
    let target_nav_u128 = target_nav as u128;

    let required_supply_after = ceil_div_u128(
        assets_after
            .checked_mul(nav_scale)
            .ok_or(CacheErrorCode::MathOverflow)?,
        target_nav_u128,
    )
    .ok_or(CacheErrorCode::MathOverflow)?;

    let current_supply = ctx.accounts.onyc_mint.supply as u128;
    require!(
        required_supply_after <= current_supply,
        CacheErrorCode::InvalidBurnTarget
    );

    let burn_amount_u128 = current_supply
        .checked_sub(required_supply_after)
        .ok_or(CacheErrorCode::MathOverflow)?;
    require!(burn_amount_u128 > 0, CacheErrorCode::NoBurnNeeded);
    require!(
        burn_amount_u128 <= u64::MAX as u128,
        CacheErrorCode::ResultOverflow
    );

    let burn_amount = burn_amount_u128 as u64;
    require!(
        burn_amount <= ctx.accounts.cache_vault_onyc_account.amount,
        CacheErrorCode::InsufficientCacheBalance
    );

    let cache_vault_authority_seeds = &[
        seeds::CACHE_VAULT_AUTHORITY,
        &[ctx.bumps.cache_vault_authority],
    ];
    let cache_vault_authority_signer_seeds = &[cache_vault_authority_seeds.as_slice()];

    burn_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.onyc_mint,
        &ctx.accounts.cache_vault_onyc_account,
        &ctx.accounts.cache_vault_authority.to_account_info(),
        cache_vault_authority_signer_seeds,
        burn_amount,
    )?;

    let cache_state = &mut ctx.accounts.cache_state;
    let new_supply = ctx.accounts.onyc_mint.supply.saturating_sub(burn_amount);
    if cache_state.lowest_supply == 0 || new_supply < cache_state.lowest_supply {
        cache_state.lowest_supply = new_supply;
    }

    emit!(CacheBurnedForNavEvent {
        burn_amount,
        asset_adjustment_amount,
        total_assets,
        target_nav,
    });

    Ok(())
}
