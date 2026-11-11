use crate::{
    constants::seeds,
    instructions::{
        buffer::{
            accrue_buffer::accrue_buffer, calculate_buffer_fee_split,
            calculate_gross_buffer_accrual, validate_buffer_onyc_vault_accounts, BufferState,
        },
        market_info::load_main_offer,
    },
    model::{seeds as model_seeds, BufferSettings, ManagedTokenBufferState, PricingDenomination},
    utils::{
        load_pda_account, mint_tokens, pricing::get_active_vector_and_current_price,
        store_pda_account, validate_max_supply,
    },
    OnreError, ID,
};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;

use crate::{model::Pricer, state::State};
use anchor_spl::token_interface::{Mint, TokenInterface};

/// Shared account view for modular trades and standalone BUFFER operations.
/// ONyc resolves to the legacy singleton during coexistence.
pub(crate) struct BufferContext<'a, 'info> {
    pub state: &'a Account<'info, State>,
    pub managed_mint: &'a InterfaceAccount<'info, Mint>,
    pub managed_program: &'a Interface<'info, TokenInterface>,
    pub buffer_state: &'a UncheckedAccount<'info>,
    pub buffer_pricer: Option<&'a Account<'info, Pricer>>,
    pub main_offer: Option<&'a UncheckedAccount<'info>>,
    pub reserve_token: &'a UncheckedAccount<'info>,
    pub management_token: &'a UncheckedAccount<'info>,
    pub performance_token: &'a UncheckedAccount<'info>,
    pub mint_authority: &'a UncheckedAccount<'info>,
}

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
pub(crate) fn accrue(
    program_id: &Pubkey,
    a: &BufferContext,
    authority_bump: u8,
    now: i64,
    max_supply: u64,
    max_mint: u64,
) -> Result<bool> {
    let legacy = uses_legacy(program_id, a)?;
    if a.buffer_state.data_is_empty() {
        require_keys_eq!(
            *a.buffer_state.owner,
            System::id(),
            OnreError::InvalidBufferStateAccount
        );
        return Ok(false);
    }
    if legacy {
        let mut buffer: BufferState = load_pda_account(
            &a.buffer_state,
            program_id,
            OnreError::InvalidBufferStateAccount.into(),
            OnreError::InvalidBufferStateAccount.into(),
        )?;
        let main_offer = a.main_offer.ok_or(OnreError::InvalidMainOffer)?;
        let offer = load_main_offer(program_id, main_offer, &a.state)?;
        validate_buffer_onyc_vault_accounts(
            program_id,
            &buffer,
            &a.reserve_token,
            &a.management_token,
            &a.performance_token,
            a.managed_mint,
            a.managed_program,
        )?;
        let result = accrue_buffer(
            &a.state,
            &mut buffer,
            &offer,
            a.managed_mint,
            a.reserve_token.to_account_info(),
            a.management_token.to_account_info(),
            a.performance_token.to_account_info(),
            a.mint_authority.to_account_info(),
            authority_bump,
            a.managed_program,
            now,
        )?;
        validate_max_supply(a.managed_mint.supply, result.buffer_mint_amount, max_supply)?;
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
    a: &BufferContext,
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
        a.managed_mint.key(),
        OnreError::InvalidMint
    );
    require!(
        now >= buffer.last_accrual_timestamp,
        OnreError::InvalidTimestamp
    );
    let pricer = a.buffer_pricer.ok_or(OnreError::IncompatiblePricer)?;
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
        (&a.reserve_token, buffer.reserve_vault),
        (&a.management_token, buffer.management_fee_vault),
        (&a.performance_token, buffer.performance_fee_vault),
    ] {
        require_keys_eq!(
            token.key(),
            get_associated_token_address_with_program_id(
                &authority,
                &buffer.managed_mint,
                &a.managed_program.key()
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
    validate_max_supply(a.managed_mint.supply, amount, max_supply)?;
    require!(
        max_mint == 0 || amount <= max_mint,
        OnreError::MaxMintAmountExceeded
    );
    for (token, amount) in [
        (&a.reserve_token, split.reserve_mint_amount),
        (&a.management_token, split.management_fee_mint_amount),
        (&a.performance_token, split.performance_fee_mint_amount),
    ] {
        if amount > 0 {
            mint_tokens(
                a.managed_program,
                a.managed_mint,
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
        .managed_mint
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

pub(crate) fn store_post_supply(a: &BufferContext, now: i64) -> Result<()> {
    if uses_legacy(&ID, a)? {
        let mut buffer: BufferState = load_pda_account(
            &a.buffer_state,
            &ID,
            OnreError::InvalidBufferStateAccount.into(),
            OnreError::InvalidBufferStateAccount.into(),
        )?;
        buffer.previous_supply = a.managed_mint.supply;
        buffer.last_accrual_timestamp = now;
        store_pda_account(&a.buffer_state, &buffer)
    } else {
        let mut buffer: ManagedTokenBufferState = load_pda_account(
            &a.buffer_state,
            &ID,
            OnreError::InvalidBufferStateAccount.into(),
            OnreError::InvalidBufferStateAccount.into(),
        )?;
        buffer.previous_supply = a.managed_mint.supply;
        buffer.last_accrual_timestamp = now;
        store_pda_account(&a.buffer_state, &buffer)
    }
}

/// Settings are read from the authoritative state, never from an inactive ONyc
/// per-mint draft. Account identity is checked even for configuration-only reads.
pub(crate) fn read_settings(program_id: &Pubkey, a: &BufferContext) -> Result<BufferSettings> {
    if uses_legacy(program_id, a)? {
        let b: BufferState = load_pda_account(
            a.buffer_state,
            program_id,
            OnreError::InvalidBufferStateAccount.into(),
            OnreError::InvalidBufferStateAccount.into(),
        )?;
        require_keys_eq!(b.onyc_mint, a.managed_mint.key(), OnreError::InvalidMint);
        Ok(BufferSettings {
            gross_apr: b.gross_apr,
            management_fee_basis_points: b.management_fee_basis_points,
            performance_fee_basis_points: b.performance_fee_basis_points,
            performance_fee_high_watermark_enabled: b.performance_fee_high_watermark_enabled,
        })
    } else {
        let b: ManagedTokenBufferState = load_pda_account(
            a.buffer_state,
            program_id,
            OnreError::InvalidBufferStateAccount.into(),
            OnreError::InvalidBufferStateAccount.into(),
        )?;
        require_keys_eq!(b.managed_mint, a.managed_mint.key(), OnreError::InvalidMint);
        Ok(BufferSettings {
            gross_apr: b.gross_apr,
            management_fee_basis_points: b.management_fee_basis_points,
            performance_fee_basis_points: b.performance_fee_basis_points,
            performance_fee_high_watermark_enabled: b.performance_fee_high_watermark_enabled,
        })
    }
}

/// Call only after accruing under the old settings. Preserve the newly committed
/// supply baseline, timestamp and watermark when applying the next interval's policy.
pub(crate) fn write_settings(
    program_id: &Pubkey,
    a: &BufferContext,
    settings: BufferSettings,
) -> Result<()> {
    read_settings(program_id, a)?;
    if uses_legacy(program_id, a)? {
        let mut b: BufferState = load_pda_account(
            a.buffer_state,
            program_id,
            OnreError::InvalidBufferStateAccount.into(),
            OnreError::InvalidBufferStateAccount.into(),
        )?;
        b.gross_apr = settings.gross_apr;
        b.management_fee_basis_points = settings.management_fee_basis_points;
        b.performance_fee_basis_points = settings.performance_fee_basis_points;
        b.performance_fee_high_watermark_enabled = settings.performance_fee_high_watermark_enabled;
        store_pda_account(a.buffer_state, &b)
    } else {
        let mut b: ManagedTokenBufferState = load_pda_account(
            a.buffer_state,
            program_id,
            OnreError::InvalidBufferStateAccount.into(),
            OnreError::InvalidBufferStateAccount.into(),
        )?;
        b.gross_apr = settings.gross_apr;
        b.management_fee_basis_points = settings.management_fee_basis_points;
        b.performance_fee_basis_points = settings.performance_fee_basis_points;
        b.performance_fee_high_watermark_enabled = settings.performance_fee_high_watermark_enabled;
        store_pda_account(a.buffer_state, &b)
    }
}

/// Global canonical state selects the ONyc route; supplying another account can
/// never choose an inactive clock or bypass an initialized BUFFER.
fn uses_legacy(program_id: &Pubkey, a: &BufferContext) -> Result<bool> {
    let onyc = a.managed_mint.key() == a.state.onyc_mint;
    let legacy = onyc && !a.state.modular_buffer_active();
    let expected = if legacy {
        Pubkey::find_program_address(&[seeds::BUFFER_STATE], program_id).0
    } else {
        Pubkey::find_program_address(
            &[model_seeds::BUFFER, a.managed_mint.key().as_ref()],
            program_id,
        )
        .0
    };
    require_keys_eq!(
        a.buffer_state.key(),
        expected,
        OnreError::InvalidBufferStateAccount
    );
    if onyc && !legacy {
        require!(
            !a.buffer_state.data_is_empty(),
            OnreError::InvalidBufferStateAccount
        );
    }
    Ok(legacy)
}

/// Reporting and TVL use State.main_offer during coexistence. After cutover,
/// adapt canonical Pricer vectors in memory to reuse the exact existing metric
/// calculations without writing or mutating the legacy Offer account.
pub(crate) fn canonical_market_offer(
    program_id: &Pubkey,
    a: &BufferContext,
) -> Result<crate::instructions::Offer> {
    require_keys_eq!(
        a.managed_mint.key(),
        a.state.onyc_mint,
        OnreError::InvalidOnycMint
    );
    if uses_legacy(program_id, a)? {
        return load_main_offer(
            program_id,
            a.main_offer.ok_or(OnreError::InvalidMainOffer)?,
            a.state,
        );
    }
    let pricer = a.buffer_pricer.ok_or(OnreError::IncompatiblePricer)?;
    let canonical = Pubkey::find_program_address(
        &[
            model_seeds::PRICER,
            a.managed_mint.key().as_ref(),
            &[PricingDenomination::Usd as u8],
            &0_u64.to_le_bytes(),
        ],
        program_id,
    )
    .0;
    require!(
        pricer.key() == canonical
            && pricer.enabled
            && pricer.managed_mint == a.managed_mint.key()
            && pricer.denomination == PricingDenomination::Usd,
        OnreError::IncompatiblePricer
    );
    // Pure in-memory valuation adapter. No legacy offer account, authority or
    // custody is read after cutover; metrics consume only the mint and vectors.
    let mut offer: crate::instructions::Offer = bytemuck::Zeroable::zeroed();
    offer.token_out_mint = a.managed_mint.key();
    offer.vectors = pricer.vectors.map(Into::into);
    Ok(offer)
}
