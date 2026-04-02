use crate::constants::{seeds, PRICE_DECIMALS};
use crate::instructions::buffer::{
    accrue_buffer::{accrue_buffer, set_buffer_baseline_after_supply_change},
    BufferBurnedForNavEvent, BufferErrorCode, BufferState,
};
use crate::instructions::market_info::{
    calculate_circulating_supply, calculate_tvl,
    offer_valuation_utils::compute_offer_current_price, refresh_market_stats_pda,
};
use crate::instructions::Offer;
use crate::state::State;
use crate::utils::math_utils::ceil_div_u128;
use crate::utils::token_utils::{burn_tokens, read_optional_token_account_amount};
use crate::OfferCoreError;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_option::COption;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct BurnForNavIncrease<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss,
        has_one = onyc_mint,
        constraint = !state.is_killed @ BufferErrorCode::KillSwitchActivated,
    )]
    pub state: Box<Account<'info, State>>,

    #[account(
        mut,
        seeds = [seeds::BUFFER_STATE],
        bump = buffer_state.bump,
    )]
    pub buffer_state: Box<Account<'info, BufferState>>,

    pub boss: Signer<'info>,

    #[account(
        address = state.main_offer @ BufferErrorCode::InvalidMainOffer
    )]
    pub main_offer: AccountLoader<'info, Offer>,

    #[account(mut)]
    pub onyc_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::OFFER_VAULT_AUTHORITY], bump)]
    pub offer_vault_authority: UncheckedAccount<'info>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        seeds = [seeds::RESERVE_VAULT_AUTHORITY],
        bump,
    )]
    pub reserve_vault_authority: UncheckedAccount<'info>,

    /// CHECK: Account is validated in instruction logic to allow uninitialized vault account
    pub vault_token_out_account: UncheckedAccount<'info>,

    #[account(mut)]
    pub reserve_vault_onyc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        seeds = [seeds::MANAGEMENT_FEE_VAULT_AUTHORITY],
        bump,
    )]
    pub management_fee_vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = onyc_mint,
        associated_token::authority = management_fee_vault_authority,
        associated_token::token_program = token_program
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
        associated_token::mint = onyc_mint,
        associated_token::authority = performance_fee_vault_authority,
        associated_token::token_program = token_program
    )]
    pub performance_fee_vault_onyc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        seeds = [seeds::MINT_AUTHORITY],
        constraint = onyc_mint.mint_authority == COption::Some(mint_authority.key()) @ BufferErrorCode::NoMintAuthority,
        bump
    )]
    pub mint_authority: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,

    /// CHECK: Validated and optionally initialized in instruction logic.
    #[account(mut)]
    pub market_stats: UncheckedAccount<'info>,
}

