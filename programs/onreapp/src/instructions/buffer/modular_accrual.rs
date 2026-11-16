use crate::{
    constants::seeds,
    instructions::{
        buffer::{
            accrue_buffer::{accrue_buffer_core, set_buffer_baseline_after_supply_change},
            calculate_buffer_fee_split, calculate_gross_buffer_accrual,
            validate_buffer_onyc_vault_accounts, BufferState,
        },
        market_info::MarketValuation,
    },
    model::{seeds as model_seeds, BufferSettings, ManagedTokenBufferState, Pricer},
    state::State,
    utils::{
        load_pda_account, managed_token::validate_canonical_usd_pricer,
        pricing::get_active_vector_and_current_price, program_minter::ProgramMinter,
        store_pda_account, validate_max_supply,
    },
    OnreError,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::get_associated_token_address_with_program_id,
    token_interface::{Mint, TokenInterface},
};

/// Shared account view for modular trades and standalone BUFFER operations.
/// Every managed mint uses its own `["buffer", mint]` state, except ONyc before the
/// one-way switch, whose active BUFFER is the legacy `["buffer_state"]` singleton
/// with its legacy vault token accounts in the same account slots.
pub(crate) struct BufferContext<'a, 'info> {
    pub state: &'a State,
    pub managed_mint: &'a InterfaceAccount<'info, Mint>,
    pub managed_program: &'a Interface<'info, TokenInterface>,
    pub buffer_state: &'a UncheckedAccount<'info>,
    pub buffer_pricer: Option<&'a Account<'info, Pricer>>,
    pub reserve_token: &'a UncheckedAccount<'info>,
    pub management_token: &'a UncheckedAccount<'info>,
    pub performance_token: &'a UncheckedAccount<'info>,
    pub mint_authority: &'a UncheckedAccount<'info>,
    /// Configured `ManagedTokenConfig::mint_multisig` and the supplied account.
    pub configured_mint_multisig: Pubkey,
    pub mint_multisig: Option<&'a UncheckedAccount<'info>>,
}

impl<'a, 'info> BufferContext<'a, 'info> {
    /// Whether this ONyc context accrues the legacy singleton (before the switch).
    pub(crate) fn uses_onyc_legacy_buffer(&self) -> bool {
        self.managed_mint.key() == self.state.onyc_mint && self.state.onyc_legacy_buffer_active()
    }

