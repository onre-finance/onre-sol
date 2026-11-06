use super::{buffer, permissions, quote::quote_offer, settlement, validation, TakeOffer};
use crate::{
    constants::seeds,
    instructions::market_info::{load_main_offer, refresh_market_stats_pda},
    model::{OfferDirection, OfferFlow},
    utils::ApprovalMessage,
    OnreError,
};
use anchor_lang::prelude::*;

/// Accounting for one completed offer execution, in mint base units.
#[event]
pub struct OfferExecutedEvent {
    pub offer_config: Pubkey,
    pub user: Pubkey,
    pub flow: OfferFlow,
    pub direction: OfferDirection,
    pub gross_input_amount: u64,
    pub fee_amount: u64,
    pub net_input_amount: u64,
    pub amount_out: u64,
    /// Selected offer Pricer's USD price, scale 9.
    pub price: u64,
    pub liquidity_refill_amount: u64,
    pub proceeds_amount: u64,
    /// Asset payout from the offer's selected liquidity vault.
    pub liquidity_amount: u64,
    pub timestamp: i64,
}

pub fn take_offer(
    ctx: Context<TakeOffer>,
    gross_input_amount: u64,
    minimum_amount_out: u64,
    deadline: u64,
    approval_message: Option<ApprovalMessage>,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let time = u64::try_from(now).map_err(|_| OnreError::InvalidTimestamp)?;
    require!(time <= deadline, OnreError::Expired);
    let a = ctx.accounts;
    let route = validation::validate(ctx.program_id, a)?;
    // Validate every configured component before dispatch. Future paths must not
    // fall through to NAV minting or silently omit required accounting.
    route.require_supported()?;
    permissions::authorize(ctx.program_id, a, approval_message.as_ref())?;
    let quote = quote_offer(
        &a.pricer,
        &a.fee_config,
        gross_input_amount,
        route.direction,
        a.token_in_mint.decimals,
        a.token_out_mint.decimals,
        time,
    )?;
    require!(
        quote.amount_out >= minimum_amount_out,
        OnreError::MinimumOutNotMet
    );

    let onyc = a.managed_mint().key() == a.state.onyc_mint;
    let max_supply = effective_limit(
        a.managed_token.max_supply,
        onyc.then_some(a.state.max_supply),
    );
    let max_mint = effective_limit(
        a.managed_token.max_mint_amount,
        onyc.then_some(a.state.max_mint_amount),
    );
    let buffer_active = buffer::accrue_before_execution(
        ctx.program_id,
        a,
        ctx.bumps.mint_authority,
        now,
        max_supply,
        max_mint,
    )?;
    a.reload_managed_mint()?;

    settlement::settle(
        a,
        &route,
        &quote,
        ctx.bumps.mint_authority,
        max_supply,
        max_mint,
    )?;
    if buffer_active {
        a.reload_managed_mint()?;
        buffer::store_post_execution_supply(a, now)?;
    }
    if onyc {
        a.reload_managed_mint()?;
        let offer = load_main_offer(
            ctx.program_id,
            a.main_offer.as_ref().ok_or(OnreError::InvalidMainOffer)?,
            &a.state,
        )?;
        require_keys_eq!(
            a.market_stats.key(),
            Pubkey::find_program_address(&[seeds::MARKET_STATS], ctx.program_id).0,
            OnreError::InvalidMarketStatsPda
        );
        refresh_market_stats_pda(
            &offer,
            a.managed_mint(),
            &a.circulating_supply_excluded_balance,
            &a.market_stats,
            &a.user.to_account_info(),
            &a.system_program.to_account_info(),
            ctx.program_id,
        )?;
    }
    emit!(OfferExecutedEvent {
        offer_config: a.offer_config.key(),
        user: a.user.key(),
        flow: a.offer_config.flow,
        direction: route.direction,
        gross_input_amount,
        fee_amount: quote.fee_amount,
        net_input_amount: quote.net_input_amount,
        amount_out: quote.amount_out,
        price: quote.price,
        liquidity_refill_amount: 0,
        proceeds_amount: if route.direction == OfferDirection::AssetToManaged {
            quote.net_input_amount
        } else {
            0
        },
        liquidity_amount: if route.direction == OfferDirection::ManagedToAsset {
            quote.amount_out
        } else {
            0
        },
        timestamp: now,
    });
    Ok(())
}

/// Zero disables a cap; when both apply, retain the stricter one.
fn effective_limit(managed: u64, legacy: Option<u64>) -> u64 {
    match (managed, legacy.unwrap_or(0)) {
        (0, other) | (other, 0) => other,
        (a, b) => a.min(b),
    }
}
