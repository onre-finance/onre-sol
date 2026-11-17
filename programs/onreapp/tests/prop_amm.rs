mod common;

use anchor_lang::{AccountDeserialize, AnchorDeserialize};
use common::*;
use onreapp::instructions::prop_amm::{
    apply_hard_wall_liquidity_factor_at_time, apply_hard_wall_reserve_curve_with_params,
    cadence_wave_target_haircut_scaled, cadence_wave_y_for_quote_scaled,
    dynamic_wall_liquidity_at_time, dynamic_wall_position, hard_wall_reserve_from_tvl,
    preview_effective_sell_volume, record_prop_amm_sell, roll_prop_amm_volume_tracker,
    PropAmmPairState, SwapQuote, HARD_WALL_SCALE,
};
use onreapp::state::ConfigurableVaultKind;
use solana_sdk::account::Account;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

const ONE_YEAR_SECONDS: u64 = 31_536_000;

struct PropAmmCtx {
    svm: litesvm::LiteSVM,
    payer: Keypair,
    usdc_mint: Pubkey,
    onyc_mint: Pubkey,
    user: Keypair,
}

fn setup_prop_amm() -> PropAmmCtx {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (offer_pda, _) = find_offer_pda(&usdc_mint, &onyc_mint);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let ix = build_configure_prop_amm_ix(&boss, &usdc_mint, &onyc_mint, true, 700, 25_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (vault_authority, _) = find_offer_vault_authority_pda();
    let (permissionless_authority, _) = find_permissionless_authority_pda();
    create_token_account(&mut svm, &usdc_mint, &vault_authority, 0);
    create_token_account(&mut svm, &onyc_mint, &vault_authority, 10_000_000_000_000);
    create_token_account(&mut svm, &usdc_mint, &permissionless_authority, 0);
    create_token_account(&mut svm, &onyc_mint, &permissionless_authority, 0);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 10_000_000_000);
    create_token_account(&mut svm, &usdc_mint, &boss, 0);

    PropAmmCtx {
        svm,
        payer,
        usdc_mint,
        onyc_mint,
        user,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_configure_prop_amm_with_params_ix(
    boss: &Pubkey,
    asset_mint: &Pubkey,
    onyc_mint: &Pubkey,
    enabled: bool,
    curve_peg_haircut_bps: u16,
    curve_exponent_scaled: u32,
    cadence_threshold: u32,
    cadence_wave_scaled: u32,
    epoch_duration_seconds: i64,
    wall_sensitivity_scaled: u32,
    minimum_sell_haircut_onyc: u64,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (offer_pda, _) = find_offer_pda(asset_mint, onyc_mint);
    let (prop_amm_pair_state_pda, _) = find_prop_amm_pair_state_pda(&offer_pda);
    let mut data = ix_discriminator("configure_prop_amm").to_vec();
    data.push(if enabled { 1 } else { 0 });
    data.extend_from_slice(&curve_peg_haircut_bps.to_le_bytes());
    data.extend_from_slice(&curve_exponent_scaled.to_le_bytes());
    data.extend_from_slice(&cadence_threshold.to_le_bytes());
    data.extend_from_slice(&cadence_wave_scaled.to_le_bytes());
    data.extend_from_slice(&epoch_duration_seconds.to_le_bytes());
    data.extend_from_slice(&wall_sensitivity_scaled.to_le_bytes());
    data.extend_from_slice(&minimum_sell_haircut_onyc.to_le_bytes());
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(offer_pda, false),
            AccountMeta::new_readonly(*asset_mint, false),
            AccountMeta::new(prop_amm_pair_state_pda, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

fn read_prop_amm_pair_state(svm: &litesvm::LiteSVM, offer: &Pubkey) -> PropAmmPairState {
    let (pair_state_pda, _) = find_prop_amm_pair_state_pda(offer);
    let account = svm
        .get_account(&pair_state_pda)
        .expect("Prop AMM pair state not found");
    let mut data = account.data.as_slice();
    PropAmmPairState::try_deserialize(&mut data).expect("failed to deserialize Prop AMM pair state")
}

fn overwrite_pair_state_pubkey(svm: &mut litesvm::LiteSVM, offer: &Pubkey, offset: usize) {
    let (pair_state_pda, _) = find_prop_amm_pair_state_pda(offer);
    let mut account = svm
        .get_account(&pair_state_pda)
        .expect("Prop AMM pair state not found");
    account.data[offset..offset + 32].copy_from_slice(Pubkey::new_unique().as_ref());
    svm.set_account(pair_state_pda, account).unwrap();
}

fn get_balance_or_zero(svm: &litesvm::LiteSVM, ata: &Pubkey) -> u64 {
    if svm.get_account(ata).is_some() {
        get_token_balance(svm, ata)
    } else {
        0
    }
}

fn add_prop_amm_vector(ctx: &mut PropAmmCtx) {
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86_400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
}

fn configure_minimum_sell_haircut(ctx: &mut PropAmmCtx, minimum_sell_haircut_onyc: u64) {
    let boss = ctx.payer.pubkey();
    let ix = build_configure_prop_amm_with_params_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        true,
        700,
        25_000,
        20,
        10_000,
        86_400,
        20_000,
        minimum_sell_haircut_onyc,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
}

fn prepare_prop_amm_sell_side(ctx: &mut PropAmmCtx, redemption_fee_bps: u16) {
    let boss = ctx.payer.pubkey();
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &ctx.onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    add_prop_amm_vector(ctx);
    configure_minimum_sell_haircut(ctx, 0);

    if redemption_fee_bps > 0 {
        let ix = build_make_redemption_offer_ix(
            &boss,
            &ctx.onyc_mint,
            &ctx.usdc_mint,
            redemption_fee_bps,
            &TOKEN_PROGRAM_ID,
            &TOKEN_PROGRAM_ID,
        );
        send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    }

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(
        &mut ctx.svm,
        &ctx.usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    create_token_account(
        &mut ctx.svm,
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        2_000_000_000_000,
    );
    let ix = build_refresh_market_stats_ix(&boss, &ctx.usdc_mint, &ctx.onyc_mint);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
}

fn initialize_prop_amm_buffer(ctx: &mut PropAmmCtx, gross_yield: u64) {
    let boss = ctx.payer.pubkey();
    let (offer_pda, _) = find_offer_pda(&ctx.usdc_mint, &ctx.onyc_mint);
    let ix = build_initialize_buffer_ix(&boss, &offer_pda, &ctx.onyc_mint);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    let ix = build_set_buffer_gross_yield_ix(&boss, &offer_pda, &ctx.onyc_mint, gross_yield);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
}

#[test]
fn test_hard_wall_curve_is_vulnerable_to_order_splitting() {
    let hard_wall_reserve = 10_000_000;
    let one_shot = apply_hard_wall_reserve_curve_with_params(
        5_000_000,
        10_000_000,
        hard_wall_reserve,
        700,
        25_000,
    )
    .unwrap();

    let mut split_total = 0_u64;
    let mut current_liquidity = 10_000_000_u64;
    for _ in 0..5 {
        let output = apply_hard_wall_reserve_curve_with_params(
            1_000_000,
            current_liquidity,
            hard_wall_reserve,
            700,
            25_000,
        )
        .unwrap();
        split_total += output;
        current_liquidity -= output;
    }

    assert_eq!(one_shot, 4_938_128);
    assert_eq!(split_total, 4_997_768);
}

#[test]
fn test_hard_wall_curve_approximation_tracks_exact_curve() {
    let actual_liquidity = 50_000_000;
    let hard_wall_reserve = 1_000_000;
    let peg_haircut_bps = 700;
    let exponents = [
        1_000_u32, 2_000, 5_000, 10_000, 15_000, 20_000, 24_000, 25_000, 30_000, 50_000, 100_000,
    ];
    let token_out_amounts = [
        1_000_u64, 5_000, 10_000, 25_000, 50_000, 100_000, 250_000, 500_000, 800_000, 1_000_000,
        1_250_000, 1_500_000, 2_000_000, 3_000_000, 5_000_000, 10_000_000, 25_000_000, 50_000_000,
    ];

    for exponent in exponents {
        for token_out_amount in token_out_amounts {
            let approximate = apply_hard_wall_reserve_curve_with_params(
                token_out_amount,
                actual_liquidity,
                hard_wall_reserve,
                peg_haircut_bps,
                exponent,
            )
            .unwrap();
            let exact = exact_hard_wall_output(
                token_out_amount,
                hard_wall_reserve,
                peg_haircut_bps,
                exponent,
            );
            assert!(
                approximate.abs_diff(exact) <= 2,
                "approximation drifted for token_out={token_out_amount}, exponent={exponent}: approximate={approximate}, exact={exact}"
            );
        }
    }
}

#[test]
fn test_hard_wall_curve_ignores_surplus_above_target_reserve() {
    let hard_wall_reserve = 5_000_000;
    let raw_sell_value_stable = 1_000_000;
    let at_target = apply_hard_wall_reserve_curve_with_params(
        raw_sell_value_stable,
        hard_wall_reserve,
        hard_wall_reserve,
        700,
        25_000,
    )
    .unwrap();
    let above_target = apply_hard_wall_reserve_curve_with_params(
        raw_sell_value_stable,
        10_000_000,
        hard_wall_reserve,
        700,
        25_000,
    )
    .unwrap();

    assert_eq!(above_target, at_target);
}

fn exact_hard_wall_output(
    token_out_amount: u64,
    effective_liquidity: u64,
    curve_peg_haircut_bps: u16,
    curve_exponent_scaled: u32,
) -> u64 {
    let utilization = (token_out_amount as u128)
        .checked_mul(HARD_WALL_SCALE)
        .unwrap()
        .checked_div(effective_liquidity as u128)
        .unwrap();
    let peg_haircut = HARD_WALL_SCALE
        .checked_mul(curve_peg_haircut_bps as u128)
        .unwrap()
        .checked_div(10_000)
        .unwrap();
    let utilization_power = exact_utilization_power_scaled(utilization, curve_exponent_scaled);
    let haircut = peg_haircut
        .saturating_mul(utilization_power)
        .checked_div(HARD_WALL_SCALE)
        .unwrap();
    let liquidity_factor = HARD_WALL_SCALE.saturating_sub(haircut);
    let dampened = (token_out_amount as u128)
        .checked_mul(liquidity_factor)
        .unwrap()
        .checked_div(HARD_WALL_SCALE)
        .unwrap();
    dampened as u64
}

fn exact_utilization_power_scaled(u: u128, exponent_scaled: u32) -> u128 {
    if exponent_scaled == 0 {
        return HARD_WALL_SCALE;
    }

    let tenths = exponent_scaled / 1_000;
    let tenth_root = exact_tenth_root_scaled(u);
    let mut value = HARD_WALL_SCALE;
    for _ in 0..tenths {
        value = exact_mul_scaled(value, tenth_root);
    }
    value
}

fn exact_tenth_root_scaled(value: u128) -> u128 {
    if value <= 1 || value == HARD_WALL_SCALE {
        return value;
    }

    let mut left = 1_u128;
    let mut right = value.max(HARD_WALL_SCALE);
    let mut answer = 1_u128;
    while left <= right {
        let mid = left + (right - left) / 2;
        if exact_pow_scaled_lte(mid, 10, value) {
            answer = mid;
            left = mid + 1;
        } else {
            right = mid - 1;
        }
    }
    answer
}

fn exact_pow_scaled_lte(base: u128, exponent: u32, limit: u128) -> bool {
    let mut value = HARD_WALL_SCALE;
    if base >= HARD_WALL_SCALE {
        for _ in 0..exponent {
            value = exact_mul_scaled(value, base);
            if value > limit {
                return false;
            }
        }
        return true;
    }

    for _ in 0..exponent {
        value = exact_mul_scaled(value, base);
        if value <= limit {
            return true;
        }
    }
    value <= limit
}

fn exact_mul_scaled(lhs: u128, rhs: u128) -> u128 {
    lhs.saturating_mul(rhs)
        .checked_div(HARD_WALL_SCALE)
        .unwrap_or(u128::MAX)
}

#[test]
fn test_hard_wall_curve_allows_zero_output_at_actual_vault_limit() {
    let state = PropAmmPairState {
        curve_peg_haircut_bps: 700,
        curve_exponent_scaled: 25_000,
        cadence_threshold: 20,
        cadence_wave_scaled: 10_000,
        epoch_duration_seconds: 86_400,
        wall_sensitivity_scaled: 20_000,
        curr_sell_value_stable: 0,
        curr_buy_value_stable: 0,
        prev_net_sell_value_stable: 0,
        curr_sell_trade_count: 0,
        epoch_start: 1,
        bump: 0,
        ..Default::default()
    };
    let output =
        apply_hard_wall_liquidity_factor_at_time(10_000_000, 10_000_000, 10_000_000, &state, 1)
            .unwrap();

    assert_eq!(output, 0);
}

#[test]
fn test_hard_wall_curve_saturates_extreme_utilization() {
    let output = apply_hard_wall_reserve_curve_with_params(
        1_000_000_000_000,
        1_000_000_000_000,
        1,
        1,
        32_000,
    )
    .unwrap();

    assert_eq!(output, 0);
}

#[test]
fn test_hard_wall_curve_rejects_raw_value_above_actual_vault() {
    let result =
        apply_hard_wall_reserve_curve_with_params(10_000_001, 10_000_000, 10_000_000, 700, 25_000);

    assert!(result.is_err());
}

#[test]
fn test_hard_wall_liquidity_rejects_output_above_actual_liquidity() {
    let state = PropAmmPairState {
        curve_peg_haircut_bps: 700,
        curve_exponent_scaled: 25_000,
        cadence_threshold: 20,
        cadence_wave_scaled: 10_000,
        epoch_duration_seconds: 86_400,
        wall_sensitivity_scaled: 20_000,
        epoch_start: 1,
        bump: 0,
        ..Default::default()
    };

    let result =
        apply_hard_wall_liquidity_factor_at_time(10_000_001, 10_000_000, 10_000_000, &state, 1);

    assert!(result.is_err());
}

#[test]
fn test_hard_wall_reserve_from_tvl_scales_to_token_out_decimals() {
    assert_eq!(
        hard_wall_reserve_from_tvl(2_000_000_000_000, 1_500, 6, 9).unwrap(),
        300_000_000
    );
    assert!(hard_wall_reserve_from_tvl(1, 1, 6, 9).is_err());
}

#[test]
fn test_dynamic_wall_preview_includes_current_sell_and_buy_relief() {
    let state = PropAmmPairState {
        curve_peg_haircut_bps: 700,
        curve_exponent_scaled: 25_000,
        cadence_threshold: 20,
        cadence_wave_scaled: 10_000,
        epoch_duration_seconds: 86_400,
        wall_sensitivity_scaled: 20_000,
        curr_sell_value_stable: 500,
        curr_buy_value_stable: 100,
        prev_net_sell_value_stable: 1_000,
        curr_sell_trade_count: 0,
        epoch_start: 1_000,
        bump: 0,
        ..Default::default()
    };

    let effective = preview_effective_sell_volume(&state, 200, 44_200).unwrap();
    assert_eq!(effective, 1_100);
}

#[test]
fn test_dynamic_wall_position_uses_effective_sell_pressure() {
    assert_eq!(
        dynamic_wall_position(15_000_000, 0, 20_000).unwrap(),
        15_000_000
    );
    assert_eq!(
        dynamic_wall_position(15_000_000, 15_000_000, 20_000).unwrap(),
        5_000_000
    );
    assert_eq!(
        dynamic_wall_position(15_000_000, 30_000_000, 20_000).unwrap(),
        3_000_000
    );
}

#[test]
fn test_dynamic_wall_liquidity_matches_graph_dynamic_wall() {
    let state = PropAmmPairState {
        epoch_duration_seconds: 86_400,
        wall_sensitivity_scaled: 20_000,
        epoch_start: 1,
        ..Default::default()
    };

    let liquidity =
        dynamic_wall_liquidity_at_time(100_000, 10_000_000_000, 20_000_000_000, &state, 1).unwrap();

    assert_eq!(liquidity, 10_000_000_000);

    let capped_liquidity =
        dynamic_wall_liquidity_at_time(100_000, 10_000_000_000, 200_000, &state, 1).unwrap();

    assert_eq!(capped_liquidity, 200_000);
}

#[test]
fn test_prop_amm_volume_tracker_rolls_and_resets_epochs() {
    let mut state = PropAmmPairState {
        epoch_duration_seconds: 100,
        epoch_start: 1_000,
        curr_sell_value_stable: 1_000,
        curr_buy_value_stable: 250,
        prev_net_sell_value_stable: 125,
        curr_sell_trade_count: 7,
        ..Default::default()
    };

    roll_prop_amm_volume_tracker(&mut state, 1_100).unwrap();
    assert_eq!(state.epoch_start, 1_100);
    assert_eq!(state.prev_net_sell_value_stable, 750);
    assert_eq!(state.curr_sell_value_stable, 0);
    assert_eq!(state.curr_buy_value_stable, 0);
    assert_eq!(state.curr_sell_trade_count, 0);

    state.curr_sell_value_stable = 500;
    state.curr_buy_value_stable = 100;
    state.curr_sell_trade_count = 3;
    roll_prop_amm_volume_tracker(&mut state, 1_300).unwrap();
    assert_eq!(state.epoch_start, 1_300);
    assert_eq!(state.prev_net_sell_value_stable, 0);
    assert_eq!(state.curr_sell_value_stable, 0);
    assert_eq!(state.curr_buy_value_stable, 0);
    assert_eq!(state.curr_sell_trade_count, 0);
}

#[test]
fn test_prop_amm_volume_tracker_uses_fixed_epoch_boundaries() {
    // Same rules as the modular tracker and the EVM Prop AMM: D=100, net sells of
    // 100 during [100, 200). Solana treats epoch_start 0 as unstarted, so the EVM
    // example's timeline is shifted by 100.
    let mut state = PropAmmPairState {
        cadence_threshold: 10,
        cadence_wave_scaled: 10_000,
        epoch_duration_seconds: 100,
        epoch_start: 100,
        curr_sell_value_stable: 100,
        curr_sell_trade_count: 5,
        ..Default::default()
    };
    // Previous-epoch pressure decays from the scheduled boundary, without a roll.
    for (now, expected) in [(200, 100), (250, 50), (299, 1), (300, 0)] {
        assert_eq!(
            preview_effective_sell_volume(&state, 0, now).unwrap(),
            expected
        );
    }
    assert_eq!(preview_effective_sell_volume(&state, 20, 299).unwrap(), 21);
    // Cadence counts expire at the same boundary.
    assert!(cadence_wave_y_for_quote_scaled(&state, 199).unwrap() > 0);
    assert_eq!(cadence_wave_y_for_quote_scaled(&state, 200).unwrap(), 0);

    // A late sell keeps the epoch aligned at 200 instead of restarting at 299.
    record_prop_amm_sell(&mut state, 20, 299).unwrap();
    assert_eq!(state.epoch_start, 200);
    assert_eq!(state.prev_net_sell_value_stable, 100);
    assert_eq!(state.curr_sell_value_stable, 20);
    assert_eq!(state.curr_sell_trade_count, 1);
    assert_eq!(preview_effective_sell_volume(&state, 0, 299).unwrap(), 21);
    // The old 100 is gone at 300; the new 20 then starts its own decay.
    assert_eq!(preview_effective_sell_volume(&state, 0, 300).unwrap(), 20);
    assert_eq!(preview_effective_sell_volume(&state, 0, 350).unwrap(), 10);

    // After two or more elapsed epochs history expires but alignment is preserved.
    roll_prop_amm_volume_tracker(&mut state, 1_234).unwrap();
    assert_eq!(state.epoch_start, 1_200);
    assert_eq!(state.prev_net_sell_value_stable, 0);
    assert_eq!(state.curr_sell_value_stable, 0);
    assert_eq!(state.curr_sell_trade_count, 0);
}

#[test]
fn test_cadence_wave_y_ramps_to_configured_maximum() {
    let state = PropAmmPairState {
        curve_peg_haircut_bps: 7_000,
        curve_exponent_scaled: 25_000,
        cadence_threshold: 20,
        cadence_wave_scaled: 10_000,
        epoch_duration_seconds: 86_400,
        wall_sensitivity_scaled: 20_000,
        curr_sell_value_stable: 0,
        curr_buy_value_stable: 0,
        prev_net_sell_value_stable: 0,
        curr_sell_trade_count: 0,
        epoch_start: 1,
        bump: 0,
        ..Default::default()
    };
    let mut half_cadence = state.clone();
    half_cadence.curr_sell_trade_count = 10;
    let mut high_cadence = state.clone();
    high_cadence.curr_sell_trade_count = 49;
    let mut threshold_cadence = state.clone();
    threshold_cadence.curr_sell_trade_count = 20;

    assert_eq!(cadence_wave_y_for_quote_scaled(&state, 1).unwrap(), 0);
    assert_eq!(
        cadence_wave_y_for_quote_scaled(&half_cadence, 1).unwrap(),
        5_000
    );
    assert_eq!(
        cadence_wave_y_for_quote_scaled(&threshold_cadence, 1).unwrap(),
        10_000
    );
    assert_eq!(
        cadence_wave_y_for_quote_scaled(&high_cadence, 1).unwrap(),
        10_000
    );

    let mut invalid_legacy_state = state;
    invalid_legacy_state.cadence_wave_scaled = 100_000;
    assert!(cadence_wave_y_for_quote_scaled(&invalid_legacy_state, 1).is_err());
}

#[test]
fn test_cadence_wave_target_matches_explorer_integer_vectors() {
    let vectors = [
        (0, 0),
        (10_000_000_000, 24_922_118_380),
        (100_000_000_000, 156_862_745_098),
        (250_000_000_000, 242_424_242_424),
        (500_000_000_000, 296_296_296_296),
        (750_000_000_000, 320_000_000_000),
        (1_000_000_000_000, 333_333_333_333),
        (2_000_000_000_000, 333_333_333_333),
    ];

    for (utilization, expected_haircut) in vectors {
        assert_eq!(
            cadence_wave_target_haircut_scaled(utilization, 10_000).unwrap(),
            expected_haircut,
            "explorer mismatch at utilization={utilization}"
        );
    }
    assert_eq!(
        cadence_wave_target_haircut_scaled(250_000_000_000, 50_000).unwrap(),
        HARD_WALL_SCALE
    );
}

#[test]
fn test_cadence_penalizes_small_split_sells() {
    let state = PropAmmPairState {
        curve_peg_haircut_bps: 7_000,
        curve_exponent_scaled: 25_000,
        cadence_threshold: 20,
        cadence_wave_scaled: 10_000,
        epoch_duration_seconds: 86_400,
        wall_sensitivity_scaled: 0,
        curr_sell_value_stable: 0,
        curr_buy_value_stable: 0,
        prev_net_sell_value_stable: 0,
        curr_sell_trade_count: 0,
        epoch_start: 1,
        bump: 0,
        ..Default::default()
    };
    let mut high_cadence = state.clone();
    high_cadence.curr_sell_trade_count = 49;

    let low_cadence_output =
        apply_hard_wall_liquidity_factor_at_time(100_000, 10_000_000, 10_000_000, &state, 1)
            .unwrap();
    let high_cadence_output =
        apply_hard_wall_liquidity_factor_at_time(100_000, 10_000_000, 10_000_000, &high_cadence, 1)
            .unwrap();

    assert_eq!(low_cadence_output, 99_999);
    assert_eq!(high_cadence_output, 97_507);
}

#[test]
fn test_quote_swap_returns_expected_quote_data() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86_400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.usdc_mint, &ctx.onyc_mint, 1_000_000);
    let metadata = send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    let quote = SwapQuote::try_from_slice(get_return_data(&metadata)).unwrap();

    assert_eq!(
        quote.offer,
        find_offer_pda(&ctx.usdc_mint, &ctx.onyc_mint).0
    );
    assert_eq!(quote.token_in_amount, 1_000_000);
    assert_eq!(quote.token_in_net_amount, 1_000_000);
    assert_eq!(quote.token_in_fee_amount, 0);
    assert_eq!(quote.token_out_amount, 1_000_000_000);
    assert_eq!(quote.minimum_out, quote.token_out_amount);
}

#[test]
fn test_quote_swap_rejects_invalid_token_pairs() {
    let mut ctx = setup_prop_amm();
    let eurc_mint = create_mint(&mut ctx.svm, &ctx.payer, 6, &ctx.payer.pubkey());

    let mut same_mint_ix =
        build_quote_swap_ix(&ctx.onyc_mint, &ctx.usdc_mint, &ctx.onyc_mint, 1_000_000);
    same_mint_ix.accounts[4] = AccountMeta::new_readonly(ctx.usdc_mint, false);
    assert!(
        send_tx(&mut ctx.svm, &[same_mint_ix], &[&ctx.payer]).is_err(),
        "quote should reject identical token in/out mints"
    );

    let mut no_onyc_ix =
        build_quote_swap_ix(&ctx.onyc_mint, &ctx.usdc_mint, &ctx.onyc_mint, 1_000_000);
    no_onyc_ix.accounts[4] = AccountMeta::new_readonly(eurc_mint, false);
    assert!(
        send_tx(&mut ctx.svm, &[no_onyc_ix], &[&ctx.payer]).is_err(),
        "quote should reject pairs that do not include ONYC"
    );
}

#[test]
fn test_dynamic_wall_accumulates_sell_pressure_and_buys_relieve_it() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &ctx.onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86_400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    configure_minimum_sell_haircut(&mut ctx, 0);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(
        &mut ctx.svm,
        &ctx.usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    create_token_account(
        &mut ctx.svm,
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        2_000_000_000_000,
    );
    let ix = build_refresh_market_stats_ix(&boss, &ctx.usdc_mint, &ctx.onyc_mint);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let sell_amount = 2_000_000_000_000;
    let quote_ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.onyc_mint, &ctx.usdc_mint, sell_amount);
    let quote_metadata = send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).unwrap();
    let first_quote = SwapQuote::try_from_slice(get_return_data(&quote_metadata)).unwrap();

    let sell_ix = build_open_swap_sell_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        sell_amount,
        first_quote.minimum_out,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[sell_ix], &[&ctx.payer, &ctx.user]).unwrap();

    advance_slot(&mut ctx.svm);
    let quote_ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.onyc_mint, &ctx.usdc_mint, sell_amount);
    let quote_metadata = send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).unwrap();
    let pressured_quote = SwapQuote::try_from_slice(get_return_data(&quote_metadata)).unwrap();
    assert_eq!(first_quote.token_out_amount, 1_994_192_046);
    assert_eq!(pressured_quote.token_out_amount, 1_970_377_785);

    let buy_quote_ix = build_quote_swap_ix(
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000_000,
    );
    let buy_quote_metadata = send_tx(&mut ctx.svm, &[buy_quote_ix], &[&ctx.payer]).unwrap();
    let buy_quote = SwapQuote::try_from_slice(get_return_data(&buy_quote_metadata)).unwrap();
    let buy_ix = build_open_swap_buy_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000_000,
        buy_quote.minimum_out,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[buy_ix], &[&ctx.payer, &ctx.user]).unwrap();

    advance_slot(&mut ctx.svm);
    let quote_ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.onyc_mint, &ctx.usdc_mint, sell_amount);
    let quote_metadata = send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).unwrap();
    let relieved_quote = SwapQuote::try_from_slice(get_return_data(&quote_metadata)).unwrap();
    assert_eq!(relieved_quote.token_out_amount, 1_971_289_607);
}