pub fn burn_for_nav_increase(
    ctx: Context<BurnForNavIncrease>,
    asset_adjustment_amount: u64,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let main_offer = ctx.accounts.main_offer.load()?;
    require_keys_eq!(
        ctx.accounts.onyc_mint.key(),
        main_offer.token_out_mint,
        OfferCoreError::InvalidTokenOutMint
    );
    let target_nav = compute_offer_current_price(&main_offer, now as u64)?;
    require!(target_nav > 0, BufferErrorCode::InvalidTargetNav);
    let expected_vault_token_out_account = get_associated_token_address_with_program_id(
        &ctx.accounts.offer_vault_authority.key(),
        &ctx.accounts.onyc_mint.key(),
        &ctx.accounts.token_program.key(),
    );
    require_keys_eq!(
        ctx.accounts.vault_token_out_account.key(),
        expected_vault_token_out_account,
        BufferErrorCode::InvalidOnycMint
    );
    let expected_reserve_vault_onyc_account = get_associated_token_address_with_program_id(
        &ctx.accounts.reserve_vault_authority.key(),
        &ctx.accounts.onyc_mint.key(),
        &ctx.accounts.token_program.key(),
    );
    require_keys_eq!(
        ctx.accounts.reserve_vault_onyc_account.key(),
        expected_reserve_vault_onyc_account,
        BufferErrorCode::InvalidOnycMint
    );
    let reserve_vault_onyc_account = ctx.accounts.reserve_vault_onyc_account.to_account_info();
    let management_fee_vault_onyc_account = ctx
        .accounts
        .management_fee_vault_onyc_account
        .to_account_info();
    let performance_fee_vault_onyc_account = ctx
        .accounts
        .performance_fee_vault_onyc_account
        .to_account_info();
    let mint_authority = ctx.accounts.mint_authority.to_account_info();
    let accrual = accrue_buffer(
        &ctx.accounts.state,
        &mut ctx.accounts.buffer_state,
        &main_offer,
        &ctx.accounts.onyc_mint,
        reserve_vault_onyc_account,
        management_fee_vault_onyc_account,
        performance_fee_vault_onyc_account,
        mint_authority,
        ctx.bumps.mint_authority,
        &ctx.accounts.token_program,
        now,
    )?;
    let current_nav = accrual.current_nav;
    let post_accrual_supply = accrual.post_accrual_supply;

    let vault_token_out_amount = read_optional_token_account_amount(
        &ctx.accounts.vault_token_out_account,
        &ctx.accounts.token_program,
    )?;
    let circulating_supply =
        calculate_circulating_supply(post_accrual_supply, vault_token_out_amount);
    let total_assets_before_burn = calculate_tvl(circulating_supply, current_nav)
        .map_err(|_| error!(BufferErrorCode::MathOverflow))?;

    require!(
        total_assets_before_burn >= asset_adjustment_amount,
        BufferErrorCode::InvalidAssetAdjustmentAmount
    );

    let nav_scale = 10u128
        .checked_pow(PRICE_DECIMALS as u32)
        .ok_or(BufferErrorCode::MathOverflow)?;
    let assets_after_adjustment = (total_assets_before_burn - asset_adjustment_amount) as u128;
    let target_nav_u128 = target_nav as u128;

    let required_supply_after = ceil_div_u128(
        assets_after_adjustment
            .checked_mul(nav_scale)
            .ok_or(BufferErrorCode::MathOverflow)?,
        target_nav_u128,
    )
    .ok_or(BufferErrorCode::MathOverflow)?;

    let current_supply = post_accrual_supply as u128;
    require!(
        required_supply_after <= current_supply,
        BufferErrorCode::InvalidBurnTarget
    );

    let burn_amount_u128 = current_supply
        .checked_sub(required_supply_after)
        .ok_or(BufferErrorCode::MathOverflow)?;
    require!(burn_amount_u128 > 0, BufferErrorCode::NoBurnNeeded);
    require!(
        burn_amount_u128 <= u64::MAX as u128,
        BufferErrorCode::ResultOverflow,
    );

    let burn_amount = burn_amount_u128 as u64;
    require!(
        burn_amount <= accrual.reserve_vault_balance_after_accrual,
        BufferErrorCode::InsufficientCacheBalance
    );

    let reserve_vault_authority_seeds = &[
        seeds::RESERVE_VAULT_AUTHORITY,
        &[ctx.bumps.reserve_vault_authority],
    ];
    let reserve_vault_authority_signer_seeds = &[reserve_vault_authority_seeds.as_slice()];

    burn_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.onyc_mint,
        &ctx.accounts.reserve_vault_onyc_account,
        &ctx.accounts.reserve_vault_authority.to_account_info(),
        reserve_vault_authority_signer_seeds,
        burn_amount,
    )?;

    let post_burn_supply = post_accrual_supply
        .checked_sub(burn_amount)
        .ok_or(BufferErrorCode::MathOverflow)?;
    set_buffer_baseline_after_supply_change(&mut ctx.accounts.buffer_state, post_burn_supply, now);

    ctx.accounts.onyc_mint.reload()?;
    refresh_market_stats_pda(
        &main_offer,
        &ctx.accounts.onyc_mint,
        &ctx.accounts.vault_token_out_account.to_account_info(),
        &ctx.accounts.token_program,
        &ctx.accounts.market_stats.to_account_info(),
        &ctx.accounts.boss.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        ctx.program_id,
    )?;

    emit!(BufferBurnedForNavEvent {
        burn_amount,
        asset_adjustment_amount,
        total_assets: total_assets_before_burn,
        target_nav,
    });

    Ok(())
}
