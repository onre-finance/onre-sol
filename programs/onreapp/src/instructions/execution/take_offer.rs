use super::{
    buffer, liquidity_target, permissions, prop_amm, quote::quote_offer, settlement, validation,
    TakeOffer,
};
use crate::{
    constants::seeds,
    instructions::market_info::{load_main_offer, refresh_market_stats_pda},
    model::{OfferDirection, OfferFlow, QuoterKind},
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
    /// Net asset input routed to the selected liquidity vault before user issuance.
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
    let mut quote = quote_offer(
        &a.pricer,
        &a.fee_config,
        gross_input_amount,
        route.direction,
        a.token_in_mint.decimals,
        a.token_out_mint.decimals,
        time,
    )?;
    if route.kind == QuoterKind::PropAmm {
        let cap = if route.refill_target_bps == 0 {
            route.liquidity_balance
        } else {
            liquidity_target::target(a, route.refill_target_bps, time)?
                .min(route.liquidity_balance as u128) as u64
        };
        quote.amount_out = prop_amm::quote(
            a.prop_amm_state
                .as_ref()
                .ok_or(OnreError::IncompatibleQuoter)?,
            route.direction,
            if route.direction == OfferDirection::AssetToManaged {
                quote.net_input_amount
            } else {
                quote.raw_amount_out
            },
            quote.raw_amount_out,
            route.liquidity_balance,
            cap,
            now,
        )?;
    }
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

    if route.direction == OfferDirection::AssetToManaged && route.refill_target_bps > 0 {
        let target = liquidity_target::target(a, route.refill_target_bps, time)?;
        quote.liquidity_refill_amount = target
            .saturating_sub(route.liquidity_balance as u128)
            .min(quote.net_input_amount as u128) as u64;
    }
    if a.offer_config.pressure_state != Pubkey::default() {
        prop_amm::record(
            a.prop_amm_state
                .as_mut()
                .ok_or(OnreError::IncompatibleQuoter)?,
            route.direction,
            if route.direction == OfferDirection::AssetToManaged {
                quote.net_input_amount
            } else {
                quote.raw_amount_out
            },
            now,
        )?;
    }
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
        liquidity_refill_amount: quote.liquidity_refill_amount,
        proceeds_amount: if route.direction == OfferDirection::AssetToManaged {
            quote.net_input_amount - quote.liquidity_refill_amount
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
