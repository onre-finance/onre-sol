use crate::instructions::buffer::accounts::{
    __client_accounts_buffer_accrual_accounts, __cpi_client_accounts_buffer_accrual_accounts,
    BufferAccrualAccountsBumps,
};
use crate::instructions::buffer::accrue_buffer::{
    accrue_buffer_from_accounts, store_buffer_post_supply,
};
use crate::instructions::buffer::BufferAccrualAccounts;
use crate::instructions::configurable_vault::{
    get_or_create_configurable_vault_token_account_pair, ConfigurableVaultTokenAccountPairParams,
};
use crate::instructions::market_info::{load_main_offer, refresh_market_stats_pda};
use crate::instructions::offer::{validate_take_offer_authorities, OfferTakenEvent};
use crate::instructions::redemption::{
    execute_redemption_operations, process_redemption_core, ExecuteRedemptionOpsParams,
};
use crate::instructions::Offer;
use crate::state::{ConfigurableVaultKind, State};
use crate::utils::{
    get_associated_token_account, get_or_create_associated_token_account, program_controls_mint,
    transfer_tokens, u64_to_dec9, EnsureAtaParams,
};
use anchor_lang::{prelude::*, Accounts};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenInterface},
};

use super::config::PropAmmPairState;
use super::pricing::{
    apply_hard_wall_liquidity_factor, record_prop_amm_sell, sell_quote_liquidity_context,
    validate_market_stats_pda,
};
use super::validation::{validate_prop_amm_pair_for_side, SwapSide};

#[derive(Accounts)]
pub struct OpenSwapSell<'info> {
    #[account(
        seeds = [
            crate::constants::seeds::OFFER,
            token_out_mint.key().as_ref(),
            token_in_mint.key().as_ref()
        ],
        bump = offer.load()?.bump
    )]
    pub offer: AccountLoader<'info, Offer>,

    #[account(
        mut,
        seeds = [crate::constants::seeds::PROP_AMM_PAIR_STATE, offer.key().as_ref()],
        bump = prop_amm_pair_state.bump
    )]
    pub prop_amm_pair_state: Box<Account<'info, PropAmmPairState>>,

    #[account(
        seeds = [
            crate::constants::seeds::REDEMPTION_OFFER,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref()
        ],
        bump
    )]
    /// CHECK: PDA address is validated by seeds; data is optional and loaded in instruction logic.
    pub redemption_offer: UncheckedAccount<'info>,

    #[account(
        seeds = [crate::constants::seeds::STATE],
        bump = state.bump,
        constraint = !state.is_killed @ crate::OnreError::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,

    /// CHECK: PDA derivation validated by seeds constraint
    #[account(seeds = [crate::constants::seeds::OFFER_VAULT_AUTHORITY], bump)]
    pub offer_vault_authority: UncheckedAccount<'info>,

    /// CHECK: PDA derivation validated by seeds constraint
    #[account(seeds = [crate::constants::seeds::REDEMPTION_OFFER_VAULT_AUTHORITY], bump)]
    pub redemption_vault_authority: UncheckedAccount<'info>,

    /// CHECK: validated as canonical ATA in instruction logic
    #[account(mut)]
    pub redemption_vault_token_in_account: UncheckedAccount<'info>,

    /// CHECK: validated as canonical ATA in instruction logic
    #[account(mut)]
    pub redemption_vault_token_out_account: UncheckedAccount<'info>,

    #[account(mut)]
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,

    pub token_in_program: Interface<'info, TokenInterface>,

    #[account(mut)]
    pub token_out_mint: Box<InterfaceAccount<'info, Mint>>,

    pub token_out_program: Interface<'info, TokenInterface>,

    /// CHECK: validated as canonical ATA in instruction logic
    #[account(mut)]
    pub user_token_in_account: UncheckedAccount<'info>,

    /// CHECK: validated and optionally initialized in instruction logic
    #[account(mut)]
    pub user_token_out_account: UncheckedAccount<'info>,

    /// CHECK: PDA derivation is validated by seeds constraint; data is validated/initialized in instruction logic.
    #[account(
        mut,
        seeds = [
            crate::constants::seeds::CONFIGURABLE_VAULT,
            crate::constants::seeds::PROP_AMM_PROCEEDS_VAULT
        ],
        bump
    )]
    pub prop_amm_proceeds_vault: UncheckedAccount<'info>,

    /// CHECK: Validated and optionally initialized in instruction logic.
    #[account(mut)]
    pub prop_amm_proceeds_token_in_account: UncheckedAccount<'info>,

    /// CHECK: PDA derivation is validated by seeds constraint; data is validated/initialized in instruction logic.
    #[account(
        mut,
        seeds = [
            crate::constants::seeds::CONFIGURABLE_VAULT,
            crate::constants::seeds::PROP_AMM_SELL_FEE_VAULT
        ],
        bump
    )]
    pub prop_amm_sell_fee_vault: UncheckedAccount<'info>,

    /// CHECK: Validated and optionally initialized in instruction logic.
    #[account(mut)]
    pub prop_amm_sell_fee_token_in_account: UncheckedAccount<'info>,

    /// CHECK: PDA derivation validated by seeds constraint
    #[account(seeds = [crate::constants::seeds::MINT_AUTHORITY], bump)]
    pub mint_authority: UncheckedAccount<'info>,

    pub buffer_accounts: BufferAccrualAccounts<'info>,

    /// CHECK: PDA derivation is validated by seeds constraint and the account is optionally initialized in instruction logic.
    #[account(mut, seeds = [crate::constants::seeds::MARKET_STATS], bump)]
    pub market_stats: UncheckedAccount<'info>,

    /// CHECK: PDA derivation is validated by seeds constraint; data loading is handled by market stats refresh.
    #[account(seeds = [crate::constants::seeds::CIRCULATING_SUPPLY_EXCLUDED_BALANCE], bump)]
    pub circulating_supply_excluded_balance: UncheckedAccount<'info>,

    /// CHECK: validated in instruction logic
    pub instructions_sysvar: UncheckedAccount<'info>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,

    /// CHECK: validated against state.main_offer in instruction logic
    pub main_offer: UncheckedAccount<'info>,

    /// CHECK: validated as canonical ONYC offer-vault ATA in instruction logic
    pub offer_vault_onyc_account: UncheckedAccount<'info>,
}