#[test]
fn test_prop_amm_pair_must_be_enabled() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();

    let ix = build_configure_prop_amm_ix(&boss, &ctx.usdc_mint, &ctx.onyc_mint, false, 700, 25_000);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let quote_ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.usdc_mint, &ctx.onyc_mint, 1_000_000);
    let result = send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]);
    assert!(
        result.is_err(),
        "disabled Prop AMM pair should reject quotes"
    );
}

#[test]
fn test_configure_prop_amm_rejects_non_boss() {
    let mut ctx = setup_prop_amm();
    let unauthorized = Keypair::new();
    ctx.svm
        .airdrop(&unauthorized.pubkey(), INITIAL_LAMPORTS)
        .unwrap();

    let ix = build_configure_prop_amm_ix(
        &unauthorized.pubkey(),
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        false,
        700,
        25_000,
    );
    let result = send_tx(&mut ctx.svm, &[ix], &[&unauthorized]);
    assert!(result.is_err(), "non-boss should not configure Prop AMM");

    let (offer_pda, _) = find_offer_pda(&ctx.usdc_mint, &ctx.onyc_mint);
    assert!(read_prop_amm_pair_state(&ctx.svm, &offer_pda).enabled);
}

#[test]
fn test_configure_prop_amm_rejects_invalid_parameters() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();

    let invalid_cases = [
        (10_001, 25_000, 20, 10_000, 86_400, 20_000),
        (700, 999, 20, 10_000, 86_400, 20_000),
        (700, 100_001, 20, 10_000, 86_400, 20_000),
        (700, 25_500, 20, 10_000, 86_400, 20_000),
        (700, 25_000, 0, 10_000, 86_400, 20_000),
        (700, 25_000, 20, 50_001, 86_400, 20_000),
        (700, 25_000, 20, 1_500, 86_400, 20_000),
        (700, 25_000, 20, 10_000, 0, 20_000),
        (700, 25_000, 20, 10_000, 86_400, 0),
    ];

    for (
        haircut_bps,
        exponent_scaled,
        cadence_threshold,
        cadence_wave_scaled,
        epoch_duration_seconds,
        wall_sensitivity_scaled,
    ) in invalid_cases
    {
        let ix = build_configure_prop_amm_with_params_ix(
            &boss,
            &ctx.usdc_mint,
            &ctx.onyc_mint,
            true,
            haircut_bps,
            exponent_scaled,
            cadence_threshold,
            cadence_wave_scaled,
            epoch_duration_seconds,
            wall_sensitivity_scaled,
            5_000_000_000,
        );
        assert!(
            send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).is_err(),
            "invalid Prop AMM config should fail: {invalid_cases:?}"
        );
    }
}

