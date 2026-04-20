use crate::instructions::buffer::accounts::{
    BufferAccrualAccountsBumps, __client_accounts_buffer_accrual_accounts,
    __cpi_client_accounts_buffer_accrual_accounts,
};
use crate::instructions::buffer::BufferAccrualAccounts;
use crate::instructions::offer::execute_take_offer_permissionless;
use crate::instructions::Offer;
use crate::state::State;
use crate::utils::{
    get_associated_token_account, get_or_create_associated_token_account, ApprovalMessage,
    EnsureAtaParams,
};
use anchor_lang::{prelude::*, Accounts};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenInterface},
};

use super::quote::{validate_canonical_offer, SwapSide};

#[derive(Accounts)]
pub struct OpenSwapBuy<'info> {
    pub offer: AccountLoader<'info, Offer>,

    #[account(
        seeds = [crate::constants::seeds::STATE],
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
    pub permissionless_authority: UncheckedAccount<'info>,

    /// CHECK: validated and optionally initialized in instruction logic
    #[account(mut)]
    pub permissionless_token_in_account: UncheckedAccount<'info>,

    /// CHECK: validated and optionally initialized in instruction logic
    #[account(mut)]
    pub permissionless_token_out_account: UncheckedAccount<'info>,

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
}

pub fn open_swap_buy<'info>(
    ctx: Context<'info, OpenSwapBuy<'info>>,
    token_in_amount: u64,
    minimum_out: u64,
    approval_message: Option<ApprovalMessage>,
) -> Result<()> {
    let side = validate_canonical_offer(
        ctx.program_id,
        &ctx.accounts.state,
        ctx.accounts.offer.key(),
        ctx.accounts.token_in_mint.key(),
        ctx.accounts.token_out_mint.key(),
    )?;
    require!(side == SwapSide::Buy, crate::OnreError::InvalidSwapPair);

    execute_open_swap_buy(ctx, token_in_amount, minimum_out, approval_message)
}

fn execute_open_swap_buy<'info>(
    ctx: Context<'info, OpenSwapBuy<'info>>,
    token_in_amount: u64,
    minimum_out: u64,
    approval_message: Option<ApprovalMessage>,
) -> Result<()> {
    let offer = ctx.accounts.offer.load()?;
    let result = crate::instructions::offer::process_offer_core(
        &offer,
        token_in_amount,
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_out_mint,
    )?;
    require!(
        result.token_out_amount >= minimum_out,
        crate::OnreError::MinimumOutNotMet
    );

    drop(offer);

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
    let permissionless_token_in_account = get_associated_token_account(
        &ctx.accounts.permissionless_token_in_account,
        &ctx.accounts.permissionless_authority.key(),
        &ctx.accounts.token_in_mint.key(),
        &ctx.accounts.token_in_program.key(),
        crate::OnreError::InvalidAmount,
    )?;
    let permissionless_token_out_account = get_associated_token_account(
        &ctx.accounts.permissionless_token_out_account,
        &ctx.accounts.permissionless_authority.key(),
        &ctx.accounts.token_out_mint.key(),
        &ctx.accounts.token_out_program.key(),
        crate::OnreError::InvalidPermissionlessTokenOutAccount,
    )?;

    execute_take_offer_permissionless(
        ctx.program_id,
        &ctx.accounts.offer,
        &ctx.accounts.state,
        &ctx.accounts.user,
        &ctx.accounts.instructions_sysvar,
        &ctx.accounts.token_in_mint,
        &mut ctx.accounts.token_out_mint,
        token_in_amount,
        &approval_message,
        &ctx.accounts.token_in_program,
        &user_token_in_account,
        &permissionless_token_in_account,
        &ctx.accounts.permissionless_authority,
        &boss_token_in_account,
        &offer_vault_token_in_account,
        &ctx.accounts.offer_vault_authority,
        &ctx.accounts.token_out_program,
        &offer_vault_token_out_account,
        &permissionless_token_out_account,
        &user_token_out_account,
        &ctx.accounts.mint_authority,
        Some(&ctx.accounts.buffer_accounts),
        Some(&ctx.accounts.market_stats),
        Some(&ctx.accounts.main_offer),
        &ctx.accounts.system_program,
    )
}
