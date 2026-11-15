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
use crate::instructions::offer::{
    calculate_redemption_vault_refill_amount, is_onyc_token_out_mint,
    load_redemption_offer_vault_target_bps_for_offer, should_accrue_onyc_mint,
};
use crate::instructions::Offer;
use crate::state::{ConfigurableVaultKind, State};
use crate::utils::{
    get_associated_token_account, get_or_create_associated_token_account, has_transfer_fee,
    mint_tokens, program_controls_mint, transfer_tokens, EnsureAtaParams,
};
use anchor_lang::{prelude::*, Accounts};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenInterface},
};

use super::config::PropAmmPairState;
use super::pricing::record_prop_amm_buy;
use super::validation::{validate_prop_amm_pair_for_side, SwapSide};

#[derive(Accounts)]
pub struct OpenSwapBuy<'info> {
    pub offer: AccountLoader<'info, Offer>,

    #[account(
        mut,
        seeds = [crate::constants::seeds::PROP_AMM_PAIR_STATE, offer.key().as_ref()],
        bump = prop_amm_pair_state.bump
    )]
    pub prop_amm_pair_state: Box<Account<'info, PropAmmPairState>>,

    /// CHECK: PDA address is validated by seeds; data is optional and loaded in instruction logic.
    #[account(
        seeds = [
            crate::constants::seeds::REDEMPTION_OFFER,
            token_out_mint.key().as_ref(),
            token_in_mint.key().as_ref()
        ],
        bump
    )]
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
    pub offer_vault_token_in_account: UncheckedAccount<'info>,

    /// CHECK: validated as canonical ATA in instruction logic
    #[account(mut)]
    pub offer_vault_token_out_account: UncheckedAccount<'info>,

    /// CHECK: validated and optionally initialized in instruction logic
    #[account(mut)]
    pub redemption_vault_token_in_account: UncheckedAccount<'info>,

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
            crate::constants::seeds::PROP_AMM_BUY_FEE_VAULT
        ],
        bump
    )]
    pub prop_amm_buy_fee_vault: UncheckedAccount<'info>,

    /// CHECK: Validated and optionally initialized in instruction logic.
    #[account(mut)]
    pub prop_amm_buy_fee_token_in_account: UncheckedAccount<'info>,

    /// CHECK: PDA derivation validated by seeds constraint
    #[account(seeds = [crate::constants::seeds::PERMISSIONLESS_AUTHORITY], bump)]
    pub permissionless_authority: UncheckedAccount<'info>,

    /// CHECK: validated and optionally initialized in instruction logic
    #[account(mut)]
    pub permissionless_token_in_account: UncheckedAccount<'info>,

    /// CHECK: validated and optionally initialized in instruction logic
    #[account(mut)]
    pub permissionless_token_out_account: UncheckedAccount<'info>,

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
}

pub fn open_swap_buy<'info>(
    ctx: Context<'info, OpenSwapBuy<'info>>,
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
        SwapSide::Buy,
    )?;

    execute_open_swap_buy(ctx, token_in_amount, minimum_out)
}