#[test]
fn test_configure_prop_amm_accepts_max_and_zero_cadence_wave() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();

    let ix = build_configure_prop_amm_with_params_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        true,
        700,
        25_000,
        7,
        50_000,
        86_400,
        20_000,
        5_000_000_000,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let (offer, _) = find_offer_pda(&ctx.usdc_mint, &ctx.onyc_mint);
    let pair = read_prop_amm_pair_state(&ctx.svm, &offer);
    assert_eq!(pair.cadence_threshold, 7);
    assert_eq!(pair.cadence_wave_scaled, 50_000);

    let ix = build_configure_prop_amm_with_params_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        true,
        700,
        25_000,
        3,
        0,
        86_400,
        20_000,
        5_000_000_000,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let pair = read_prop_amm_pair_state(&ctx.svm, &offer);
    assert_eq!(pair.cadence_threshold, 3);
    assert_eq!(pair.cadence_wave_scaled, 0);
}

#[test]
fn test_prop_amm_rejects_pair_state_for_different_offer() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    let eurc_mint = create_mint(&mut ctx.svm, &ctx.payer, 6, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &eurc_mint,
        &ctx.onyc_mint,
        0,
        false,
        true,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    let ix = build_configure_prop_amm_ix(&boss, &eurc_mint, &ctx.onyc_mint, true, 1_200, 20_000);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let (usdc_offer_pda, _) = find_offer_pda(&ctx.usdc_mint, &ctx.onyc_mint);
    let (usdc_pair_state_pda, _) = find_prop_amm_pair_state_pda(&usdc_offer_pda);
    let mut ix = build_quote_swap_ix(&ctx.onyc_mint, &eurc_mint, &ctx.onyc_mint, 1_000_000);
    ix.accounts[1] = AccountMeta::new_readonly(usdc_pair_state_pda, false);

    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]);
    assert!(
        result.is_err(),
        "Prop AMM should reject a pair-state PDA derived from another offer"
    );
}

