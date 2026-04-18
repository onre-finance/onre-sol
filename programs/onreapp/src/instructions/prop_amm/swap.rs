use crate::constants::seeds;
use crate::instructions::buffer::accounts::{
    BufferAccrualAccountsBumps, __client_accounts_buffer_accrual_accounts,
    __cpi_client_accounts_buffer_accrual_accounts,
};
use crate::instructions::buffer::{
    accrue_buffer::{accrue_buffer_from_accounts, store_buffer_post_supply},
    BufferAccrualAccounts,
};
use crate::instructions::market_info::{load_main_offer, refresh_market_stats_pda};
use crate::instructions::offer::{
    is_onyc_token_out_mint, process_offer_core, should_accrue_onyc_mint,
    validate_take_offer_authorities, verify_offer_approval, OfferTakenEvent,
};
use crate::instructions::redemption::{
    execute_redemption_operations, process_redemption_core, ExecuteRedemptionOpsParams,
};
use crate::instructions::Offer;
use crate::state::State;
use crate::utils::{
    execute_token_operations, get_associated_token_account, get_or_create_associated_token_account,
    transfer_tokens, u64_to_dec9, ApprovalMessage, EnsureAtaParams, ExecTokenOpsParams,
};
use anchor_lang::solana_program::program::set_return_data;
use anchor_lang::{prelude::*, Accounts, AnchorDeserialize, AnchorSerialize};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenInterface},
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwapSide {
    Buy,
    Sell,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct SwapQuote {
    pub offer: Pubkey,
    pub token_in_mint: Pubkey,
    pub token_out_mint: Pubkey,
    pub token_in_amount: u64,
    pub token_in_net_amount: u64,
    pub token_in_fee_amount: u64,
    pub token_out_amount: u64,
    pub minimum_out: u64,
    pub current_price: u64,
    pub quoted_at: i64,
    pub quote_expiry: i64,
}

#[derive(Accounts)]
pub struct QuoteSwap<'info> {
    pub offer: AccountLoader<'info, Offer>,

    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        constraint = state.is_killed == false @ crate::OnreError::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,

    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_out_mint: Box<InterfaceAccount<'info, Mint>>,
}

#[derive(Accounts)]
pub struct OpenSwap<'info> {
    pub offer: AccountLoader<'info, Offer>,

    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss @ crate::OnreError::InvalidBoss,
        constraint = state.is_killed == false @ crate::OnreError::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,

    /// CHECK: validated through state.has_one
    pub boss: UncheckedAccount<'info>,

    /// CHECK: PDA derivation validated in instruction logic
    pub offer_vault_authority: UncheckedAccount<'info>,

    /// CHECK: validated as canonical ATA in instruction logic
    #[account(mut)]
    pub offer_vault_token_in_account: UncheckedAccount<'info>,

    /// CHECK: validated as canonical ATA in instruction logic
    #[account(mut)]
    pub offer_vault_token_out_account: UncheckedAccount<'info>,

    /// CHECK: PDA derivation validated by seeds constraint
    #[account(seeds = [seeds::REDEMPTION_OFFER_VAULT_AUTHORITY], bump)]
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

    /// CHECK: validated as canonical ATA in instruction logic
    #[account(mut)]
    pub boss_token_in_account: UncheckedAccount<'info>,

    /// CHECK: PDA derivation validated in instruction logic
    pub mint_authority: UncheckedAccount<'info>,

    pub buffer_accounts: BufferAccrualAccounts<'info>,

    /// CHECK: validated in instruction logic
    #[account(mut)]
    pub market_stats: UncheckedAccount<'info>,

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

pub fn validate_quote_expiry(current_time: i64, quote_expiry: i64) -> Result<()> {
    require!(quote_expiry >= current_time, crate::OnreError::QuoteExpired);

    let max_expiry = current_time
        .checked_add(crate::constants::MAX_QUOTE_LIFETIME_SECONDS)
        .ok_or(crate::OnreError::MathOverflow)?;

    require!(
        quote_expiry <= max_expiry,
        crate::OnreError::QuoteExpiryTooLarge
    );

    Ok(())
}

