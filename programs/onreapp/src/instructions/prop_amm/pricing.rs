use crate::constants::{seeds, MAX_BASIS_POINTS};
use crate::instructions::market_info::read_market_stats_account;
use crate::instructions::redemption::load_optional_checked_redemption_offer;
use anchor_lang::prelude::*;

use super::config::{
    PropAmmPairState, CADENCE_WAVE_CAP_DIVISOR, CADENCE_WAVE_EASE, CADENCE_WAVE_SCALE,
    CADENCE_WAVE_STEP, DEFAULT_CADENCE_THRESHOLD, MAX_CADENCE_WAVE_SCALED, WALL_SENSITIVITY_SCALE,
};
use super::hard_wall_math::{
    bps_to_hard_wall_scale, utilization_power_scaled, validate_curve_exponent_scaled,
    HARD_WALL_SCALE,
};

pub(crate) struct RedemptionOfferConfig {
    pub fee_basis_points_prop_amm_sell: u16,
    pub vault_target_bps: u16,
}

pub(crate) struct SellQuoteLiquidityContext {
    pub fee_basis_points_prop_amm_sell: u16,
    pub actual_liquidity: u64,
    pub hard_wall_reserve: u64,
}

pub(crate) fn redemption_offer_config(
    program_id: &Pubkey,
    redemption_offer_account: &UncheckedAccount,
    offer_key: Pubkey,
    token_in_mint: Pubkey,
    token_out_mint: Pubkey,
) -> Result<RedemptionOfferConfig> {
    let Some(redemption_offer) = load_optional_checked_redemption_offer(
        program_id,
        redemption_offer_account,
        offer_key,
        token_in_mint,
        token_out_mint,
    )?
    else {
        return Ok(RedemptionOfferConfig {
            fee_basis_points_prop_amm_sell: 0,
            vault_target_bps: 0,
        });
    };
    redemption_offer.require_enabled()?;

    Ok(RedemptionOfferConfig {
        fee_basis_points_prop_amm_sell: redemption_offer.fee_basis_points_prop_amm_sell,
        vault_target_bps: redemption_offer.vault_target_bps,
    })
}

pub(crate) fn sell_quote_liquidity_context(
    program_id: &Pubkey,
    redemption_offer_account: &UncheckedAccount,
    offer_key: Pubkey,
    token_in_mint: Pubkey,
    token_out_mint: Pubkey,
    market_stats_account: &UncheckedAccount,
    actual_liquidity: u64,
    token_out_decimals: u8,
    onyc_decimals: u8,
) -> Result<SellQuoteLiquidityContext> {
    let redemption_config = redemption_offer_config(
        program_id,
        redemption_offer_account,
        offer_key,
        token_in_mint,
        token_out_mint,
    )?;
    let hard_wall_reserve = resolve_hard_wall_reserve(
        market_stats_account,
        actual_liquidity,
        redemption_config.vault_target_bps,
        token_out_decimals,
        onyc_decimals,
    )?;
    Ok(SellQuoteLiquidityContext {
        fee_basis_points_prop_amm_sell: redemption_config.fee_basis_points_prop_amm_sell,
        actual_liquidity,
        hard_wall_reserve,
    })
}

pub(crate) fn validate_market_stats_pda(
    program_id: &Pubkey,
    market_stats_account: &UncheckedAccount,
) -> Result<()> {
    let (market_stats_pda, _) = Pubkey::find_program_address(&[seeds::MARKET_STATS], program_id);
    require_keys_eq!(
        market_stats_pda,
        market_stats_account.key(),
        crate::OnreError::InvalidMarketStatsPda
    );
    Ok(())
}

pub(crate) fn resolve_hard_wall_reserve(
    market_stats_account: &UncheckedAccount,
    actual_liquidity: u64,
    vault_target_bps: u16,
    token_out_decimals: u8,
    onyc_decimals: u8,
) -> Result<u64> {
    if vault_target_bps == 0 {
        return Ok(actual_liquidity);
    }

    // The sell curve cannot use liquidity that is not actually in the redemption
    // vault. If operators configure a target below the vault's current balance,
    // surplus balance is ignored for pricing by taking min(actual, TVL target).
    let market_stats = read_market_stats_account(&market_stats_account.to_account_info())?;
    let target_reserve = hard_wall_reserve_from_tvl(
        market_stats.tvl,
        vault_target_bps,
        token_out_decimals,
        onyc_decimals,
    )?;
    Ok(actual_liquidity.min(target_reserve))
}