    /// Program minting path for the managed mint in its configured mode.
    pub(crate) fn minter(&self, authority_bump: u8) -> Result<ProgramMinter<'a, 'info>> {
        ProgramMinter::resolve(
            self.configured_mint_multisig,
            self.managed_mint,
            self.mint_authority.as_ref(),
            authority_bump,
            self.mint_multisig.map(AsRef::as_ref),
        )
    }
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

/// Even an absent BUFFER must be supplied at its canonical address so omission
/// cannot bypass an existing accrual engine. Returns whether BUFFER is initialized.
pub(crate) fn accrue(
    program_id: &Pubkey,
    a: &BufferContext,
    authority_bump: u8,
    now: i64,
    max_supply: u64,
    max_mint: u64,
) -> Result<bool> {
    if a.uses_onyc_legacy_buffer() {
        return accrue_onyc_legacy(program_id, a, authority_bump, now, max_supply, max_mint);
    }
    require_canonical_buffer_state(program_id, a)?;
    if a.buffer_state.data_is_empty() {
        require_keys_eq!(
            *a.buffer_state.owner,
            System::id(),
            OnreError::InvalidBufferStateAccount
        );
        return Ok(false);
    }
    let mut buffer = load_buffer(program_id, a)?;
    let minter = a.minter(authority_bump)?;
    require!(
        now >= buffer.last_accrual_timestamp,
        OnreError::InvalidTimestamp
    );
    let pricer = a.buffer_pricer.ok_or(OnreError::IncompatiblePricer)?;
    validate_canonical_usd_pricer(program_id, &buffer.managed_mint, pricer)?;
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
            minter.mint(
                a.managed_program,
                a.managed_mint,
                token,
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
    store_pda_account(a.buffer_state, &buffer)?;
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
    Ok(true)
}

pub(crate) fn store_post_supply(program_id: &Pubkey, a: &BufferContext, now: i64) -> Result<()> {
    if a.uses_onyc_legacy_buffer() {
        let mut buffer = load_onyc_legacy_buffer(program_id, a)?;
        set_buffer_baseline_after_supply_change(&mut buffer, a.managed_mint.supply, now);
        return store_pda_account(a.buffer_state, &buffer);
    }
    let mut buffer = load_buffer(program_id, a)?;
    buffer.previous_supply = a.managed_mint.supply;
    buffer.last_accrual_timestamp = now;
    store_pda_account(a.buffer_state, &buffer)
}

/// ONyc before the switch: accrue the legacy singleton with the canonical Pricer's
/// APR and NAV and the configured program minter, under the effective issuance
/// limits. An uninitialized singleton must be the empty canonical account.
fn accrue_onyc_legacy(
    program_id: &Pubkey,
    a: &BufferContext,
    authority_bump: u8,
    now: i64,
    max_supply: u64,
    max_mint: u64,
) -> Result<bool> {
    require_onyc_legacy_buffer_address(program_id, a)?;
    if a.buffer_state.data_is_empty() {
        require_keys_eq!(
            *a.buffer_state.owner,
            System::id(),
            OnreError::InvalidBufferStateAccount
        );
        return Ok(false);
    }
    let mut buffer = load_onyc_legacy_buffer(program_id, a)?;
    validate_buffer_onyc_vault_accounts(
        program_id,
        &buffer,
        a.reserve_token,
        a.management_token,
        a.performance_token,
        a.managed_mint,
        a.managed_program,
    )?;
    let minter = a.minter(authority_bump)?;
    let valuation = canonical_market_valuation(program_id, a)?;
    let (vector, nav) = get_active_vector_and_current_price(valuation.vectors, now as u64)?;
    accrue_buffer_core(
        &mut buffer,
        vector.apr,
        nav,
        Pubkey::default(),
        a.managed_mint,
        a.reserve_token.to_account_info(),
        a.management_token.to_account_info(),
        a.performance_token.to_account_info(),
        a.managed_program,
        max_supply,
        max_mint,
        now,
        &mut |destination, amount| {
            minter.mint(
                a.managed_program,
                a.managed_mint,
                destination,
                amount,
                max_supply,
                max_mint,
            )
        },
    )?;
    store_pda_account(a.buffer_state, &buffer)?;
    Ok(true)
}

fn require_onyc_legacy_buffer_address(program_id: &Pubkey, a: &BufferContext) -> Result<()> {
    let (expected, _) = Pubkey::find_program_address(&[seeds::BUFFER_STATE], program_id);
    require_keys_eq!(
        a.buffer_state.key(),
        expected,
        OnreError::InvalidBufferStateAccount
    );
    Ok(())
}

pub(crate) fn load_onyc_legacy_buffer(
    program_id: &Pubkey,
    a: &BufferContext,
) -> Result<BufferState> {
    require_onyc_legacy_buffer_address(program_id, a)?;
    load_pda_account(
        a.buffer_state,
        program_id,
        OnreError::InvalidBufferStateAccount.into(),
        OnreError::InvalidBufferStateAccount.into(),
    )
}

/// One-way ONyc switch: the initialized per-mint BUFFER takes over the legacy
/// singleton's settings and high-water mark and starts accruing from the current
/// supply and `now`, so neither engine accrues the same interval twice.
pub(crate) fn take_over_onyc_legacy_buffer(
    program_id: &Pubkey,
    a: &BufferContext,
    legacy: &BufferState,
    now: i64,
) -> Result<()> {
    require_canonical_buffer_state(program_id, a)?;
    let mut buffer = load_buffer(program_id, a)?;
    buffer.previous_supply = a.managed_mint.supply;
    buffer.last_accrual_timestamp = now;
    buffer.gross_apr = legacy.gross_apr;
    buffer.management_fee_basis_points = legacy.management_fee_basis_points;
    buffer.performance_fee_basis_points = legacy.performance_fee_basis_points;
    buffer.performance_fee_high_watermark = legacy.performance_fee_high_watermark;
    buffer.performance_fee_high_watermark_enabled = legacy.performance_fee_high_watermark_enabled;
    store_pda_account(a.buffer_state, &buffer)
}

/// The initialized BUFFER's reserve vault, after canonical identity checks.
pub(crate) fn reserve_vault(program_id: &Pubkey, a: &BufferContext) -> Result<Pubkey> {
    require_canonical_buffer_state(program_id, a)?;
    Ok(load_buffer(program_id, a)?.reserve_vault)
}

/// Account identity is checked even for configuration-only reads.
pub(crate) fn read_settings(program_id: &Pubkey, a: &BufferContext) -> Result<BufferSettings> {
    require_canonical_buffer_state(program_id, a)?;
    let b = load_buffer(program_id, a)?;
    Ok(BufferSettings {
        gross_apr: b.gross_apr,
        management_fee_basis_points: b.management_fee_basis_points,
        performance_fee_basis_points: b.performance_fee_basis_points,
        performance_fee_high_watermark_enabled: b.performance_fee_high_watermark_enabled,
    })
}

/// Call only after accruing under the old settings. Preserve the newly committed
/// supply baseline, timestamp and watermark when applying the next interval's policy.
pub(crate) fn write_settings(
    program_id: &Pubkey,
    a: &BufferContext,
    settings: BufferSettings,
) -> Result<()> {
    let mut b = load_buffer(program_id, a)?;
    b.gross_apr = settings.gross_apr;
    b.management_fee_basis_points = settings.management_fee_basis_points;
    b.performance_fee_basis_points = settings.performance_fee_basis_points;
    b.performance_fee_high_watermark_enabled = settings.performance_fee_high_watermark_enabled;
    store_pda_account(a.buffer_state, &b)
}

/// Token-wide valuation always comes from the canonical USD instance-zero Pricer.
pub(crate) fn canonical_market_valuation(
    program_id: &Pubkey,
    a: &BufferContext,
) -> Result<MarketValuation> {
    let pricer = a.buffer_pricer.ok_or(OnreError::IncompatiblePricer)?;
    validate_canonical_usd_pricer(program_id, &a.managed_mint.key(), pricer)?;
    Ok(MarketValuation::from(&**pricer))
}

fn require_canonical_buffer_state(program_id: &Pubkey, a: &BufferContext) -> Result<()> {
    let (expected, _) = Pubkey::find_program_address(
        &[model_seeds::BUFFER, a.managed_mint.key().as_ref()],
        program_id,
    );
    require_keys_eq!(
        a.buffer_state.key(),
        expected,
        OnreError::InvalidBufferStateAccount
    );
    Ok(())
}

fn load_buffer(program_id: &Pubkey, a: &BufferContext) -> Result<ManagedTokenBufferState> {
    let buffer: ManagedTokenBufferState = load_pda_account(
        a.buffer_state,
        program_id,
        OnreError::InvalidBufferStateAccount.into(),
        OnreError::InvalidBufferStateAccount.into(),
    )?;
    require_keys_eq!(
        buffer.managed_mint,
        a.managed_mint.key(),
        OnreError::InvalidMint
    );
    Ok(buffer)
}
