use crate::constants::seeds;
use crate::instructions::market_info::{
    load_main_offer, read_market_stats_account, refresh_market_stats_pda,
};
use crate::instructions::offer::{
    validate_take_offer_authorities, verify_offer_approval, OfferTakenEvent,
};
use crate::instructions::redemption::{
    execute_redemption_operations, process_redemption_core, ExecuteRedemptionOpsParams,
};
use crate::instructions::Offer;
use crate::state::State;
use crate::utils::{
    get_associated_token_account, get_or_create_associated_token_account, transfer_tokens,
    u64_to_dec9, ApprovalMessage, EnsureAtaParams,
};
use anchor_lang::{prelude::*, Accounts};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenInterface},
};

use super::config::PropAmmState;
use super::quote::{
    apply_hard_wall_liquidity_factor, hard_wall_reserve_from_tvl, redemption_offer_fee_basis_points,
};
use super::quote::{validate_canonical_offer, SwapSide};

#[derive(Accounts)]
pub struct OpenSwapSell<'info> {
    pub offer: AccountLoader<'info, Offer>,

    #[account(
        seeds = [crate::constants::seeds::PROP_AMM_STATE],
        bump = prop_amm_state.bump
    )]
    pub prop_amm_state: Account<'info, PropAmmState>,

    #[account(
        seeds = [
            crate::constants::seeds::REDEMPTION_OFFER,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref()
        ],
        bump
    )]
    /// CHECK: PDA address is validated by seeds; data is optional and loaded in instruction logic.
    pub redemption_offer: UncheckedAccount<'info>,

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

    /// CHECK: PDA derivation validated by seeds constraint
    #[account(seeds = [crate::constants::seeds::REDEMPTION_OFFER_VAULT_AUTHORITY], bump)]
    pub redemption_vault_authority: UncheckedAccount<'info>,

    /// CHECK: validated as canonical ATA in instruction logic
    #[account(mut)]
    pub redemption_vault_token_in_account: UncheckedAccount<'info>,

    /// CHECK: validated as canonical ATA in instruction logic
    #[account(mut)]
    pub redemption_vault_token_out_account: UncheckedAccount<'info>,

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
    pub mint_authority: UncheckedAccount<'info>,

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

    /// CHECK: validated as canonical ONYC offer-vault ATA in instruction logic
    pub offer_vault_onyc_account: UncheckedAccount<'info>,
}

pub fn open_swap_sell<'info>(
    ctx: Context<'info, OpenSwapSell<'info>>,
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
    require!(side == SwapSide::Sell, crate::OnreError::InvalidSwapPair);

    execute_open_swap_sell(ctx, token_in_amount, minimum_out, approval_message)
}