pub(crate) fn apply_hard_wall_liquidity_factor(
    token_out_amount: u64,
    actual_liquidity: u64,
    hard_wall_reserve: u64,
    prop_amm_pair_state: &PropAmmPairState,
) -> Result<u64> {
    let now = Clock::get()?.unix_timestamp;
    apply_hard_wall_liquidity_factor_at_time(
        token_out_amount,
        actual_liquidity,
        hard_wall_reserve,
        prop_amm_pair_state,
        now,
    )
}

pub fn apply_hard_wall_liquidity_factor_at_time(
    token_out_amount: u64,
    actual_liquidity: u64,
    hard_wall_reserve: u64,
    prop_amm_pair_state: &PropAmmPairState,
    now: i64,
) -> Result<u64> {
    require!(actual_liquidity > 0, crate::OnreError::InsufficientBalance);
    require!(hard_wall_reserve > 0, crate::OnreError::InsufficientBalance);
    require!(
        token_out_amount <= actual_liquidity,
        crate::OnreError::InsufficientBalance
    );

    // Final sell output is:
    //   effective_liquidity = min(dynamic_wall(actual, sell_pressure), hard_wall_reserve)
    //   utilization = token_out_amount / effective_liquidity
    //   base_haircut = peg_haircut * utilization^curve_exponent
    //   cadence_target = cadence_wave(utilization, prior_sell_count)
    //   haircut = max(base_haircut, cadence_target)
    //   output = token_out_amount * max(0, 1 - haircut)
    //
    // `actual_liquidity` is the solvency bound. `hard_wall_reserve` is either the
    // same actual balance or min(actual balance, TVL target), depending on
    // redemption-offer configuration. The base exponent power is approximated
    // in hard_wall_math.rs; the cadence target uses integer-only rational math.
    let effective_liquidity = dynamic_wall_liquidity_at_time(
        token_out_amount,
        actual_liquidity,
        hard_wall_reserve,
        prop_amm_pair_state,
        now,
    )?;

    let utilization_scaled = if token_out_amount == 0 {
        0
    } else if token_out_amount == effective_liquidity {
        HARD_WALL_SCALE
    } else {
        (token_out_amount as u128)
            .checked_mul(HARD_WALL_SCALE)
            .ok_or(crate::OnreError::MathOverflow)?
            .checked_div(effective_liquidity as u128)
            .ok_or(crate::OnreError::DivByZero)?
    };
    let base_haircut = redemption_haircut_scaled(
        utilization_scaled,
        prop_amm_pair_state.curve_peg_haircut_bps,
        prop_amm_pair_state.curve_exponent_scaled,
    )?;
    let cadence_wave_y_scaled = cadence_wave_y_for_quote_scaled(prop_amm_pair_state, now)?;
    let cadence_target_haircut =
        cadence_wave_target_haircut_scaled(utilization_scaled, cadence_wave_y_scaled)?;
    let haircut = base_haircut.max(cadence_target_haircut);
    let liquidity_factor = HARD_WALL_SCALE.saturating_sub(haircut);
    let dampened_amount = (token_out_amount as u128)
        .checked_mul(liquidity_factor)
        .ok_or(crate::OnreError::MathOverflow)?
        .checked_div(HARD_WALL_SCALE)
        .ok_or(crate::OnreError::DivByZero)?;

    require!(
        dampened_amount <= u64::MAX as u128,
        crate::OnreError::MathOverflow
    );
    Ok(dampened_amount as u64)
}

pub fn roll_prop_amm_volume_tracker(
    prop_amm_pair_state: &mut PropAmmPairState,
    now: i64,
) -> Result<()> {
    let epoch_duration = prop_amm_pair_state.epoch_duration_seconds;
    require!(epoch_duration > 0, crate::OnreError::InvalidAmount);

    if prop_amm_pair_state.epoch_start == 0 || now < prop_amm_pair_state.epoch_start {
        prop_amm_pair_state.epoch_start = now;
        prop_amm_pair_state.prev_net_sell_value_stable = 0;
        prop_amm_pair_state.curr_sell_value_stable = 0;
        prop_amm_pair_state.curr_buy_value_stable = 0;
        prop_amm_pair_state.curr_sell_trade_count = 0;
        return Ok(());
    }

    let elapsed = now
        .checked_sub(prop_amm_pair_state.epoch_start)
        .ok_or(crate::OnreError::ArithmeticUnderflow)?;
    if elapsed >= epoch_duration.saturating_mul(2) {
        prop_amm_pair_state.prev_net_sell_value_stable = 0;
        prop_amm_pair_state.curr_sell_value_stable = 0;
        prop_amm_pair_state.curr_buy_value_stable = 0;
        prop_amm_pair_state.curr_sell_trade_count = 0;
        prop_amm_pair_state.epoch_start = now;
    } else if elapsed >= epoch_duration {
        prop_amm_pair_state.prev_net_sell_value_stable = prop_amm_pair_state
            .curr_sell_value_stable
            .saturating_sub(prop_amm_pair_state.curr_buy_value_stable);
        prop_amm_pair_state.curr_sell_value_stable = 0;
        prop_amm_pair_state.curr_buy_value_stable = 0;
        prop_amm_pair_state.curr_sell_trade_count = 0;
        prop_amm_pair_state.epoch_start = now;
    }

    Ok(())
}