#[test]
fn test_prop_amm_rejects_pair_state_with_mismatched_stored_mints() {
    let mut ctx = setup_prop_amm();
    let (offer_pda, _) = find_offer_pda(&ctx.usdc_mint, &ctx.onyc_mint);

    overwrite_pair_state_pubkey(&mut ctx.svm, &offer_pda, 8 + 32);
    let quote_ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.usdc_mint, &ctx.onyc_mint, 1_000_000);
    assert!(
        send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).is_err(),
        "Prop AMM should reject a pair state with the wrong stored asset mint"
    );

    let boss = ctx.payer.pubkey();
    let ix = build_configure_prop_amm_ix(&boss, &ctx.usdc_mint, &ctx.onyc_mint, true, 700, 26_000);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    overwrite_pair_state_pubkey(&mut ctx.svm, &offer_pda, 8 + 32 + 32);
    let quote_ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.usdc_mint, &ctx.onyc_mint, 1_000_000);
    assert!(
        send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).is_err(),
        "Prop AMM should reject a pair state with the wrong stored ONYC mint"
    );
}

#[test]
fn test_open_swap_enforces_minimum_out() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86_400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let quote_ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.usdc_mint, &ctx.onyc_mint, 1_000_000);
    let quote_metadata = send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).unwrap();
    let quote = SwapQuote::try_from_slice(get_return_data(&quote_metadata)).unwrap();

    let ix = build_open_swap_buy_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000,
        quote.minimum_out + 1,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]);
    assert!(result.is_err());

    let ix = build_open_swap_buy_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000,
        quote.minimum_out,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]).unwrap();

    let user_onyc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.onyc_mint),
    );
    assert_eq!(user_onyc, quote.token_out_amount);
}

