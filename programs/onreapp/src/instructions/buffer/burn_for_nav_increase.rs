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
    let quoted_nav = compute_offer_current_price(&main_offer, now as u64)?;
    require!(quoted_nav > 0, BufferErrorCode::InvalidTargetNav);
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
    // This path burns supply to preserve the current quoted NAV after reducing the
    // effective asset base by `asset_adjustment_amount`.
    let total_assets_before_burn = calculate_tvl(circulating_supply, current_nav)
        .map_err(|_| error!(BufferErrorCode::MathOverflow))?;
    let burn_amount = calculate_burn_amount(
        circulating_supply,
        asset_adjustment_amount,
        current_nav,
        quoted_nav,
    )?;
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
        target_nav: quoted_nav,
    });

    Ok(())
}

fn calculate_burn_amount(
    circulating_supply: u64,
    asset_adjustment_amount: u64,
    nav_before_adjustment: u64,
    nav_after_adjustment: u64,
) -> Result<u64> {
    let total_assets_before_burn = calculate_tvl(circulating_supply, nav_before_adjustment)
        .map_err(|_| error!(BufferErrorCode::MathOverflow))?;

    require!(
        total_assets_before_burn >= asset_adjustment_amount,
        BufferErrorCode::InvalidAssetAdjustmentAmount
    );

    let nav_scale = 10u128
        .checked_pow(PRICE_DECIMALS as u32)
        .ok_or(BufferErrorCode::MathOverflow)?;
    let assets_after_adjustment = (total_assets_before_burn - asset_adjustment_amount) as u128;
    let required_supply_after = ceil_div_u128(
        assets_after_adjustment
            .checked_mul(nav_scale)
            .ok_or(BufferErrorCode::MathOverflow)?,
        nav_after_adjustment as u128,
    )
    .ok_or(BufferErrorCode::MathOverflow)?;

    let current_supply = circulating_supply as u128;
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

    Ok(burn_amount_u128 as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAV_1_0: u64 = 1_000_000_000;
    const NAV_1_1: u64 = 1_100_000_000;

    #[test]
    fn burn_amount_uses_circulating_supply_basis() {
        let total_supply = 1_100;
        let vault_supply = 100;
        let circulating_supply = calculate_circulating_supply(total_supply, vault_supply);

        let burn_amount = calculate_burn_amount(circulating_supply, 100, NAV_1_0, NAV_1_1).unwrap();
        let total_supply_based_burn = (total_supply as u128).checked_sub(819).unwrap() as u64;

        assert_eq!(burn_amount, 181);
        assert_eq!(total_supply_based_burn - burn_amount, vault_supply);
    }

    #[test]
    fn burn_amount_is_zero_when_assets_already_match_target_nav() {
        let err = calculate_burn_amount(1_000, 0, NAV_1_0, NAV_1_0).unwrap_err();
        assert_eq!(err, error!(BufferErrorCode::NoBurnNeeded));
    }
}
