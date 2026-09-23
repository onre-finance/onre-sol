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
    get_or_create_configurable_vault_token_account_pair, ConfigurableVaultTokenAccountPairParams,
};
use crate::instructions::market_info::{load_main_offer, refresh_market_stats_pda};
use crate::instructions::offer::offer_utils::{
    calculate_redemption_vault_refill_amount, is_onyc_token_out_mint,
    load_redemption_offer_vault_target_bps_for_offer, process_offer_core, should_accrue_onyc_mint,
    verify_offer_approval,
};
use crate::instructions::Offer;
use crate::state::{ConfigurableVaultKind, State};
use crate::utils::{
    execute_token_operations, get_or_create_associated_token_account, program_controls_mint,
    u64_to_dec9, ApprovalMessage, EnsureAtaParams, ExecTokenOpsParams,
};
use anchor_lang::{prelude::*, Accounts};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use solana_instructions_sysvar::ID as INSTRUCTIONS_SYSVAR_ID;

// Shared offer-execution event definitions.

/// Event emitted when an offer is successfully taken
///
/// Provides transparency for tracking offer execution and token exchange details.
#[event]
pub struct OfferTakenEvent {
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
pub struct TakeOfferV2<'info> {
    #[account(
        mut,
        seeds = [
            seeds::OFFER,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref()
        ],
        bump = offer.load()?.bump
    )]
    pub offer: AccountLoader<'info, Offer>,

    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        constraint = !state.is_killed @ crate::OnreError::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,

    /// CHECK: PDA derivation is validated by seeds constraint
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

    #[account(
        mut,
        constraint =
            token_in_mint.key() == offer.load()?.token_in_mint
            @ crate::OnreError::InvalidTokenInMint
    )]
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,

    pub token_in_program: Interface<'info, TokenInterface>,

    #[account(
        mut,
        constraint =
            token_out_mint.key() == offer.load()?.token_out_mint
            @ crate::OnreError::InvalidTokenOutMint
    )]
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
    pub offer_fee_vault: UncheckedAccount<'info>,

    /// CHECK: Validated and optionally initialized in instruction logic.
    #[account(mut)]
    pub offer_fee_token_in_account: UncheckedAccount<'info>,

    /// CHECK: PDA derivation is validated through seeds constraint
    pub mint_authority: UncheckedAccount<'info>,

    pub buffer_accounts: BufferAccrualAccounts<'info>,

    /// CHECK: The handler validates PDA, writability, owner, and account data layout.
    #[account(mut)]
    pub market_stats: UncheckedAccount<'info>,

    /// CHECK: PDA validation and data loading are handled by market stats refresh.
    pub circulating_supply_excluded_balance: UncheckedAccount<'info>,

    /// CHECK: Validated through address constraint to instructions sysvar
    pub instructions_sysvar: UncheckedAccount<'info>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,

    /// CHECK: Validated in instruction logic against state.main_offer.
    pub main_offer: UncheckedAccount<'info>,
}