#[test]
fn test_open_swap_buy_creates_prefunded_user_output_ata() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    add_prop_amm_vector(&mut ctx);

    let user_onyc_ata = get_associated_token_address(&ctx.user.pubkey(), &ctx.onyc_mint);
    ctx.svm
        .set_account(
            user_onyc_ata,
            Account {
                executable: false,
                data: Vec::new(),
                lamports: 1,
                owner: SYSTEM_PROGRAM_ID,
                rent_epoch: 0,
            },
        )
        .unwrap();

    let quote_ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.usdc_mint, &ctx.onyc_mint, 1_000_000);
    let quote_metadata = send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).unwrap();
    let quote = SwapQuote::try_from_slice(get_return_data(&quote_metadata)).unwrap();

    let ix = build_open_swap_buy_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000,
        quote.minimum_out,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]).unwrap();

    assert_eq!(
        get_token_balance(&ctx.svm, &user_onyc_ata),
        quote.token_out_amount
    );
}

#[test]
fn test_open_swap_sell_enforces_minimum_out() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    prepare_prop_amm_sell_side(&mut ctx, 0);

    let sell_amount = 100_000_000;
    let quote_ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.onyc_mint, &ctx.usdc_mint, sell_amount);
    let quote_metadata = send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).unwrap();
    let quote = SwapQuote::try_from_slice(get_return_data(&quote_metadata)).unwrap();

    let ix = build_open_swap_sell_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        sell_amount,
        quote.minimum_out + 1,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    assert!(send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]).is_err());
}

#[test]
fn test_open_swap_sell_applies_default_minimum_haircut() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &ctx.onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    add_prop_amm_vector(&mut ctx);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(
        &mut ctx.svm,
        &ctx.usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    create_token_account(
        &mut ctx.svm,
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        20_000_000_000,
    );
    let ix = build_refresh_market_stats_ix(&boss, &ctx.usdc_mint, &ctx.onyc_mint);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let dust_quote_ix =
        build_quote_swap_ix(&ctx.onyc_mint, &ctx.onyc_mint, &ctx.usdc_mint, 100_000_000);
    assert!(
        send_tx(&mut ctx.svm, &[dust_quote_ix], &[&ctx.payer]).is_err(),
        "gross input below 5 ONYC minimum haircut should fail"
    );

    let zero_net_sell_amount = 5_000_000_000;
    let quote_ix = build_quote_swap_ix(
        &ctx.onyc_mint,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        zero_net_sell_amount,
    );
    assert!(
        send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).is_err(),
        "positive input whose minimum haircut leaves zero net amount should fail"
    );

    let ix = build_open_swap_sell_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        zero_net_sell_amount,
        0,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    assert!(
        send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]).is_err(),
        "positive input whose minimum haircut leaves zero output should fail"
    );

    let sell_amount = 5_100_000_000;
    let quote_ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.onyc_mint, &ctx.usdc_mint, sell_amount);
    let quote_metadata = send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).unwrap();
    let quote = SwapQuote::try_from_slice(get_return_data(&quote_metadata)).unwrap();

    assert_eq!(quote.token_in_amount, sell_amount);
    assert_eq!(quote.token_in_fee_amount, 5_000_000_000);
    assert_eq!(quote.token_in_net_amount, 100_000_000);

    let ix = build_open_swap_sell_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        sell_amount,
        quote.minimum_out,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]).unwrap();

    let fee_vault_ata =
        get_associated_token_address(&find_prop_amm_sell_fee_vault_pda().0, &ctx.onyc_mint);
    assert_eq!(get_token_balance(&ctx.svm, &fee_vault_ata), 5_000_000_000);
}

#[test]
fn test_open_swap_sell_uses_prop_amm_sell_redemption_fee_and_vault() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    prepare_prop_amm_sell_side(&mut ctx, 100);

    let ix = build_update_redemption_offer_prop_amm_sell_fee_ix(
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        300,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let sell_amount = 1_000_000_000;
    let quote_ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.onyc_mint, &ctx.usdc_mint, sell_amount);
    let quote_metadata = send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).unwrap();
    let quote = SwapQuote::try_from_slice(get_return_data(&quote_metadata)).unwrap();
    assert_eq!(quote.token_in_fee_amount, 30_000_000);
    assert_eq!(quote.token_in_net_amount, 970_000_000);

    let ix = build_open_swap_sell_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        sell_amount,
        quote.minimum_out,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]).unwrap();

    let sell_fee_ata =
        get_associated_token_address(&find_prop_amm_sell_fee_vault_pda().0, &ctx.onyc_mint);
    let old_prop_amm_buy_fee_ata =
        get_associated_token_address(&find_prop_amm_buy_fee_vault_pda().0, &ctx.onyc_mint);
    assert_eq!(get_token_balance(&ctx.svm, &sell_fee_ata), 30_000_000);
    assert_eq!(get_balance_or_zero(&ctx.svm, &old_prop_amm_buy_fee_ata), 0);
}

#[test]
fn test_open_swap_buy_uses_permissionless_offer_fee() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    add_prop_amm_vector(&mut ctx);

    let ix = build_update_offer_fee_ix(&boss, &ctx.usdc_mint, &ctx.onyc_mint, 100);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    let ix = build_update_offer_permissionless_fee_ix(&boss, &ctx.usdc_mint, &ctx.onyc_mint, 300);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let buy_amount = 1_000_000;
    let quote_ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.usdc_mint, &ctx.onyc_mint, buy_amount);
    let quote_metadata = send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).unwrap();
    let quote = SwapQuote::try_from_slice(get_return_data(&quote_metadata)).unwrap();
    assert_eq!(quote.token_in_fee_amount, 30_000);
    assert_eq!(quote.token_in_net_amount, 970_000);

    let ix = build_open_swap_buy_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        buy_amount,
        quote.minimum_out,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]).unwrap();

    let prop_amm_buy_fee_ata =
        get_associated_token_address(&find_prop_amm_buy_fee_vault_pda().0, &ctx.usdc_mint);
    assert_eq!(get_token_balance(&ctx.svm, &prop_amm_buy_fee_ata), 30_000);
}

#[test]
fn test_prop_amm_rejects_quotes_and_swaps_when_kill_switch_active() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    add_prop_amm_vector(&mut ctx);

    let ix = build_set_kill_switch_ix(&boss, true);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let quote_ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.usdc_mint, &ctx.onyc_mint, 1_000_000);
    assert!(
        send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).is_err(),
        "buy quote should reject while the kill switch is active"
    );

    let buy_ix = build_open_swap_buy_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000,
        0,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    assert!(
        send_tx(&mut ctx.svm, &[buy_ix], &[&ctx.payer, &ctx.user]).is_err(),
        "buy execution should reject while the kill switch is active"
    );

    let mut sell_ctx = setup_prop_amm();
    let sell_boss = sell_ctx.payer.pubkey();
    prepare_prop_amm_sell_side(&mut sell_ctx, 0);
    let ix = build_set_kill_switch_ix(&sell_boss, true);
    send_tx(&mut sell_ctx.svm, &[ix], &[&sell_ctx.payer]).unwrap();

    let sell_quote_ix = build_quote_swap_ix(
        &sell_ctx.onyc_mint,
        &sell_ctx.onyc_mint,
        &sell_ctx.usdc_mint,
        100,
    );
    assert!(
        send_tx(&mut sell_ctx.svm, &[sell_quote_ix], &[&sell_ctx.payer]).is_err(),
        "sell quote should reject while the kill switch is active"
    );

    let sell_ix = build_open_swap_sell_ix(
        &sell_ctx.onyc_mint,
        &sell_ctx.user.pubkey(),
        &sell_boss,
        &sell_ctx.onyc_mint,
        &sell_ctx.usdc_mint,
        100,
        0,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    assert!(
        send_tx(
            &mut sell_ctx.svm,
            &[sell_ix],
            &[&sell_ctx.payer, &sell_ctx.user]
        )
        .is_err(),
        "sell execution should reject while the kill switch is active"
    );
}

