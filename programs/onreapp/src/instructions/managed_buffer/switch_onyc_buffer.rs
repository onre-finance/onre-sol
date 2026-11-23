//! Boss-only, one-way switch of ONyc's active BUFFER from the legacy singleton to
//! its per-mint BUFFER, so exactly one engine accrues ONyc at any time.
use crate::{
    constants::seeds::{MINT_AUTHORITY, STATE},
    instructions::buffer::modular_accrual::{self, BufferContext},
    model::{seeds::MANAGED_TOKEN, ManagedTokenConfig, Pricer},
    state::State,
    utils::managed_token::effective_mint_limits,
    OnreError,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};

#[event]
pub struct OnycBufferSwitchedToPerMintEvent {
    pub boss: Pubkey,
    pub legacy_buffer_state: Pubkey,
    pub buffer_state: Pubkey,
    /// Supply both engines agree on at the switch; the per-mint BUFFER starts here.
    pub supply: u64,
    pub gross_apr: u64,
    pub management_fee_basis_points: u16,
    pub performance_fee_basis_points: u16,
    pub performance_fee_high_watermark: u64,
    pub performance_fee_high_watermark_enabled: bool,
    pub timestamp: i64,
}

#[derive(Accounts)]
pub struct SwitchOnycBufferToPerMint<'info> {
    #[account(
        mut,
        seeds = [STATE],
        bump = state.bump,
        has_one = boss @ OnreError::InvalidBoss,
        has_one = onyc_mint @ OnreError::InvalidOnycMint,
        constraint = !state.is_killed @ OnreError::KillSwitchActivated,
        constraint = state.onyc_legacy_buffer_active() @ OnreError::LegacyBufferRetired,
    )]
    pub state: Box<Account<'info, State>>,

    pub boss: Signer<'info>,

    #[account(
        seeds = [MANAGED_TOKEN, onyc_mint.key().as_ref()],
        bump = managed_token.bump,
    )]
    pub managed_token: Box<Account<'info, ManagedTokenConfig>>,

    #[account(mut, mint::token_program = token_program)]
    pub onyc_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: Canonical mint signing identity, validated by seeds.
    #[account(seeds = [MINT_AUTHORITY], bump)]
    pub mint_authority: UncheckedAccount<'info>,

    /// CHECK: Required exactly when `ManagedTokenConfig::mint_multisig` is set.
    pub mint_multisig: Option<UncheckedAccount<'info>>,

    pub token_program: Interface<'info, TokenInterface>,

    /// Canonical USD instance-zero ONyc Pricer valuing the final legacy accrual.
    pub buffer_pricer: Box<Account<'info, Pricer>>,

    /// CHECK: Legacy `["buffer_state"]` singleton; must be initialized.
    #[account(mut)]
    pub legacy_buffer_state: UncheckedAccount<'info>,

    /// CHECK: Legacy reserve vault's ONyc ATA, validated against the singleton.
    #[account(mut)]
    pub legacy_reserve_token: UncheckedAccount<'info>,

    /// CHECK: Legacy management-fee vault's ONyc ATA, validated against the singleton.
    #[account(mut)]
    pub legacy_management_token: UncheckedAccount<'info>,

    /// CHECK: Legacy performance-fee vault's ONyc ATA, validated against the singleton.
    #[account(mut)]
    pub legacy_performance_token: UncheckedAccount<'info>,

    /// CHECK: Canonical `["buffer", onyc_mint]` per-mint BUFFER; must be initialized.
    #[account(mut)]
    pub buffer_state: UncheckedAccount<'info>,
}

/// Accrues the legacy singleton up to now, has the per-mint BUFFER take over its
/// settings and high-water mark from the current supply, and permanently marks the
/// per-mint BUFFER active. Afterwards legacy BUFFER settlement and settings fail
/// and legacy trades no longer accrue. Vault balances stay in the legacy vaults.
pub fn switch_onyc_buffer_to_per_mint(ctx: Context<SwitchOnycBufferToPerMint>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let program_id = ctx.program_id;
    let a = ctx.accounts;
    let (max_supply, max_mint) = effective_mint_limits(&a.state, &a.managed_token);
    let settings = {
        let legacy = BufferContext {
            state: &a.state,
            managed_mint: &a.onyc_mint,
            managed_program: &a.token_program,
            buffer_state: &a.legacy_buffer_state,
            buffer_pricer: Some(&a.buffer_pricer),
            reserve_token: &a.legacy_reserve_token,
            management_token: &a.legacy_management_token,
            performance_token: &a.legacy_performance_token,
            mint_authority: &a.mint_authority,
            configured_mint_multisig: a.managed_token.mint_multisig(),
            mint_multisig: a.mint_multisig.as_ref(),
        };
        require!(
            modular_accrual::accrue(
                program_id,
                &legacy,
                ctx.bumps.mint_authority,
                now,
                max_supply,
                max_mint,
            )?,
            OnreError::InvalidBufferStateAccount
        );
        modular_accrual::load_onyc_legacy_buffer(program_id, &legacy)?
    };
    a.onyc_mint.reload()?;

    let per_mint = BufferContext {
        state: &a.state,
        managed_mint: &a.onyc_mint,
        managed_program: &a.token_program,
        buffer_state: &a.buffer_state,
        buffer_pricer: Some(&a.buffer_pricer),
        reserve_token: &a.legacy_reserve_token,
        management_token: &a.legacy_management_token,
        performance_token: &a.legacy_performance_token,
        mint_authority: &a.mint_authority,
        configured_mint_multisig: a.managed_token.mint_multisig(),
        mint_multisig: a.mint_multisig.as_ref(),
    };
    modular_accrual::take_over_onyc_legacy_buffer(program_id, &per_mint, &settings, now)?;
    let supply = a.onyc_mint.supply;
    a.state.onyc_per_mint_buffer_active = true;
    emit!(OnycBufferSwitchedToPerMintEvent {
        boss: a.boss.key(),
        legacy_buffer_state: a.legacy_buffer_state.key(),
        buffer_state: a.buffer_state.key(),
        supply,
        gross_apr: settings.gross_apr,
        management_fee_basis_points: settings.management_fee_basis_points,
        performance_fee_basis_points: settings.performance_fee_basis_points,
        performance_fee_high_watermark: settings.performance_fee_high_watermark,
        performance_fee_high_watermark_enabled: settings.performance_fee_high_watermark_enabled,
        timestamp: now,
    });
    Ok(())
}
