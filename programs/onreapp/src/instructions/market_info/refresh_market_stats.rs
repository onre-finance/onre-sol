use crate::constants::seeds;
use crate::instructions::market_info::{
    load_main_offer, recompute_market_stats, update_market_stats_account,
};
use crate::state::{MarketStats, State};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use anchor_spl::token_interface::{Mint, TokenInterface};

/// Event emitted when the canonical market-stats PDA is refreshed.
#[event]
pub struct MarketStatsRefreshedEvent {
    /// Canonical market-stats PDA.
    pub market_stats_pda: Pubkey,
    /// Offer PDA used for recomputation.
    pub offer_pda: Pubkey,
    /// Unix timestamp of the successful refresh.
    pub timestamp: i64,
    /// Slot of the successful refresh.
    pub slot: u64,
}

/// Account structure for permissionless market-stats refreshes.
#[derive(Accounts)]
pub struct RefreshMarketStats<'info> {
    /// CHECK: Validated in instruction logic against state.main_offer.
    pub main_offer: UncheckedAccount<'info>,

    /// The input mint paired with ONyc for the tracked offer.
    pub token_in_mint: InterfaceAccount<'info, Mint>,

    /// Program state holding the canonical ONyc mint.
    #[account(seeds = [seeds::STATE], bump = state.bump, has_one = onyc_mint)]
    pub state: Box<Account<'info, State>>,

    /// The canonical ONyc mint for global market stats.
    pub onyc_mint: InterfaceAccount<'info, Mint>,

    /// Offer vault authority PDA that owns the ONyc vault ATA.
    /// CHECK: PDA derivation is validated by seeds constraint.
    #[account(seeds = [seeds::OFFER_VAULT_AUTHORITY], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    /// ONyc vault ATA used to exclude vault-held supply from circulation.
    /// CHECK: Address is validated against the canonical ATA derivation.
    #[account(
        constraint = onyc_vault_account.key()
            == get_associated_token_address_with_program_id(
                &vault_authority.key(),
                &onyc_mint.key(),
                &token_program.key(),
            ) @ crate::OnreError::InvalidVaultAccount
    )]
    pub onyc_vault_account: UncheckedAccount<'info>,

    /// Token program that owns the ONyc mint and vault ATA.
    pub token_program: Interface<'info, TokenInterface>,

    /// Canonical global market-stats PDA updated by refreshes and purchases.
    #[account(
        init_if_needed,
        payer = signer,
        space = 8 + MarketStats::INIT_SPACE,
        seeds = [seeds::MARKET_STATS],
        bump
    )]
    pub market_stats: Box<Account<'info, MarketStats>>,

    /// Any signer can pay for PDA creation and trigger a refresh.
    #[account(mut)]
    pub signer: Signer<'info>,

    /// System program for account creation.
    pub system_program: Program<'info, System>,
}

/// Recomputes and writes the canonical market-stats PDA without requiring admin access.
pub fn refresh_market_stats(ctx: Context<RefreshMarketStats>) -> Result<()> {
    let main_offer = load_main_offer(
        ctx.program_id,
        &ctx.accounts.main_offer.to_account_info(),
        &ctx.accounts.state,
    )?;
    require_keys_eq!(
        ctx.accounts.token_in_mint.key(),
        main_offer.token_in_mint,
        crate::OnreError::InvalidTokenInMint
    );
    require_keys_eq!(
        ctx.accounts.onyc_mint.key(),
        main_offer.token_out_mint,
        crate::OnreError::InvalidTokenOutMint
    );
    let snapshot = recompute_market_stats(
        &main_offer,
        &ctx.accounts.onyc_mint,
        &ctx.accounts.onyc_vault_account.to_account_info(),
        &ctx.accounts.token_program,
    )?;

    let market_stats = &mut ctx.accounts.market_stats;
    market_stats.bump = ctx.bumps.market_stats;
    update_market_stats_account(market_stats, snapshot)?;

    let clock = Clock::get()?;
    emit!(MarketStatsRefreshedEvent {
        market_stats_pda: ctx.accounts.market_stats.key(),
        offer_pda: ctx.accounts.main_offer.key(),
        timestamp: clock.unix_timestamp,
        slot: clock.slot,
    });

    Ok(())
}