#[test]
fn test_open_swap_buy_respects_max_supply_and_max_mint_amount() {
    let mut max_supply_ctx = setup_prop_amm();
    let boss = max_supply_ctx.payer.pubkey();
    add_prop_amm_vector(&mut max_supply_ctx);
    let ix = build_transfer_mint_authority_to_program_ix(
        &boss,
        &max_supply_ctx.onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut max_supply_ctx.svm, &[ix], &[&max_supply_ctx.payer]).unwrap();

    let quote_ix = build_quote_swap_ix(
        &max_supply_ctx.onyc_mint,
        &max_supply_ctx.usdc_mint,
        &max_supply_ctx.onyc_mint,
        1_000_000,
    );
    let quote_metadata = send_tx(
        &mut max_supply_ctx.svm,
        &[quote_ix],
        &[&max_supply_ctx.payer],
    )
    .unwrap();
    let quote = SwapQuote::try_from_slice(get_return_data(&quote_metadata)).unwrap();
    let current_supply = get_mint_supply(&max_supply_ctx.svm, &max_supply_ctx.onyc_mint);
    let ix = build_configure_max_supply_ix(&boss, current_supply + quote.token_out_amount - 1);
    send_tx(&mut max_supply_ctx.svm, &[ix], &[&max_supply_ctx.payer]).unwrap();
    let buy_ix = build_open_swap_buy_ix(
        &max_supply_ctx.onyc_mint,
        &max_supply_ctx.user.pubkey(),
        &boss,
        &max_supply_ctx.usdc_mint,
        &max_supply_ctx.onyc_mint,
        1_000_000,
        quote.minimum_out,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    assert!(
        send_tx(
            &mut max_supply_ctx.svm,
            &[buy_ix],
            &[&max_supply_ctx.payer, &max_supply_ctx.user],
        )
        .is_err(),
        "Prop AMM buy should enforce max supply"
    );

    let mut max_mint_ctx = setup_prop_amm();
    let boss = max_mint_ctx.payer.pubkey();
    add_prop_amm_vector(&mut max_mint_ctx);
    let ix = build_transfer_mint_authority_to_program_ix(
        &boss,
        &max_mint_ctx.onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut max_mint_ctx.svm, &[ix], &[&max_mint_ctx.payer]).unwrap();
    let ix = build_configure_max_mint_amount_ix(&boss, quote.token_out_amount - 1);
    send_tx(&mut max_mint_ctx.svm, &[ix], &[&max_mint_ctx.payer]).unwrap();
    let buy_ix = build_open_swap_buy_ix(
        &max_mint_ctx.onyc_mint,
        &max_mint_ctx.user.pubkey(),
        &boss,
        &max_mint_ctx.usdc_mint,
        &max_mint_ctx.onyc_mint,
        1_000_000,
        quote.minimum_out,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    assert!(
        send_tx(
            &mut max_mint_ctx.svm,
            &[buy_ix],
            &[&max_mint_ctx.payer, &max_mint_ctx.user],
        )
        .is_err(),
        "Prop AMM buy should enforce max minted amount per mint"
    );
}

#[test]
fn test_open_swap_buy_rejects_noncanonical_mint_authority() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    add_prop_amm_vector(&mut ctx);
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &ctx.onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let fake_mint_authority = Keypair::new();
    ctx.svm
        .airdrop(&fake_mint_authority.pubkey(), INITIAL_LAMPORTS)
        .unwrap();

    let mut ix = build_open_swap_buy_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000,
        0,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    ix.accounts[22] = AccountMeta::new_readonly(fake_mint_authority.pubkey(), false);

    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]);
    assert!(
        result.is_err(),
        "Prop AMM buy should reject a non-canonical mint authority account"
    );
}

#[test]
fn test_open_swap_buy_rejects_token_in_transfer_fee() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let usdg_mint = create_mint_2022_with_transfer_fee(&mut svm, &payer, 6, &boss, 500, 1_000_000);

    let ix = build_make_offer_ix(
        &boss,
        &usdg_mint,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let (offer_pda, _) = find_offer_pda(&usdg_mint, &onyc_mint);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let ix = build_configure_prop_amm_ix(&boss, &usdg_mint, &onyc_mint, true, 700, 25_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdg_mint,
        &onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86_400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account_2022(&mut svm, &usdg_mint, &user.pubkey(), 10_000_000);

    let ix = build_open_swap_buy_ix(
        &onyc_mint,
        &user.pubkey(),
        &boss,
        &usdg_mint,
        &onyc_mint,
        1_000_000,
        0,
        &TOKEN_2022_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    let result = send_tx(&mut svm, &[ix], &[&payer, &user]);
    assert!(
        result.is_err(),
        "Prop AMM buy should reject Token-2022 transfer-fee assets"
    );
}

#[test]
fn test_quote_swap_buy_rejects_token_in_transfer_fee() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let usdg_mint = create_mint_2022_with_transfer_fee(&mut svm, &payer, 6, &boss, 500, 1_000_000);

    let ix = build_make_offer_ix(
        &boss,
        &usdg_mint,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let (offer_pda, _) = find_offer_pda(&usdg_mint, &onyc_mint);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let ix = build_configure_prop_amm_ix(&boss, &usdg_mint, &onyc_mint, true, 700, 25_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let quote_ix = build_quote_swap_ix(&onyc_mint, &usdg_mint, &onyc_mint, 1_000_000);
    let result = send_tx(&mut svm, &[quote_ix], &[&payer]);
    assert!(
        result.is_err(),
        "Prop AMM buy quote should reject Token-2022 transfer-fee assets"
    );
}

#[test]
fn test_quote_swap_sell_rejects_token_out_transfer_fee() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let asset_mint = create_mint_2022_with_transfer_fee(&mut svm, &payer, 6, &boss, 500, 1_000_000);

    let ix = build_make_offer_ix(
        &boss,
        &asset_mint,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let (offer_pda, _) = find_offer_pda(&asset_mint, &onyc_mint);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let ix = build_configure_prop_amm_ix(&boss, &asset_mint, &onyc_mint, true, 700, 25_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    let redemption_vault_asset = create_token_account_2022(
        &mut svm,
        &asset_mint,
        &redemption_vault_authority,
        1_000_000,
    );
    let mut quote_ix = build_quote_swap_ix(&onyc_mint, &onyc_mint, &asset_mint, 1_000_000_000);
    quote_ix.accounts[5] = AccountMeta::new_readonly(redemption_vault_asset, false);
    quote_ix.accounts[8] = AccountMeta::new_readonly(TOKEN_2022_PROGRAM_ID, false);

    let result = send_tx(&mut svm, &[quote_ix], &[&payer]);
    assert!(
        result.is_err(),
        "Prop AMM sell quote should reject Token-2022 transfer-fee payout assets"
    );
}

#[test]
fn test_open_swap_sell_rolls_epoch_tracker_before_recording_trade() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    let ix = build_configure_prop_amm_with_params_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        true,
        700,
        25_000,
        20,
        10_000,
        10,
        20_000,
        0,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    prepare_prop_amm_sell_side(&mut ctx, 0);
    advance_slot(&mut ctx.svm);
    let ix = build_configure_prop_amm_with_params_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        true,
        700,
        25_000,
        20,
        10_000,
        10,
        20_000,
        0,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let sell_amount = 100_000_000;
    let sell_ix = build_open_swap_sell_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        sell_amount,
        0,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[sell_ix], &[&ctx.payer, &ctx.user]).unwrap();

    let (offer_pda, _) = find_offer_pda(&ctx.usdc_mint, &ctx.onyc_mint);
    let first_state = read_prop_amm_pair_state(&ctx.svm, &offer_pda);
    assert_eq!(first_state.curr_sell_trade_count, 1);
    assert_eq!(first_state.curr_sell_value_stable, sell_amount / 1_000);

    advance_clock_by(&mut ctx.svm, 11);
    let sell_ix = build_open_swap_sell_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        sell_amount,
        0,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[sell_ix], &[&ctx.payer, &ctx.user]).unwrap();
    let rolled_state = read_prop_amm_pair_state(&ctx.svm, &offer_pda);

    assert!(rolled_state.epoch_start > first_state.epoch_start);
    assert_eq!(
        rolled_state.prev_net_sell_value_stable,
        first_state.curr_sell_value_stable
    );
    assert_eq!(rolled_state.curr_sell_trade_count, 1);
    assert_eq!(rolled_state.curr_sell_value_stable, sell_amount / 1_000);
}

#[test]
fn test_open_swap_buy_refills_redemption_vault_until_target_then_overflows_to_boss() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86_400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    let ix = build_make_redemption_offer_ix(
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        0,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    let ix =
        build_update_redemption_offer_vault_target_ix(&boss, &ctx.onyc_mint, &ctx.usdc_mint, 1_500);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let (vault_authority, _) = find_offer_vault_authority_pda();
    set_and_refresh_circulating_supply_exclusions(
        &mut ctx.svm,
        &ctx.payer,
        &ctx.onyc_mint,
        &[vault_authority],
    );

    let first_quote_ix =
        build_quote_swap_ix(&ctx.onyc_mint, &ctx.usdc_mint, &ctx.onyc_mint, 1_000_000);
    let first_quote_metadata = send_tx(&mut ctx.svm, &[first_quote_ix], &[&ctx.payer]).unwrap();
    let first_quote = SwapQuote::try_from_slice(get_return_data(&first_quote_metadata)).unwrap();

    let first_buy_ix = build_open_swap_buy_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000,
        first_quote.minimum_out,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[first_buy_ix], &[&ctx.payer, &ctx.user]).unwrap();

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    let redemption_vault_usdc = derive_ata(
        &redemption_vault_authority,
        &ctx.usdc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let proceeds_usdc =
        get_associated_token_address(&find_prop_amm_proceeds_vault_pda().0, &ctx.usdc_mint);
    assert_eq!(get_token_balance(&ctx.svm, &redemption_vault_usdc), 0);
    assert_eq!(get_token_balance(&ctx.svm, &proceeds_usdc), 1_000_000);

    advance_slot(&mut ctx.svm);
    refresh_circulating_supply_excluded_balance(
        &mut ctx.svm,
        &ctx.payer,
        &ctx.onyc_mint,
        &[vault_authority],
    );
    let ix = build_refresh_market_stats_ix(&boss, &ctx.usdc_mint, &ctx.onyc_mint);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let second_quote_ix =
        build_quote_swap_ix(&ctx.onyc_mint, &ctx.usdc_mint, &ctx.onyc_mint, 1_000_001);
    let second_quote_metadata = send_tx(&mut ctx.svm, &[second_quote_ix], &[&ctx.payer]).unwrap();
    let second_quote = SwapQuote::try_from_slice(get_return_data(&second_quote_metadata)).unwrap();

    let second_buy_ix = build_open_swap_buy_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_001,
        second_quote.minimum_out,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[second_buy_ix], &[&ctx.payer, &ctx.user]).unwrap();

    assert_eq!(get_token_balance(&ctx.svm, &redemption_vault_usdc), 150_000);
    assert_eq!(get_token_balance(&ctx.svm, &proceeds_usdc), 1_850_001);

    let destination = Keypair::new();
    ctx.svm
        .airdrop(&destination.pubkey(), INITIAL_LAMPORTS)
        .unwrap();
    let (prop_amm_proceeds_vault_pda, _) = find_prop_amm_proceeds_vault_pda();
    let ix = build_set_configurable_vault_destination_ix(
        &boss,
        &prop_amm_proceeds_vault_pda,
        ConfigurableVaultKind::PropAmmProceeds.as_u8(),
        &destination.pubkey(),
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    let ix = build_withdraw_configurable_vault_ix(
        &destination.pubkey(),
        &prop_amm_proceeds_vault_pda,
        &destination.pubkey(),
        &ctx.usdc_mint,
        ConfigurableVaultKind::PropAmmProceeds.as_u8(),
        0,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&destination]).unwrap();
    assert_eq!(get_token_balance(&ctx.svm, &proceeds_usdc), 0);
    assert_eq!(
        get_token_balance(
            &ctx.svm,
            &get_associated_token_address(&destination.pubkey(), &ctx.usdc_mint),
        ),
        1_850_001
    );
}

