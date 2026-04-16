use crate::constants::{seeds, MAX_BASIS_POINTS};
use crate::instructions::buffer::accounts::{
    BufferAccrualAccountsBumps, __client_accounts_buffer_accrual_accounts,
    __cpi_client_accounts_buffer_accrual_accounts,
};
use crate::instructions::buffer::{
    accrue_buffer::accrue_buffer_from_accounts, BufferAccrualAccounts, BufferFeeConfigUpdatedEvent
};
use crate::instructions::market_info::market_stats::refresh_market_stats_pda;
use crate::instructions::Offer;
use crate::state::State;
use anchor_lang::solana_program::program_option::COption;
use anchor_lang::{prelude::*, Accounts};
use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use anchor_spl::token_interface::{Mint, TokenInterface};

#[derive(Accounts)]
pub struct SetBufferFeeConfig<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss,
        has_one = onyc_mint,
    )]
    pub state: Box<Account<'info, State>>,

    pub boss: Signer<'info>,

    #[account(address = state.main_offer @ crate::OnreError::InvalidMainOffer)]
    pub main_offer: AccountLoader<'info, Offer>,

    #[account(mut)]
    pub onyc_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: PDA derivation is validated in instruction logic.
    #[account(seeds = [seeds::OFFER_VAULT_AUTHORITY], bump)]
    pub offer_vault_authority: UncheckedAccount<'info>,

    /// CHECK: Account is validated in instruction logic to allow uninitialized vault account.
    pub vault_token_out_account: UncheckedAccount<'info>,

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

    /// CHECK: Validated in instruction logic.
    #[account(mut)]
    pub market_stats: UncheckedAccount<'info>,
}

pub fn set_buffer_fee_config(
    ctx: Context<SetBufferFeeConfig>,
    management_fee_basis_points: u16,
    management_fee_wallet: Pubkey,
    performance_fee_basis_points: u16,
    performance_fee_wallet: Pubkey,
) -> Result<()> {
    require!(
        management_fee_basis_points <= MAX_BASIS_POINTS,
        crate::OnreError::InvalidFee
    );
    require!(
        performance_fee_basis_points <= MAX_BASIS_POINTS,
        crate::OnreError::InvalidFee
    );
    require!(
        management_fee_basis_points == 0 || management_fee_wallet != Pubkey::default(),
        crate::OnreError::InvalidFeeWallet
    );
    require!(
        performance_fee_basis_points == 0 || performance_fee_wallet != Pubkey::default(),
        crate::OnreError::InvalidFeeWallet
    );

    let mut buffer_state = ctx.accounts.buffer_accounts.load_buffer_state()?;
    require!(
        buffer_state.management_fee_basis_points != management_fee_basis_points
            || buffer_state.management_fee_wallet != management_fee_wallet
            || buffer_state.performance_fee_basis_points != performance_fee_basis_points
            || buffer_state.performance_fee_wallet != performance_fee_wallet,
        crate::OnreError::NoChange
    );

    let old_management_fee_basis_points = buffer_state.management_fee_basis_points;
    let old_management_fee_wallet = buffer_state.management_fee_wallet;
    let old_performance_fee_basis_points = buffer_state.performance_fee_basis_points;
    let old_performance_fee_wallet = buffer_state.performance_fee_wallet;

    let expected_vault_token_out_account = get_associated_token_address_with_program_id(
        &ctx.accounts.offer_vault_authority.key(),
        &ctx.accounts.onyc_mint.key(),
        &ctx.accounts.token_program.key(),
    );
    require_keys_eq!(
        ctx.accounts.vault_token_out_account.key(),
        expected_vault_token_out_account,
        crate::OnreError::InvalidOnycMint
    );

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
        &ctx.accounts.vault_token_out_account.to_account_info(),
        &ctx.accounts.token_program,
        &ctx.accounts.market_stats.to_account_info(),
        &ctx.accounts.boss.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        ctx.program_id,
    )?;

    buffer_state = ctx.accounts.buffer_accounts.load_buffer_state()?;
    buffer_state.management_fee_basis_points = management_fee_basis_points;
    buffer_state.management_fee_wallet = management_fee_wallet;
    buffer_state.performance_fee_basis_points = performance_fee_basis_points;
    buffer_state.performance_fee_wallet = performance_fee_wallet;
    ctx.accounts
        .buffer_accounts
        .store_buffer_state(&buffer_state)?;

    emit!(BufferFeeConfigUpdatedEvent {
        old_management_fee_basis_points,
        new_management_fee_basis_points: management_fee_basis_points,
        old_management_fee_wallet,
        new_management_fee_wallet: management_fee_wallet,
        old_performance_fee_basis_points,
        new_performance_fee_basis_points: performance_fee_basis_points,
        old_performance_fee_wallet,
        new_performance_fee_wallet: performance_fee_wallet,
    });

    Ok(())
}
