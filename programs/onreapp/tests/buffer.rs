mod common;

use common::*;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

const ONE_YEAR_SECONDS: u64 = 31_536_000;
const ONE_DAY_SECONDS: u64 = 86_400;
// NAV/price scale is 1e9: 1.0 NAV = 1_000_000_000.
const NAV_1_0: u64 = 1_000_000_000;

// BUFFER math reference used by tests:
//
// Accrue:
//   spread = max(0, gross_yield - current_yield)        // APR scale 1e6
//   gross_mint = lowest_supply * spread * dt / YEAR / 1e6
//
// Fee split on accrual:
//   management_slice_apr = min(spread, management_fee_apr)
//   management_fee = floor(gross_mint * management_slice_apr / spread)
//   remaining = gross_mint - management_fee
//   if current_nav > performance_hwm_nav:
//     performance_fee = floor(remaining * performance_fee_bps / 10000)
//   else:
//     performance_fee = 0
//   buffer_mint = remaining - performance_fee
//
// High-water mark:
// - HWM is tracked in NAV/price units, not vault balance.
// - Performance fees are charged only while NAV is above the stored HWM.
//
// Burn for NAV support:
//   total_assets      = circulating_supply * current_nav / 1e9
//   assets_after      = total_assets - asset_adjustment_amount
//   required_supply   = ceil(assets_after * 1e9 / target_nav)
//   burn_amount       = current_supply - required_supply
//
// Units:
// - target_nav/current_nav: 1e9 scale
// - asset_adjustment_amount/total_assets: token-in mint base units
//   (e.g. USDC offer => micro-USDC)
// - supply and burn amount: ONyc base units