/// Executes an approval-gated offer with configurable fee/proceeds routing,
/// redemption liquidity refill, and BUFFER accrual for ONyc issuance.
pub fn take_offer_v2<'info>(
    ctx: Context<'info, TakeOfferV2<'info>>,
    token_in_amount: u64,
    approval_message: Option<ApprovalMessage>,
) -> Result<()> {
    execute_take_offer_v2(ctx, token_in_amount, approval_message)
}
pub fn execute_take_offer_v2<'info>(
    ctx: Context<'info, TakeOfferV2<'info>>,
    token_in_amount: u64,
    approval_message: Option<ApprovalMessage>,
) -> Result<()> {
    let (vault_authority_bump, mint_authority_bump) = validate_take_offer_authorities(
        ctx.program_id,
        &ctx.accounts.vault_authority,
        &ctx.accounts.mint_authority,
        &ctx.accounts.instructions_sysvar,
    )?;
    let offer = ctx.accounts.offer.load()?;
    offer.require_enabled()?;
    let buffer_is_initialized = ctx
        .accounts
        .buffer_accounts
        .check_is_initialized(ctx.program_id)?;
    let should_accrue_onyc_mint = should_accrue_onyc_mint(
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

    verify_offer_approval(
        &offer,
        &approval_message,
        ctx.program_id,
        &ctx.accounts.user.key(),
        &ctx.accounts.state.approver1,
        &ctx.accounts.state.approver2,
        &ctx.accounts.instructions_sysvar,
    )?;

    let result = process_offer_core(
        &offer,
        token_in_amount,
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_out_mint,
        offer.fee_basis_points,
    )?;
    let redemption_vault_target_bps = load_redemption_offer_vault_target_bps_for_offer(
        ctx.program_id,
        &ctx.accounts.redemption_offer,
        ctx.accounts.offer.key(),
        ctx.accounts.token_in_mint.key(),
        ctx.accounts.token_out_mint.key(),
    )?
    .unwrap_or(0);
    let (offer_proceeds_token_in_account, offer_fee_token_in_account) =
        get_or_create_configurable_vault_token_account_pair::<
            { ConfigurableVaultKind::OfferProceeds.as_u8() },
            { ConfigurableVaultKind::OfferFee.as_u8() },
        >(ConfigurableVaultTokenAccountPairParams {
            first_vault: &ctx.accounts.offer_proceeds_vault,
            first_token_account: &ctx.accounts.offer_proceeds_token_in_account,
            second_vault: &ctx.accounts.offer_fee_vault,
            second_token_account: &ctx.accounts.offer_fee_token_in_account,
            payer: ctx.accounts.user.to_account_info(),
            mint_account: ctx.accounts.token_in_mint.to_account_info(),
            token_program: ctx.accounts.token_in_program.to_account_info(),
            associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            program_id: ctx.program_id,
        })?;
    let should_refill_redemption_vault = redemption_vault_target_bps > 0
        && !program_controls_mint(
            &ctx.accounts.token_in_mint,
            &ctx.accounts.mint_authority.to_account_info(),
        );
    let redemption_vault_token_in_account = if should_refill_redemption_vault {
        Some(get_or_create_associated_token_account(EnsureAtaParams {
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
        })?)
    } else {
        None
    };
    let redemption_vault_refill_amount = redemption_vault_token_in_account
        .as_ref()
        .map(|account| {
            calculate_redemption_vault_refill_amount(
                &ctx.accounts.market_stats.to_account_info(),
                redemption_vault_target_bps,
                &ctx.accounts.token_in_mint,
                &ctx.accounts.token_out_mint,
                account.amount,
                result.token_in_net_amount,
            )
        })
        .unwrap_or(0);

    get_or_create_associated_token_account(EnsureAtaParams {
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
    let user_token_out_account = InterfaceAccount::try_from(&ctx.accounts.user_token_out_account)?;
    let accrual = if should_accrue_onyc_mint {
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

    execute_token_operations(ExecTokenOpsParams {
        token_in_program: &ctx.accounts.token_in_program,
        token_in_mint: &ctx.accounts.token_in_mint,
        token_in_net_amount: result.token_in_net_amount,
        token_in_fee_amount: result.token_in_fee_amount,
        token_in_authority: &ctx.accounts.user,
        token_in_source_signer_seeds: None,
        vault_authority_signer_seeds: Some(&[&[
            seeds::OFFER_VAULT_AUTHORITY,
            &[vault_authority_bump],
        ]]),
        token_in_source_account: &ctx.accounts.user_token_in_account,
        token_in_destination_account: &offer_proceeds_token_in_account,
        token_in_refill_destination_account: redemption_vault_token_in_account.as_ref(),
        token_in_refill_amount: redemption_vault_refill_amount,
        token_in_fee_destination_account: &offer_fee_token_in_account,
        token_in_burn_account: &ctx.accounts.vault_token_in_account,
        token_in_burn_authority: &ctx.accounts.vault_authority.to_account_info(),
        token_out_program: &ctx.accounts.token_out_program,
        token_out_mint: &ctx.accounts.token_out_mint,
        token_out_amount: result.token_out_amount,
        token_out_authority: &ctx.accounts.vault_authority.to_account_info(),
        token_out_source_account: &ctx.accounts.vault_token_out_account,
        token_out_destination_account: &user_token_out_account,
        mint_authority_pda: &ctx.accounts.mint_authority.to_account_info(),
        mint_authority_bump: &[mint_authority_bump],
        token_out_max_supply: ctx.accounts.state.max_supply,
        token_out_max_mint_amount: ctx.accounts.state.max_mint_amount,
    })?;

    if let Some(accrual) = accrual {
        let post_offer_supply = accrual
            .post_accrual_supply
            .checked_add(result.token_out_amount)
            .ok_or(crate::OnreError::MathOverflow)?;
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
        let (market_stats_pda, _) =
            Pubkey::find_program_address(&[seeds::MARKET_STATS], ctx.program_id);
        require_keys_eq!(
            market_stats_pda,
            ctx.accounts.market_stats.key(),
            crate::OnreError::InvalidMarketStatsPda
        );
        require!(
            ctx.accounts.market_stats.is_writable,
            crate::OnreError::MarketStatsNotWritable
        );
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

    msg!(
        "Offer taken - PDA: {}, token_in(+fee): {}(+{}), token_out: {}, user: {}, price: {}",
        ctx.accounts.offer.key(),
        result.token_in_net_amount,
        result.token_in_fee_amount,
        result.token_out_amount,
        ctx.accounts.user.key,
        u64_to_dec9(result.current_price)
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

pub fn validate_take_offer_authorities(
    program_id: &Pubkey,
    vault_authority: &UncheckedAccount,
    mint_authority: &UncheckedAccount,
    instructions_sysvar: &UncheckedAccount,
) -> Result<(u8, u8)> {
    let (expected_vault_authority, vault_authority_bump) =
        Pubkey::find_program_address(&[seeds::OFFER_VAULT_AUTHORITY], program_id);
    require_keys_eq!(
        expected_vault_authority,
        vault_authority.key(),
        crate::OnreError::InvalidVaultAuthority
    );

    let (expected_mint_authority, mint_authority_bump) =
        Pubkey::find_program_address(&[seeds::MINT_AUTHORITY], program_id);
    require_keys_eq!(
        expected_mint_authority,
        mint_authority.key(),
        crate::OnreError::InvalidMintAuthority
    );

    require_keys_eq!(
        INSTRUCTIONS_SYSVAR_ID,
        instructions_sysvar.key(),
        crate::OnreError::InvalidInstructionsSysvar
    );

    Ok((vault_authority_bump, mint_authority_bump))
}
