use crate::constants::seeds;
use crate::instructions::buffer::{
    calculate_buffer_fee_split, calculate_gross_buffer_accrual, BufferErrorCode,
    BufferManagedEvent, BufferState,
};
use crate::instructions::market_info::offer_valuation_utils::get_active_vector_and_current_price;
use crate::instructions::Offer;
use crate::state::State;
use crate::utils::token_utils::{mint_tokens, TokenUtilsErrorCode};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_option::COption;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct ManageBuffer<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = onyc_mint,
    )]
    pub state: Box<Account<'info, State>>,

    #[account(
        mut,
        seeds = [seeds::BUFFER_STATE],
        bump = buffer_state.bump,
        has_one = onyc_mint,
    )]
    pub buffer_state: Box<Account<'info, BufferState>>,

    #[account(address = state.main_offer @ BufferErrorCode::InvalidMainOffer)]
    pub offer: AccountLoader<'info, Offer>,

    #[account(mut)]
    pub onyc_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        seeds = [seeds::BUFFER_VAULT_AUTHORITY],
        bump,
    )]
    pub buffer_vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = onyc_mint,
        associated_token::authority = buffer_vault_authority,
        associated_token::token_program = token_program
    )]
    pub buffer_vault_onyc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        seeds = [seeds::MANAGEMENT_FEE_VAULT_AUTHORITY],
        bump,
    )]
    pub management_fee_vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = onyc_mint,
        associated_token::authority = management_fee_vault_authority,
        associated_token::token_program = token_program
    )]
    pub management_fee_vault_onyc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        seeds = [seeds::PERFORMANCE_FEE_VAULT_AUTHORITY],
        bump,
    )]
    pub performance_fee_vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = onyc_mint,
        associated_token::authority = performance_fee_vault_authority,
        associated_token::token_program = token_program
    )]
    pub performance_fee_vault_onyc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        seeds = [seeds::MINT_AUTHORITY],
        constraint = onyc_mint.mint_authority == COption::Some(mint_authority.key()) @ BufferErrorCode::NoMintAuthority,
        bump
    )]
    pub mint_authority: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

pub fn manage_buffer(ctx: Context<ManageBuffer>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let buffer_state = &mut ctx.accounts.buffer_state;
    require!(
        now >= buffer_state.last_accrual_timestamp,
        BufferErrorCode::InvalidTimestamp
    );
    let offer = ctx.accounts.offer.load()?;
    let (active_vector, current_nav) = get_active_vector_and_current_price(&offer, now as u64)?;
    let current_apr = active_vector.apr;

    let seconds_elapsed = (now - buffer_state.last_accrual_timestamp) as u64;
    let spread = buffer_state.gross_apr.saturating_sub(current_apr);
    let previous_lowest_supply = buffer_state.lowest_supply;
    let current_supply_before_mint = ctx.accounts.onyc_mint.supply;
    let previous_performance_fee_high_watermark = buffer_state.performance_fee_high_watermark;

    if previous_lowest_supply == 0 {
        buffer_state.lowest_supply = current_supply_before_mint;
        buffer_state.last_accrual_timestamp = now;

        emit!(BufferManagedEvent {
            seconds_elapsed,
            spread,
            gross_mint_amount: 0,
            buffer_mint_amount: 0,
            management_fee_mint_amount: 0,
            performance_fee_mint_amount: 0,
            previous_lowest_supply,
            new_lowest_supply: buffer_state.lowest_supply,
            previous_performance_fee_high_watermark,
            new_performance_fee_high_watermark: buffer_state.performance_fee_high_watermark,
            timestamp: now,
        });

        return Ok(());
    }

    let gross_mint_amount = calculate_gross_buffer_accrual(
        previous_lowest_supply,
        buffer_state.gross_apr,
        current_apr,
        seconds_elapsed,
    )?;
    let fee_split = calculate_buffer_fee_split(
        gross_mint_amount,
        spread,
        buffer_state.management_fee_basis_points,
        buffer_state.performance_fee_basis_points,
        current_nav,
        previous_performance_fee_high_watermark,
    )?;

    if gross_mint_amount > 0 {
        if ctx.accounts.state.max_supply > 0 {
            let new_supply = ctx
                .accounts
                .onyc_mint
                .supply
                .checked_add(gross_mint_amount)
                .ok_or(BufferErrorCode::MathOverflow)?;
            require!(
                new_supply <= ctx.accounts.state.max_supply,
                TokenUtilsErrorCode::MaxSupplyExceeded
            );
        }

        let mint_authority_seeds = &[seeds::MINT_AUTHORITY, &[ctx.bumps.mint_authority]];
        let mint_authority_signer_seeds = &[mint_authority_seeds.as_slice()];

        if fee_split.buffer_mint_amount > 0 {
            mint_tokens(
                &ctx.accounts.token_program,
                &ctx.accounts.onyc_mint,
                &ctx.accounts.buffer_vault_onyc_account,
                &ctx.accounts.mint_authority.to_account_info(),
                mint_authority_signer_seeds,
                fee_split.buffer_mint_amount,
                0,
            )?;
        }

        if fee_split.management_fee_mint_amount > 0 {
            mint_tokens(
                &ctx.accounts.token_program,
                &ctx.accounts.onyc_mint,
                &ctx.accounts.management_fee_vault_onyc_account,
                &ctx.accounts.mint_authority.to_account_info(),
                mint_authority_signer_seeds,
                fee_split.management_fee_mint_amount,
                0,
            )?;
        }

        if fee_split.performance_fee_mint_amount > 0 {
            mint_tokens(
                &ctx.accounts.token_program,
                &ctx.accounts.onyc_mint,
                &ctx.accounts.performance_fee_vault_onyc_account,
                &ctx.accounts.mint_authority.to_account_info(),
                mint_authority_signer_seeds,
                fee_split.performance_fee_mint_amount,
                0,
            )?;
        }
    }

    buffer_state.performance_fee_high_watermark = fee_split.new_performance_fee_high_watermark;
    buffer_state.lowest_supply = current_supply_before_mint
        .checked_add(gross_mint_amount)
        .ok_or(BufferErrorCode::MathOverflow)?;
    buffer_state.last_accrual_timestamp = now;

    emit!(BufferManagedEvent {
        seconds_elapsed,
        spread,
        gross_mint_amount: fee_split.gross_mint_amount,
        buffer_mint_amount: fee_split.buffer_mint_amount,
        management_fee_mint_amount: fee_split.management_fee_mint_amount,
        performance_fee_mint_amount: fee_split.performance_fee_mint_amount,
        previous_lowest_supply,
        new_lowest_supply: buffer_state.lowest_supply,
        previous_performance_fee_high_watermark,
        new_performance_fee_high_watermark: buffer_state.performance_fee_high_watermark,
        timestamp: now,
    });

    Ok(())
}