fn setup_buffer_context(
    gross_yield: u64,
    current_yield: u64,
    management_fee_basis_points: u16,
    performance_fee_basis_points: u16,
) -> (litesvm::LiteSVM, Keypair, Pubkey, Pubkey, Keypair) {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let token_in_mint = create_mint(&mut svm, &payer, 6, &boss);
    let yield_token_in_mint = create_mint(&mut svm, &payer, 6, &boss);
    let buffer_admin = Keypair::new();
    svm.airdrop(&buffer_admin.pubkey(), INITIAL_LAMPORTS)
        .unwrap();

    let ix = build_make_offer_ix(
        &boss,
        &token_in_mint,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let now = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in_mint,
        &onyc_mint,
        Some(now),
        now,
        NAV_1_0,
        0,
        86_400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_offer_ix(
        &boss,
        &yield_token_in_mint,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);
    let (offer_pda, _) = find_offer_pda(&yield_token_in_mint, &onyc_mint);

    let ix = build_add_offer_vector_ix(
        &boss,
        &yield_token_in_mint,
        &onyc_mint,
        Some(now),
        now,
        NAV_1_0,
        current_yield,
        86_400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_mint_to_ix(&boss, &onyc_mint, 1_000_000_000, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_initialize_buffer_ix(&boss, &offer_pda, &onyc_mint, &buffer_admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_set_buffer_gross_yield_ix(&boss, gross_yield);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    if management_fee_basis_points != 0 || performance_fee_basis_points != 0 {
        let ix = build_set_buffer_fee_config_ix(
            &boss,
            management_fee_basis_points,
            performance_fee_basis_points,
        );
        send_tx(&mut svm, &[ix], &[&payer]).unwrap();
        advance_slot(&mut svm);
    }

    (svm, payer, token_in_mint, onyc_mint, buffer_admin)
}

fn setup_buffer_with_balance() -> (litesvm::LiteSVM, Keypair, Pubkey, Pubkey, Keypair) {
    let (mut svm, payer, token_in_mint, onyc_mint, buffer_admin) =
        setup_buffer_context(150_000, 50_000, 0, 0);
    let offer_pda = read_state(&svm).main_offer;

    // First accrual initializes lowest_supply to current supply.
    let ix = build_manage_buffer_ix(&buffer_admin.pubkey(), &offer_pda, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&buffer_admin]).unwrap();
    advance_slot(&mut svm);

    // Second accrual after one year should mint 10% of 1_000_000_000 = 100_000_000.
    advance_clock_by(&mut svm, ONE_YEAR_SECONDS);
    let ix = build_manage_buffer_ix(&buffer_admin.pubkey(), &offer_pda, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&buffer_admin]).unwrap();

    (svm, payer, token_in_mint, onyc_mint, buffer_admin)
}

fn setup_buffer_with_fee_split(
    management_fee_basis_points: u16,
    performance_fee_basis_points: u16,
    accrual_periods: usize,
) -> (litesvm::LiteSVM, Keypair, Pubkey, Pubkey, Keypair) {
    let (mut svm, payer, token_in_mint, onyc_mint, buffer_admin) = setup_buffer_context(
        150_000,
        50_000,
        management_fee_basis_points,
        performance_fee_basis_points,
    );
    let offer_pda = read_state(&svm).main_offer;

    let ix = build_manage_buffer_ix(&buffer_admin.pubkey(), &offer_pda, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&buffer_admin]).unwrap();

    for _ in 0..accrual_periods {
        advance_slot(&mut svm);
        advance_clock_by(&mut svm, ONE_YEAR_SECONDS);
        let ix = build_manage_buffer_ix(&buffer_admin.pubkey(), &offer_pda, &onyc_mint);
        send_tx(&mut svm, &[ix], &[&buffer_admin]).unwrap();
    }

    (svm, payer, token_in_mint, onyc_mint, buffer_admin)
}

#[test]
fn test_initialize_buffer_success() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let buffer_admin = Keypair::new();
    let token_in_mint = create_mint(&mut svm, &payer, 6, &boss);
    let ix = build_make_offer_ix(
        &boss,
        &token_in_mint,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let (offer_pda, _) = find_offer_pda(&token_in_mint, &onyc_mint);

    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_initialize_buffer_ix(&boss, &offer_pda, &onyc_mint, &buffer_admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let buffer_state = read_buffer_state(&svm);
    assert_eq!(buffer_state.onyc_mint, onyc_mint);
    assert_eq!(buffer_state.buffer_admin, buffer_admin.pubkey());
    assert_eq!(buffer_state.gross_yield, 0);
    assert_eq!(buffer_state.lowest_supply, 0);
    assert_eq!(buffer_state.management_fee_basis_points, 0);
    assert_eq!(buffer_state.performance_fee_basis_points, 0);
    assert_eq!(buffer_state.performance_fee_high_watermark, 0);

    let (buffer_vault_authority_pda, _) = find_buffer_vault_authority_pda();
    let buffer_vault_ata = derive_ata(&buffer_vault_authority_pda, &onyc_mint, &TOKEN_PROGRAM_ID);
    let (management_fee_vault_authority_pda, _) = find_management_fee_vault_authority_pda();
    let management_fee_vault_ata = derive_ata(
        &management_fee_vault_authority_pda,
        &onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let (performance_fee_vault_authority_pda, _) = find_performance_fee_vault_authority_pda();
    let performance_fee_vault_ata = derive_ata(
        &performance_fee_vault_authority_pda,
        &onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    assert!(svm.get_account(&buffer_vault_ata).is_some());
    assert!(svm.get_account(&management_fee_vault_ata).is_some());
    assert!(svm.get_account(&performance_fee_vault_ata).is_some());
}

#[test]
fn test_initialize_buffer_requires_state_main_offer() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let buffer_admin = Keypair::new();
    let token_in_mint = create_mint(&mut svm, &payer, 6, &boss);
    let ix = build_make_offer_ix(
        &boss,
        &token_in_mint,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let (offer_pda, _) = find_offer_pda(&token_in_mint, &onyc_mint);

    let ix = build_initialize_buffer_ix(&boss, &offer_pda, &onyc_mint, &buffer_admin.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "initialize_buffer should require state.main_offer"
    );
}

#[test]
fn test_set_buffer_admin_boss_only() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let admin1 = Keypair::new();
    let admin2 = Keypair::new();
    let token_in_mint = create_mint(&mut svm, &payer, 6, &boss);
    let ix = build_make_offer_ix(
        &boss,
        &token_in_mint,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let (offer_pda, _) = find_offer_pda(&token_in_mint, &onyc_mint);

    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_initialize_buffer_ix(&boss, &offer_pda, &onyc_mint, &admin1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_set_buffer_admin_ix(&boss, &admin2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    assert_eq!(read_buffer_state(&svm).buffer_admin, admin2.pubkey());

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();
    let ix = build_set_buffer_admin_ix(&non_boss.pubkey(), &admin1.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(result.is_err(), "non-boss should not set buffer admin");
}

#[test]
fn test_set_buffer_admin_rejects_no_change() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let admin = Keypair::new();
    let token_in_mint = create_mint(&mut svm, &payer, 6, &boss);
    let ix = build_make_offer_ix(
        &boss,
        &token_in_mint,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let (offer_pda, _) = find_offer_pda(&token_in_mint, &onyc_mint);

    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_initialize_buffer_ix(&boss, &offer_pda, &onyc_mint, &admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_set_buffer_admin_ix(&boss, &admin.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "setting same buffer admin should fail");
}

#[test]
fn test_set_main_offer_updates_program_state() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let token_in_mint_a = create_mint(&mut svm, &payer, 6, &boss);
    let token_in_mint_b = create_mint(&mut svm, &payer, 6, &boss);
    let buffer_admin = Keypair::new();

    let ix = build_make_offer_ix(
        &boss,
        &token_in_mint_a,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let ix = build_make_offer_ix(
        &boss,
        &token_in_mint_b,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (offer_a_pda, _) = find_offer_pda(&token_in_mint_a, &onyc_mint);
    let (offer_b_pda, _) = find_offer_pda(&token_in_mint_b, &onyc_mint);

    let ix = build_set_main_offer_ix(&boss, &offer_a_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_initialize_buffer_ix(&boss, &offer_a_pda, &onyc_mint, &buffer_admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_set_main_offer_ix(&boss, &offer_b_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    assert_eq!(read_state(&svm).main_offer, offer_b_pda);
}

#[test]
fn test_set_buffer_gross_yield_rejects_no_change() {
    let (mut svm, payer, _token_in_mint, _onyc_mint, _buffer_admin) =
        setup_buffer_context(150_000, 50_000, 0, 0);
    let boss = payer.pubkey();

    let ix = build_set_buffer_gross_yield_ix(&boss, 150_000);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "setting same gross yield should fail");
}

#[test]
fn test_manage_buffer_first_call_sets_lowest_supply_no_mint() {
    let (mut svm, _payer, _token_in_mint, onyc_mint, buffer_admin) =
        setup_buffer_context(150_000, 50_000, 0, 0);
    let initial_supply = get_mint_supply(&svm, &onyc_mint);
    let offer_pda = read_state(&svm).main_offer;

    let ix = build_manage_buffer_ix(&buffer_admin.pubkey(), &offer_pda, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&buffer_admin]).unwrap();

    let (buffer_vault_authority_pda, _) = find_buffer_vault_authority_pda();
    let buffer_vault_ata = derive_ata(&buffer_vault_authority_pda, &onyc_mint, &TOKEN_PROGRAM_ID);
    let buffer_state = read_buffer_state(&svm);

    assert_eq!(get_token_balance(&svm, &buffer_vault_ata), 0);
    assert_eq!(get_mint_supply(&svm, &onyc_mint), initial_supply);
    assert_eq!(buffer_state.lowest_supply, initial_supply);
}

#[test]
fn test_manage_buffer_zero_spread_mints_nothing() {
    let (mut svm, _payer, _token_in_mint, onyc_mint, buffer_admin) =
        setup_buffer_context(50_000, 50_000, 0, 0);
    let offer_pda = read_state(&svm).main_offer;

    // Baseline call sets lowest_supply.
    let ix = build_manage_buffer_ix(&buffer_admin.pubkey(), &offer_pda, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&buffer_admin]).unwrap();
    advance_slot(&mut svm);

    advance_clock_by(&mut svm, ONE_YEAR_SECONDS);
    let ix = build_manage_buffer_ix(&buffer_admin.pubkey(), &offer_pda, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&buffer_admin]).unwrap();

    let (buffer_vault_authority_pda, _) = find_buffer_vault_authority_pda();
    let buffer_vault_ata = derive_ata(&buffer_vault_authority_pda, &onyc_mint, &TOKEN_PROGRAM_ID);
    assert_eq!(get_token_balance(&svm, &buffer_vault_ata), 0);
    assert_eq!(get_mint_supply(&svm, &onyc_mint), 1_000_000_000);
}

#[test]
fn test_set_buffer_fee_config_rejects_no_change() {
    let (mut svm, payer, _token_in_mint, _onyc_mint, _buffer_admin) =
        setup_buffer_context(150_000, 50_000, 100, 1_000);
    let boss = payer.pubkey();

    let ix = build_set_buffer_fee_config_ix(&boss, 100, 1_000);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "setting same fee config should fail");
}

#[test]
fn test_manage_buffer_splits_fees_into_separate_vaults() {
    let (svm, _payer, _token_in_mint, onyc_mint, _buffer_admin) =
        setup_buffer_with_fee_split(100, 1_000, 1);

    let (buffer_vault_authority_pda, _) = find_buffer_vault_authority_pda();
    let buffer_vault_ata = derive_ata(&buffer_vault_authority_pda, &onyc_mint, &TOKEN_PROGRAM_ID);
    let (management_fee_vault_authority_pda, _) = find_management_fee_vault_authority_pda();
    let management_fee_vault_ata = derive_ata(
        &management_fee_vault_authority_pda,
        &onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let (performance_fee_vault_authority_pda, _) = find_performance_fee_vault_authority_pda();
    let performance_fee_vault_ata = derive_ata(
        &performance_fee_vault_authority_pda,
        &onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let buffer_state = read_buffer_state(&svm);

    assert_eq!(get_token_balance(&svm, &buffer_vault_ata), 81_000_000);
    assert_eq!(
        get_token_balance(&svm, &management_fee_vault_ata),
        10_000_000
    );
    assert_eq!(
        get_token_balance(&svm, &performance_fee_vault_ata),
        9_000_000
    );
    assert!(buffer_state.performance_fee_high_watermark > NAV_1_0);
    assert_eq!(get_mint_supply(&svm, &onyc_mint), 1_100_000_000);
}

#[test]
fn test_withdraw_fees_updates_vault_balances() {
    let (mut svm, payer, _token_in_mint, onyc_mint, _buffer_admin) =
        setup_buffer_with_fee_split(100, 1_000, 1);
    let boss = payer.pubkey();

    let ix = build_withdraw_management_fees_ix(&boss, &onyc_mint, 400_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_withdraw_performance_fees_ix(&boss, &onyc_mint, 900_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (management_fee_vault_authority_pda, _) = find_management_fee_vault_authority_pda();
    let management_fee_vault_ata = derive_ata(
        &management_fee_vault_authority_pda,
        &onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let (performance_fee_vault_authority_pda, _) = find_performance_fee_vault_authority_pda();
    let performance_fee_vault_ata = derive_ata(
        &performance_fee_vault_authority_pda,
        &onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let boss_onyc_ata = derive_ata(&boss, &onyc_mint, &TOKEN_PROGRAM_ID);
    let buffer_state = read_buffer_state(&svm);

    assert_eq!(
        get_token_balance(&svm, &management_fee_vault_ata),
        9_600_000
    );
    assert_eq!(
        get_token_balance(&svm, &performance_fee_vault_ata),
        8_100_000
    );
    assert_eq!(get_token_balance(&svm, &boss_onyc_ata), 1_001_300_000);
    let _ = buffer_state;
}

#[test]
fn test_performance_fee_waits_for_high_watermark_recovery() {
    let (mut svm, payer, token_in_mint, onyc_mint, buffer_admin) =
        setup_buffer_with_fee_split(100, 1_000, 2);
    let boss = payer.pubkey();
    let offer_pda = read_state(&svm).main_offer;

    let ix =
        build_burn_for_nav_increase_ix(&boss, &token_in_mint, &onyc_mint, 110_000_000, NAV_1_0);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    assert!(read_buffer_state(&svm).performance_fee_high_watermark > NAV_1_0);
    let (performance_fee_vault_authority_pda, _) = find_performance_fee_vault_authority_pda();
    let performance_fee_vault_ata = derive_ata(
        &performance_fee_vault_authority_pda,
        &onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let performance_fee_balance_before = get_token_balance(&svm, &performance_fee_vault_ata);

    advance_clock_by(&mut svm, ONE_YEAR_SECONDS);
    let ix = build_manage_buffer_ix(&buffer_admin.pubkey(), &offer_pda, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&buffer_admin]).unwrap();

    let (buffer_vault_authority_pda, _) = find_buffer_vault_authority_pda();
    let buffer_vault_ata = derive_ata(&buffer_vault_authority_pda, &onyc_mint, &TOKEN_PROGRAM_ID);
    let buffer_state = read_buffer_state(&svm);

    assert!(get_token_balance(&svm, &buffer_vault_ata) < 162_000_000);
    assert!(get_token_balance(&svm, &performance_fee_vault_ata) > performance_fee_balance_before);
    assert!(buffer_state.performance_fee_high_watermark > NAV_1_0);
}

#[test]
fn test_manage_buffer_zero_seconds_mints_nothing() {
    let (mut svm, _payer, _token_in_mint, onyc_mint, buffer_admin) =
        setup_buffer_context(150_000, 50_000, 0, 0);
    let offer_pda = read_state(&svm).main_offer;

    let ix = build_manage_buffer_ix(&buffer_admin.pubkey(), &offer_pda, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&buffer_admin]).unwrap();
    advance_slot(&mut svm);

    // Same timestamp call should mint zero.
    let ix = build_manage_buffer_ix(&buffer_admin.pubkey(), &offer_pda, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&buffer_admin]).unwrap();

    let (buffer_vault_authority_pda, _) = find_buffer_vault_authority_pda();
    let buffer_vault_ata = derive_ata(&buffer_vault_authority_pda, &onyc_mint, &TOKEN_PROGRAM_ID);
    assert_eq!(get_token_balance(&svm, &buffer_vault_ata), 0);
}

#[test]
fn test_manage_buffer_partial_period_math() {
    let (mut svm, _payer, _token_in_mint, onyc_mint, buffer_admin) =
        setup_buffer_context(150_000, 50_000, 0, 0);
    let offer_pda = read_state(&svm).main_offer;

    // Baseline call sets lowest_supply.
    let ix = build_manage_buffer_ix(&buffer_admin.pubkey(), &offer_pda, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&buffer_admin]).unwrap();
    advance_slot(&mut svm);

    advance_clock_by(&mut svm, ONE_DAY_SECONDS);
    let ix = build_manage_buffer_ix(&buffer_admin.pubkey(), &offer_pda, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&buffer_admin]).unwrap();

    // Accrual formula:
    // mint = lowest_supply * spread * elapsed / SECONDS_PER_YEAR / YIELD_SCALE
    // Here:
    // lowest_supply = 1_000_000_000
    // spread = gross - current = 150_000 - 50_000 = 100_000 (10% APR in 1e6 scale)
    // elapsed = 86_400 (1 day)
    // mint = 1_000_000_000 * 100_000 * 86_400 / 31_536_000 / 1_000_000 = 273_972
    let expected_mint = 273_972u64;
    let (buffer_vault_authority_pda, _) = find_buffer_vault_authority_pda();
    let buffer_vault_ata = derive_ata(&buffer_vault_authority_pda, &onyc_mint, &TOKEN_PROGRAM_ID);
    assert_eq!(get_token_balance(&svm, &buffer_vault_ata), expected_mint);
    assert_eq!(
        get_mint_supply(&svm, &onyc_mint),
        1_000_000_000 + expected_mint
    );
}

#[test]
fn test_manage_buffer_mints_expected_amount() {
    let (svm, _payer, _token_in_mint, onyc_mint, _buffer_admin) = setup_buffer_with_balance();

    let (buffer_vault_authority_pda, _) = find_buffer_vault_authority_pda();
    let buffer_vault_ata = derive_ata(&buffer_vault_authority_pda, &onyc_mint, &TOKEN_PROGRAM_ID);

    assert_eq!(get_token_balance(&svm, &buffer_vault_ata), 100_000_000);
    assert_eq!(get_mint_supply(&svm, &onyc_mint), 1_100_000_000);
    let market_stats = read_market_stats(&svm);
    assert_eq!(market_stats.circulating_supply, 1_100_000_000);
    assert!(market_stats.tvl > market_stats.circulating_supply);
}

#[test]
fn test_manage_buffer_allows_non_buffer_admin() {
    let (mut svm, payer, _token_in_mint, onyc_mint, buffer_admin) = setup_buffer_with_balance();
    let boss = payer.pubkey();
    let offer_pda = read_state(&svm).main_offer;

    // Change admin to a new key, then ensure the old admin can still call manage_buffer.
    let new_admin = Keypair::new();
    let ix = build_set_buffer_admin_ix(&boss, &new_admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    advance_clock_by(&mut svm, ONE_YEAR_SECONDS);
    let ix = build_manage_buffer_ix(&buffer_admin.pubkey(), &offer_pda, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&buffer_admin]).unwrap();
}

#[test]
fn test_burn_for_nav_increase_success() {
    let (mut svm, payer, token_in_mint, onyc_mint, _buffer_admin) = setup_buffer_with_balance();
    let boss = payer.pubkey();

    // `asset_adjustment_amount` is in token-in units.
    // For this test token_in has 6 decimals (USDC-like), so 50_000_000 = 50 USDC.
    //
    // With NAV = 1.0 (1e9 scale), each burned ONyc base unit removes one unit of
    // ONyc supply needed to keep NAV target after a same-sized asset adjustment.
    // So a 50 USDC adjustment burns 50_000_000 ONyc base units in this setup.
    let ix = build_burn_for_nav_increase_ix(&boss, &token_in_mint, &onyc_mint, 50_000_000, NAV_1_0);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (buffer_vault_authority_pda, _) = find_buffer_vault_authority_pda();
    let buffer_vault_ata = derive_ata(&buffer_vault_authority_pda, &onyc_mint, &TOKEN_PROGRAM_ID);

    assert_eq!(get_token_balance(&svm, &buffer_vault_ata), 50_000_000);
    assert_eq!(get_mint_supply(&svm, &onyc_mint), 1_050_000_000);
    let market_stats = read_market_stats(&svm);
    assert_eq!(market_stats.circulating_supply, 1_050_000_000);
    assert_eq!(market_stats.tvl, 1_050_000_000);
}

#[test]
fn test_burn_for_nav_increase_rejects_non_boss() {
    let (mut svm, _payer, token_in_mint, onyc_mint, _buffer_admin) = setup_buffer_with_balance();

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_burn_for_nav_increase_ix(
        &non_boss.pubkey(),
        &token_in_mint,
        &onyc_mint,
        50_000_000,
        NAV_1_0,
    );
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(result.is_err(), "non-boss should not be able to burn");
}

#[test]
fn test_burn_for_nav_increase_rejects_target_nav_zero() {
    let (mut svm, payer, token_in_mint, onyc_mint, _buffer_admin) = setup_buffer_with_balance();
    let boss = payer.pubkey();

    let ix = build_burn_for_nav_increase_ix(&boss, &token_in_mint, &onyc_mint, 10_000_000, 0);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "target NAV=0 should fail");
}

#[test]
fn test_burn_for_nav_increase_rejects_asset_adjustment_above_total_assets() {
    let (mut svm, payer, token_in_mint, onyc_mint, _buffer_admin) = setup_buffer_with_balance();
    let boss = payer.pubkey();

    // Asset adjustment is token-in units (USDC-like 6 decimals here).
    let ix =
        build_burn_for_nav_increase_ix(&boss, &token_in_mint, &onyc_mint, 2_000_000_000, NAV_1_0);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "asset adjustment above total assets should fail"
    );
}

#[test]
fn test_burn_for_nav_increase_rejects_insufficient_buffer_balance() {
    let (mut svm, payer, token_in_mint, onyc_mint, _buffer_admin) = setup_buffer_with_balance();
    let boss = payer.pubkey();

    let ix =
        build_burn_for_nav_increase_ix(&boss, &token_in_mint, &onyc_mint, 200_000_000, NAV_1_0);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "burn amount above cache balance should fail"
    );
}

#[test]
fn test_burn_for_nav_increase_rejects_no_burn_needed() {
    let (mut svm, payer, token_in_mint, onyc_mint, _buffer_admin) = setup_buffer_with_balance();
    let boss = payer.pubkey();

    let ix = build_burn_for_nav_increase_ix(&boss, &token_in_mint, &onyc_mint, 0, NAV_1_0);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "zero asset adjustment at current NAV should require no burn"
    );
}

#[test]
fn test_burn_for_nav_increase_rejects_invalid_burn_target() {
    let (mut svm, payer, token_in_mint, onyc_mint, _buffer_admin) = setup_buffer_with_balance();
    let boss = payer.pubkey();

    let ix = build_burn_for_nav_increase_ix(&boss, &token_in_mint, &onyc_mint, 0, 900_000_000);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "target NAV below current implied NAV should fail"
    );
}
