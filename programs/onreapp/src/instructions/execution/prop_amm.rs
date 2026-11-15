//! Symmetric modular Prop AMM pricing/accounting with scheduled epoch boundaries;
//! legacy PropAmmPairState and its tracker remain unchanged.
use crate::{
    instructions::prop_amm::{
        cadence_wave_target_haircut_scaled, pricing::redemption_haircut_scaled, HARD_WALL_SCALE,
    },
    model::{OfferDirection, PropAmmState},
    OnreError,
};
use anchor_lang::prelude::*;

const WALL_SCALE: u128 = 10_000;
const CADENCE_SCALE: u128 = 10_000;

fn elapsed(state: &PropAmmState, now: i64) -> Result<Option<u64>> {
    require!(
        state.config.epoch_duration_seconds > 0,
        OnreError::InvalidAmount
    );
    require!(
        now >= 0 && state.epoch_start >= 0,
        OnreError::InvalidTimestamp
    );
    if state.epoch_start == 0 || now < state.epoch_start {
        Ok(None)
    } else {
        Ok(Some((now - state.epoch_start) as u64))
    }
}

fn net_volume(state: &PropAmmState, direction: OfferDirection) -> u128 {
    match direction {
        OfferDirection::AssetToManaged => state
            .current_buy_value_stable
            .saturating_sub(state.current_sell_value_stable),
        OfferDirection::ManagedToAsset => state
            .current_sell_value_stable
            .saturating_sub(state.current_buy_value_stable),
    }
}

/// Pure directional preview including this trade's asset value, without mutation.
fn effective_volume(
    state: &PropAmmState,
    direction: OfferDirection,
    raw: u64,
    now: i64,
) -> Result<u128> {
    let duration = state.config.epoch_duration_seconds as u128;
    let net = net_volume(state, direction);
    let previous_net = match direction {
        OfferDirection::AssetToManaged => state.previous_net_buy_value_stable,
        OfferDirection::ManagedToAsset => state.previous_net_sell_value_stable,
    };
    let (previous, current, offset) = match elapsed(state, now)? {
        None => (0, 0, 0),
        Some(e) if e as u128 >= duration * 2 => (0, 0, 0),
        Some(e) if e as u128 >= duration => (net, 0, e as u128 - duration),
        Some(e) => (previous_net, net, e as u128),
    };
    // Split the product to retain the full u128 accumulator range during decay.
    let remaining = duration - offset;
    let decayed = (previous / duration)
        .checked_mul(remaining)
        .and_then(|v| v.checked_add((previous % duration) * remaining / duration))
        .ok_or(OnreError::MathOverflow)?;
    decayed
        .checked_add(current)
        .and_then(|v| v.checked_add(raw as u128))
        .ok_or_else(|| OnreError::MathOverflow.into())
}

/// Apply the same asset-denominated curve to either direction's NAV output.
pub(super) fn quote(
    state: &PropAmmState,
    direction: OfferDirection,
    asset_value: u64,
    raw_output: u64,
    actual: u64,
    hard_wall: u64,
    now: i64,
) -> Result<u64> {
    require!(
        actual > 0 && hard_wall > 0 && asset_value <= actual,
        OnreError::InsufficientBalance
    );
    let volume = effective_volume(state, direction, asset_value, now)?;
    let sensitivity = volume
        .checked_mul(state.config.wall_sensitivity_scaled as u128)
        .ok_or(OnreError::MathOverflow)?
        / actual as u128;
    let denominator = WALL_SCALE
        .checked_add(sensitivity)
        .ok_or(OnreError::MathOverflow)?;
    let wall = ((actual as u128 * WALL_SCALE / denominator).max(1) as u64).min(hard_wall);
    let utilization = asset_value as u128 * HARD_WALL_SCALE / wall as u128;
    let base = redemption_haircut_scaled(
        utilization,
        state.config.curve_peg_haircut_bps,
        state.config.curve_exponent_scaled,
    )?;
    let mut wave = 0;
    if matches!(elapsed(state, now)?, Some(e) if e < state.config.epoch_duration_seconds) {
        require!(state.config.cadence_threshold > 0, OnreError::InvalidAmount);
        let count = match direction {
            OfferDirection::AssetToManaged => state.current_buy_trade_count,
            OfferDirection::ManagedToAsset => state.current_sell_trade_count,
        };
        let ramp = (count as u128 * CADENCE_SCALE / state.config.cadence_threshold as u128)
            .min(CADENCE_SCALE);
        wave = state.config.cadence_wave_scaled as u128 * ramp / CADENCE_SCALE;
    }
    let cadence = cadence_wave_target_haircut_scaled(utilization, wave)?;
    let factor = HARD_WALL_SCALE.saturating_sub(base.max(cadence));
    let out = (raw_output as u128 * factor / HARD_WALL_SCALE) as u64;
    require!(out > 0, OnreError::InvalidAmount);
    Ok(out)
}