pub fn record_prop_amm_sell(
    prop_amm_pair_state: &mut PropAmmPairState,
    sell_value_stable: u64,
    now: i64,
) -> Result<()> {
    roll_prop_amm_volume_tracker(prop_amm_pair_state, now)?;
    prop_amm_pair_state.curr_sell_value_stable = prop_amm_pair_state
        .curr_sell_value_stable
        .checked_add(sell_value_stable)
        .ok_or(crate::OnreError::MathOverflow)?;
    prop_amm_pair_state.curr_sell_trade_count = prop_amm_pair_state
        .curr_sell_trade_count
        .checked_add(1)
        .ok_or(crate::OnreError::MathOverflow)?;
    Ok(())
}

pub fn record_prop_amm_buy(
    prop_amm_pair_state: &mut PropAmmPairState,
    buy_value_stable: u64,
    now: i64,
) -> Result<()> {
    roll_prop_amm_volume_tracker(prop_amm_pair_state, now)?;
    prop_amm_pair_state.curr_buy_value_stable = prop_amm_pair_state
        .curr_buy_value_stable
        .checked_add(buy_value_stable)
        .ok_or(crate::OnreError::MathOverflow)?;
    Ok(())
}

pub fn preview_effective_sell_volume(
    prop_amm_pair_state: &PropAmmPairState,
    current_sell_value_stable: u64,
    now: i64,
) -> Result<u64> {
    let epoch_duration = prop_amm_pair_state.epoch_duration_seconds;
    require!(epoch_duration > 0, crate::OnreError::InvalidAmount);

    let current_net = prop_amm_pair_state
        .curr_sell_value_stable
        .saturating_sub(prop_amm_pair_state.curr_buy_value_stable);
    let (prev_net, curr_net, elapsed) =
        if prop_amm_pair_state.epoch_start == 0 || now < prop_amm_pair_state.epoch_start {
            (0_u64, 0_u64, 0_i64)
        } else {
            let elapsed = now
                .checked_sub(prop_amm_pair_state.epoch_start)
                .ok_or(crate::OnreError::ArithmeticUnderflow)?;
            if elapsed >= epoch_duration.saturating_mul(2) {
                (0, 0, 0)
            } else if elapsed >= epoch_duration {
                (current_net, 0, 0)
            } else {
                (
                    prop_amm_pair_state.prev_net_sell_value_stable,
                    current_net,
                    elapsed,
                )
            }
        };

    let remaining = epoch_duration
        .checked_sub(elapsed)
        .ok_or(crate::OnreError::ArithmeticUnderflow)?;
    let decayed_prev = (prev_net as u128)
        .checked_mul(remaining as u128)
        .ok_or(crate::OnreError::MathOverflow)?
        .checked_div(epoch_duration as u128)
        .ok_or(crate::OnreError::DivByZero)?;
    let effective = decayed_prev
        .checked_add(curr_net as u128)
        .and_then(|value| value.checked_add(current_sell_value_stable as u128))
        .ok_or(crate::OnreError::MathOverflow)?;
    require!(
        effective <= u64::MAX as u128,
        crate::OnreError::MathOverflow
    );
    Ok(effective as u64)
}

