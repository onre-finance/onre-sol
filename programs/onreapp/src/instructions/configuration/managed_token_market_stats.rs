use crate::{
    constants::seeds::{MARKET_STATS, STATE},
    model::{seeds::MANAGED_TOKEN, ManagedTokenConfig},
    state::{MarketStats, State},
    utils::PdaAccountInit,
    OnreError,
};
use anchor_lang::prelude::*;

#[event]
pub struct ManagedTokenMarketStatsSetEvent {
    pub boss: Pubkey,
    pub managed_mint: Pubkey,
    pub previous: Pubkey,
    pub market_stats: Pubkey,
    pub timestamp: i64,
}

/// Creates a zeroed `["market_stats", mint]` account and selects it for the managed token.
#[derive(Accounts)]
pub struct InitializeManagedTokenMarketStats<'info> {
    #[account(
        mut,
        seeds = [MANAGED_TOKEN, managed_token.managed_mint.as_ref()],
        bump = managed_token.bump,
    )]
    pub managed_token: Box<Account<'info, ManagedTokenConfig>>,

    #[account(
        init,
        payer = boss,
        space = 8 + MarketStats::INIT_SPACE,
        seeds = [MARKET_STATS, managed_token.managed_mint.as_ref()],
        bump
    )]
    pub market_stats: Box<Account<'info, MarketStats>>,

    #[account(
        seeds = [STATE],
        bump = state.bump,
        has_one = boss @ OnreError::InvalidBoss
    )]
    pub state: Box<Account<'info, State>>,

    #[account(mut)]
    pub boss: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_managed_token_market_stats(
    ctx: Context<InitializeManagedTokenMarketStats>,
) -> Result<()> {
    ctx.accounts
        .market_stats
        .set_inner(MarketStats::init_value(ctx.bumps.market_stats));
    select(
        &mut ctx.accounts.managed_token,
        ctx.accounts.market_stats.key(),
        ctx.accounts.boss.key(),
    )
}

/// Selects any existing program-owned MarketStats account. `Account` enforces the
/// program owner and MarketStats discriminator; the address itself is unconstrained.
#[derive(Accounts)]
pub struct SetManagedTokenMarketStats<'info> {
    #[account(
        mut,
        seeds = [MANAGED_TOKEN, managed_token.managed_mint.as_ref()],
        bump = managed_token.bump,
    )]
    pub managed_token: Box<Account<'info, ManagedTokenConfig>>,

    #[account(
        constraint = market_stats.key() != managed_token.market_stats @ OnreError::NoChange
    )]
    pub market_stats: Box<Account<'info, MarketStats>>,

    #[account(
        seeds = [STATE],
        bump = state.bump,
        has_one = boss @ OnreError::InvalidBoss
    )]
    pub state: Box<Account<'info, State>>,

    pub boss: Signer<'info>,
}

pub fn set_managed_token_market_stats(ctx: Context<SetManagedTokenMarketStats>) -> Result<()> {
    select(
        &mut ctx.accounts.managed_token,
        ctx.accounts.market_stats.key(),
        ctx.accounts.boss.key(),
    )
}

/// Clears the managed token's MarketStats selection, disabling modular reporting.
/// The MarketStats account itself is left untouched.
#[derive(Accounts)]
pub struct UnsetManagedTokenMarketStats<'info> {
    #[account(
        mut,
        seeds = [MANAGED_TOKEN, managed_token.managed_mint.as_ref()],
        bump = managed_token.bump,
        constraint = managed_token.market_stats().is_some() @ OnreError::NoChange,
    )]
    pub managed_token: Box<Account<'info, ManagedTokenConfig>>,

    #[account(
        seeds = [STATE],
        bump = state.bump,
        has_one = boss @ OnreError::InvalidBoss
    )]
    pub state: Box<Account<'info, State>>,

    pub boss: Signer<'info>,
}

pub fn unset_managed_token_market_stats(ctx: Context<UnsetManagedTokenMarketStats>) -> Result<()> {
    select(
        &mut ctx.accounts.managed_token,
        Pubkey::default(),
        ctx.accounts.boss.key(),
    )
}

fn select(
    managed_token: &mut ManagedTokenConfig,
    market_stats: Pubkey,
    boss: Pubkey,
) -> Result<()> {
    // Callers' constraints reject a no-op; a freshly created account cannot be selected.
    let previous = managed_token.market_stats;
    managed_token.market_stats = market_stats;
    emit!(ManagedTokenMarketStatsSetEvent {
        boss,
        managed_mint: managed_token.managed_mint,
        previous,
        market_stats,
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