/// Record successful net asset input or raw NAV sell value and directional cadence.
pub(super) fn record(
    state: &mut PropAmmState,
    direction: OfferDirection,
    value: u64,
    now: i64,
) -> Result<()> {
    roll(state, now)?;
    match direction {
        OfferDirection::AssetToManaged => {
            state.current_buy_value_stable = state
                .current_buy_value_stable
                .checked_add(value as u128)
                .ok_or(OnreError::MathOverflow)?;
            state.current_buy_trade_count = state
                .current_buy_trade_count
                .checked_add(1)
                .ok_or(OnreError::MathOverflow)?;
        }
        OfferDirection::ManagedToAsset => {
            state.current_sell_value_stable = state
                .current_sell_value_stable
                .checked_add(value as u128)
                .ok_or(OnreError::MathOverflow)?;
            state.current_sell_trade_count = state
                .current_sell_trade_count
                .checked_add(1)
                .ok_or(OnreError::MathOverflow)?;
        }
    }
    Ok(())
}

fn roll(state: &mut PropAmmState, now: i64) -> Result<()> {
    let duration = state.config.epoch_duration_seconds;
    let offset = elapsed(state, now)?;
    let start = match offset {
        None => {
            state.previous_net_sell_value_stable = 0;
            state.previous_net_buy_value_stable = 0;
            now
        }
        Some(e) if e >= duration => {
            state.previous_net_buy_value_stable = if e as u128 >= duration as u128 * 2 {
                0
            } else {
                net_volume(state, OfferDirection::AssetToManaged)
            };
            state.previous_net_sell_value_stable = if e as u128 >= duration as u128 * 2 {
                0
            } else {
                state
                    .current_sell_value_stable
                    .saturating_sub(state.current_buy_value_stable)
            };
            now - (e % duration) as i64
        }
        _ => return Ok(()),
    };
    state.current_sell_value_stable = 0;
    state.current_buy_value_stable = 0;
    state.current_sell_trade_count = 0;
    state.current_buy_trade_count = 0;
    state.epoch_start = start;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PropAmmConfig;

    fn state() -> PropAmmState {
        PropAmmState {
            quoter: Pubkey::default(),
            asset_mint: Pubkey::default(),
            managed_mint: Pubkey::default(),
            config: PropAmmConfig {
                epoch_duration_seconds: 100,
                curve_exponent_scaled: 20_000,
                cadence_threshold: 2,
                cadence_wave_scaled: 10_000,
                wall_sensitivity_scaled: 20_000,
                curve_peg_haircut_bps: 700,
            },
            current_sell_value_stable: 100,
            current_buy_value_stable: 0,
            previous_net_sell_value_stable: 20,
            previous_net_buy_value_stable: 0,
            current_buy_trade_count: 0,
            epoch_start: 100,
            current_sell_trade_count: 2,
            bump: 0,
        }
    }

    fn quote_sell(s: &PropAmmState, raw: u64, actual: u64, cap: u64, now: i64) -> Result<u64> {
        quote(
            s,
            OfferDirection::ManagedToAsset,
            raw,
            raw,
            actual,
            cap,
            now,
        )
    }

    #[test]
    fn both_directions_have_identical_factors_and_epoch_behavior() {
        let mut sell = state();
        let mut buy = state();
        buy.current_buy_value_stable = sell.current_sell_value_stable;
        buy.current_sell_value_stable = sell.current_buy_value_stable;
        buy.previous_net_buy_value_stable = sell.previous_net_sell_value_stable;
        buy.previous_net_sell_value_stable = 0;
        buy.current_buy_trade_count = sell.current_sell_trade_count;
        buy.current_sell_trade_count = 0;
        for now in [100, 199, 200, 250, 299, 300, 450] {
            assert_eq!(
                effective_volume(&sell, OfferDirection::ManagedToAsset, 50, now).unwrap(),
                effective_volume(&buy, OfferDirection::AssetToManaged, 50, now).unwrap()
            );
            assert_eq!(
                quote_sell(&sell, 50, 1000, 1000, now).unwrap(),
                quote(
                    &buy,
                    OfferDirection::AssetToManaged,
                    50,
                    50,
                    1000,
                    1000,
                    now
                )
                .unwrap()
            );
        }
        record(&mut sell, OfferDirection::AssetToManaged, 1000, 200).unwrap();
        record(&mut buy, OfferDirection::ManagedToAsset, 1000, 200).unwrap();
        assert_eq!(
            sell.previous_net_sell_value_stable,
            buy.previous_net_buy_value_stable
        );
        assert_eq!(sell.current_buy_trade_count, buy.current_sell_trade_count);
        assert_eq!(
            quote_sell(&sell, 50, 1000, 1000, 200).unwrap(),
            quote(
                &buy,
                OfferDirection::AssetToManaged,
                50,
                50,
                1000,
                1000,
                200
            )
            .unwrap()
        );
        buy.current_buy_trade_count = u32::MAX;
        assert!(record(&mut buy, OfferDirection::AssetToManaged, 1, 200).is_err());
    }

    #[test]
    fn scheduled_boundary_preview_and_recording_agree() {
        for (now, expected) in [
            (199, 110),
            (200, 110),
            (250, 60),
            (299, 11),
            (300, 10),
            (450, 10),
        ] {
            let mut s = state();
            assert_eq!(
                effective_volume(&s, OfferDirection::ManagedToAsset, 10, now).unwrap(),
                expected
            );
            record(&mut s, OfferDirection::AssetToManaged, 0, now).unwrap();
            assert_eq!(
                effective_volume(&s, OfferDirection::ManagedToAsset, 10, now).unwrap(),
                expected
            );
            if now >= 200 {
                assert_eq!(s.epoch_start, now - now % 100);
                assert_eq!(s.current_sell_trade_count, 0);
            }
        }
    }

    #[test]
    fn buy_relief_preserves_cadence_and_cannot_erase_this_sell() {
        let mut s = state();
        let before = effective_volume(&s, OfferDirection::ManagedToAsset, 50, 100).unwrap();
        record(&mut s, OfferDirection::AssetToManaged, 1000, 100).unwrap();
        assert_eq!(s.current_sell_trade_count, 2);
        assert_eq!(
            effective_volume(&s, OfferDirection::ManagedToAsset, 50, 100).unwrap(),
            70
        );
        assert!(effective_volume(&s, OfferDirection::ManagedToAsset, 50, 100).unwrap() < before);
        assert!(quote_sell(&s, 1_000_001, 1_000_000, 1_000_000, 100).is_err());
        assert!(quote_sell(&s, 1, 0, 0, 100).is_err());
    }

    #[test]
    fn curve_matches_reference_golden_vectors() {
        // EVM: test_PropAmmMathMatchesSolanaCurveAndCadenceVectors.
        for (raw, exponent, expected) in [
            (100_000, 25_000, 99_999),
            (5_000_000, 25_000, 4_938_128),
            (5_000_000, 10_000, 4_825_000),
            (5_000_000, 20_000, 4_912_500),
        ] {
            let mut s = state();
            s.current_sell_value_stable = 0;
            s.previous_net_sell_value_stable = 0;
            s.current_sell_trade_count = 0;
            s.config.cadence_wave_scaled = 0;
            s.config.wall_sensitivity_scaled = 1;
            s.config.curve_exponent_scaled = exponent;
            assert_eq!(
                quote_sell(&s, raw, 10_000_000, 10_000_000, 100).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn relief_improves_sell_quote_and_cadence_resets_at_scheduled_boundary() {
        let mut s = state();
        s.current_sell_value_stable = 1_000_000;
        s.previous_net_sell_value_stable = 0;
        s.config.cadence_wave_scaled = 0;
        let before = quote_sell(&s, 100_000, 1_000_000, 1_000_000, 100).unwrap();
        record(&mut s, OfferDirection::AssetToManaged, 1_000_000, 100).unwrap();
        assert!(quote_sell(&s, 100_000, 1_000_000, 1_000_000, 100).unwrap() > before);
        s.current_sell_value_stable = 0;
        s.current_buy_value_stable = 0;
        s.config.cadence_wave_scaled = 50_000;
        let before = quote_sell(&s, 100_000, 1_000_000, 1_000_000, 199).unwrap();
        let after = quote_sell(&s, 100_000, 1_000_000, 1_000_000, 200).unwrap();
        assert!(after > before);
        record(&mut s, OfferDirection::AssetToManaged, 0, 200).unwrap();
        assert_eq!(
            quote_sell(&s, 100_000, 1_000_000, 1_000_000, 200).unwrap(),
            after
        );
    }

    #[test]
    fn counters_widen_beyond_single_transfers_and_overflow_fails() {
        let mut s = state();
        s.current_buy_value_stable = u64::MAX as u128;
        record(&mut s, OfferDirection::AssetToManaged, 1, 100).unwrap();
        assert_eq!(s.current_buy_value_stable, u64::MAX as u128 + 1);
        s.current_buy_value_stable = u128::MAX;
        assert!(record(&mut s, OfferDirection::AssetToManaged, 1, 100).is_err());
        s.previous_net_sell_value_stable = u128::MAX;
        s.current_sell_value_stable = 0;
        assert_eq!(
            effective_volume(&s, OfferDirection::ManagedToAsset, 0, 100).unwrap(),
            u128::MAX
        );
    }
}
