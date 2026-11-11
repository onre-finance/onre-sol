use crate::{
    constants::seeds,
    instructions::{
        buffer::{accrue_buffer::accrue_buffer, BufferState},
        market_info::{load_main_offer, refresh_market_stats_pda},
    },
    model::{
        seeds as model_seeds, ManagedTokenBufferState, ManagedTokenConfig, Pricer,
        PricingDenomination, VaultConfig, VaultKind,
    },
    state::{ConfigurableVault, State},
    utils::{program_controls_mint, transfer_tokens, validate_max_supply},
    OnreError,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

#[event]
pub struct OnycBufferCutOverEvent {
    pub boss: Pubkey,
    pub managed_mint: Pubkey,
    pub legacy_buffer_state: Pubkey,
    pub buffer_state: Pubkey,
    pub reserve_amount: u64,
    pub management_amount: u64,
    pub performance_amount: u64,
    pub previous_supply: u64,
    pub performance_fee_high_watermark: u64,
    pub timestamp: i64,
}

/// One-way, boss-authorized ONyc cutover. Settle the legacy interval, preserve
/// its settings and accounting, move all three balances, then activate native
/// per-mint BUFFER and retire legacy accrual in the same atomic transaction.
#[derive(Accounts)]
pub struct CutOverOnycBuffer<'info> {
    #[account(mut,seeds=[seeds::STATE],bump=state.bump,has_one=boss @ OnreError::InvalidBoss,
        constraint=state.onyc_mint==managed_mint.key() @ OnreError::InvalidOnycMint,
        constraint=!state.is_killed @ OnreError::KillSwitchActivated)]
    pub state: Box<Account<'info, State>>,
    #[account(seeds=[model_seeds::MANAGED_TOKEN,managed_mint.key().as_ref()],bump=managed_token.bump,
        constraint=managed_token.enabled @ OnreError::ConfigurationDisabled,
        constraint=managed_token.managed_mint==managed_mint.key() @ OnreError::InvalidMint,
        constraint=managed_token.decimals==managed_mint.decimals @ OnreError::InvalidMint)]
    pub managed_token: Box<Account<'info, ManagedTokenConfig>>,
    #[account(mut,mint::token_program=token_program,
        constraint=program_controls_mint(&managed_mint,&mint_authority) @ OnreError::ProgramNotMintAuthority)]
    pub managed_mint: Box<InterfaceAccount<'info, Mint>>,
    /// CHECK: Canonical mint signing authority.
    #[account(seeds=[seeds::MINT_AUTHORITY],bump)]
    pub mint_authority: UncheckedAccount<'info>,
    #[account(mut,seeds=[seeds::BUFFER_STATE],bump=legacy_buffer_state.bump,
        constraint=legacy_buffer_state.onyc_mint==managed_mint.key() @ OnreError::InvalidMint)]
    pub legacy_buffer_state: Box<Account<'info, BufferState>>,
    #[account(mut,seeds=[model_seeds::BUFFER,managed_mint.key().as_ref()],bump=buffer_state.bump,
        constraint=buffer_state.managed_mint==managed_mint.key() @ OnreError::InvalidMint,
        constraint=buffer_state.reserve_vault==reserve_vault.key() @ OnreError::InvalidConfigurableVault,
        constraint=buffer_state.management_fee_vault==management_fee_vault.key() @ OnreError::InvalidConfigurableVault,
        constraint=buffer_state.performance_fee_vault==performance_fee_vault.key() @ OnreError::InvalidConfigurableVault)]
    pub buffer_state: Box<Account<'info, ManagedTokenBufferState>>,
    /// CHECK: Canonical legacy NAV source, loaded and checked by handler.
    #[account(address=state.main_offer @ OnreError::InvalidMainOffer)]
    pub main_offer: UncheckedAccount<'info>,
    #[account(seeds=[model_seeds::PRICER,managed_mint.key().as_ref(),&[PricingDenomination::Usd as u8],&0_u64.to_le_bytes()],bump=pricer.bump,
        constraint=pricer.enabled @ OnreError::ConfigurationDisabled,
        constraint=pricer.managed_mint==managed_mint.key() && pricer.denomination==PricingDenomination::Usd @ OnreError::IncompatiblePricer)]
    pub pricer: Box<Account<'info, Pricer>>,
    /// CHECK: Canonical legacy reserve signing identity.
    #[account(seeds=[seeds::RESERVE_VAULT_AUTHORITY],bump)]
    pub legacy_reserve_authority: UncheckedAccount<'info>,
    #[account(seeds=[seeds::CONFIGURABLE_VAULT,seeds::MANAGEMENT_FEE_VAULT],bump=legacy_management_vault.bump,
        constraint=legacy_management_vault.kind==1 @ OnreError::InvalidConfigurableVaultKind)]
    pub legacy_management_vault: Box<Account<'info, ConfigurableVault>>,
    #[account(seeds=[seeds::CONFIGURABLE_VAULT,seeds::PERFORMANCE_FEE_VAULT],bump=legacy_performance_vault.bump,
        constraint=legacy_performance_vault.kind==2 @ OnreError::InvalidConfigurableVaultKind)]
    pub legacy_performance_vault: Box<Account<'info, ConfigurableVault>>,
    #[account(mut,associated_token::mint=managed_mint,associated_token::authority=legacy_reserve_authority,associated_token::token_program=token_program)]
    pub legacy_reserve_token: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut,associated_token::mint=managed_mint,associated_token::authority=legacy_management_vault,associated_token::token_program=token_program)]
    pub legacy_management_token: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut,associated_token::mint=managed_mint,associated_token::authority=legacy_performance_vault,associated_token::token_program=token_program)]
    pub legacy_performance_token: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(seeds=[model_seeds::BUFFER_RESERVE,managed_mint.key().as_ref()],bump=reserve_vault.bump,
        constraint=reserve_vault.kind==VaultKind::BufferReserve && reserve_vault.instance_id==0 @ OnreError::InvalidConfigurableVaultKind)]
    pub reserve_vault: Box<Account<'info, VaultConfig>>,
    #[account(seeds=[model_seeds::BUFFER_MANAGEMENT,managed_mint.key().as_ref()],bump=management_fee_vault.bump,
        constraint=management_fee_vault.kind==VaultKind::Fee && management_fee_vault.instance_id==1 @ OnreError::InvalidConfigurableVaultKind)]
    pub management_fee_vault: Box<Account<'info, VaultConfig>>,
    #[account(seeds=[model_seeds::BUFFER_PERFORMANCE,managed_mint.key().as_ref()],bump=performance_fee_vault.bump,
        constraint=performance_fee_vault.kind==VaultKind::Fee && performance_fee_vault.instance_id==2 @ OnreError::InvalidConfigurableVaultKind)]
    pub performance_fee_vault: Box<Account<'info, VaultConfig>>,
    #[account(init_if_needed,payer=boss,associated_token::mint=managed_mint,associated_token::authority=reserve_vault,associated_token::token_program=token_program)]
    pub reserve_token: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(init_if_needed,payer=boss,associated_token::mint=managed_mint,associated_token::authority=management_fee_vault,associated_token::token_program=token_program)]
    pub management_token: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(init_if_needed,payer=boss,associated_token::mint=managed_mint,associated_token::authority=performance_fee_vault,associated_token::token_program=token_program)]
    pub performance_token: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut)]
    pub boss: Signer<'info>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    /// CHECK: Canonical MarketStats PDA is validated by refresh.
    #[account(mut)]
    pub market_stats: UncheckedAccount<'info>,
    /// CHECK: Canonical excluded-balance cache is validated by refresh.
    pub circulating_supply_excluded_balance: UncheckedAccount<'info>,
}

