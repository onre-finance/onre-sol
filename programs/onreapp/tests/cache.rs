mod common;

use common::*;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

const ONE_YEAR_SECONDS: u64 = 31_536_000;
const ONE_DAY_SECONDS: u64 = 86_400;
// NAV/price scale is 1e9: 1.0 NAV = 1_000_000_000.
const NAV_1_0: u64 = 1_000_000_000;

// CACHE math reference used by tests:
//
// Accrue:
//   spread = max(0, gross_yield - current_yield)        // APR scale 1e6
//   mint   = lowest_supply * spread * dt / YEAR / 1e6
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

fn setup_cache_context(
    gross_yield: u64,
    current_yield: u64,
) -> (litesvm::LiteSVM, Keypair, Pubkey, Pubkey, Keypair) {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let token_in_mint = create_mint(&mut svm, &payer, 6, &boss);
    let cache_admin = Keypair::new();
    svm.airdrop(&cache_admin.pubkey(), INITIAL_LAMPORTS)
        .unwrap();

    let ix = build_make_offer_ix(&boss, &token_in_mint, &onyc_mint, 0, false, true);
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

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_mint_to_ix(&boss, &onyc_mint, 1_000_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_initialize_cache_ix(&boss, &onyc_mint, &cache_admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_set_cache_yields_ix(&boss, gross_yield, current_yield);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    (svm, payer, token_in_mint, onyc_mint, cache_admin)
}

fn setup_cache_with_balance() -> (litesvm::LiteSVM, Keypair, Pubkey, Pubkey, Keypair) {
    let (mut svm, payer, token_in_mint, onyc_mint, cache_admin) =
        setup_cache_context(150_000, 50_000);

    // First accrual initializes lowest_supply to current supply.
    let ix = build_accrue_cache_ix(&cache_admin.pubkey(), &onyc_mint);
    send_tx(&mut svm, &[ix], &[&cache_admin]).unwrap();
    advance_slot(&mut svm);

    // Second accrual after one year should mint 10% of 1_000_000_000 = 100_000_000.
    advance_clock_by(&mut svm, ONE_YEAR_SECONDS);
    let ix = build_accrue_cache_ix(&cache_admin.pubkey(), &onyc_mint);
    send_tx(&mut svm, &[ix], &[&cache_admin]).unwrap();

    (svm, payer, token_in_mint, onyc_mint, cache_admin)
}

#[test]
fn test_initialize_cache_success() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let cache_admin = Keypair::new();

    let ix = build_initialize_cache_ix(&boss, &onyc_mint, &cache_admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let cache_state = read_cache_state(&svm);
    assert_eq!(cache_state.onyc_mint, onyc_mint);
    assert_eq!(cache_state.cache_admin, cache_admin.pubkey());
    assert_eq!(cache_state.gross_yield, 0);
    assert_eq!(cache_state.current_yield, 0);
    assert_eq!(cache_state.lowest_supply, 0);

    let (cache_vault_authority_pda, _) = find_cache_vault_authority_pda();
    let cache_vault_ata = derive_ata(&cache_vault_authority_pda, &onyc_mint, &TOKEN_PROGRAM_ID);
    assert!(svm.get_account(&cache_vault_ata).is_some());
}

#[test]
fn test_set_cache_admin_boss_only() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let admin1 = Keypair::new();
    let admin2 = Keypair::new();

    let ix = build_initialize_cache_ix(&boss, &onyc_mint, &admin1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_set_cache_admin_ix(&boss, &admin2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    assert_eq!(read_cache_state(&svm).cache_admin, admin2.pubkey());

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();
    let ix = build_set_cache_admin_ix(&non_boss.pubkey(), &admin1.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(result.is_err(), "non-boss should not set cache admin");
}

#[test]
fn test_set_cache_admin_rejects_no_change() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let admin = Keypair::new();

    let ix = build_initialize_cache_ix(&boss, &onyc_mint, &admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_set_cache_admin_ix(&boss, &admin.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "setting same cache admin should fail");
}

#[test]
fn test_set_cache_yields_rejects_no_change() {
    let (mut svm, payer, _token_in_mint, _onyc_mint, _cache_admin) =
        setup_cache_context(150_000, 50_000);
    let boss = payer.pubkey();

    let ix = build_set_cache_yields_ix(&boss, 150_000, 50_000);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "setting same yields should fail");
}

#[test]
fn test_accrue_cache_first_call_sets_lowest_supply_no_mint() {
    let (mut svm, _payer, _token_in_mint, onyc_mint, cache_admin) =
        setup_cache_context(150_000, 50_000);
    let initial_supply = get_mint_supply(&svm, &onyc_mint);

    let ix = build_accrue_cache_ix(&cache_admin.pubkey(), &onyc_mint);
    send_tx(&mut svm, &[ix], &[&cache_admin]).unwrap();

    let (cache_vault_authority_pda, _) = find_cache_vault_authority_pda();
    let cache_vault_ata = derive_ata(&cache_vault_authority_pda, &onyc_mint, &TOKEN_PROGRAM_ID);
    let cache_state = read_cache_state(&svm);

    assert_eq!(get_token_balance(&svm, &cache_vault_ata), 0);
    assert_eq!(get_mint_supply(&svm, &onyc_mint), initial_supply);
    assert_eq!(cache_state.lowest_supply, initial_supply);
}

#[test]
fn test_accrue_cache_zero_spread_mints_nothing() {
    let (mut svm, _payer, _token_in_mint, onyc_mint, cache_admin) =
        setup_cache_context(50_000, 50_000);

    // Baseline call sets lowest_supply.
    let ix = build_accrue_cache_ix(&cache_admin.pubkey(), &onyc_mint);
    send_tx(&mut svm, &[ix], &[&cache_admin]).unwrap();
    advance_slot(&mut svm);

    advance_clock_by(&mut svm, ONE_YEAR_SECONDS);
    let ix = build_accrue_cache_ix(&cache_admin.pubkey(), &onyc_mint);
    send_tx(&mut svm, &[ix], &[&cache_admin]).unwrap();

    let (cache_vault_authority_pda, _) = find_cache_vault_authority_pda();
    let cache_vault_ata = derive_ata(&cache_vault_authority_pda, &onyc_mint, &TOKEN_PROGRAM_ID);
    assert_eq!(get_token_balance(&svm, &cache_vault_ata), 0);
    assert_eq!(get_mint_supply(&svm, &onyc_mint), 1_000_000_000);
}

#[test]
fn test_accrue_cache_zero_seconds_mints_nothing() {
    let (mut svm, _payer, _token_in_mint, onyc_mint, cache_admin) =
        setup_cache_context(150_000, 50_000);

    let ix = build_accrue_cache_ix(&cache_admin.pubkey(), &onyc_mint);
    send_tx(&mut svm, &[ix], &[&cache_admin]).unwrap();
    advance_slot(&mut svm);

    // Same timestamp call should mint zero.
    let ix = build_accrue_cache_ix(&cache_admin.pubkey(), &onyc_mint);
    send_tx(&mut svm, &[ix], &[&cache_admin]).unwrap();

    let (cache_vault_authority_pda, _) = find_cache_vault_authority_pda();
    let cache_vault_ata = derive_ata(&cache_vault_authority_pda, &onyc_mint, &TOKEN_PROGRAM_ID);
    assert_eq!(get_token_balance(&svm, &cache_vault_ata), 0);
}

#[test]
fn test_accrue_cache_partial_period_math() {
    let (mut svm, _payer, _token_in_mint, onyc_mint, cache_admin) =
        setup_cache_context(150_000, 50_000);

    // Baseline call sets lowest_supply.
    let ix = build_accrue_cache_ix(&cache_admin.pubkey(), &onyc_mint);
    send_tx(&mut svm, &[ix], &[&cache_admin]).unwrap();
    advance_slot(&mut svm);

    advance_clock_by(&mut svm, ONE_DAY_SECONDS);
    let ix = build_accrue_cache_ix(&cache_admin.pubkey(), &onyc_mint);
    send_tx(&mut svm, &[ix], &[&cache_admin]).unwrap();

    // Accrual formula:
    // mint = lowest_supply * spread * elapsed / SECONDS_PER_YEAR / YIELD_SCALE
    // Here:
    // lowest_supply = 1_000_000_000
    // spread = gross - current = 150_000 - 50_000 = 100_000 (10% APR in 1e6 scale)
    // elapsed = 86_400 (1 day)
    // mint = 1_000_000_000 * 100_000 * 86_400 / 31_536_000 / 1_000_000 = 273_972
    let expected_mint = 273_972u64;
    let (cache_vault_authority_pda, _) = find_cache_vault_authority_pda();
    let cache_vault_ata = derive_ata(&cache_vault_authority_pda, &onyc_mint, &TOKEN_PROGRAM_ID);
    assert_eq!(get_token_balance(&svm, &cache_vault_ata), expected_mint);
    assert_eq!(
        get_mint_supply(&svm, &onyc_mint),
        1_000_000_000 + expected_mint
    );
}

#[test]
fn test_update_lowest_supply_no_change_when_supply_higher() {
    let (mut svm, payer, _token_in_mint, onyc_mint, _cache_admin) = setup_cache_with_balance();
    let before = read_cache_state(&svm).lowest_supply;

    let ix = build_update_lowest_supply_ix(&onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let after = read_cache_state(&svm).lowest_supply;
    assert_eq!(after, before, "lowest supply should remain unchanged");
}

#[test]
fn test_accrue_cache_mints_expected_amount() {
    let (svm, _payer, _token_in_mint, onyc_mint, _cache_admin) = setup_cache_with_balance();

    let (cache_vault_authority_pda, _) = find_cache_vault_authority_pda();
    let cache_vault_ata = derive_ata(&cache_vault_authority_pda, &onyc_mint, &TOKEN_PROGRAM_ID);

    assert_eq!(get_token_balance(&svm, &cache_vault_ata), 100_000_000);
    assert_eq!(get_mint_supply(&svm, &onyc_mint), 1_100_000_000);
}

#[test]
fn test_accrue_cache_rejects_non_cache_admin() {
    let (mut svm, payer, _token_in_mint, onyc_mint, cache_admin) = setup_cache_with_balance();
    let boss = payer.pubkey();

    // Change admin to a new key, then try with old admin.
    let new_admin = Keypair::new();
    let ix = build_set_cache_admin_ix(&boss, &new_admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_accrue_cache_ix(&cache_admin.pubkey(), &onyc_mint);
    let result = send_tx(&mut svm, &[ix], &[&cache_admin]);
    assert!(result.is_err(), "old cache admin should be rejected");
}

#[test]
fn test_burn_for_nav_increase_success() {
    let (mut svm, payer, token_in_mint, onyc_mint, _cache_admin) = setup_cache_with_balance();
    let boss = payer.pubkey();

    // `asset_adjustment_amount` is in token-in units.
    // For this test token_in has 6 decimals (USDC-like), so 50_000_000 = 50 USDC.
    //
    // With NAV = 1.0 (1e9 scale), each burned ONyc base unit removes one unit of
    // ONyc supply needed to keep NAV target after a same-sized asset adjustment.
    // So a 50 USDC adjustment burns 50_000_000 ONyc base units in this setup.
    let ix =
        build_burn_for_nav_increase_ix(&boss, &token_in_mint, &onyc_mint, 50_000_000, NAV_1_0);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (cache_vault_authority_pda, _) = find_cache_vault_authority_pda();
    let cache_vault_ata = derive_ata(&cache_vault_authority_pda, &onyc_mint, &TOKEN_PROGRAM_ID);

    assert_eq!(get_token_balance(&svm, &cache_vault_ata), 50_000_000);
    assert_eq!(get_mint_supply(&svm, &onyc_mint), 1_050_000_000);
}

#[test]
fn test_burn_for_nav_increase_rejects_non_boss() {
    let (mut svm, _payer, token_in_mint, onyc_mint, _cache_admin) = setup_cache_with_balance();

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
    let (mut svm, payer, token_in_mint, onyc_mint, _cache_admin) = setup_cache_with_balance();
    let boss = payer.pubkey();

    let ix = build_burn_for_nav_increase_ix(&boss, &token_in_mint, &onyc_mint, 10_000_000, 0);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "target NAV=0 should fail");
}

#[test]
fn test_burn_for_nav_increase_rejects_asset_adjustment_above_total_assets() {
    let (mut svm, payer, token_in_mint, onyc_mint, _cache_admin) = setup_cache_with_balance();
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
fn test_burn_for_nav_increase_rejects_insufficient_cache_balance() {
    let (mut svm, payer, token_in_mint, onyc_mint, _cache_admin) = setup_cache_with_balance();
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
    let (mut svm, payer, token_in_mint, onyc_mint, _cache_admin) = setup_cache_with_balance();
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
    let (mut svm, payer, token_in_mint, onyc_mint, _cache_admin) = setup_cache_with_balance();
    let boss = payer.pubkey();

    let ix = build_burn_for_nav_increase_ix(&boss, &token_in_mint, &onyc_mint, 0, 900_000_000);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "target NAV below current implied NAV should fail"
    );
}
