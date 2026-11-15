use crate::constants::seeds;
use crate::instructions::buffer::accounts::{
    __client_accounts_buffer_accrual_accounts, __cpi_client_accounts_buffer_accrual_accounts,
    BufferAccrualAccountsBumps,
};
use crate::instructions::buffer::{
    accrue_buffer::{accrue_buffer_from_accounts, store_buffer_post_supply},
    BufferAccrualAccounts,
};
use crate::instructions::configurable_vault::{
    get_or_create_configurable_vault_token_account, ConfigurableVaultTokenAccountParams,
};
use crate::instructions::market_info::{load_main_offer, refresh_market_stats_pda};
use crate::instructions::offer::offer_utils::{
    calculate_redemption_vault_refill_amount, is_onyc_token_out_mint,
    load_redemption_offer_vault_target_bps_for_offer, process_offer_core, should_accrue_onyc_mint,
};
use crate::instructions::Offer;
use crate::state::{ConfigurableVaultKind, State};
use crate::utils::{
    execute_token_operations, get_associated_token_account, get_or_create_associated_token_account,
    program_controls_mint, transfer_tokens, u64_to_dec9, EnsureAtaParams, ExecTokenOpsParams,
};
use anchor_lang::{prelude::*, Accounts};
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

// Shared permissionless offer-execution event definitions.

/// Event emitted when an offer is successfully executed via permissionless flow
///
/// Provides transparency for tracking permissionless offer execution with intermediary routing.
#[event]
pub struct OfferTakenPermissionlessEvent {
    /// The PDA address of the offer that was executed
    pub offer_pda: Pubkey,
    /// Amount of token_in paid by the user after fee deduction
    pub token_in_amount: u64,
    /// Amount of token_out received by the user
    pub token_out_amount: u64,
    /// Fee amount deducted from the original token_in payment
    pub fee_amount: u64,
    /// Public key of the user who executed the offer
    pub user: Pubkey,
}

#[derive(Accounts)]
pub struct TakeOfferPermissionlessV2<'info> {
    #[account(
        mut,
        seeds = [
            seeds::OFFER,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref()
        ],
        bump
    )]
    pub offer: AccountLoader<'info, Offer>,

    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        constraint = !state.is_killed @ crate::OnreError::KillSwitchActivated,
    )]
    pub state: Box<Account<'info, State>>,

    /// CHECK: PDA derivation is validated by explicit key check in the handler
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = token_in_program
    )]
    pub vault_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = token_out_mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = token_out_program
    )]
    pub vault_token_out_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: PDA derivation is validated by explicit key check in the handler
    pub permissionless_authority: UncheckedAccount<'info>,

    /// CHECK: Validated in instruction logic against the expected permissionless ATA.
    #[account(mut)]
    pub permissionless_token_in_account: UncheckedAccount<'info>,

    /// CHECK: Validated in instruction logic against the expected permissionless ATA.
    #[account(mut)]
    pub permissionless_token_out_account: UncheckedAccount<'info>,

    #[account(mut)]
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_in_program: Interface<'info, TokenInterface>,

    #[account(mut)]
    pub token_out_mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_out_program: Interface<'info, TokenInterface>,

    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = user,
        associated_token::token_program = token_in_program
    )]
    pub user_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: Validated and optionally initialized in instruction logic.
    #[account(mut)]
    pub user_token_out_account: UncheckedAccount<'info>,

    /// CHECK: Redemption offer PDA for the opposite offer direction; may be uninitialized.
    pub redemption_offer: UncheckedAccount<'info>,

    /// CHECK: PDA derivation is validated by seeds constraint.
    #[account(seeds = [seeds::REDEMPTION_OFFER_VAULT_AUTHORITY], bump)]
    pub redemption_vault_authority: UncheckedAccount<'info>,

    /// CHECK: Validated and optionally initialized in instruction logic.
    #[account(mut)]
    pub redemption_vault_token_in_account: UncheckedAccount<'info>,

    /// CHECK: PDA and data are validated/initialized in instruction logic.
    #[account(mut)]
    pub offer_proceeds_vault: UncheckedAccount<'info>,

    /// CHECK: Validated and optionally initialized in instruction logic.
    #[account(mut)]
    pub offer_proceeds_token_in_account: UncheckedAccount<'info>,

    /// CHECK: PDA and data are validated/initialized in instruction logic.
    #[account(mut)]
    pub permissionless_offer_fee_vault: UncheckedAccount<'info>,

    /// CHECK: Validated and optionally initialized in instruction logic.
    #[account(mut)]
    pub permissionless_offer_fee_token_in_account: UncheckedAccount<'info>,

    /// CHECK: PDA derivation is validated by explicit key check in the handler
    pub mint_authority: UncheckedAccount<'info>,
    pub buffer_accounts: BufferAccrualAccounts<'info>,

    /// CHECK: The handler validates PDA, writability, owner, and account data layout.
    #[account(mut)]
    pub market_stats: UncheckedAccount<'info>,

    /// CHECK: PDA validation and data loading are handled by market stats refresh.
    pub circulating_supply_excluded_balance: UncheckedAccount<'info>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,

    /// CHECK: Validated in instruction logic against state.main_offer.
    pub main_offer: UncheckedAccount<'info>,
}