#[test]
fn test_quote_and_open_swap_support_sell_side() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &ctx.onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86_400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    configure_minimum_sell_haircut(&mut ctx, 0);

    let ix = build_make_redemption_offer_ix(
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        500,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    let ix = build_update_redemption_offer_prop_amm_sell_fee_ix(
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        500,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(
        &mut ctx.svm,
        &ctx.usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    create_token_account(
        &mut ctx.svm,
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        2_000_000_000,
    );
    let ix = build_refresh_market_stats_ix(&boss, &ctx.usdc_mint, &ctx.onyc_mint);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let sell_amount = 100_000_000;
    let quote_ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.onyc_mint, &ctx.usdc_mint, sell_amount);
    let quote_metadata = send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).unwrap();
    let quote = SwapQuote::try_from_slice(get_return_data(&quote_metadata)).unwrap();

    assert_eq!(
        quote.offer,
        find_offer_pda(&ctx.usdc_mint, &ctx.onyc_mint).0
    );
    assert_eq!(quote.token_in_net_amount, 95_000_000);
    assert_eq!(quote.token_in_fee_amount, 5_000_000);
    assert_eq!(quote.token_out_amount, 95_000);

    let supply_before = get_mint_supply(&ctx.svm, &ctx.onyc_mint);
    let vault_before = get_token_balance(
        &ctx.svm,
        &derive_ata(
            &redemption_vault_authority,
            &ctx.usdc_mint,
            &TOKEN_PROGRAM_ID,
        ),
    );

    let ix = build_open_swap_sell_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        sell_amount,
        quote.minimum_out,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]).unwrap();

    let user_usdc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.usdc_mint),
    );
    assert_eq!(user_usdc, 10_000_000_000 + quote.token_out_amount);
    assert_eq!(
        get_mint_supply(&ctx.svm, &ctx.onyc_mint),
        supply_before - 95_000_000
    );
    assert_eq!(
        get_token_balance(
            &ctx.svm,
            &derive_ata(
                &redemption_vault_authority,
                &ctx.usdc_mint,
                &TOKEN_PROGRAM_ID
            ),
        ),
        vault_before - quote.token_out_amount
    );
}

#[test]
fn test_open_swap_sell_refreshes_market_stats_before_hard_wall_target() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &ctx.onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86_400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    configure_minimum_sell_haircut(&mut ctx, 0);

    let ix = build_make_redemption_offer_ix(
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        0,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    let ix =
        build_update_redemption_offer_vault_target_ix(&boss, &ctx.onyc_mint, &ctx.usdc_mint, 5_000);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(
        &mut ctx.svm,
        &ctx.usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    create_token_account(
        &mut ctx.svm,
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        2_000_000_000,
    );

    let (market_stats_pda, _) = find_market_stats_pda();
    assert!(ctx.svm.get_account(&market_stats_pda).is_none());

    let sell_amount = 100_000_000;
    let ix = build_open_swap_sell_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        sell_amount,
        0,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]).unwrap();

    let market_stats = read_market_stats(&ctx.svm);
    assert_eq!(
        market_stats.circulating_supply,
        get_mint_supply(&ctx.svm, &ctx.onyc_mint)
    );
}

#[test]
fn test_quote_swap_sell_caps_hard_wall_reserve_by_vault_target() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &ctx.onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86_400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    configure_minimum_sell_haircut(&mut ctx, 0);

    let ix = build_make_redemption_offer_ix(
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        0,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(
        &mut ctx.svm,
        &ctx.usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    create_token_account(
        &mut ctx.svm,
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        2_000_000_000_000,
    );
    let ix = build_refresh_market_stats_ix(&boss, &ctx.usdc_mint, &ctx.onyc_mint);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    let market_stats = read_market_stats(&ctx.svm);
    assert!(market_stats.tvl > 0);

    let sell_amount = 1_000_000_000;
    let quote_ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.onyc_mint, &ctx.usdc_mint, sell_amount);
    let quote_metadata = send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).unwrap();
    let vault_balance_quote = SwapQuote::try_from_slice(get_return_data(&quote_metadata)).unwrap();

    let ix =
        build_update_redemption_offer_vault_target_ix(&boss, &ctx.onyc_mint, &ctx.usdc_mint, 1);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    advance_slot(&mut ctx.svm);
    let target_reserve = hard_wall_reserve_from_tvl(market_stats.tvl, 1, 6, 9).unwrap();
    assert!(target_reserve < 10_000_000_000);

    let quote_ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.onyc_mint, &ctx.usdc_mint, sell_amount);
    let quote_metadata = send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).unwrap();
    let target_quote = SwapQuote::try_from_slice(get_return_data(&quote_metadata)).unwrap();

    assert_eq!(
        target_quote.token_in_net_amount,
        vault_balance_quote.token_in_net_amount
    );
    assert!(
        target_quote.token_out_amount < vault_balance_quote.token_out_amount,
        "vault target should cap the hard-wall reserve: target_reserve={target_reserve}, vault_quote={}, target_quote={}",
        vault_balance_quote.token_out_amount,
        target_quote.token_out_amount
    );
}