pub fn dynamic_wall_position(
    actual_liquidity: u64,
    effective_sell_volume: u64,
    wall_sensitivity_scaled: u32,
) -> Result<u64> {
    require!(actual_liquidity > 0, crate::OnreError::InsufficientBalance);
    if effective_sell_volume == 0 {
        return Ok(actual_liquidity);
    }

    // W = L / (1 + sensitivity * V / L)
    //
    // `L` is the current redemption-vault balance and `V` is decayed sell
    // pressure plus this order's raw sell value. Larger sell pressure lowers W,
    // making the same raw sell amount consume more of the wall.
    let sensitivity_component = (wall_sensitivity_scaled as u128)
        .checked_mul(effective_sell_volume as u128)
        .ok_or(crate::OnreError::MathOverflow)?
        .checked_div(actual_liquidity as u128)
        .ok_or(crate::OnreError::DivByZero)?;
    let denominator = WALL_SENSITIVITY_SCALE
        .checked_add(sensitivity_component)
        .ok_or(crate::OnreError::MathOverflow)?;
    let wall = (actual_liquidity as u128)
        .checked_mul(WALL_SENSITIVITY_SCALE)
        .ok_or(crate::OnreError::MathOverflow)?
        .checked_div(denominator)
        .ok_or(crate::OnreError::DivByZero)?;
    require!(wall <= u64::MAX as u128, crate::OnreError::MathOverflow);
    Ok((wall as u64).max(1))
}

pub fn dynamic_wall_liquidity(
    current_sell_value_stable: u64,
    actual_liquidity: u64,
    hard_wall_reserve: u64,
    prop_amm_pair_state: &PropAmmPairState,
) -> Result<u64> {
    let now = Clock::get()?.unix_timestamp;
    dynamic_wall_liquidity_at_time(
        current_sell_value_stable,
        actual_liquidity,
        hard_wall_reserve,
        prop_amm_pair_state,
        now,
    )
}

pub fn dynamic_wall_liquidity_at_time(
    current_sell_value_stable: u64,
    actual_liquidity: u64,
    hard_wall_reserve: u64,
    prop_amm_pair_state: &PropAmmPairState,
    now: i64,
) -> Result<u64> {
    if prop_amm_pair_state.wall_sensitivity_scaled == 0 {
        return Ok(actual_liquidity.min(hard_wall_reserve));
    }

    let effective_sell_volume =
        preview_effective_sell_volume(prop_amm_pair_state, current_sell_value_stable, now)?;
    if effective_sell_volume == 0 {
        require!(actual_liquidity > 0, crate::OnreError::InsufficientBalance);
        return Ok(actual_liquidity.min(hard_wall_reserve));
    }

    let wall_position = dynamic_wall_position(
        actual_liquidity,
        effective_sell_volume,
        prop_amm_pair_state.wall_sensitivity_scaled,
    )?;
    Ok(wall_position.min(hard_wall_reserve))
}

pub fn apply_hard_wall_reserve_curve_with_params(
    token_out_amount: u64,
    actual_liquidity: u64,
    hard_wall_reserve: u64,
    curve_peg_haircut_bps: u16,
    curve_exponent_scaled: u32,
) -> Result<u64> {
    let prop_amm_pair_state = PropAmmPairState {
        curve_peg_haircut_bps,
        curve_exponent_scaled,
        cadence_threshold: DEFAULT_CADENCE_THRESHOLD,
        cadence_wave_scaled: 0,
        epoch_duration_seconds: super::config::DEFAULT_EPOCH_DURATION_SECONDS,
        wall_sensitivity_scaled: 0,
        curr_sell_value_stable: 0,
        curr_buy_value_stable: 0,
        prev_net_sell_value_stable: 0,
        curr_sell_trade_count: 0,
        epoch_start: Clock::get().map(|clock| clock.unix_timestamp).unwrap_or(0),
        bump: 0,
        ..Default::default()
    };
    apply_hard_wall_liquidity_factor_at_time(
        token_out_amount,
        actual_liquidity,
        hard_wall_reserve,
        &prop_amm_pair_state,
        prop_amm_pair_state.epoch_start,
    )
}

pub fn hard_wall_reserve_from_tvl(
    tvl: u64,
    vault_target_bps: u16,
    token_out_decimals: u8,
    onyc_decimals: u8,
) -> Result<u64> {
    let target_in_onyc_decimals = (tvl as u128)
        .checked_mul(vault_target_bps as u128)
        .ok_or(crate::OnreError::MathOverflow)?
        .checked_div(MAX_BASIS_POINTS as u128)
        .ok_or(crate::OnreError::DivByZero)?;
    let token_out_scale = 10_u128
        .checked_pow(token_out_decimals as u32)
        .ok_or(crate::OnreError::MathOverflow)?;
    let onyc_scale = 10_u128
        .checked_pow(onyc_decimals as u32)
        .ok_or(crate::OnreError::MathOverflow)?;
    let target = target_in_onyc_decimals
        .checked_mul(token_out_scale)
        .ok_or(crate::OnreError::MathOverflow)?
        .checked_div(onyc_scale)
        .ok_or(crate::OnreError::DivByZero)?;
    require!(target > 0, crate::OnreError::InsufficientBalance);
    require!(target <= u64::MAX as u128, crate::OnreError::MathOverflow);
    Ok(target as u64)
}

