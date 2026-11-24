use super::{
    buffer, liquidity_target, permissions, prop_amm,
    quote::{quote_offer, OfferQuote},
    settlement, validation, Execution, ExecutionAccounts, TakeOffer,
};
use crate::{
    instructions::market_info::{refresh_modular_market_stats, MarketValuation},
    model::{OfferAccess, OfferDirection, OfferExecution, QuoterKind},
    utils::{managed_token::effective_mint_limits, ApprovalMessage},
    OnreError,
};
use anchor_lang::prelude::*;

/// Accounting for one completed offer execution, in mint base units.
#[event]
pub struct OfferExecutedEvent {
    pub offer_config: Pubkey,
    pub user: Pubkey,
    pub access: OfferAccess,
    pub execution: OfferExecution,
    pub direction: OfferDirection,
    pub gross_input_amount: u64,
    pub fee_amount: u64,
    pub net_input_amount: u64,
    pub amount_out: u64,
    /// Selected offer Pricer's USD price, scale 9.
    pub price: u64,
    /// Net asset input refilled into destination custody before output settlement.
    pub liquidity_refill_amount: u64,
    pub proceeds_amount: u64,
    /// Output paid from source inventory, in the output mint's base units.
    pub source_amount: u64,
    /// Net input deposited into destination custody, in input mint base units.
    pub destination_amount: u64,
    /// Managed input retained in the Haircut vault instead of burned (Prop AMM sells).
    pub haircut_amount: u64,
    pub timestamp: i64,
}

/// Executes a permissioned or permissionless Instant offer atomically for the
/// signing user. Worker offers execute only through fulfillment requests.
pub fn take_offer(
    ctx: Context<TakeOffer>,
    gross_input_amount: u64,
    minimum_amount_out: u64,
    deadline: u64,
    approval_message: Option<ApprovalMessage>,
) -> Result<()> {
    let time =
        u64::try_from(Clock::get()?.unix_timestamp).map_err(|_| OnreError::InvalidTimestamp)?;
    require!(time <= deadline, OnreError::Expired);
    let program_id = ctx.program_id;
    let a = ctx.accounts;
    let sysvar = a.instructions_sysvar.clone();
    let has_permission = permissions::holds_permission(
        ctx.remaining_accounts.first(),
        &a.config.offer_config.key(),
        a.user.key,
    );
    execute(
        program_id,
        a,
        ctx.bumps.mint_authority,
        gross_input_amount,
        minimum_amount_out,
        |e| {
            permissions::authorize(
                program_id,
                e.config.offer_config.access,
                &e.user,
                e.state,
                &sysvar,
                approval_message.as_ref(),
                has_permission,
            )
        },
    )?;
    Ok(())
}

/// One offer execution, shared by `take_offer` and worker fills: validates the
/// configuration against the route, authorizes, quotes at the current price and
/// fee, accrues the active BUFFER, records pressure, settles, then refreshes
/// BUFFER supply and market stats. Returns the executed quote.
pub(crate) fn execute<'info, A: ExecutionAccounts<'info>>(
    program_id: &Pubkey,
    a: &mut A,
    authority_bump: u8,
    gross_input_amount: u64,
    minimum_amount_out: u64,
    authorize: impl FnOnce(&Execution) -> Result<()>,
) -> Result<OfferQuote> {
    let now = Clock::get()?.unix_timestamp;
    let time = u64::try_from(now).map_err(|_| OnreError::InvalidTimestamp)?;
    let (route, mut quote, max_supply, max_mint, buffer_active) = {
        let e = a.execution()?;
        let route = validation::validate(program_id, &e, authority_bump)?;
        authorize(&e)?;
        let mut quote = quote_offer(
            &e.config.pricer,
            &e.config.fee_config,
            gross_input_amount,
            route.direction,
            e.token_in_mint.decimals,
            e.token_out_mint.decimals,
            time,
        )?;
        if route.kind == QuoterKind::PropAmm {
            let cap = if route.curve_target_bps == 0 {
                route.curve_balance
            } else {
                liquidity_target::target(&e, route.curve_target_bps, time)?
                    .min(route.curve_balance as u128) as u64
            };
            quote.amount_out = prop_amm::quote(
                e.custody
                    .prop_amm_state
                    .as_ref()
                    .ok_or(OnreError::IncompatibleQuoter)?,
                route.direction,
                if route.direction == OfferDirection::AssetToManaged {
                    quote.net_input_amount
                } else {
                    quote.raw_amount_out
                },
                quote.raw_amount_out,
                route.curve_balance,
                cap,
                now,
            )?;
        }
        require!(
            quote.amount_out >= minimum_amount_out,
            OnreError::MinimumOutNotMet
        );

        let (max_supply, max_mint) = effective_mint_limits(e.state, &e.config.managed_token);
        let buffer_active = buffer::accrue_before_execution(
            program_id,
            &e,
            authority_bump,
            now,
            max_supply,
            max_mint,
        )?;
        (route, quote, max_supply, max_mint, buffer_active)
    };
    a.reload_managed_mint()?;

    let pressure_linked = {
        let e = a.execution()?;
        if route.direction == OfferDirection::AssetToManaged && route.destination_target_bps > 0 {
            let target = liquidity_target::target(&e, route.destination_target_bps, time)?;
            quote.liquidity_refill_amount = target
                .saturating_sub(route.destination_balance as u128)
                .min(quote.net_input_amount as u128)
                as u64;
        }
        e.config.offer_config.pressure_state().is_some()
    };
    if pressure_linked {
        prop_amm::record(
            a.prop_amm_state_mut()
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
        &a.execution()?,
        &route,
        &quote,
        authority_bump,
        max_supply,
        max_mint,
    )?;
    a.reload_managed_mint()?;
    let e = a.execution()?;
    if buffer_active {
        buffer::store_post_execution_supply(program_id, &e, now)?;
    }
    refresh_market_stats(program_id, &e)?;
    let offer = &e.config.offer_config;
    emit!(OfferExecutedEvent {
        offer_config: offer.key(),
        user: e.user,
        access: offer.access,
        execution: offer.execution,
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
        source_amount: if offer.source_vault().is_some() {
            quote.amount_out
        } else {
            0
        },
        destination_amount: match route.direction {
            OfferDirection::AssetToManaged => quote.liquidity_refill_amount,
            OfferDirection::ManagedToAsset if offer.destination_vault().is_some() => {
                quote.net_input_amount
            }
            _ => 0,
        },
        haircut_amount: settlement::retained_haircut(offer, &quote)?,
        timestamp: now,
    });
    Ok(quote)
}

fn refresh_market_stats(program_id: &Pubkey, a: &Execution) -> Result<()> {
    let legacy = a
        .buffer
        .legacy_market_stats
        .as_ref()
        .map(|account| account.to_account_info());
    refresh_modular_market_stats(
        program_id,
        a.state,
        &a.config.managed_token,
        a.managed_mint(),
        &a.buffer.supply_exclusions,
        &a.buffer.market_stats,
        legacy.as_ref(),
        Some((a.payer, &a.system_program.to_account_info())),
        || liquidity_target::canonical_pricer(a).map(|p| MarketValuation::from(&**p)),
    )
}
