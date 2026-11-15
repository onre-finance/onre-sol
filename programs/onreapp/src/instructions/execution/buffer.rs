use crate::{
    constants::seeds,
    instructions::{
        buffer::{
            accrue_buffer::accrue_buffer, calculate_buffer_fee_split,
            calculate_gross_buffer_accrual, validate_buffer_onyc_vault_accounts, BufferState,
        },
        market_info::load_main_offer,
    },
    model::{seeds as model_seeds, ManagedTokenBufferState, PricingDenomination},
    utils::{
        load_pda_account, mint_tokens, pricing::get_active_vector_and_current_price,
        store_pda_account, validate_max_supply,
    },
    OnreError, ID,
};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;

use super::TakeOffer;

#[event]
pub struct ManagedTokenBufferAccruedEvent {
    pub buffer_state: Pubkey,
    pub managed_mint: Pubkey,
    pub canonical_pricer: Pubkey,
    pub buffer_mint_amount: u64,
    pub reserve_mint_amount: u64,
    pub management_fee_mint_amount: u64,
    pub performance_fee_mint_amount: u64,
    pub previous_supply: u64,
    pub post_accrual_supply: u64,
    pub performance_fee_high_watermark: u64,
    pub timestamp: i64,
}

/// ONyc always uses the existing baseline while V2 remains enabled. Other mints
/// use their per-mint account. Even an absent BUFFER must be supplied at its
/// canonical address so omission cannot bypass an existing accrual engine.
pub fn accrue_before_execution(
    program_id: &Pubkey,
    a: &TakeOffer,
    authority_bump: u8,
    now: i64,
    max_supply: u64,
    max_mint: u64,
) -> Result<bool> {
    let onyc = a.managed_mint().key() == a.state.onyc_mint;
    let expected = if onyc {
        Pubkey::find_program_address(&[seeds::BUFFER_STATE], program_id).0
    } else {
        Pubkey::find_program_address(
            &[model_seeds::BUFFER, a.managed_mint().key().as_ref()],
            program_id,
        )
        .0
    };
    require_keys_eq!(
        a.buffer_state.key(),
        expected,
        OnreError::InvalidBufferStateAccount
    );
    if a.buffer_state.data_is_empty() {
        require_keys_eq!(
            *a.buffer_state.owner,
            System::id(),
            OnreError::InvalidBufferStateAccount
        );
        return Ok(false);
    }
    if onyc {
        let mut buffer: BufferState = load_pda_account(
            &a.buffer_state,
            program_id,
            OnreError::InvalidBufferStateAccount.into(),
            OnreError::InvalidBufferStateAccount.into(),
        )?;
        let main_offer = a.main_offer.as_ref().ok_or(OnreError::InvalidMainOffer)?;
        let offer = load_main_offer(program_id, main_offer, &a.state)?;
        validate_buffer_onyc_vault_accounts(
            program_id,
            &buffer,
            &a.buffer_reserve_token,
            &a.buffer_management_token,
            &a.buffer_performance_token,
            a.managed_mint(),
            a.managed_program(),
        )?;
        let result = accrue_buffer(
            &a.state,
            &mut buffer,
            &offer,
            a.managed_mint(),
            a.buffer_reserve_token.to_account_info(),
            a.buffer_management_token.to_account_info(),
            a.buffer_performance_token.to_account_info(),
            a.mint_authority.to_account_info(),
            authority_bump,
            a.managed_program(),
            now,
        )?;
        validate_max_supply(
            a.managed_mint().supply,
            result.buffer_mint_amount,
            max_supply,
        )?;
        require!(
            max_mint == 0 || result.buffer_mint_amount <= max_mint,
            OnreError::MaxMintAmountExceeded
        );
        store_pda_account(&a.buffer_state, &buffer)?;
    } else {
        accrue_managed_buffer(program_id, a, authority_bump, now, max_supply, max_mint)?;
    }
    Ok(true)
}

