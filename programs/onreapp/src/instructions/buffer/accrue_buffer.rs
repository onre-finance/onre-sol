use crate::constants::seeds;
use crate::instructions::buffer::{
    calculate_buffer_fee_split, calculate_gross_buffer_accrual,
    validate_buffer_onyc_vault_accounts, BufferAccrualAccounts, BufferAccruedEvent,
    BufferErrorCode, BufferState,
};
use crate::instructions::market_info::offer_valuation_utils::get_active_vector_and_current_price;
use crate::instructions::Offer;
use crate::state::State;
use crate::utils::token_utils::{
    mint_tokens, read_optional_token_account_amount, TokenUtilsErrorCode,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};

pub(crate) struct BufferAccrualResult {
    pub seconds_elapsed: u64,
    pub apr_delta: u64,
    pub buffer_mint_amount: u64,
    pub reserve_mint_amount: u64,
    pub management_fee_mint_amount: u64,
    pub performance_fee_mint_amount: u64,
    pub old_previous_supply: u64,
    pub new_previous_supply: u64,
    pub old_previous_performance_fee_high_watermark: u64,
    pub new_performance_fee_high_watermark: u64,
    pub timestamp: i64,
    pub current_nav: u64,
    pub post_accrual_supply: u64,
    pub reserve_vault_balance_after_accrual: u64,
}

pub(crate) fn accrue_buffer<'info>(
    state: &Account<'info, State>,
    buffer_state: &mut BufferState,
    offer: &Offer,
    onyc_mint: &InterfaceAccount<'info, Mint>,
    reserve_vault_onyc_account: AccountInfo<'info>,
    management_fee_vault_onyc_account: AccountInfo<'info>,
    performance_fee_vault_onyc_account: AccountInfo<'info>,
    mint_authority: AccountInfo<'info>,
    mint_authority_bump: u8,
    token_program: &Interface<'info, TokenInterface>,
    now: i64,
) -> Result<BufferAccrualResult> {
    require!(
        now >= buffer_state.last_accrual_timestamp,
        BufferErrorCode::InvalidTimestamp
    );

    let (active_vector, current_nav) = get_active_vector_and_current_price(offer, now as u64)?;
    let seconds_elapsed = (now - buffer_state.last_accrual_timestamp) as u64;
    let apr_delta = buffer_state.gross_apr.saturating_sub(active_vector.apr);
    let old_previous_supply = buffer_state.previous_supply;
    let current_supply_before_mint = onyc_mint.supply;
    let reserve_vault_balance_before_mint =
        read_optional_token_account_amount(&reserve_vault_onyc_account, token_program)?;
    let old_previous_performance_fee_high_watermark = buffer_state.performance_fee_high_watermark;

    if old_previous_supply == 0 {
        set_buffer_baseline_after_supply_change(buffer_state, current_supply_before_mint, now);

        let result = BufferAccrualResult {
            seconds_elapsed,
            apr_delta,
            buffer_mint_amount: 0,
            reserve_mint_amount: 0,
            management_fee_mint_amount: 0,
            performance_fee_mint_amount: 0,
            old_previous_supply,
            new_previous_supply: buffer_state.previous_supply,
            old_previous_performance_fee_high_watermark,
            new_performance_fee_high_watermark: buffer_state.performance_fee_high_watermark,
            timestamp: now,
            current_nav,
            post_accrual_supply: buffer_state.previous_supply,
            reserve_vault_balance_after_accrual: reserve_vault_balance_before_mint,
        };
        emit_buffer_accrued_event(offer, onyc_mint, &result);

        return Ok(result);
    }

    let buffer_mint_amount = calculate_gross_buffer_accrual(
        old_previous_supply,
        buffer_state.gross_apr,
        active_vector.apr,
        seconds_elapsed,
    )?;
    let fee_split = calculate_buffer_fee_split(
        buffer_mint_amount,
        apr_delta,
        buffer_state.management_fee_basis_points,
        buffer_state.performance_fee_basis_points,
        current_nav,
        old_previous_performance_fee_high_watermark,
    )?;

    if buffer_mint_amount > 0 {
        if state.max_supply > 0 {
            let new_supply = current_supply_before_mint
                .checked_add(buffer_mint_amount)
                .ok_or(BufferErrorCode::MathOverflow)?;
            require!(
                new_supply <= state.max_supply,
                TokenUtilsErrorCode::MaxSupplyExceeded
            );
        }

        let mint_authority_seeds = &[seeds::MINT_AUTHORITY, &[mint_authority_bump]];
        let mint_authority_signer_seeds = &[mint_authority_seeds.as_slice()];

        if fee_split.reserve_mint_amount > 0 {
            mint_tokens(
                token_program,
                onyc_mint,
                &reserve_vault_onyc_account,
                &mint_authority,
                mint_authority_signer_seeds,
                fee_split.reserve_mint_amount,
                state.max_supply,
            )?;
        }

        if fee_split.management_fee_mint_amount > 0 {
            mint_tokens(
                token_program,
                onyc_mint,
                &management_fee_vault_onyc_account,
                &mint_authority,
                mint_authority_signer_seeds,
                fee_split.management_fee_mint_amount,
                state.max_supply,
            )?;
        }

        if fee_split.performance_fee_mint_amount > 0 {
            mint_tokens(
                token_program,
                onyc_mint,
                &performance_fee_vault_onyc_account,
                &mint_authority,
                mint_authority_signer_seeds,
                fee_split.performance_fee_mint_amount,
                state.max_supply,
            )?;
        }
    }

    let post_accrual_supply = current_supply_before_mint
        .checked_add(buffer_mint_amount)
        .ok_or(BufferErrorCode::MathOverflow)?;
    let reserve_vault_balance_after_accrual = reserve_vault_balance_before_mint
        .checked_add(fee_split.reserve_mint_amount)
        .ok_or(BufferErrorCode::MathOverflow)?;

    buffer_state.performance_fee_high_watermark = fee_split.new_performance_fee_high_watermark;
    set_buffer_baseline_after_supply_change(buffer_state, post_accrual_supply, now);

    let result = BufferAccrualResult {
        seconds_elapsed,
        apr_delta,
        buffer_mint_amount: fee_split.buffer_mint_amount,
        reserve_mint_amount: fee_split.reserve_mint_amount,
        management_fee_mint_amount: fee_split.management_fee_mint_amount,
        performance_fee_mint_amount: fee_split.performance_fee_mint_amount,
        old_previous_supply,
        new_previous_supply: buffer_state.previous_supply,
        old_previous_performance_fee_high_watermark,
        new_performance_fee_high_watermark: buffer_state.performance_fee_high_watermark,
        timestamp: now,
        current_nav,
        post_accrual_supply,
        reserve_vault_balance_after_accrual,
    };
    emit_buffer_accrued_event(offer, onyc_mint, &result);

    Ok(result)
}

