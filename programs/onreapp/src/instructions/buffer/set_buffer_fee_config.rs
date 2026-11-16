use crate::constants::{seeds, MAX_BASIS_POINTS};
use crate::instructions::buffer::accounts::{
    __client_accounts_buffer_accrual_accounts, __cpi_client_accounts_buffer_accrual_accounts,
    BufferAccrualAccountsBumps,
};
use crate::instructions::buffer::{
    accrue_buffer::accrue_buffer_from_accounts, BufferAccrualAccounts, BufferFeeConfigUpdatedEvent,
};
use crate::instructions::market_info::market_stats::refresh_market_stats_pda;
use crate::instructions::Offer;
use crate::state::State;
use anchor_lang::solana_program::program_option::COption;
use anchor_lang::{prelude::*, Accounts};
use anchor_spl::token_interface::{Mint, TokenInterface};

#[derive(Accounts)]
pub struct SetBufferFeeConfig<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss,
        has_one = onyc_mint,
        constraint = !state.is_killed @ crate::OnreError::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,

    #[account(mut)]
    pub boss: Signer<'info>,

    #[account(address = state.main_offer @ crate::OnreError::InvalidMainOffer)]
    pub main_offer: AccountLoader<'info, Offer>,

    #[account(mut)]
    pub onyc_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: PDA derivation is validated by seeds constraint.
    #[account(seeds = [seeds::OFFER_VAULT_AUTHORITY], bump)]
    pub offer_vault_authority: UncheckedAccount<'info>,

    /// CHECK: PDA derivation is validated by seeds constraint.
    #[account(
        seeds = [seeds::MINT_AUTHORITY],
        constraint = onyc_mint.mint_authority == COption::Some(mint_authority.key()) @ crate::OnreError::NoMintAuthority,
        bump
    )]
    pub mint_authority: UncheckedAccount<'info>,

    pub buffer_accounts: BufferAccrualAccounts<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,

    /// CHECK: PDA derivation is validated by seeds constraint and the account is optionally initialized in settlement helper.
    #[account(mut, seeds = [seeds::MARKET_STATS], bump)]
    pub market_stats: UncheckedAccount<'info>,

    /// CHECK: PDA derivation is validated by seeds constraint; data loading is handled by market stats refresh.
    #[account(seeds = [seeds::CIRCULATING_SUPPLY_EXCLUDED_BALANCE], bump)]
    pub circulating_supply_excluded_balance: UncheckedAccount<'info>,
}

pub fn set_buffer_fee_config(
    ctx: Context<SetBufferFeeConfig>,
    management_fee_basis_points: u16,
    performance_fee_basis_points: u16,
    performance_fee_high_watermark_enabled: bool,
) -> Result<()> {
    ctx.accounts.state.require_onyc_legacy_buffer_active()?;
    require!(
        management_fee_basis_points <= MAX_BASIS_POINTS,
        crate::OnreError::InvalidFee
    );
    require!(
        performance_fee_basis_points <= MAX_BASIS_POINTS,
        crate::OnreError::InvalidFee
    );

    let mut buffer_state = ctx.accounts.buffer_accounts.load_buffer_state()?;
    require!(
        buffer_state.management_fee_basis_points != management_fee_basis_points
            || buffer_state.performance_fee_basis_points != performance_fee_basis_points
            || buffer_state.performance_fee_high_watermark_enabled
                != performance_fee_high_watermark_enabled,
        crate::OnreError::NoChange
    );

    let old_management_fee_basis_points = buffer_state.management_fee_basis_points;
    let old_performance_fee_basis_points = buffer_state.performance_fee_basis_points;
    let old_performance_fee_high_watermark_enabled =
        buffer_state.performance_fee_high_watermark_enabled;

    let offer = ctx.accounts.main_offer.load()?;
    require_keys_eq!(
        ctx.accounts.onyc_mint.key(),
        offer.token_out_mint,
        crate::OnreError::InvalidTokenOutMint
    );

    accrue_buffer_from_accounts(
        ctx.program_id,
        &ctx.accounts.state,
        &ctx.accounts.buffer_accounts,
        &offer,
        &ctx.accounts.onyc_mint,
        ctx.accounts.mint_authority.to_account_info(),
        ctx.bumps.mint_authority,
        &ctx.accounts.token_program,
    )?;

    ctx.accounts.onyc_mint.reload()?;
    refresh_market_stats_pda(
        &offer,
        &ctx.accounts.onyc_mint,
        &ctx.accounts
            .circulating_supply_excluded_balance
            .to_account_info(),
        &ctx.accounts.market_stats.to_account_info(),
        &ctx.accounts.boss.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        ctx.program_id,
    )?;

    buffer_state = ctx.accounts.buffer_accounts.load_buffer_state()?;
    buffer_state.management_fee_basis_points = management_fee_basis_points;
    buffer_state.performance_fee_basis_points = performance_fee_basis_points;
    buffer_state.performance_fee_high_watermark_enabled = performance_fee_high_watermark_enabled;
    ctx.accounts
        .buffer_accounts
        .store_buffer_state(&buffer_state)?;

    emit!(BufferFeeConfigUpdatedEvent {
        old_management_fee_basis_points,
        new_management_fee_basis_points: management_fee_basis_points,
        old_performance_fee_basis_points,
        new_performance_fee_basis_points: performance_fee_basis_points,
        old_performance_fee_high_watermark_enabled,
        new_performance_fee_high_watermark_enabled: performance_fee_high_watermark_enabled,
    });

    Ok(())
}