#[test]
fn test_open_swap_sell_accrues_buffer_before_burning_onyc() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &ctx.onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let (main_asset_mint, main_offer) = configure_main_offer_with_asset_and_apr(
        &mut ctx.svm,
        &ctx.payer,
        &ctx.onyc_mint,
        &TOKEN_PROGRAM_ID,
        50_000,
    );
    let current_time = get_clock_time(&ctx.svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86_400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    configure_minimum_sell_haircut(&mut ctx, 0);

    create_token_account(
        &mut ctx.svm,
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        2_000_000_000,
    );

    let ix = build_initialize_buffer_ix(&boss, &main_offer, &ctx.onyc_mint);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let ix = build_set_buffer_gross_yield_ix(&boss, &main_offer, &ctx.onyc_mint, 150_000);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(
        &mut ctx.svm,
        &ctx.usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    let ix = build_refresh_market_stats_ix(&boss, &main_asset_mint, &ctx.onyc_mint);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    advance_clock_by(&mut ctx.svm, ONE_YEAR_SECONDS);

    let buffer_state_before = read_buffer_state(&ctx.svm);
    let supply_before = get_mint_supply(&ctx.svm, &ctx.onyc_mint);
    let buffer_vault = derive_ata(
        &find_reserve_vault_authority_pda().0,
        &ctx.onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let buffer_vault_before = get_token_balance(&ctx.svm, &buffer_vault);

    let sell_amount = 100_000_000;
    let ix = build_open_swap_sell_ix_with_main_offer(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        sell_amount,
        0,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
        &main_offer,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]).unwrap();

    let expected_buffer_accrual =
        (buffer_state_before.previous_supply as u128 * 100_000 / (1_000_000 + 50_000)) as u64;
    let supply_after = get_mint_supply(&ctx.svm, &ctx.onyc_mint);
    let buffer_state_after = read_buffer_state(&ctx.svm);

    assert_eq!(buffer_state_before.previous_supply, supply_before);
    assert_eq!(
        get_token_balance(&ctx.svm, &buffer_vault) - buffer_vault_before,
        expected_buffer_accrual
    );
    assert_eq!(
        supply_after,
        supply_before + expected_buffer_accrual - sell_amount
    );
    assert_eq!(buffer_state_after.previous_supply, supply_after);
}

#[test]
fn test_open_swap_buy_prices_buffer_from_main_offer() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &ctx.onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let (_main_asset_mint, main_offer) = configure_main_offer_with_asset_and_apr(
        &mut ctx.svm,
        &ctx.payer,
        &ctx.onyc_mint,
        &TOKEN_PROGRAM_ID,
        50_000,
    );
    add_prop_amm_vector(&mut ctx);

    let ix = build_mint_to_ix_for_offer(
        &boss,
        &ctx.onyc_mint,
        1_000_000_000,
        &TOKEN_PROGRAM_ID,
        &main_offer,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    let ix = build_initialize_buffer_ix(&boss, &main_offer, &ctx.onyc_mint);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    let ix = build_set_buffer_gross_yield_ix(&boss, &main_offer, &ctx.onyc_mint, 150_000);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    advance_clock_by(&mut ctx.svm, ONE_YEAR_SECONDS);

    let buffer_state_before = read_buffer_state(&ctx.svm);
    let supply_before = get_mint_supply(&ctx.svm, &ctx.onyc_mint);
    let buffer_vault = derive_ata(
        &find_reserve_vault_authority_pda().0,
        &ctx.onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let buffer_vault_before = get_token_balance(&ctx.svm, &buffer_vault);

    let ix = build_open_swap_buy_ix_with_main_offer(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000,
        0,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
        &main_offer,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]).unwrap();

    let expected_buffer_accrual =
        (buffer_state_before.previous_supply as u128 * 100_000 / (1_000_000 + 50_000)) as u64;
    let user_mint = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.onyc_mint),
    );
    let supply_after = get_mint_supply(&ctx.svm, &ctx.onyc_mint);

    assert_eq!(buffer_state_before.previous_supply, supply_before);
    assert_eq!(
        get_token_balance(&ctx.svm, &buffer_vault) - buffer_vault_before,
        expected_buffer_accrual
    );
    assert_eq!(user_mint, 1_000_000_000);
    assert_eq!(
        supply_after,
        supply_before + expected_buffer_accrual + user_mint
    );
    assert_eq!(read_buffer_state(&ctx.svm).previous_supply, supply_after);
}

#[test]
fn test_open_swap_buy_rejects_invalid_buffer_accounts() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    add_prop_amm_vector(&mut ctx);
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &ctx.onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    initialize_prop_amm_buffer(&mut ctx, 100_000);
    advance_clock_by(&mut ctx.svm, ONE_YEAR_SECONDS);

    for (account_index, label) in [
        (23, "buffer state"),
        (24, "reserve vault"),
        (25, "management fee vault"),
        (26, "performance fee vault"),
    ] {
        let previous_supply = read_buffer_state(&ctx.svm).previous_supply;
        let mut ix = build_open_swap_buy_ix(
            &ctx.onyc_mint,
            &ctx.user.pubkey(),
            &boss,
            &ctx.usdc_mint,
            &ctx.onyc_mint,
            1_000_000,
            0,
            &TOKEN_PROGRAM_ID,
            &TOKEN_PROGRAM_ID,
        );
        ix.accounts[account_index] = AccountMeta::new(Pubkey::new_unique(), false);

        assert!(
            send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]).is_err(),
            "Prop AMM buy should reject invalid {label}"
        );
        assert_eq!(read_buffer_state(&ctx.svm).previous_supply, previous_supply);
    }
}

#[test]
fn test_open_swap_sell_rejects_invalid_buffer_accounts() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    prepare_prop_amm_sell_side(&mut ctx, 0);
    initialize_prop_amm_buffer(&mut ctx, 100_000);
    advance_clock_by(&mut ctx.svm, ONE_YEAR_SECONDS);

    for (account_index, label) in [
        (19, "buffer state"),
        (20, "reserve vault"),
        (21, "management fee vault"),
        (22, "performance fee vault"),
    ] {
        let previous_supply = read_buffer_state(&ctx.svm).previous_supply;
        let mut ix = build_open_swap_sell_ix(
            &ctx.onyc_mint,
            &ctx.user.pubkey(),
            &boss,
            &ctx.onyc_mint,
            &ctx.usdc_mint,
            100_000_000,
            0,
            &TOKEN_PROGRAM_ID,
            &TOKEN_PROGRAM_ID,
        );
        ix.accounts[account_index] = AccountMeta::new(Pubkey::new_unique(), false);

        assert!(
            send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]).is_err(),
            "Prop AMM sell should reject invalid {label}"
        );
        assert_eq!(read_buffer_state(&ctx.svm).previous_supply, previous_supply);
    }
}

#[test]
fn test_sell_side_uses_zero_fee_when_redemption_offer_is_uninitialized() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &ctx.onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86_400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    configure_minimum_sell_haircut(&mut ctx, 0);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(
        &mut ctx.svm,
        &ctx.usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    create_token_account(
        &mut ctx.svm,
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        2_000_000_000,
    );
    let ix = build_refresh_market_stats_ix(&boss, &ctx.usdc_mint, &ctx.onyc_mint);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let sell_amount = 100_000_000;
    let quote_ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.onyc_mint, &ctx.usdc_mint, sell_amount);
    let quote_metadata = send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).unwrap();
    let quote = SwapQuote::try_from_slice(get_return_data(&quote_metadata)).unwrap();

    assert_eq!(quote.token_in_net_amount, 100_000_000);
    assert_eq!(quote.token_in_fee_amount, 0);
    assert_eq!(quote.token_out_amount, 100_000);

    let supply_before = get_mint_supply(&ctx.svm, &ctx.onyc_mint);
    let vault_before = get_token_balance(
        &ctx.svm,
        &derive_ata(
            &redemption_vault_authority,
            &ctx.usdc_mint,
            &TOKEN_PROGRAM_ID,
        ),
    );

    let ix = build_open_swap_sell_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        sell_amount,
        quote.minimum_out,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]).unwrap();

    let user_usdc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.usdc_mint),
    );
    assert_eq!(user_usdc, 10_000_000_000 + quote.token_out_amount);
    assert_eq!(
        get_mint_supply(&ctx.svm, &ctx.onyc_mint),
        supply_before - 100_000_000
    );
    assert_eq!(
        get_token_balance(
            &ctx.svm,
            &derive_ata(
                &redemption_vault_authority,
                &ctx.usdc_mint,
                &TOKEN_PROGRAM_ID
            ),
        ),
        vault_before - quote.token_out_amount
    );
}