pub(crate) fn accrue_buffer_from_accounts<'info>(
    program_id: &Pubkey,
    state: &Account<'info, State>,
    buffer_accounts: &BufferAccrualAccounts<'info>,
    offer: &Offer,
    onyc_mint: &InterfaceAccount<'info, Mint>,
    mint_authority: AccountInfo<'info>,
    mint_authority_bump: u8,
    token_program: &Interface<'info, TokenInterface>,
) -> Result<BufferAccrualResult> {
    let now = Clock::get()?.unix_timestamp;
    let mut buffer_state = buffer_accounts.load_buffer_state()?;
    let reserve_vault_onyc_account = buffer_accounts.reserve_vault_onyc_account_info();
    let management_fee_vault_onyc_account =
        buffer_accounts.management_fee_vault_onyc_account_info();
    let performance_fee_vault_onyc_account =
        buffer_accounts.performance_fee_vault_onyc_account_info();

    validate_buffer_onyc_vault_accounts(
        program_id,
        &buffer_state,
        &reserve_vault_onyc_account,
        &management_fee_vault_onyc_account,
        &performance_fee_vault_onyc_account,
        onyc_mint,
        token_program,
    )?;

    let result = accrue_buffer(
        state,
        &mut buffer_state,
        offer,
        onyc_mint,
        reserve_vault_onyc_account,
        management_fee_vault_onyc_account,
        performance_fee_vault_onyc_account,
        mint_authority,
        mint_authority_bump,
        token_program,
        now,
    )?;

    buffer_accounts.store_buffer_state(&buffer_state)?;

    Ok(result)
}

pub(crate) fn set_buffer_baseline_after_supply_change(
    buffer_state: &mut BufferState,
    post_change_supply: u64,
    now: i64,
) {
    buffer_state.previous_supply = post_change_supply;
    buffer_state.last_accrual_timestamp = now;
}

pub(crate) fn store_buffer_post_supply(
    buffer_accounts: &BufferAccrualAccounts,
    post_change_supply: u64,
    now: i64,
) -> Result<()> {
    let mut buffer_state = buffer_accounts.load_buffer_state()?;
    set_buffer_baseline_after_supply_change(&mut buffer_state, post_change_supply, now);
    buffer_accounts.store_buffer_state(&buffer_state)
}

fn emit_buffer_accrued_event(
    offer: &Offer,
    onyc_mint: &InterfaceAccount<'_, Mint>,
    result: &BufferAccrualResult,
) {
    emit!(BufferAccruedEvent {
        token_in_mint: offer.token_in_mint,
        onyc_mint: onyc_mint.key(),
        seconds_elapsed: result.seconds_elapsed,
        apr_delta: result.apr_delta,
        buffer_mint_amount: result.buffer_mint_amount,
        reserve_mint_amount: result.reserve_mint_amount,
        management_fee_mint_amount: result.management_fee_mint_amount,
        performance_fee_mint_amount: result.performance_fee_mint_amount,
        old_previous_supply: result.old_previous_supply,
        new_previous_supply: result.new_previous_supply,
        old_previous_performance_fee_high_watermark: result
            .old_previous_performance_fee_high_watermark,
        new_performance_fee_high_watermark: result.new_performance_fee_high_watermark,
        current_nav: result.current_nav,
        post_accrual_supply: result.post_accrual_supply,
        timestamp: result.timestamp,
    });
}
