use crate::instructions::buffer::accounts::{
    BufferAccrualAccountsBumps, __client_accounts_buffer_accrual_accounts,
    __cpi_client_accounts_buffer_accrual_accounts,
};
use crate::instructions::buffer::accrue_buffer::{
    accrue_buffer_from_accounts, store_buffer_post_supply,
};
use crate::instructions::buffer::BufferAccrualAccounts;
use crate::instructions::market_info::{
    load_main_offer, read_market_stats_account, refresh_market_stats_pda,
};
use crate::instructions::offer::{
    is_onyc_token_out_mint, should_accrue_onyc_mint, verify_offer_approval,
};
use crate::instructions::Offer;
use crate::state::State;
use crate::utils::{
    get_associated_token_account, get_or_create_associated_token_account, mint_tokens,
    program_controls_mint, transfer_tokens, ApprovalMessage, EnsureAtaParams,
};
use anchor_lang::{prelude::*, Accounts};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenInterface},
};

use super::config::PropAmmState;
use super::quote::{record_prop_amm_buy, validate_canonical_offer, SwapSide};

#[derive(Accounts)]
pub struct OpenSwapBuy<'info> {
    pub offer: AccountLoader<'info, Offer>,

    #[account(
        mut,
        seeds = [crate::constants::seeds::PROP_AMM_STATE],
        bump = prop_amm_state.bump
    )]
    pub prop_amm_state: Account<'info, PropAmmState>,

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
    pub offer_vault_token_in_account: UncheckedAccount<'info>,

    /// CHECK: validated as canonical ATA in instruction logic
    #[account(mut)]
    pub offer_vault_token_out_account: UncheckedAccount<'info>,

    /// CHECK: validated and optionally initialized in instruction logic
    #[account(mut)]
    pub redemption_vault_token_in_account: UncheckedAccount<'info>,

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
    require!(
        offer.allow_permissionless(),
        crate::OnreError::PermissionlessNotAllowed
    );
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
    let _offer_vault_token_in_account = get_associated_token_account(
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

    verify_offer_approval(
        &*ctx.accounts.offer.load()?,
        &approval_message,
        ctx.program_id,
        &ctx.accounts.user.key(),
        &ctx.accounts.state.approver1,
        &ctx.accounts.state.approver2,
        &ctx.accounts.instructions_sysvar,
    )?;

    let (_, permissionless_authority_bump) = Pubkey::find_program_address(
        &[crate::constants::seeds::PERMISSIONLESS_AUTHORITY],
        ctx.program_id,
    );
    let (_, offer_vault_authority_bump) = Pubkey::find_program_address(
        &[crate::constants::seeds::OFFER_VAULT_AUTHORITY],
        ctx.program_id,
    );
    let (_, mint_authority_bump) =
        Pubkey::find_program_address(&[crate::constants::seeds::MINT_AUTHORITY], ctx.program_id);

    let buffer_is_initialized = ctx
        .accounts
        .buffer_accounts
        .check_is_initialized(ctx.program_id)?;
    let should_accrue = should_accrue_onyc_mint(
        &ctx.accounts.state,
        &ctx.accounts.token_out_mint,
        buffer_is_initialized,
        &ctx.accounts.mint_authority.to_account_info(),
    );
    let accrual = if should_accrue {
        Some(accrue_buffer_from_accounts(
            ctx.program_id,
            &ctx.accounts.state,
            &ctx.accounts.buffer_accounts,
            &*ctx.accounts.offer.load()?,
            &mut ctx.accounts.token_out_mint,
            ctx.accounts.mint_authority.to_account_info(),
            mint_authority_bump,
            &ctx.accounts.token_out_program,
        )?)
    } else {
        None
    };

    if accrual.is_some() {
        ctx.accounts.token_out_mint.reload()?;
    }

    transfer_tokens(
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_in_program,
        &user_token_in_account,
        &permissionless_token_in_account,
        &ctx.accounts.user.to_account_info(),
        None,
        token_in_amount,
    )?;

    let target_liquidity = read_market_stats_account(&ctx.accounts.market_stats.to_account_info())
        .map(|market_stats| {
            let target_in_onyc_decimals = (market_stats.tvl as u128)
                .saturating_mul(ctx.accounts.prop_amm_state.pool_target_bps as u128)
                .saturating_div(crate::constants::MAX_BASIS_POINTS as u128);
            target_in_onyc_decimals
                .saturating_mul(10_u128.pow(ctx.accounts.token_in_mint.decimals as u32))
                .saturating_div(10_u128.pow(ctx.accounts.token_out_mint.decimals as u32))
        })
        .unwrap_or(0);
    let current_liquidity = redemption_vault_token_in_account.amount as u128;
    let refill_amount = if target_liquidity > current_liquidity {
        let deficit = target_liquidity - current_liquidity;
        deficit.min(result.token_in_net_amount as u128) as u64
    } else {
        0
    };
    let boss_net_amount = result
        .token_in_net_amount
        .checked_sub(refill_amount)
        .ok_or(crate::OnreError::ArithmeticUnderflow)?;

    let permissionless_signer_seeds: &[&[&[u8]]] = &[&[
        crate::constants::seeds::PERMISSIONLESS_AUTHORITY,
        &[permissionless_authority_bump],
    ]];

    if refill_amount > 0 {
        transfer_tokens(
            &ctx.accounts.token_in_mint,
            &ctx.accounts.token_in_program,
            &permissionless_token_in_account,
            &redemption_vault_token_in_account,
            &ctx.accounts.permissionless_authority.to_account_info(),
            Some(permissionless_signer_seeds),
            refill_amount,
        )?;
    }
    let boss_total_amount = boss_net_amount
        .checked_add(result.token_in_fee_amount)
        .ok_or(crate::OnreError::MathOverflow)?;
    if boss_total_amount > 0 {
        transfer_tokens(
            &ctx.accounts.token_in_mint,
            &ctx.accounts.token_in_program,
            &permissionless_token_in_account,
            &boss_token_in_account,
            &ctx.accounts.permissionless_authority.to_account_info(),
            Some(permissionless_signer_seeds),
            boss_total_amount,
        )?;
    }

    record_prop_amm_buy(
        &mut ctx.accounts.prop_amm_state,
        result.token_in_net_amount,
        Clock::get()?.unix_timestamp,
    )?;

    if program_controls_mint(
        &ctx.accounts.token_out_mint,
        &ctx.accounts.mint_authority.to_account_info(),
    ) {
        let mint_authority_signer_seeds: &[&[&[u8]]] = &[&[
            crate::constants::seeds::MINT_AUTHORITY,
            &[mint_authority_bump],
        ]];
        mint_tokens(
            &ctx.accounts.token_out_program,
            &ctx.accounts.token_out_mint,
            &user_token_out_account.to_account_info(),
            &ctx.accounts.mint_authority.to_account_info(),
            mint_authority_signer_seeds,
            result.token_out_amount,
            ctx.accounts.state.max_supply,
        )?;
    } else {
        let offer_vault_signer_seeds: &[&[&[u8]]] = &[&[
            crate::constants::seeds::OFFER_VAULT_AUTHORITY,
            &[offer_vault_authority_bump],
        ]];
        transfer_tokens(
            &ctx.accounts.token_out_mint,
            &ctx.accounts.token_out_program,
            &offer_vault_token_out_account,
            &permissionless_token_out_account,
            &ctx.accounts.offer_vault_authority.to_account_info(),
            Some(offer_vault_signer_seeds),
            result.token_out_amount,
        )?;
        transfer_tokens(
            &ctx.accounts.token_out_mint,
            &ctx.accounts.token_out_program,
            &permissionless_token_out_account,
            &user_token_out_account,
            &ctx.accounts.permissionless_authority.to_account_info(),
            Some(permissionless_signer_seeds),
            result.token_out_amount,
        )?;
    }

    if let Some(accrual) = accrual {
        let post_offer_supply = accrual
            .post_accrual_supply
            .checked_add(result.token_out_amount)
            .ok_or(crate::OnreError::OverflowError)?;
        store_buffer_post_supply(
            &ctx.accounts.buffer_accounts,
            post_offer_supply,
            accrual.timestamp,
        )?;
    }

    if is_onyc_token_out_mint(&ctx.accounts.state, &ctx.accounts.token_out_mint) {
        let main_offer = load_main_offer(
            ctx.program_id,
            &ctx.accounts.main_offer.to_account_info(),
            &ctx.accounts.state,
        )?;
        ctx.accounts.token_out_mint.reload()?;
        refresh_market_stats_pda(
            &main_offer,
            &ctx.accounts.token_out_mint,
            &offer_vault_token_out_account.to_account_info(),
            &ctx.accounts.token_out_program,
            &ctx.accounts.market_stats.to_account_info(),
            &ctx.accounts.user.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            ctx.program_id,
        )?;
    }

    Ok(())
}