fn accrue_managed_buffer(
    program_id: &Pubkey,
    a: &TakeOffer,
    authority_bump: u8,
    now: i64,
    max_supply: u64,
    max_mint: u64,
) -> Result<()> {
    let mut buffer: ManagedTokenBufferState = load_pda_account(
        &a.buffer_state,
        program_id,
        OnreError::InvalidBufferStateAccount.into(),
        OnreError::InvalidBufferStateAccount.into(),
    )?;
    require_keys_eq!(
        buffer.managed_mint,
        a.managed_mint().key(),
        OnreError::InvalidMint
    );
    require!(
        now >= buffer.last_accrual_timestamp,
        OnreError::InvalidTimestamp
    );
    let pricer = a
        .buffer_pricer
        .as_ref()
        .ok_or(OnreError::IncompatiblePricer)?;
    let canonical = Pubkey::find_program_address(
        &[
            model_seeds::PRICER,
            buffer.managed_mint.as_ref(),
            &[PricingDenomination::Usd as u8],
            &0_u64.to_le_bytes(),
        ],
        program_id,
    )
    .0;
    require!(
        pricer.key() == canonical
            && pricer.managed_mint == buffer.managed_mint
            && pricer.denomination == PricingDenomination::Usd
            && pricer.enabled,
        OnreError::IncompatiblePricer
    );
    for (token, authority) in [
        (&a.buffer_reserve_token, buffer.reserve_vault),
        (&a.buffer_management_token, buffer.management_fee_vault),
        (&a.buffer_performance_token, buffer.performance_fee_vault),
    ] {
        require_keys_eq!(
            token.key(),
            get_associated_token_address_with_program_id(
                &authority,
                &buffer.managed_mint,
                &a.managed_program().key()
            ),
            OnreError::InvalidConfigurableVaultTokenAccount
        );
    }
    let (vector, nav) =
        get_active_vector_and_current_price(pricer.vectors.iter().copied(), now as u64)?;
    let previous_supply = buffer.previous_supply;
    let amount = calculate_gross_buffer_accrual(
        previous_supply,
        buffer.gross_apr,
        vector.apr,
        (now - buffer.last_accrual_timestamp) as u64,
    )?;
    let split = calculate_buffer_fee_split(
        amount,
        buffer.gross_apr.saturating_sub(vector.apr),
        buffer.management_fee_basis_points,
        buffer.performance_fee_basis_points,
        nav,
        buffer.performance_fee_high_watermark,
        buffer.performance_fee_high_watermark_enabled,
    )?;
    validate_max_supply(a.managed_mint().supply, amount, max_supply)?;
    require!(
        max_mint == 0 || amount <= max_mint,
        OnreError::MaxMintAmountExceeded
    );
    for (token, amount) in [
        (&a.buffer_reserve_token, split.reserve_mint_amount),
        (&a.buffer_management_token, split.management_fee_mint_amount),
        (
            &a.buffer_performance_token,
            split.performance_fee_mint_amount,
        ),
    ] {
        if amount > 0 {
            mint_tokens(
                a.managed_program(),
                a.managed_mint(),
                token,
                &a.mint_authority,
                &[&[seeds::MINT_AUTHORITY, &[authority_bump]]],
                amount,
                max_supply,
                max_mint,
            )?;
        }
    }
    buffer.performance_fee_high_watermark = if previous_supply == 0 {
        nav
    } else {
        split.new_performance_fee_high_watermark
    };
    buffer.previous_supply = a
        .managed_mint()
        .supply
        .checked_add(amount)
        .ok_or(OnreError::MathOverflow)?;
    buffer.last_accrual_timestamp = now;
    store_pda_account(&a.buffer_state, &buffer)?;
    emit!(ManagedTokenBufferAccruedEvent {
        buffer_state: a.buffer_state.key(),
        managed_mint: buffer.managed_mint,
        canonical_pricer: pricer.key(),
        buffer_mint_amount: amount,
        reserve_mint_amount: split.reserve_mint_amount,
        management_fee_mint_amount: split.management_fee_mint_amount,
        performance_fee_mint_amount: split.performance_fee_mint_amount,
        previous_supply,
        post_accrual_supply: buffer.previous_supply,
        performance_fee_high_watermark: buffer.performance_fee_high_watermark,
        timestamp: now,
    });
    Ok(())
}

pub fn store_post_execution_supply(a: &TakeOffer, now: i64) -> Result<()> {
    if a.managed_mint().key() == a.state.onyc_mint {
        let mut buffer: BufferState = load_pda_account(
            &a.buffer_state,
            &ID,
            OnreError::InvalidBufferStateAccount.into(),
            OnreError::InvalidBufferStateAccount.into(),
        )?;
        buffer.previous_supply = a.managed_mint().supply;
        buffer.last_accrual_timestamp = now;
        store_pda_account(&a.buffer_state, &buffer)
    } else {
        let mut buffer: ManagedTokenBufferState = load_pda_account(
            &a.buffer_state,
            &ID,
            OnreError::InvalidBufferStateAccount.into(),
            OnreError::InvalidBufferStateAccount.into(),
        )?;
        buffer.previous_supply = a.managed_mint().supply;
        buffer.last_accrual_timestamp = now;
        store_pda_account(&a.buffer_state, &buffer)
    }
}