pub fn cut_over_onyc_buffer(ctx: Context<CutOverOnycBuffer>) -> Result<()> {
    let a = ctx.accounts;
    a.state.require_legacy_buffer_active()?;
    a.legacy_buffer_state.require_active()?;
    let now = Clock::get()?.unix_timestamp;
    let mut offer = load_main_offer(ctx.program_id, &a.main_offer, &a.state)?;
    let (legacy_vector,legacy_nav)=crate::instructions::market_info::offer_valuation_utils::get_active_vector_and_current_price(&offer,now as u64)?;
    let (vector, nav) = crate::utils::pricing::get_active_vector_and_current_price(
        a.pricer.vectors.iter().copied(),
        now as u64,
    )?;
    require!(
        vector.apr == legacy_vector.apr && nav == legacy_nav,
        OnreError::IncompatiblePricer
    );
    let result = accrue_buffer(
        &a.state,
        &mut a.legacy_buffer_state,
        &offer,
        &a.managed_mint,
        a.legacy_reserve_token.to_account_info(),
        a.legacy_management_token.to_account_info(),
        a.legacy_performance_token.to_account_info(),
        a.mint_authority.to_account_info(),
        ctx.bumps.mint_authority,
        &a.token_program,
        now,
    )?;
    validate_max_supply(
        a.managed_mint.supply,
        result.buffer_mint_amount,
        a.managed_token.max_supply,
    )?;
    require!(
        a.managed_token.max_mint_amount == 0
            || result.buffer_mint_amount <= a.managed_token.max_mint_amount,
        OnreError::MaxMintAmountExceeded
    );
    a.managed_mint.reload()?;
    a.legacy_reserve_token.reload()?;
    a.legacy_management_token.reload()?;
    a.legacy_performance_token.reload()?;
    let amounts = [
        a.legacy_reserve_token.amount,
        a.legacy_management_token.amount,
        a.legacy_performance_token.amount,
    ];
    let reserve_bump = [ctx.bumps.legacy_reserve_authority];
    let management_bump = [a.legacy_management_vault.bump];
    let performance_bump = [a.legacy_performance_vault.bump];
    let reserve_seeds: &[&[u8]] = &[seeds::RESERVE_VAULT_AUTHORITY, &reserve_bump];
    let management_seeds: &[&[u8]] = &[
        seeds::CONFIGURABLE_VAULT,
        seeds::MANAGEMENT_FEE_VAULT,
        &management_bump,
    ];
    let performance_seeds: &[&[u8]] = &[
        seeds::CONFIGURABLE_VAULT,
        seeds::PERFORMANCE_FEE_VAULT,
        &performance_bump,
    ];
    for (source, destination, authority, signer_seeds, amount) in [
        (
            &a.legacy_reserve_token,
            &a.reserve_token,
            a.legacy_reserve_authority.to_account_info(),
            reserve_seeds,
            amounts[0],
        ),
        (
            &a.legacy_management_token,
            &a.management_token,
            a.legacy_management_vault.to_account_info(),
            management_seeds,
            amounts[1],
        ),
        (
            &a.legacy_performance_token,
            &a.performance_token,
            a.legacy_performance_vault.to_account_info(),
            performance_seeds,
            amounts[2],
        ),
    ] {
        if amount > 0 {
            transfer_tokens(
                &a.managed_mint,
                &a.token_program,
                source,
                destination,
                &authority,
                Some(&[signer_seeds]),
                amount,
            )?;
        }
    }
    let old = &a.legacy_buffer_state;
    a.buffer_state.previous_supply = old.previous_supply;
    a.buffer_state.performance_fee_high_watermark = old.performance_fee_high_watermark;
    a.buffer_state.last_accrual_timestamp = old.last_accrual_timestamp;
    a.buffer_state.gross_apr = old.gross_apr;
    a.buffer_state.management_fee_basis_points = old.management_fee_basis_points;
    a.buffer_state.performance_fee_basis_points = old.performance_fee_basis_points;
    a.buffer_state.performance_fee_high_watermark_enabled =
        old.performance_fee_high_watermark_enabled;
    // Preserve the initial new vault references/bump while replacing every stale
    // draft accounting value with the just-settled authoritative legacy values.
    a.state.reserved[0] = 1;
    a.legacy_buffer_state.reserved[0] = 1;
    offer.vectors = a.pricer.vectors.map(Into::into);
    refresh_market_stats_pda(
        &offer,
        &a.managed_mint,
        &a.circulating_supply_excluded_balance,
        &a.market_stats,
        &a.boss.to_account_info(),
        &a.system_program.to_account_info(),
        ctx.program_id,
    )?;
    emit!(OnycBufferCutOverEvent {
        boss: a.boss.key(),
        managed_mint: a.managed_mint.key(),
        legacy_buffer_state: a.legacy_buffer_state.key(),
        buffer_state: a.buffer_state.key(),
        reserve_amount: amounts[0],
        management_amount: amounts[1],
        performance_amount: amounts[2],
        previous_supply: a.buffer_state.previous_supply,
        performance_fee_high_watermark: a.buffer_state.performance_fee_high_watermark,
        timestamp: now
    });
    Ok(())
}