pub(crate) struct PermissionlessMarketStatsRefresh<'a, 'info> {
    pub market_stats: &'a UncheckedAccount<'info>,
    pub circulating_supply_excluded_balance: &'a UncheckedAccount<'info>,
    pub main_offer_account: &'a UncheckedAccount<'info>,
}

pub(crate) struct PermissionlessRedemptionVaultRefill<'a, 'info> {
    pub redemption_offer: &'a UncheckedAccount<'info>,
    pub redemption_vault_authority: &'a UncheckedAccount<'info>,
    pub redemption_vault_token_in_account: &'a UncheckedAccount<'info>,
    pub associated_token_program: &'a Program<'info, AssociatedToken>,
}

/// Executes a permissionless offer through intermediary accounts, routing fees
/// and proceeds to configured vaults and applying refill and ONyc BUFFER accrual.
#[inline(never)]
pub fn take_offer_permissionless_v2<'info>(
    ctx: Context<'info, TakeOfferPermissionlessV2<'info>>,
    token_in_amount: u64,
) -> Result<()> {
    let offer_proceeds_token_in_account = get_or_create_configurable_vault_token_account::<
        { ConfigurableVaultKind::OfferProceeds.as_u8() },
    >(ConfigurableVaultTokenAccountParams {
        vault: &ctx.accounts.offer_proceeds_vault,
        token_account: &ctx.accounts.offer_proceeds_token_in_account,
        payer: ctx.accounts.user.to_account_info(),
        mint_account: ctx.accounts.token_in_mint.to_account_info(),
        token_program: ctx.accounts.token_in_program.to_account_info(),
        associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        program_id: ctx.program_id,
    })?;
    let permissionless_token_out_account = get_associated_token_account(
        &ctx.accounts.permissionless_token_out_account,
        &ctx.accounts.permissionless_authority.key(),
        &ctx.accounts.token_out_mint.key(),
        &ctx.accounts.token_out_program.key(),
        crate::OnreError::InvalidPermissionlessTokenOutAccount,
    )?;
    let permissionless_token_in_account = get_associated_token_account(
        &ctx.accounts.permissionless_token_in_account,
        &ctx.accounts.permissionless_authority.key(),
        &ctx.accounts.token_in_mint.key(),
        &ctx.accounts.token_in_program.key(),
        crate::OnreError::InvalidVaultTokenInAccount,
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
    let permissionless_offer_fee_token_in_account = get_or_create_configurable_vault_token_account::<
        { ConfigurableVaultKind::PermissionlessOfferFee.as_u8() },
    >(ConfigurableVaultTokenAccountParams {
        vault: &ctx.accounts.permissionless_offer_fee_vault,
        token_account: &ctx.accounts.permissionless_offer_fee_token_in_account,
        payer: ctx.accounts.user.to_account_info(),
        mint_account: ctx.accounts.token_in_mint.to_account_info(),
        token_program: ctx.accounts.token_in_program.to_account_info(),
        associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        program_id: ctx.program_id,
    })?;
    let fee_basis_points_permissionless = {
        let offer = ctx.accounts.offer.load()?;
        offer.permissionless_fee_basis_points()
    };
    execute_take_offer_permissionless(
        ctx.program_id,
        &ctx.accounts.offer,
        &ctx.accounts.state,
        &ctx.accounts.user,
        &ctx.accounts.token_in_mint,
        &mut ctx.accounts.token_out_mint,
        token_in_amount,
        &ctx.accounts.token_in_program,
        &ctx.accounts.user_token_in_account,
        &permissionless_token_in_account,
        &ctx.accounts.permissionless_authority,
        &offer_proceeds_token_in_account,
        &permissionless_offer_fee_token_in_account,
        &ctx.accounts.vault_token_in_account,
        &ctx.accounts.vault_authority,
        &ctx.accounts.token_out_program,
        &ctx.accounts.vault_token_out_account,
        &permissionless_token_out_account,
        &user_token_out_account,
        &ctx.accounts.mint_authority,
        Some(fee_basis_points_permissionless),
        Some(&ctx.accounts.buffer_accounts),
        Some(PermissionlessMarketStatsRefresh {
            market_stats: &ctx.accounts.market_stats,
            circulating_supply_excluded_balance: &ctx.accounts.circulating_supply_excluded_balance,
            main_offer_account: &ctx.accounts.main_offer,
        }),
        Some(PermissionlessRedemptionVaultRefill {
            redemption_offer: &ctx.accounts.redemption_offer,
            redemption_vault_authority: &ctx.accounts.redemption_vault_authority,
            redemption_vault_token_in_account: &ctx.accounts.redemption_vault_token_in_account,
            associated_token_program: &ctx.accounts.associated_token_program,
        }),
        &ctx.accounts.system_program,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_take_offer_permissionless<'info>(
    program_id: &Pubkey,
    offer_account: &AccountLoader<'info, Offer>,
    state: &Account<'info, State>,
    user: &Signer<'info>,
    token_in_mint: &InterfaceAccount<'info, Mint>,
    token_out_mint: &mut InterfaceAccount<'info, Mint>,
    token_in_amount: u64,
    token_in_program: &Interface<'info, TokenInterface>,
    user_token_in_account: &InterfaceAccount<'info, TokenAccount>,
    permissionless_token_in_account: &InterfaceAccount<'info, TokenAccount>,
    permissionless_authority: &UncheckedAccount<'info>,
    token_in_destination_account: &InterfaceAccount<'info, TokenAccount>,
    offer_fee_token_in_account: &InterfaceAccount<'info, TokenAccount>,
    vault_token_in_account: &InterfaceAccount<'info, TokenAccount>,
    vault_authority: &UncheckedAccount<'info>,
    token_out_program: &Interface<'info, TokenInterface>,
    vault_token_out_account: &InterfaceAccount<'info, TokenAccount>,
    permissionless_token_out_account: &InterfaceAccount<'info, TokenAccount>,
    user_token_out_account: &InterfaceAccount<'info, TokenAccount>,
    mint_authority: &UncheckedAccount<'info>,
    fee_basis_points_override: Option<u16>,
    buffer_accounts: Option<&BufferAccrualAccounts<'info>>,
    market_stats_refresh: Option<PermissionlessMarketStatsRefresh<'_, 'info>>,
    redemption_vault_refill: Option<PermissionlessRedemptionVaultRefill<'info, 'info>>,
    system_program: &Program<'info, System>,
) -> Result<()> {
    let (va, va_bump) = Pubkey::find_program_address(&[seeds::OFFER_VAULT_AUTHORITY], program_id);
    require_keys_eq!(va, vault_authority.key());
    let (pa, pa_bump) =
        Pubkey::find_program_address(&[seeds::PERMISSIONLESS_AUTHORITY], program_id);
    require_keys_eq!(pa, permissionless_authority.key());
    let (ma, ma_bump) = Pubkey::find_program_address(&[seeds::MINT_AUTHORITY], program_id);
    require_keys_eq!(ma, mint_authority.key());

    let offer = offer_account.load()?;
    offer.require_enabled()?;
    offer.require_mints(token_in_mint.key(), token_out_mint.key())?;
    // Validate if offer allows permissionless access
    require!(
        offer.allow_permissionless(),
        crate::OnreError::PermissionlessNotAllowed
    );

    let result = process_offer_core(
        &offer,
        token_in_amount,
        token_in_mint,
        token_out_mint,
        fee_basis_points_override.unwrap_or(offer.fee_basis_points),
    )?;
    let buffer_is_initialized = if let Some(accounts) = buffer_accounts {
        accounts.check_is_initialized(program_id)?
    } else {
        false
    };
    let should_accrue = buffer_accounts
        .map(|_| {
            should_accrue_onyc_mint(
                state,
                token_out_mint,
                buffer_is_initialized,
                &mint_authority.to_account_info(),
            )
        })
        .unwrap_or(false);
    let main_offer = if is_onyc_token_out_mint(state, token_out_mint) {
        market_stats_refresh
            .as_ref()
            .map(|refresh| {
                load_main_offer(
                    program_id,
                    &refresh.main_offer_account.to_account_info(),
                    state,
                )
            })
            .transpose()?
    } else {
        None
    };
    let accrual = if should_accrue {
        let buffer_accounts = buffer_accounts.expect("checked above");
        let canonical_offer = main_offer
            .as_ref()
            .ok_or(crate::OnreError::InvalidMainOffer)?;
        Some(accrue_buffer_from_accounts(
            program_id,
            state,
            buffer_accounts,
            canonical_offer,
            token_out_mint,
            mint_authority.to_account_info(),
            ma_bump,
            token_out_program,
        )?)
    } else {
        None
    };

    if accrual.is_some() {
        token_out_mint.reload()?;
    }

    let redemption_vault_token_in_account = if let Some(refill) = redemption_vault_refill.as_ref() {
        let target_bps = load_redemption_offer_vault_target_bps_for_offer(
            program_id,
            refill.redemption_offer,
            offer_account.key(),
            token_in_mint.key(),
            token_out_mint.key(),
        )?
        .unwrap_or(0);
        if target_bps > 0
            && !program_controls_mint(token_in_mint, &mint_authority.to_account_info())
        {
            let account = get_or_create_associated_token_account(EnsureAtaParams {
                ata_account: refill.redemption_vault_token_in_account,
                payer: user.to_account_info(),
                authority_account: refill.redemption_vault_authority.to_account_info(),
                mint_account: token_in_mint.to_account_info(),
                token_program: token_in_program.to_account_info(),
                associated_token_program: refill.associated_token_program.to_account_info(),
                system_program: system_program.to_account_info(),
                authority: refill.redemption_vault_authority.key(),
                mint: token_in_mint.key(),
                token_program_id: token_in_program.key(),
                invalid_account_error: crate::OnreError::InvalidVaultTokenInAccount,
            })?;
            let refill_amount = if let Some(refresh) = market_stats_refresh.as_ref() {
                calculate_redemption_vault_refill_amount(
                    &refresh.market_stats.to_account_info(),
                    target_bps,
                    token_in_mint,
                    token_out_mint,
                    account.amount,
                    result.token_in_net_amount,
                )
            } else {
                0
            };
            Some((account, refill_amount))
        } else {
            None
        }
    } else {
        None
    };

    transfer_tokens(
        token_in_mint,
        token_in_program,
        user_token_in_account,
        permissionless_token_in_account,
        user,
        None,
        token_in_amount,
    )?;
    msg!("Transferred token_in from user to permissionless intermediary");

    execute_token_operations(ExecTokenOpsParams {
        token_in_program,
        token_in_mint,
        token_in_net_amount: result.token_in_net_amount,
        token_in_fee_amount: result.token_in_fee_amount,
        token_in_authority: &permissionless_authority.to_account_info(),
        token_in_source_signer_seeds: Some(&[&[seeds::PERMISSIONLESS_AUTHORITY, &[pa_bump]]]),
        vault_authority_signer_seeds: Some(&[&[seeds::OFFER_VAULT_AUTHORITY, &[va_bump]]]),
        token_in_source_account: permissionless_token_in_account,
        token_in_destination_account,
        token_in_refill_destination_account: redemption_vault_token_in_account
            .as_ref()
            .map(|(account, _)| account),
        token_in_refill_amount: redemption_vault_token_in_account
            .as_ref()
            .map(|(_, amount)| *amount)
            .unwrap_or(0),
        token_in_fee_destination_account: offer_fee_token_in_account,
        token_in_burn_account: vault_token_in_account,
        token_in_burn_authority: &vault_authority.to_account_info(),
        token_out_program,
        token_out_mint,
        token_out_amount: result.token_out_amount,
        token_out_authority: &vault_authority.to_account_info(),
        token_out_source_account: vault_token_out_account,
        token_out_destination_account: permissionless_token_out_account,
        mint_authority_pda: &mint_authority.to_account_info(),
        mint_authority_bump: &[ma_bump],
        token_out_max_supply: state.max_supply,
        token_out_max_mint_amount: state.max_mint_amount,
    })?;

    transfer_tokens(
        token_out_mint,
        token_out_program,
        permissionless_token_out_account,
        user_token_out_account,
        &permissionless_authority.to_account_info(),
        Some(&[&[seeds::PERMISSIONLESS_AUTHORITY, &[pa_bump]]]),
        result.token_out_amount,
    )?;

    if let Some(accrual) = accrual {
        let post_offer_supply = accrual
            .post_accrual_supply
            .checked_add(result.token_out_amount)
            .ok_or(crate::OnreError::OverflowError)?;
        store_buffer_post_supply(
            buffer_accounts.expect("accrual implies buffer accounts"),
            post_offer_supply,
            accrual.timestamp,
        )?;
    }

    if is_onyc_token_out_mint(state, token_out_mint) {
        if let Some(refresh) = market_stats_refresh.as_ref() {
            let main_offer = main_offer
                .as_ref()
                .ok_or(crate::OnreError::InvalidMainOffer)?;
            let (market_stats_pda, _) =
                Pubkey::find_program_address(&[seeds::MARKET_STATS], program_id);
            require_keys_eq!(
                market_stats_pda,
                refresh.market_stats.key(),
                crate::OnreError::InvalidMarketStatsPda
            );
            require!(
                refresh.market_stats.is_writable,
                crate::OnreError::MarketStatsNotWritable
            );
            token_out_mint.reload()?;
            refresh_market_stats_pda(
                main_offer,
                token_out_mint,
                &refresh
                    .circulating_supply_excluded_balance
                    .to_account_info(),
                &refresh.market_stats.to_account_info(),
                &user.to_account_info(),
                &system_program.to_account_info(),
                program_id,
            )?;
        }
    }

    msg!(
        "Offer taken (permissionless) - PDA: {}, token_in(excluding fee): {}, fee: {}, token_out: {}, user: {}, price: {}",
        offer_account.key(),
        result.token_in_net_amount,
        result.token_in_fee_amount,
        result.token_out_amount,
        user.key,
        u64_to_dec9(result.current_price)
    );

    emit!(OfferTakenPermissionlessEvent {
        offer_pda: offer_account.key(),
        token_in_amount: result.token_in_net_amount,
        token_out_amount: result.token_out_amount,
        fee_amount: result.token_in_fee_amount,
        user: user.key(),
    });

    Ok(())
}