fn resolve_swap_side(
    state: &State,
    token_in_mint: Pubkey,
    token_out_mint: Pubkey,
) -> Result<(SwapSide, Pubkey)> {
    require!(
        token_in_mint != token_out_mint,
        crate::OnreError::InvalidSwapPair
    );

    if token_out_mint == state.onyc_mint && token_in_mint != state.onyc_mint {
        return Ok((SwapSide::Buy, token_in_mint));
    }

    if token_in_mint == state.onyc_mint && token_out_mint != state.onyc_mint {
        return Ok((SwapSide::Sell, token_out_mint));
    }

    Err(error!(crate::OnreError::InvalidSwapPair))
}

fn validate_canonical_offer(
    program_id: &Pubkey,
    state: &State,
    offer_key: Pubkey,
    token_in_mint: Pubkey,
    token_out_mint: Pubkey,
) -> Result<SwapSide> {
    let (side, asset_mint) = resolve_swap_side(state, token_in_mint, token_out_mint)?;
    let (expected_offer, _) = Pubkey::find_program_address(
        &[seeds::OFFER, asset_mint.as_ref(), state.onyc_mint.as_ref()],
        program_id,
    );
    require_keys_eq!(offer_key, expected_offer, crate::OnreError::OfferMismatch);
    Ok(side)
}

pub fn build_swap_quote(
    program_id: &Pubkey,
    state: &State,
    offer_key: Pubkey,
    offer: &Offer,
    token_in_amount: u64,
    token_in_mint: &InterfaceAccount<Mint>,
    token_out_mint: &InterfaceAccount<Mint>,
    quote_expiry: i64,
) -> Result<SwapQuote> {
    let quoted_at = Clock::get()?.unix_timestamp;
    validate_quote_expiry(quoted_at, quote_expiry)?;

    let side = validate_canonical_offer(
        program_id,
        state,
        offer_key,
        token_in_mint.key(),
        token_out_mint.key(),
    )?;

    let result = match side {
        SwapSide::Buy => process_offer_core(offer, token_in_amount, token_in_mint, token_out_mint)
            .map(|result| {
                (
                    result.current_price,
                    result.token_in_net_amount,
                    result.token_in_fee_amount,
                    result.token_out_amount,
                )
            })?,
        SwapSide::Sell => {
            process_redemption_core(offer, token_in_amount, token_in_mint, token_out_mint, 0).map(
                |result| {
                    (
                        result.price,
                        result.token_in_net_amount,
                        result.token_in_fee_amount,
                        result.token_out_amount,
                    )
                },
            )?
        }
    };

    Ok(SwapQuote {
        offer: offer_key,
        token_in_mint: token_in_mint.key(),
        token_out_mint: token_out_mint.key(),
        token_in_amount,
        token_in_net_amount: result.1,
        token_in_fee_amount: result.2,
        token_out_amount: result.3,
        minimum_out: result.3,
        current_price: result.0,
        quoted_at,
        quote_expiry,
    })
}

pub fn quote_swap(ctx: Context<QuoteSwap>, token_in_amount: u64, quote_expiry: i64) -> Result<()> {
    let offer = ctx.accounts.offer.load()?;
    let quote = build_swap_quote(
        ctx.program_id,
        &ctx.accounts.state,
        ctx.accounts.offer.key(),
        &offer,
        token_in_amount,
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_out_mint,
        quote_expiry,
    )?;
    let mut serialized_quote = Vec::new();
    quote.serialize(&mut serialized_quote)?;
    set_return_data(&serialized_quote);

    msg!(
        "Swap quote - offer: {}, token_in: {}, minimum_out: {}, expiry: {}",
        quote.offer,
        quote.token_in_amount,
        quote.minimum_out,
        quote.quote_expiry
    );

    Ok(())
}

pub fn open_swap<'info>(
    ctx: Context<'info, OpenSwap<'info>>,
    token_in_amount: u64,
    minimum_out: u64,
    quote_expiry: i64,
    approval_message: Option<ApprovalMessage>,
) -> Result<()> {
    let side = validate_canonical_offer(
        ctx.program_id,
        &ctx.accounts.state,
        ctx.accounts.offer.key(),
        ctx.accounts.token_in_mint.key(),
        ctx.accounts.token_out_mint.key(),
    )?;

    match side {
        SwapSide::Buy => execute_open_swap_buy(
            ctx,
            token_in_amount,
            minimum_out,
            quote_expiry,
            approval_message,
        ),
        SwapSide::Sell => execute_open_swap_sell(
            ctx,
            token_in_amount,
            minimum_out,
            quote_expiry,
            approval_message,
        ),
    }
}