fn execute_open_swap_buy<'info>(
    ctx: Context<'info, OpenSwapBuy<'info>>,
    token_in_amount: u64,
    minimum_out: u64,
) -> Result<()> {
    let offer = ctx.accounts.offer.load()?;
    offer.require_enabled()?;
    require!(
        offer.allow_permissionless(),
        crate::OnreError::PermissionlessNotAllowed
    );
    let result = crate::instructions::offer::process_offer_core(
        &offer,
        token_in_amount,
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_out_mint,
        offer.permissionless_fee_basis_points(),
    )?;
    require!(
        !has_transfer_fee(&ctx.accounts.token_in_mint)?,
        crate::OnreError::TransferFeeNotSupported
    );
    require!(
        !has_transfer_fee(&ctx.accounts.token_out_mint)?,
        crate::OnreError::TransferFeeNotSupported
    );
    require!(
        result.token_out_amount >= minimum_out,
        crate::OnreError::MinimumOutNotMet
    );

    drop(offer);

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
    let (prop_amm_proceeds_token_in_account, prop_amm_buy_fee_token_in_account) =
        get_or_create_configurable_vault_token_account_pair::<
            { ConfigurableVaultKind::PropAmmProceeds.as_u8() },
            { ConfigurableVaultKind::PropAmmBuyFee.as_u8() },
        >(ConfigurableVaultTokenAccountPairParams {
            first_vault: &ctx.accounts.prop_amm_proceeds_vault,
            first_token_account: &ctx.accounts.prop_amm_proceeds_token_in_account,
            second_vault: &ctx.accounts.prop_amm_buy_fee_vault,
            second_token_account: &ctx.accounts.prop_amm_buy_fee_token_in_account,
            payer: ctx.accounts.user.to_account_info(),
            mint_account: ctx.accounts.token_in_mint.to_account_info(),
            token_program: ctx.accounts.token_in_program.to_account_info(),
            associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            program_id: ctx.program_id,
        })?;
    let _offer_vault_token_in_account = get_associated_token_account(
        &ctx.accounts.offer_vault_token_in_account,
        &ctx.accounts.offer_vault_authority.key(),
        &ctx.accounts.token_in_mint.key(),
        &ctx.accounts.token_in_program.key(),
        crate::OnreError::InvalidVaultTokenInAccount,
    )?;
    let offer_vault_token_out_account = get_associated_token_account(
        &ctx.accounts.offer_vault_token_out_account,
        &ctx.accounts.offer_vault_authority.key(),
        &ctx.accounts.token_out_mint.key(),
        &ctx.accounts.token_out_program.key(),
        crate::OnreError::InvalidVaultTokenOutAccount,
    )?;
    let permissionless_token_in_account = get_associated_token_account(
        &ctx.accounts.permissionless_token_in_account,
        &ctx.accounts.permissionless_authority.key(),
        &ctx.accounts.token_in_mint.key(),
        &ctx.accounts.token_in_program.key(),
        crate::OnreError::InvalidAmount,
    )?;
    let permissionless_token_out_account = get_associated_token_account(
        &ctx.accounts.permissionless_token_out_account,
        &ctx.accounts.permissionless_authority.key(),
        &ctx.accounts.token_out_mint.key(),
        &ctx.accounts.token_out_program.key(),
        crate::OnreError::InvalidPermissionlessTokenOutAccount,
    )?;
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

    let (_, permissionless_authority_bump) = Pubkey::find_program_address(
        &[crate::constants::seeds::PERMISSIONLESS_AUTHORITY],
        ctx.program_id,
    );
    let (_, offer_vault_authority_bump) = Pubkey::find_program_address(
        &[crate::constants::seeds::OFFER_VAULT_AUTHORITY],
        ctx.program_id,
    );
    let (_, mint_authority_bump) =
        Pubkey::find_program_address(&[crate::constants::seeds::MINT_AUTHORITY], ctx.program_id);

    let buffer_is_initialized = ctx
        .accounts
        .buffer_accounts
        .check_is_initialized(ctx.program_id)?;
    let should_accrue = should_accrue_onyc_mint(
        &ctx.accounts.state,
        &ctx.accounts.token_out_mint,
        buffer_is_initialized,
        &ctx.accounts.mint_authority.to_account_info(),
    );
    let main_offer = if is_onyc_token_out_mint(&ctx.accounts.state, &ctx.accounts.token_out_mint) {
        Some(load_main_offer(
            ctx.program_id,
            &ctx.accounts.main_offer.to_account_info(),
            &ctx.accounts.state,
        )?)
    } else {
        None
    };
    let accrual = if should_accrue {
        let canonical_offer = main_offer
            .as_ref()
            .ok_or(crate::OnreError::InvalidMainOffer)?;
        Some(accrue_buffer_from_accounts(
            ctx.program_id,
            &ctx.accounts.state,
            &ctx.accounts.buffer_accounts,
            canonical_offer,
            &ctx.accounts.token_out_mint,
            ctx.accounts.mint_authority.to_account_info(),
            mint_authority_bump,
            &ctx.accounts.token_out_program,
        )?)
    } else {
        None
    };

    if accrual.is_some() {
        ctx.accounts.token_out_mint.reload()?;
    }

    transfer_tokens(
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_in_program,
        &user_token_in_account,
        &permissionless_token_in_account,
        &ctx.accounts.user.to_account_info(),
        None,
        token_in_amount,
    )?;

    let redemption_vault_target_bps = load_redemption_offer_vault_target_bps_for_offer(
        ctx.program_id,
        &ctx.accounts.redemption_offer,
        ctx.accounts.offer.key(),
        ctx.accounts.token_in_mint.key(),
        ctx.accounts.token_out_mint.key(),
    )?
    .unwrap_or(0);
    let refill_amount = calculate_redemption_vault_refill_amount(
        &ctx.accounts.market_stats.to_account_info(),
        redemption_vault_target_bps,
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_out_mint,
        redemption_vault_token_in_account.amount,
        result.token_in_net_amount,
    );
    let boss_net_amount = result
        .token_in_net_amount
        .checked_sub(refill_amount)
        .ok_or(crate::OnreError::ArithmeticUnderflow)?;

    let permissionless_signer_seeds: &[&[&[u8]]] = &[&[
        crate::constants::seeds::PERMISSIONLESS_AUTHORITY,
        &[permissionless_authority_bump],
    ]];

    if refill_amount > 0 {
        transfer_tokens(
            &ctx.accounts.token_in_mint,
            &ctx.accounts.token_in_program,
            &permissionless_token_in_account,
            &redemption_vault_token_in_account,
            &ctx.accounts.permissionless_authority.to_account_info(),
            Some(permissionless_signer_seeds),
            refill_amount,
        )?;
    }
    if boss_net_amount > 0 {
        transfer_tokens(
            &ctx.accounts.token_in_mint,
            &ctx.accounts.token_in_program,
            &permissionless_token_in_account,
            &prop_amm_proceeds_token_in_account,
            &ctx.accounts.permissionless_authority.to_account_info(),
            Some(permissionless_signer_seeds),
            boss_net_amount,
        )?;
    }
    if result.token_in_fee_amount > 0 {
        transfer_tokens(
            &ctx.accounts.token_in_mint,
            &ctx.accounts.token_in_program,
            &permissionless_token_in_account,
            &prop_amm_buy_fee_token_in_account,
            &ctx.accounts.permissionless_authority.to_account_info(),
            Some(permissionless_signer_seeds),
            result.token_in_fee_amount,
        )?;
    }

    record_prop_amm_buy(
        &mut ctx.accounts.prop_amm_pair_state,
        result.token_in_net_amount,
        Clock::get()?.unix_timestamp,
    )?;

    if program_controls_mint(
        &ctx.accounts.token_out_mint,
        &ctx.accounts.mint_authority.to_account_info(),
    ) {
        let mint_authority_signer_seeds: &[&[&[u8]]] = &[&[
            crate::constants::seeds::MINT_AUTHORITY,
            &[mint_authority_bump],
        ]];
        mint_tokens(
            &ctx.accounts.token_out_program,
            &ctx.accounts.token_out_mint,
            &user_token_out_account.to_account_info(),
            &ctx.accounts.mint_authority.to_account_info(),
            mint_authority_signer_seeds,
            result.token_out_amount,
            ctx.accounts.state.max_supply,
            ctx.accounts.state.max_mint_amount,
        )?;
    } else {
        let offer_vault_signer_seeds: &[&[&[u8]]] = &[&[
            crate::constants::seeds::OFFER_VAULT_AUTHORITY,
            &[offer_vault_authority_bump],
        ]];
        transfer_tokens(
            &ctx.accounts.token_out_mint,
            &ctx.accounts.token_out_program,
            &offer_vault_token_out_account,
            &permissionless_token_out_account,
            &ctx.accounts.offer_vault_authority.to_account_info(),
            Some(offer_vault_signer_seeds),
            result.token_out_amount,
        )?;
        transfer_tokens(
            &ctx.accounts.token_out_mint,
            &ctx.accounts.token_out_program,
            &permissionless_token_out_account,
            &user_token_out_account,
            &ctx.accounts.permissionless_authority.to_account_info(),
            Some(permissionless_signer_seeds),
            result.token_out_amount,
        )?;
    }

    if let Some(accrual) = accrual {
        let post_offer_supply = accrual
            .post_accrual_supply
            .checked_add(result.token_out_amount)
            .ok_or(crate::OnreError::OverflowError)?;
        store_buffer_post_supply(
            &ctx.accounts.buffer_accounts,
            post_offer_supply,
            accrual.timestamp,
        )?;
    }

    if is_onyc_token_out_mint(&ctx.accounts.state, &ctx.accounts.token_out_mint) {
        let main_offer = main_offer
            .as_ref()
            .ok_or(crate::OnreError::InvalidMainOffer)?;
        ctx.accounts.token_out_mint.reload()?;
        refresh_market_stats_pda(
            main_offer,
            &ctx.accounts.token_out_mint,
            &ctx.accounts
                .circulating_supply_excluded_balance
                .to_account_info(),
            &ctx.accounts.market_stats.to_account_info(),
            &ctx.accounts.user.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            ctx.program_id,
        )?;
    }

    Ok(())
}
