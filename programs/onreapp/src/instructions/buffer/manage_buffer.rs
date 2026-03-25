use crate::constants::seeds;
use crate::instructions::buffer::{
    calculate_buffer_fee_split, calculate_gross_buffer_accrual, BufferErrorCode,
    BufferManagedEvent, BufferState, BufferAccrualAccounts, validate_buffer_onyc_vault_accounts,
};
use crate::instructions::market_info::refresh_market_stats_pda;
use crate::instructions::market_info::offer_valuation_utils::get_active_vector_and_current_price;
use crate::instructions::Offer;
use crate::state::State;
use crate::utils::token_utils::{mint_tokens, read_optional_token_account_amount, TokenUtilsErrorCode};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_option::COption;
use anchor_spl::associated_token::{get_associated_token_address_with_program_id, AssociatedToken};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

pub(crate) struct BufferAccrualResult {
    pub seconds_elapsed: u64,
    pub spread: u64,
    pub gross_mint_amount: u64,
    pub buffer_mint_amount: u64,
    pub management_fee_mint_amount: u64,
    pub performance_fee_mint_amount: u64,
    pub previous_lowest_supply: u64,
    pub new_lowest_supply: u64,
    pub previous_performance_fee_high_watermark: u64,
    pub new_performance_fee_high_watermark: u64,
    pub timestamp: i64,
    pub current_nav: u64,
    pub post_accrual_supply: u64,
    pub buffer_vault_balance_after_accrual: u64,
}

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

    #[account(mut)]
    pub caller: Signer<'info>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::OFFER_VAULT_AUTHORITY], bump)]
    pub offer_vault_authority: UncheckedAccount<'info>,

    /// CHECK: Address is validated against the canonical ATA derivation.
    #[account(
        constraint = offer_vault_onyc_account.key()
            == get_associated_token_address_with_program_id(
                &offer_vault_authority.key(),
                &onyc_mint.key(),
                &token_program.key(),
            ) @ crate::instructions::market_info::GetCirculatingSupplyErrorCode::InvalidVaultAccount
    )]
    pub offer_vault_onyc_account: UncheckedAccount<'info>,

    /// CHECK: Validated and optionally initialized in instruction logic.
    #[account(mut)]
    pub market_stats: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn manage_buffer(ctx: Context<ManageBuffer>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let offer = ctx.accounts.offer.load()?;
    let buffer_vault_onyc_account = ctx.accounts.buffer_vault_onyc_account.to_account_info();
    let management_fee_vault_onyc_account =
        ctx.accounts.management_fee_vault_onyc_account.to_account_info();
    let performance_fee_vault_onyc_account =
        ctx.accounts.performance_fee_vault_onyc_account.to_account_info();
    let mint_authority = ctx.accounts.mint_authority.to_account_info();
    let result = accrue_buffer(
        &ctx.accounts.state,
        &mut ctx.accounts.buffer_state,
        &offer,
        &ctx.accounts.onyc_mint,
        buffer_vault_onyc_account,
        management_fee_vault_onyc_account,
        performance_fee_vault_onyc_account,
        mint_authority,
        ctx.bumps.mint_authority,
        &ctx.accounts.token_program,
        now,
    )?;

    ctx.accounts.onyc_mint.reload()?;
    refresh_market_stats_pda(
        &offer,
        &ctx.accounts.onyc_mint,
        &ctx.accounts.offer_vault_onyc_account.to_account_info(),
        &ctx.accounts.token_program,
        &ctx.accounts.market_stats.to_account_info(),
        &ctx.accounts.caller.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        ctx.program_id,
    )?;

    emit!(BufferManagedEvent {
        seconds_elapsed: result.seconds_elapsed,
        spread: result.spread,
        gross_mint_amount: result.gross_mint_amount,
        buffer_mint_amount: result.buffer_mint_amount,
        management_fee_mint_amount: result.management_fee_mint_amount,
        performance_fee_mint_amount: result.performance_fee_mint_amount,
        previous_lowest_supply: result.previous_lowest_supply,
        new_lowest_supply: result.new_lowest_supply,
        previous_performance_fee_high_watermark: result.previous_performance_fee_high_watermark,
        new_performance_fee_high_watermark: result.new_performance_fee_high_watermark,
        timestamp: result.timestamp,
    });

    Ok(())
}