fn execute_open_swap_sell<'info>(
    ctx: Context<'info, OpenSwapSell<'info>>,
    token_in_amount: u64,
    minimum_out: u64,
    approval_message: Option<ApprovalMessage>,
) -> Result<()> {
    let (_, mint_authority_bump) = validate_take_offer_authorities(
        ctx.program_id,
        &ctx.accounts.offer_vault_authority,
        &ctx.accounts.mint_authority,
        &ctx.accounts.instructions_sysvar,
    )?;
    let offer = ctx.accounts.offer.load()?;
    let (market_stats_pda, _) =
        Pubkey::find_program_address(&[seeds::MARKET_STATS], ctx.program_id);
    require_keys_eq!(
        market_stats_pda,
        ctx.accounts.market_stats.key(),
        crate::OnreError::InvalidMarketStatsPda
    );
    let market_stats = read_market_stats_account(&ctx.accounts.market_stats.to_account_info())?;
    let hard_wall_reserve = hard_wall_reserve_from_tvl(
        market_stats.tvl,
        ctx.accounts.prop_amm_state.pool_target_bps,
        ctx.accounts.token_out_mint.decimals,
        ctx.accounts.token_in_mint.decimals,
    )?;
    let redemption_fee_basis_points = redemption_offer_fee_basis_points(
        ctx.program_id,
        &ctx.accounts.redemption_offer,
        ctx.accounts.offer.key(),
        ctx.accounts.token_in_mint.key(),
        ctx.accounts.token_out_mint.key(),
    )?;

    verify_offer_approval(
        &offer,
        &approval_message,
        ctx.program_id,
        &ctx.accounts.user.key(),
        &ctx.accounts.state.approver1,
        &ctx.accounts.state.approver2,
        &ctx.accounts.instructions_sysvar,
    )?;

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
    let boss_token_in_account = get_or_create_associated_token_account(EnsureAtaParams {
        ata_account: &ctx.accounts.boss_token_in_account,
        payer: ctx.accounts.user.to_account_info(),
        authority_account: ctx.accounts.boss.to_account_info(),
        mint_account: ctx.accounts.token_in_mint.to_account_info(),
        token_program: ctx.accounts.token_in_program.to_account_info(),
        associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        authority: ctx.accounts.boss.key(),
        mint: ctx.accounts.token_in_mint.key(),
        token_program_id: ctx.accounts.token_in_program.key(),
        invalid_account_error: crate::OnreError::InvalidBossTokenInAccount,
    })?;
    let redemption_vault_token_in_account =
        get_or_create_associated_token_account(EnsureAtaParams {
            ata_account: &ctx.accounts.redemption_vault_token_in_account,
            payer: ctx.accounts.user.to_account_info(),
            authority_account: ctx.accounts.redemption_vault_authority.to_account_info(),
            mint_account: ctx.accounts.token_in_mint.to_account_info(),
            token_program: ctx.accounts.token_in_program.to_account_info(),
            associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            authority: ctx.accounts.redemption_vault_authority.key(),
            mint: ctx.accounts.token_in_mint.key(),
            token_program_id: ctx.accounts.token_in_program.key(),
            invalid_account_error: crate::OnreError::InvalidVaultTokenInAccount,
        })?;
    let redemption_vault_token_out_account = get_associated_token_account(
        &ctx.accounts.redemption_vault_token_out_account,
        &ctx.accounts.redemption_vault_authority.key(),
        &ctx.accounts.token_out_mint.key(),
        &ctx.accounts.token_out_program.key(),
        crate::OnreError::InvalidVaultTokenOutAccount,
    )?;
    let offer_vault_onyc_account = get_associated_token_account(
        &ctx.accounts.offer_vault_onyc_account,
        &ctx.accounts.offer_vault_authority.key(),
        &ctx.accounts.state.onyc_mint,
        &ctx.accounts.token_in_program.key(),
        crate::OnreError::InvalidOfferVaultOnycAccount,
    )?;

    let mut result = process_redemption_core(
        &offer,
        token_in_amount,
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_out_mint,
        redemption_fee_basis_points,
    )?;
    result.token_out_amount = apply_hard_wall_liquidity_factor(
        result.token_out_amount,
        redemption_vault_token_out_account.amount,
        hard_wall_reserve,
        &ctx.accounts.prop_amm_state,
    )?;
    require!(
        result.token_out_amount >= minimum_out,
        crate::OnreError::MinimumOutNotMet
    );

    transfer_tokens(
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_in_program,
        &user_token_in_account,
        &redemption_vault_token_in_account,
        &ctx.accounts.user.to_account_info(),
        None,
        token_in_amount,
    )?;

    execute_redemption_operations(ExecuteRedemptionOpsParams {
        token_in_program: &ctx.accounts.token_in_program,
        token_out_program: &ctx.accounts.token_out_program,
        token_in_mint: &ctx.accounts.token_in_mint,
        token_in_net_amount: result.token_in_net_amount,
        token_in_fee_amount: result.token_in_fee_amount,
        vault_token_in_account: &redemption_vault_token_in_account,
        boss_token_in_account: &boss_token_in_account,
        fee_destination_token_in_account: &boss_token_in_account,
        redemption_vault_authority: &ctx.accounts.redemption_vault_authority.to_account_info(),
        redemption_vault_authority_bump: ctx.bumps.redemption_vault_authority,
        token_out_mint: &ctx.accounts.token_out_mint,
        token_out_amount: result.token_out_amount,
        vault_token_out_account: &redemption_vault_token_out_account,
        user_token_out_account: &user_token_out_account,
        mint_authority_pda: &ctx.accounts.mint_authority.to_account_info(),
        mint_authority_bump,
        token_out_max_supply: ctx.accounts.state.max_supply,
    })?;

    let main_offer = load_main_offer(
        ctx.program_id,
        &ctx.accounts.main_offer.to_account_info(),
        &ctx.accounts.state,
    )?;
    refresh_market_stats_pda(
        &main_offer,
        &ctx.accounts.token_in_mint,
        &offer_vault_onyc_account.to_account_info(),
        &ctx.accounts.token_in_program,
        &ctx.accounts.market_stats.to_account_info(),
        &ctx.accounts.user.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        ctx.program_id,
    )?;

    msg!(
        "Open swap sell - offer: {}, token_in(+fee): {}(+{}), token_out: {}, user: {}, price: {}",
        ctx.accounts.offer.key(),
        result.token_in_net_amount,
        result.token_in_fee_amount,
        result.token_out_amount,
        ctx.accounts.user.key(),
        u64_to_dec9(result.price)
    );

    emit!(OfferTakenEvent {
        offer_pda: ctx.accounts.offer.key(),
        token_in_amount: result.token_in_net_amount,
        token_out_amount: result.token_out_amount,
        fee_amount: result.token_in_fee_amount,
        user: ctx.accounts.user.key(),
    });

    Ok(())
}