fn execute_open_swap_buy<'info>(
    ctx: Context<'info, OpenSwap<'info>>,
    token_in_amount: u64,
    minimum_out: u64,
    quote_expiry: i64,
    approval_message: Option<ApprovalMessage>,
) -> Result<()> {
    let (vault_authority_bump, mint_authority_bump) = validate_take_offer_authorities(
        ctx.program_id,
        &ctx.accounts.offer_vault_authority,
        &ctx.accounts.mint_authority,
        &ctx.accounts.instructions_sysvar,
    )?;
    let offer = ctx.accounts.offer.load()?;
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

    verify_offer_approval(
        &offer,
        &approval_message,
        ctx.program_id,
        &ctx.accounts.user.key(),
        &ctx.accounts.state.approver1,
        &ctx.accounts.state.approver2,
        &ctx.accounts.instructions_sysvar,
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
    let boss_token_in_account = get_associated_token_account(
        &ctx.accounts.boss_token_in_account,
        &ctx.accounts.boss.key(),
        &ctx.accounts.token_in_mint.key(),
        &ctx.accounts.token_in_program.key(),
        crate::OnreError::InvalidBossTokenInAccount,
    )?;
    let offer_vault_token_in_account = get_associated_token_account(
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

    let result = process_offer_core(
        &offer,
        token_in_amount,
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_out_mint,
    )?;
    validate_quote_expiry(Clock::get()?.unix_timestamp, quote_expiry)?;
    require!(
        result.token_out_amount >= minimum_out,
        crate::OnreError::MinimumOutNotMet
    );

    let accrual = if should_accrue_onyc_mint {
        Some(accrue_buffer_from_accounts(
            ctx.program_id,
            &ctx.accounts.state,
            &ctx.accounts.buffer_accounts,
            &offer,
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
        token_in_authority: &ctx.accounts.user.to_account_info(),
        token_in_source_signer_seeds: None,
        vault_authority_signer_seeds: Some(&[&[
            seeds::OFFER_VAULT_AUTHORITY,
            &[vault_authority_bump],
        ]]),
        token_in_source_account: &user_token_in_account,
        token_in_destination_account: &boss_token_in_account,
        token_in_burn_account: &offer_vault_token_in_account,
        token_in_burn_authority: &ctx.accounts.offer_vault_authority.to_account_info(),
        token_out_program: &ctx.accounts.token_out_program,
        token_out_mint: &ctx.accounts.token_out_mint,
        token_out_amount: result.token_out_amount,
        token_out_authority: &ctx.accounts.offer_vault_authority.to_account_info(),
        token_out_source_account: &offer_vault_token_out_account,
        token_out_destination_account: &user_token_out_account,
        mint_authority_pda: &ctx.accounts.mint_authority.to_account_info(),
        mint_authority_bump: &[mint_authority_bump],
        token_out_max_supply: ctx.accounts.state.max_supply,
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

    refresh_market_stats_for_swap(
        ctx.program_id,
        &ctx.accounts.state,
        &ctx.accounts.main_offer,
        &ctx.accounts.token_out_mint,
        &offer_vault_token_out_account.to_account_info(),
        &ctx.accounts.token_out_program,
        &ctx.accounts.market_stats,
        &ctx.accounts.user.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
    )?;

    msg!(
        "Open swap buy - offer: {}, token_in(+fee): {}(+{}), token_out: {}, user: {}, price: {}",
        ctx.accounts.offer.key(),
        result.token_in_net_amount,
        result.token_in_fee_amount,
        result.token_out_amount,
        ctx.accounts.user.key(),
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

fn execute_open_swap_sell<'info>(
    ctx: Context<'info, OpenSwap<'info>>,
    token_in_amount: u64,
    minimum_out: u64,
    quote_expiry: i64,
    approval_message: Option<ApprovalMessage>,
) -> Result<()> {
    let (_, mint_authority_bump) = validate_take_offer_authorities(
        ctx.program_id,
        &ctx.accounts.offer_vault_authority,
        &ctx.accounts.mint_authority,
        &ctx.accounts.instructions_sysvar,
    )?;
    let offer = ctx.accounts.offer.load()?;

    verify_offer_approval(
        &offer,
        &approval_message,
        ctx.program_id,
        &ctx.accounts.user.key(),
        &ctx.accounts.state.approver1,
        &ctx.accounts.state.approver2,
        &ctx.accounts.instructions_sysvar,
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
    let boss_token_in_account = get_or_create_associated_token_account(EnsureAtaParams {
        ata_account: &ctx.accounts.boss_token_in_account,
        payer: ctx.accounts.user.to_account_info(),
        authority_account: ctx.accounts.boss.to_account_info(),
        mint_account: ctx.accounts.token_in_mint.to_account_info(),
        token_program: ctx.accounts.token_in_program.to_account_info(),
        associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        authority: ctx.accounts.boss.key(),
        mint: ctx.accounts.token_in_mint.key(),
        token_program_id: ctx.accounts.token_in_program.key(),
        invalid_account_error: crate::OnreError::InvalidBossTokenInAccount,
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
    let redemption_vault_token_out_account = get_associated_token_account(
        &ctx.accounts.redemption_vault_token_out_account,
        &ctx.accounts.redemption_vault_authority.key(),
        &ctx.accounts.token_out_mint.key(),
        &ctx.accounts.token_out_program.key(),
        crate::OnreError::InvalidVaultTokenOutAccount,
    )?;
    let offer_vault_onyc_account = get_associated_token_account(
        &ctx.accounts.offer_vault_onyc_account,
        &ctx.accounts.offer_vault_authority.key(),
        &ctx.accounts.state.onyc_mint,
        &ctx.accounts.token_in_program.key(),
        crate::OnreError::InvalidOfferVaultOnycAccount,
    )?;

    let result = process_redemption_core(
        &offer,
        token_in_amount,
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_out_mint,
        0,
    )?;
    validate_quote_expiry(Clock::get()?.unix_timestamp, quote_expiry)?;
    require!(
        result.token_out_amount >= minimum_out,
        crate::OnreError::MinimumOutNotMet
    );

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
        boss_token_in_account: &boss_token_in_account,
        fee_destination_token_in_account: &boss_token_in_account,
        redemption_vault_authority: &ctx.accounts.redemption_vault_authority.to_account_info(),
        redemption_vault_authority_bump: ctx.bumps.redemption_vault_authority,
        token_out_mint: &ctx.accounts.token_out_mint,
        token_out_amount: result.token_out_amount,
        vault_token_out_account: &redemption_vault_token_out_account,
        user_token_out_account: &user_token_out_account,
        mint_authority_pda: &ctx.accounts.mint_authority.to_account_info(),
        mint_authority_bump,
        token_out_max_supply: ctx.accounts.state.max_supply,
    })?;

    let main_offer = load_main_offer(
        ctx.program_id,
        &ctx.accounts.main_offer.to_account_info(),
        &ctx.accounts.state,
    )?;
    refresh_market_stats_pda(
        &main_offer,
        &ctx.accounts.token_in_mint,
        &offer_vault_onyc_account.to_account_info(),
        &ctx.accounts.token_in_program,
        &ctx.accounts.market_stats.to_account_info(),
        &ctx.accounts.user.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        ctx.program_id,
    )?;

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

fn refresh_market_stats_for_swap<'info>(
    program_id: &Pubkey,
    state: &Account<'info, State>,
    main_offer_account: &UncheckedAccount<'info>,
    onyc_mint: &InterfaceAccount<'info, Mint>,
    onyc_vault_account: &AccountInfo<'info>,
    token_program: &Interface<'info, TokenInterface>,
    market_stats: &UncheckedAccount<'info>,
    payer: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
) -> Result<()> {
    if is_onyc_token_out_mint(state, onyc_mint) {
        let main_offer = load_main_offer(program_id, &main_offer_account.to_account_info(), state)?;
        refresh_market_stats_pda(
            &main_offer,
            onyc_mint,
            onyc_vault_account,
            token_program,
            &market_stats.to_account_info(),
            payer,
            system_program,
            program_id,
        )?;
    }

    Ok(())
}