pub fn open_swap_sell<'info>(
    ctx: Context<'info, OpenSwapSell<'info>>,
    token_in_amount: u64,
    minimum_out: u64,
) -> Result<()> {
    validate_prop_amm_pair_for_side(
        ctx.program_id,
        &ctx.accounts.state,
        &ctx.accounts.prop_amm_pair_state,
        ctx.accounts.offer.key(),
        ctx.accounts.token_in_mint.key(),
        ctx.accounts.token_out_mint.key(),
        SwapSide::Sell,
    )?;

    execute_open_swap_sell(ctx, token_in_amount, minimum_out)
}

fn execute_open_swap_sell<'info>(
    ctx: Context<'info, OpenSwapSell<'info>>,
    token_in_amount: u64,
    minimum_out: u64,
) -> Result<()> {
    let (_, mint_authority_bump) = validate_take_offer_authorities(
        ctx.program_id,
        &ctx.accounts.offer_vault_authority,
        &ctx.accounts.mint_authority,
        &ctx.accounts.instructions_sysvar,
    )?;
    let offer = ctx.accounts.offer.load()?;
    offer.require_enabled()?;
    validate_market_stats_pda(ctx.program_id, &ctx.accounts.market_stats)?;
    let main_offer = load_main_offer(
        ctx.program_id,
        &ctx.accounts.main_offer.to_account_info(),
        &ctx.accounts.state,
    )?;
    refresh_market_stats_pda(
        &main_offer,
        &ctx.accounts.token_in_mint,
        &ctx.accounts
            .circulating_supply_excluded_balance
            .to_account_info(),
        &ctx.accounts.market_stats.to_account_info(),
        &ctx.accounts.user.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        ctx.program_id,
    )?;
    let redemption_vault_token_out_account = get_associated_token_account(
        &ctx.accounts.redemption_vault_token_out_account,
        &ctx.accounts.redemption_vault_authority.key(),
        &ctx.accounts.token_out_mint.key(),
        &ctx.accounts.token_out_program.key(),
        crate::OnreError::InvalidVaultTokenOutAccount,
    )?;
    let liquidity = sell_quote_liquidity_context(
        ctx.program_id,
        &ctx.accounts.redemption_offer,
        ctx.accounts.offer.key(),
        ctx.accounts.token_in_mint.key(),
        ctx.accounts.token_out_mint.key(),
        &ctx.accounts.market_stats,
        redemption_vault_token_out_account.amount,
        ctx.accounts.token_out_mint.decimals,
        ctx.accounts.token_in_mint.decimals,
    )?;

    let user_token_in_account = get_associated_token_account(
        &ctx.accounts.user_token_in_account,
        &ctx.accounts.user.key(),
        &ctx.accounts.token_in_mint.key(),
        &ctx.accounts.token_in_program.key(),
        crate::OnreError::InvalidAmount,
    )?;
    let user_token_out_account = get_or_create_associated_token_account(EnsureAtaParams {
        ata_account: &ctx.accounts.user_token_out_account,
        payer: ctx.accounts.user.to_account_info(),
        authority_account: ctx.accounts.user.to_account_info(),
        mint_account: ctx.accounts.token_out_mint.to_account_info(),
        token_program: ctx.accounts.token_out_program.to_account_info(),
        associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        authority: ctx.accounts.user.key(),
        mint: ctx.accounts.token_out_mint.key(),
        token_program_id: ctx.accounts.token_out_program.key(),
        invalid_account_error: crate::OnreError::InvalidUserTokenOutAccount,
    })?;
    let (prop_amm_proceeds_token_in_account, prop_amm_sell_fee_token_in_account) =
        get_or_create_configurable_vault_token_account_pair::<
            { ConfigurableVaultKind::PropAmmProceeds.as_u8() },
            { ConfigurableVaultKind::PropAmmSellFee.as_u8() },
        >(ConfigurableVaultTokenAccountPairParams {
            first_vault: &ctx.accounts.prop_amm_proceeds_vault,
            first_token_account: &ctx.accounts.prop_amm_proceeds_token_in_account,
            second_vault: &ctx.accounts.prop_amm_sell_fee_vault,
            second_token_account: &ctx.accounts.prop_amm_sell_fee_token_in_account,
            payer: ctx.accounts.user.to_account_info(),
            mint_account: ctx.accounts.token_in_mint.to_account_info(),
            token_program: ctx.accounts.token_in_program.to_account_info(),
            associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            program_id: ctx.program_id,
        })?;
    let redemption_vault_token_in_account =
        get_or_create_associated_token_account(EnsureAtaParams {
            ata_account: &ctx.accounts.redemption_vault_token_in_account,
            payer: ctx.accounts.user.to_account_info(),
            authority_account: ctx.accounts.redemption_vault_authority.to_account_info(),
            mint_account: ctx.accounts.token_in_mint.to_account_info(),
            token_program: ctx.accounts.token_in_program.to_account_info(),
            associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            authority: ctx.accounts.redemption_vault_authority.key(),
            mint: ctx.accounts.token_in_mint.key(),
            token_program_id: ctx.accounts.token_in_program.key(),
            invalid_account_error: crate::OnreError::InvalidVaultTokenInAccount,
        })?;
    let _offer_vault_onyc_account = get_associated_token_account(
        &ctx.accounts.offer_vault_onyc_account,
        &ctx.accounts.offer_vault_authority.key(),
        &ctx.accounts.state.onyc_mint,
        &ctx.accounts.token_in_program.key(),
        crate::OnreError::InvalidOfferVaultOnycAccount,
    )?;

    let should_refresh_market_stats = ctx.accounts.token_in_mint.key()
        == ctx.accounts.state.onyc_mint
        && program_controls_mint(
            &ctx.accounts.token_in_mint,
            &ctx.accounts.mint_authority.to_account_info(),
        );
    let buffer_is_initialized = ctx
        .accounts
        .buffer_accounts
        .check_is_initialized(ctx.program_id)?;
    let accrual = if should_refresh_market_stats && buffer_is_initialized {
        Some(accrue_buffer_from_accounts(
            ctx.program_id,
            &ctx.accounts.state,
            &ctx.accounts.buffer_accounts,
            &main_offer,
            &ctx.accounts.token_in_mint,
            ctx.accounts.mint_authority.to_account_info(),
            mint_authority_bump,
            &ctx.accounts.token_in_program,
        )?)
    } else {
        None
    };

    if accrual.is_some() {
        ctx.accounts.token_in_mint.reload()?;
    }

    let mut result = process_redemption_core(
        &offer,
        token_in_amount,
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_out_mint,
        liquidity.fee_basis_points_prop_amm_sell,
        ctx.accounts.prop_amm_pair_state.minimum_sell_haircut_onyc,
    )?;
    let raw_sell_value_stable = result.token_out_amount;
    result.token_out_amount = apply_hard_wall_liquidity_factor(
        result.token_out_amount,
        liquidity.actual_liquidity,
        liquidity.hard_wall_reserve,
        &ctx.accounts.prop_amm_pair_state,
    )?;
    require!(
        result.token_out_amount >= minimum_out,
        crate::OnreError::MinimumOutNotMet
    );
    record_prop_amm_sell(
        &mut ctx.accounts.prop_amm_pair_state,
        raw_sell_value_stable,
        Clock::get()?.unix_timestamp,
    )?;

    transfer_tokens(
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_in_program,
        &user_token_in_account,
        &redemption_vault_token_in_account,
        &ctx.accounts.user.to_account_info(),
        None,
        token_in_amount,
    )?;

    execute_redemption_operations(ExecuteRedemptionOpsParams {
        token_in_program: &ctx.accounts.token_in_program,
        token_out_program: &ctx.accounts.token_out_program,
        token_in_mint: &ctx.accounts.token_in_mint,
        token_in_net_amount: result.token_in_net_amount,
        token_in_fee_amount: result.token_in_fee_amount,
        vault_token_in_account: &redemption_vault_token_in_account,
        token_in_destination_account: &prop_amm_proceeds_token_in_account,
        fee_destination_token_in_account: &prop_amm_sell_fee_token_in_account,
        redemption_vault_authority: &ctx.accounts.redemption_vault_authority.to_account_info(),
        redemption_vault_authority_bump: ctx.bumps.redemption_vault_authority,
        token_out_mint: &ctx.accounts.token_out_mint,
        token_out_amount: result.token_out_amount,
        vault_token_out_account: &redemption_vault_token_out_account,
        user_token_out_account: &user_token_out_account,
        mint_authority_pda: &ctx.accounts.mint_authority.to_account_info(),
    })?;

    if let Some(accrual) = accrual {
        let post_burn_supply = accrual
            .post_accrual_supply
            .checked_sub(result.token_in_net_amount)
            .ok_or(crate::OnreError::MathOverflow)?;
        store_buffer_post_supply(
            &ctx.accounts.buffer_accounts,
            post_burn_supply,
            accrual.timestamp,
        )?;
    }

    if should_refresh_market_stats {
        ctx.accounts.token_in_mint.reload()?;
        refresh_market_stats_pda(
            &main_offer,
            &ctx.accounts.token_in_mint,
            &ctx.accounts
                .circulating_supply_excluded_balance
                .to_account_info(),
            &ctx.accounts.market_stats.to_account_info(),
            &ctx.accounts.user.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            ctx.program_id,
        )?;
    }

    msg!(
        "Open swap sell - offer: {}, token_in(+fee): {}(+{}), token_out: {}, user: {}, price: {}",
        ctx.accounts.offer.key(),
        result.token_in_net_amount,
        result.token_in_fee_amount,
        result.token_out_amount,
        ctx.accounts.user.key(),
        u64_to_dec9(result.price)
    );

    emit!(OfferTakenEvent {
        offer_pda: ctx.accounts.offer.key(),
        token_in_amount: result.token_in_net_amount,
        token_out_amount: result.token_out_amount,
        fee_amount: result.token_in_fee_amount,
        user: ctx.accounts.user.key(),
    });

    Ok(())
}