pub(crate) fn accrue_buffer<'info>(
    state: &Account<'info, State>,
    buffer_state: &mut BufferState,
    offer: &Offer,
    onyc_mint: &InterfaceAccount<'info, Mint>,
    buffer_vault_onyc_account: AccountInfo<'info>,
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
    let spread = buffer_state.gross_apr.saturating_sub(active_vector.apr);
    let previous_lowest_supply = buffer_state.lowest_supply;
    let current_supply_before_mint = onyc_mint.supply;
    let buffer_vault_balance_before_mint =
        read_optional_token_account_amount(&buffer_vault_onyc_account, token_program)?;
    let previous_performance_fee_high_watermark = buffer_state.performance_fee_high_watermark;

    if previous_lowest_supply == 0 {
        set_buffer_baseline_after_supply_change(buffer_state, current_supply_before_mint, now);

        return Ok(BufferAccrualResult {
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
            current_nav,
            post_accrual_supply: buffer_state.lowest_supply,
            buffer_vault_balance_after_accrual: buffer_vault_balance_before_mint,
        });
    }

    let gross_mint_amount = calculate_gross_buffer_accrual(
        previous_lowest_supply,
        buffer_state.gross_apr,
        active_vector.apr,
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
        if state.max_supply > 0 {
            let new_supply = current_supply_before_mint
                .checked_add(gross_mint_amount)
                .ok_or(BufferErrorCode::MathOverflow)?;
            require!(
                new_supply <= state.max_supply,
                TokenUtilsErrorCode::MaxSupplyExceeded
            );
        }

        let mint_authority_seeds = &[seeds::MINT_AUTHORITY, &[mint_authority_bump]];
        let mint_authority_signer_seeds = &[mint_authority_seeds.as_slice()];

        if fee_split.buffer_mint_amount > 0 {
            mint_tokens(
                token_program,
                onyc_mint,
                &buffer_vault_onyc_account,
                &mint_authority,
                mint_authority_signer_seeds,
                fee_split.buffer_mint_amount,
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
        .checked_add(gross_mint_amount)
        .ok_or(BufferErrorCode::MathOverflow)?;
    let buffer_vault_balance_after_accrual = buffer_vault_balance_before_mint
        .checked_add(fee_split.buffer_mint_amount)
        .ok_or(BufferErrorCode::MathOverflow)?;

    buffer_state.performance_fee_high_watermark = fee_split.new_performance_fee_high_watermark;
    set_buffer_baseline_after_supply_change(buffer_state, post_accrual_supply, now);

    Ok(BufferAccrualResult {
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
        current_nav,
        post_accrual_supply,
        buffer_vault_balance_after_accrual,
    })
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
    now: i64,
) -> Result<BufferAccrualResult> {
    let mut buffer_state = buffer_accounts.load_buffer_state()?;
    let buffer_vault_onyc_account = buffer_accounts.buffer_vault_onyc_account_info();
    let management_fee_vault_onyc_account = buffer_accounts.management_fee_vault_onyc_account_info();
    let performance_fee_vault_onyc_account = buffer_accounts.performance_fee_vault_onyc_account_info();

    validate_buffer_onyc_vault_accounts(
        program_id,
        &buffer_state,
        &buffer_vault_onyc_account,
        &management_fee_vault_onyc_account,
        &performance_fee_vault_onyc_account,
        onyc_mint,
        token_program,
    )?;

    accrue_buffer(
        state,
        &mut buffer_state,
        offer,
        onyc_mint,
        buffer_vault_onyc_account,
        management_fee_vault_onyc_account,
        performance_fee_vault_onyc_account,
        mint_authority,
        mint_authority_bump,
        token_program,
        now,
    )
}

pub(crate) fn set_buffer_baseline_after_supply_change(
    buffer_state: &mut BufferState,
    post_change_supply: u64,
    now: i64,
) {
    buffer_state.lowest_supply = post_change_supply;
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
