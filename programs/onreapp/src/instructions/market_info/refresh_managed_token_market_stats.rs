//! Permissionless refresh of a managed token's MarketStats between trades.
use crate::{
    constants::seeds::STATE,
    instructions::market_info::{refresh_modular_market_stats, MarketValuation},
    model::{seeds::MANAGED_TOKEN, ManagedTokenConfig, Pricer},
    state::State,
    utils::managed_token::validate_canonical_usd_pricer,
    OnreError,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

#[event]
pub struct ManagedTokenMarketStatsRefreshedEvent {
    pub managed_mint: Pubkey,
    /// Selected MarketStats account; default when only the ONyc singleton was refreshed.
    pub market_stats: Pubkey,
    /// Legacy `["market_stats"]` singleton refreshed for ONyc; default otherwise.
    pub legacy_market_stats: Pubkey,
    pub timestamp: i64,
    pub slot: u64,
}

#[derive(Accounts)]
pub struct RefreshManagedTokenMarketStats<'info> {
    #[account(seeds = [STATE], bump = state.bump)]
    pub state: Box<Account<'info, State>>,

    #[account(
        seeds = [MANAGED_TOKEN, managed_mint.key().as_ref()],
        bump = managed_token.bump,
        constraint = managed_token.enabled @ OnreError::ConfigurationDisabled,
    )]
    pub managed_token: Box<Account<'info, ManagedTokenConfig>>,

    pub managed_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Canonical USD instance-zero Pricer of the managed mint, the token-wide
    /// valuation source also used by trades and BUFFER settlement.
    pub pricer: Box<Account<'info, Pricer>>,

    /// CHECK: Must equal `ManagedTokenConfig::market_stats` when one is selected;
    /// ignored otherwise.
    #[account(mut)]
    pub market_stats: UncheckedAccount<'info>,

    /// CHECK: Legacy `["market_stats"]` singleton; required for ONyc, ignored for
    /// other mints. Address validated in the handler; created on first use.
    #[account(mut)]
    pub legacy_market_stats: Option<UncheckedAccount<'info>>,

    /// CHECK: ONyc subtracts the canonical cached excluded balance; other mints ignore it.
    pub circulating_supply_excluded_balance: UncheckedAccount<'info>,

    /// Any signer; pays only if the ONyc legacy singleton must be created.
    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Recomputes the managed token's selected MarketStats (and, for ONyc, the legacy
/// singleton) from current supply and the canonical Pricer, so NAV, APY and TVL
/// stay current without trades. Fails when there is nothing to refresh. Not
/// blocked by the kill switch, like the legacy `refresh_market_stats`.
pub fn refresh_managed_token_market_stats(
    ctx: Context<RefreshManagedTokenMarketStats>,
) -> Result<()> {
    let a = &ctx.accounts;
    let mint = a.managed_mint.key();
    validate_canonical_usd_pricer(ctx.program_id, &mint, &a.pricer)?;
    let onyc = mint == a.state.onyc_mint;
    require!(
        onyc || a.managed_token.market_stats().is_some(),
        OnreError::MarketStatsNotSelected
    );
    let legacy = a
        .legacy_market_stats
        .as_ref()
        .map(|account| account.to_account_info());
    refresh_modular_market_stats(
        ctx.program_id,
        &a.state,
        &a.managed_token,
        &a.managed_mint,
        &a.circulating_supply_excluded_balance,
        &a.market_stats,
        legacy.as_ref(),
        Some((
            &a.payer.to_account_info(),
            &a.system_program.to_account_info(),
        )),
        || Ok(MarketValuation::from(&**a.pricer)),
    )?;
    let clock = Clock::get()?;
    emit!(ManagedTokenMarketStatsRefreshedEvent {
        managed_mint: mint,
        market_stats: a.managed_token.market_stats,
        legacy_market_stats: legacy
            .filter(|_| onyc)
            .map(|account| account.key())
            .unwrap_or_default(),
        timestamp: clock.unix_timestamp,
        slot: clock.slot,
    });
    Ok(())
}