pub(crate) fn redemption_haircut_scaled(
    u: u128,
    curve_peg_haircut_bps: u16,
    curve_exponent_scaled: u32,
) -> Result<u128> {
    let peg_haircut = bps_to_hard_wall_scale(curve_peg_haircut_bps)?;
    if peg_haircut == 0 {
        validate_curve_exponent_scaled(curve_exponent_scaled)?;
        return Ok(0);
    }

    let utilization_power = utilization_power_scaled(u, curve_exponent_scaled)?;
    let curve_haircut = peg_haircut
        .saturating_mul(utilization_power)
        .checked_div(HARD_WALL_SCALE)
        .ok_or(crate::OnreError::DivByZero)?;
    Ok(curve_haircut)
}

pub fn cadence_wave_y_for_quote_scaled(
    prop_amm_pair_state: &PropAmmPairState,
    now: i64,
) -> Result<u128> {
    let threshold = prop_amm_pair_state.cadence_threshold;
    require!(threshold > 0, crate::OnreError::InvalidAmount);
    require!(
        prop_amm_pair_state.cadence_wave_scaled <= MAX_CADENCE_WAVE_SCALED,
        crate::OnreError::InvalidAmount
    );
    require!(
        prop_amm_pair_state
            .cadence_wave_scaled
            .is_multiple_of(CADENCE_WAVE_STEP),
        crate::OnreError::InvalidAmount
    );

    let max_wave_y = prop_amm_pair_state.cadence_wave_scaled as u128;
    if max_wave_y == 0 {
        return Ok(0);
    }

    let quote_trade_count = preview_current_sell_trade_count(prop_amm_pair_state, now)?;
    if quote_trade_count == 0 {
        return Ok(0);
    }

    let ramp = if quote_trade_count >= threshold {
        CADENCE_WAVE_SCALE
    } else {
        (quote_trade_count as u128)
            .checked_mul(CADENCE_WAVE_SCALE)
            .ok_or(crate::OnreError::MathOverflow)?
            .checked_div(threshold as u128)
            .ok_or(crate::OnreError::DivByZero)?
    };
    let wave_y = max_wave_y
        .checked_mul(ramp)
        .ok_or(crate::OnreError::MathOverflow)?
        .checked_div(CADENCE_WAVE_SCALE)
        .ok_or(crate::OnreError::DivByZero)?;
    Ok(wave_y)
}

pub fn cadence_wave_target_haircut_scaled(u: u128, wave_y_scaled: u128) -> Result<u128> {
    if wave_y_scaled == 0 {
        return Ok(0);
    }
    let normalized = u.min(HARD_WALL_SCALE);
    if normalized == 0 {
        return Ok(0);
    }

    let remaining = HARD_WALL_SCALE.saturating_sub(normalized);
    let eased_numerator = normalized.saturating_mul(CADENCE_WAVE_EASE);
    let eased_denominator = eased_numerator
        .checked_add(remaining)
        .ok_or(crate::OnreError::MathOverflow)?;
    let eased_rise = eased_numerator
        .saturating_mul(HARD_WALL_SCALE)
        .checked_div(eased_denominator)
        .ok_or(crate::OnreError::DivByZero)?;
    let target_haircut = eased_rise
        .saturating_mul(wave_y_scaled)
        .checked_div(
            CADENCE_WAVE_SCALE
                .checked_mul(CADENCE_WAVE_CAP_DIVISOR)
                .ok_or(crate::OnreError::MathOverflow)?,
        )
        .ok_or(crate::OnreError::DivByZero)?;
    Ok(target_haircut.min(HARD_WALL_SCALE))
}

fn preview_current_sell_trade_count(
    prop_amm_pair_state: &PropAmmPairState,
    now: i64,
) -> Result<u32> {
    let epoch_duration = prop_amm_pair_state.epoch_duration_seconds;
    require!(epoch_duration > 0, crate::OnreError::InvalidAmount);

    if prop_amm_pair_state.epoch_start == 0 || now < prop_amm_pair_state.epoch_start {
        return Ok(0);
    }
    let elapsed = now
        .checked_sub(prop_amm_pair_state.epoch_start)
        .ok_or(crate::OnreError::ArithmeticUnderflow)?;
    if elapsed >= epoch_duration {
        return Ok(0);
    }
    Ok(prop_amm_pair_state.curr_sell_trade_count)
}
