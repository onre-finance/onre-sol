mod common;

use anchor_lang::AccountDeserialize;
use common::*;
use onreapp::state::{CirculatingSupplyExcludedAccounts, CirculatingSupplyExcludedBalance};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

fn setup_offer_with_vector(
    apr: u64,
    base_price: u64,
    price_fix_duration: u64,
) -> (
    litesvm::LiteSVM,
    Keypair,
    solana_sdk::pubkey::Pubkey,
    solana_sdk::pubkey::Pubkey,
) {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time,
        base_price,
        apr,
        price_fix_duration,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Advance so vector becomes active
    advance_clock_by(&mut svm, 1);

    (svm, payer, token_in, token_out)
}

fn setup_onyc_offer_with_supply(
    apr: u64,
    base_price: u64,
    price_fix_duration: u64,
    minted_supply: u64,
    vault_balance: u64,
) -> (
    litesvm::LiteSVM,
    Keypair,
    solana_sdk::pubkey::Pubkey,
    solana_sdk::pubkey::Pubkey,
) {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let token_in = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &onyc_mint,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let (offer_pda, _) = find_offer_pda(&token_in, &onyc_mint);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &onyc_mint,
        None,
        current_time,
        base_price,
        apr,
        price_fix_duration,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    if vault_balance > 0 {
        let (vault_authority, _) = find_offer_vault_authority_pda();
        create_token_account(&mut svm, &onyc_mint, &vault_authority, vault_balance);
    }

    let mut mint_data = svm.get_account(&onyc_mint).unwrap();
    mint_data.data[36..44].copy_from_slice(&minted_supply.to_le_bytes());
    svm.set_account(onyc_mint, mint_data).unwrap();

    (svm, payer, token_in, onyc_mint)
}

fn read_circulating_supply_excluded_accounts(
    svm: &litesvm::LiteSVM,
) -> CirculatingSupplyExcludedAccounts {
    let (pda, _) = find_circulating_supply_excluded_accounts_pda();
    let account = svm
        .get_account(&pda)
        .expect("excluded accounts PDA not found");
    let mut data = account.data.as_slice();
    CirculatingSupplyExcludedAccounts::try_deserialize(&mut data)
        .expect("failed to deserialize excluded accounts PDA")
}

fn read_circulating_supply_excluded_balance(
    svm: &litesvm::LiteSVM,
) -> CirculatingSupplyExcludedBalance {
    let (pda, _) = find_circulating_supply_excluded_balance_pda();
    let account = svm
        .get_account(&pda)
        .expect("excluded balance PDA not found");
    let mut data = account.data.as_slice();
    CirculatingSupplyExcludedBalance::try_deserialize(&mut data)
        .expect("failed to deserialize excluded balance PDA")
}

// ---------------------------------------------------------------------------
// get_nav
// ---------------------------------------------------------------------------

#[test]
fn test_get_nav_success() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(
        36_500,        // 3.65% APR
        1_000_000_000, // base_price = 1.0
        86400,         // 1 day
    );

    let ix = build_get_nav_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let nav = get_return_u64(&result);

    // Step pricing: step=0, interval=(0+1)*86400, so price grows slightly from base
    // price = 1e9 * (1 + 36500 * 86400 / (1e6 * 31536000)) = 1_000_100_000
    assert_eq!(nav, 1_000_100_000);
}

#[test]
fn test_get_nav_price_growth() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(
        36_500, // 3.65% APR
        1_000_000_000,
        86400, // 1 day
    );

    // Advance 1 day so price should have grown
    advance_clock_by(&mut svm, 86400);

    let ix = build_get_nav_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let nav = get_return_u64(&result);

    // After 1 day with 3.65% APR: price = 1.0 * (1 + 0.0365 * 86400 / 31536000) = ~1.0001
    // Step price: elapsed=86400, step = 86400/86400 = 1, interval = 2 * 86400 = 172800
    // price = 1_000_000_000 * (1 + 36500 * 172800 / (1_000_000 * 31_536_000))
    // = 1_000_000_000 * (1 + 6307200000 / 31536000000000)
    // = 1_000_000_000 * (1 + 0.0002)
    // = 1_000_200_010 after daily compounding
    assert_eq!(nav, 1_000_200_010);
}

#[test]
fn test_get_nav_zero_apr() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(
        0, // 0% APR
        1_000_000_000,
        86400,
    );

    advance_clock_by(&mut svm, 86400 * 30);

    let ix = build_get_nav_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let nav = get_return_u64(&result);

    // With 0% APR, price stays the same regardless of time
    assert_eq!(nav, 1_000_000_000);
}

#[test]
fn test_get_nav_fails_no_active_vector() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // No vectors added
    let ix = build_get_nav_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "should fail with no active vector");
}

#[test]
fn test_get_nav_fails_all_vectors_future() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 100_000,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_get_nav_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "should fail when all vectors are in the future"
    );
}

// ---------------------------------------------------------------------------
// get_apy
// ---------------------------------------------------------------------------

#[test]
fn test_get_apy_success() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(
        100_000, // 10% APR
        1_000_000_000,
        86400,
    );

    let ix = build_get_apy_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let apy = get_return_u64(&result);

    // 10% APR -> ~10.52% APY with daily compounding.
    assert_eq!(apy, 105_156);
}

#[test]
fn test_get_apy_zero_apr() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(0, 1_000_000_000, 86400);

    let ix = build_get_apy_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let apy = get_return_u64(&result);

    assert_eq!(apy, 0, "0% APR should give 0% APY");
}

#[test]
fn test_get_apy_fails_no_active_vector() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_get_apy_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "should fail with no active vector");
}

// ---------------------------------------------------------------------------
// refresh_market_stats
// ---------------------------------------------------------------------------

#[test]
fn test_refresh_market_stats_permissionless_creates_and_updates_pda() {
    let (mut svm, payer, token_in, onyc_mint) =
        setup_onyc_offer_with_supply(36_500, 1_000_000_000, 86_400, 5_000_000_000, 2_000_000_000);
    let caller = Keypair::new();
    svm.airdrop(&caller.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix =
        build_refresh_market_stats_ix(&caller.pubkey(), &payer.pubkey(), &token_in, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&caller]).unwrap();

    let market_stats = read_market_stats(&svm);
    assert_eq!(market_stats.bump, find_market_stats_pda().1);
    assert_eq!(market_stats.apy, 37_172);
    assert_eq!(market_stats.nav, 1_000_100_000);
    assert_eq!(market_stats.nav_adjustment, 1_000_100_000);
    assert_eq!(market_stats.circulating_supply, 3_000_000_000);
    assert_eq!(market_stats.tvl, 3_000_300_000);
    assert_eq!(market_stats.last_updated_at, 1_704_067_201);
    assert_eq!(market_stats.last_updated_slot, 3);
}

#[test]
fn test_refresh_market_stats_succeeds_without_recent_purchases() {
    let (mut svm, payer, token_in, onyc_mint) =
        setup_onyc_offer_with_supply(0, 1_000_000_000, 86_400, 7_000_000_000, 1_500_000_000);

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &payer.pubkey(), &token_in, &onyc_mint);
    send_tx(&mut svm, &[ix.clone()], &[&payer]).unwrap();
    let initial = read_market_stats(&svm);

    advance_clock_by(&mut svm, 86_400);

    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let refreshed = read_market_stats(&svm);

    assert_eq!(initial.circulating_supply, 5_500_000_000);
    assert_eq!(initial.nav, 1_000_000_000);
    assert_eq!(refreshed.circulating_supply, initial.circulating_supply);
    assert_eq!(refreshed.nav, initial.nav);
    assert_eq!(initial.last_updated_at, 1_704_067_201);
    assert_eq!(initial.last_updated_slot, 3);
    assert_eq!(refreshed.last_updated_at, 1_704_153_601);
    assert_eq!(refreshed.last_updated_slot, 4);
}

// ---------------------------------------------------------------------------
// get_nav_adjustment
// ---------------------------------------------------------------------------

#[test]
fn test_get_nav_adjustment_first_vector() {
    let (mut svm, payer, token_in, token_out) =
        setup_offer_with_vector(36_500, 1_000_000_000, 86400);

    let ix = build_get_nav_adjustment_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let adjustment = get_return_i64(&result);

    // First vector: adjustment = current_price (no previous)
    // Step pricing: first interval gives slight growth from base_price
    assert_eq!(adjustment, 1_000_100_000);
}

#[test]
fn test_get_nav_adjustment_positive() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);

    // Vector 1: base_price = 1.0
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Vector 2: base_price = 1.1, starts later
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 100,
        1_100_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Advance to make vector 2 active
    advance_clock_by(&mut svm, 101);

    let ix = build_get_nav_adjustment_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let adjustment = get_return_i64(&result);

    // Adjustment = 1.1 - 1.0 = 0.1 = 100_000_000
    assert_eq!(adjustment, 100_000_000);
}

#[test]
fn test_get_nav_adjustment_fails_no_active_vector() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_get_nav_adjustment_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "should fail with no active vector");
}

// ---------------------------------------------------------------------------
// get_tvl
// ---------------------------------------------------------------------------

#[test]
fn test_get_tvl_success() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(0, 1_000_000_000, 86400);

    // Mint some token_out supply
    let mut mint_data = svm.get_account(&token_out).unwrap();
    mint_data.data[36..44].copy_from_slice(&1_000_000_000_000u64.to_le_bytes()); // 1000 tokens
    svm.set_account(token_out, mint_data).unwrap();

    let ix = build_get_tvl_ix(&payer.pubkey(), &token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let tvl = get_return_u64(&result);

    // TVL = supply * price / 10^9 = 1000e9 * 1e9 / 1e9 = 1000e9
    assert_eq!(tvl, 1_000_000_000_000);
}

#[test]
fn test_get_tvl_fails_no_active_vector() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_get_tvl_ix(&payer.pubkey(), &token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "should fail with no active vector");
}

// ---------------------------------------------------------------------------
// get_circulating_supply
// ---------------------------------------------------------------------------

#[test]
fn test_get_circulating_supply_no_vault() {
    let (mut svm, payer, onyc_mint) = setup_initialized();

    // Set some supply on the onyc mint
    let mut mint_data = svm.get_account(&onyc_mint).unwrap();
    mint_data.data[36..44].copy_from_slice(&500_000_000_000u64.to_le_bytes()); // 500 tokens
    svm.set_account(onyc_mint, mint_data).unwrap();

    let ix = build_get_circulating_supply_ix(&payer.pubkey(), &onyc_mint);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let supply = get_return_u64(&result);

    // No vault, so circulating = total
    assert_eq!(supply, 500_000_000_000);
}

#[test]
fn test_get_circulating_supply_with_vault() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    // Create boss token account with tokens and deposit to vault
    create_token_account(&mut svm, &onyc_mint, &boss, 500_000_000_000);
    let mut mint_data = svm.get_account(&onyc_mint).unwrap();
    mint_data.data[36..44].copy_from_slice(&1_000_000_000_000u64.to_le_bytes()); // 1000 tokens
    svm.set_account(onyc_mint, mint_data).unwrap();
    let ix = build_offer_vault_deposit_ix(&boss, &onyc_mint, 200_000_000_000, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_get_circulating_supply_ix(&payer.pubkey(), &onyc_mint);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let supply = get_return_u64(&result);

    // circulating = total - (vault + boss ONyc) = 1000e9 - (200e9 + 300e9) = 500e9
    assert_eq!(supply, 500_000_000_000);
}

// ---------------------------------------------------------------------------
// circulating supply excluded balance PDA
// ---------------------------------------------------------------------------

#[test]
fn test_set_circulating_supply_excluded_accounts_boss_only_and_stores_owners() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let owner_a = Pubkey::new_unique();
    let owner_b = Pubkey::new_unique();
    let mut owners = [Pubkey::default(); 20];
    owners[0] = owner_a;
    owners[1] = owner_b;

    let ix = build_set_circulating_supply_excluded_accounts_ix(&boss, &owners);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let excluded_accounts = read_circulating_supply_excluded_accounts(&svm);
    assert_eq!(excluded_accounts.owners, owners);

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();
    let mut updated_owners = owners;
    updated_owners[2] = Pubkey::new_unique();
    let ix = build_set_circulating_supply_excluded_accounts_ix(&non_boss.pubkey(), &updated_owners);
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(
        result.is_err(),
        "non-boss should not update excluded owners"
    );
}

#[test]
fn test_set_circulating_supply_excluded_accounts_rejects_duplicate_owners() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let owner = Pubkey::new_unique();
    let mut owners = [Pubkey::default(); 20];
    owners[0] = owner;
    owners[1] = owner;

    let ix = build_set_circulating_supply_excluded_accounts_ix(&boss, &owners);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "duplicate non-default owners should fail");
}

#[test]
fn test_update_circulating_supply_excluded_balance_sums_configured_atas() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let owner_a = Pubkey::new_unique();
    let owner_b = Pubkey::new_unique();
    let mut owners = [Pubkey::default(); 20];
    owners[0] = owner_a;
    owners[1] = owner_b;

    let set_ix = build_set_circulating_supply_excluded_accounts_ix(&boss, &owners);
    send_tx(&mut svm, &[set_ix], &[&payer]).unwrap();

    let ata_a = create_token_account(&mut svm, &onyc_mint, &owner_a, 125_000_000);
    let ata_b = create_token_account(&mut svm, &onyc_mint, &owner_b, 875_000_000);
    let update_ix = build_update_circulating_supply_excluded_balance_ix(
        &boss,
        &onyc_mint,
        &[ata_a, ata_b],
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[update_ix], &[&payer]).unwrap();

    let excluded_balance = read_circulating_supply_excluded_balance(&svm);
    assert_eq!(excluded_balance.amount, 1_000_000_000);
    assert_eq!(excluded_balance.last_updated_at, 1_704_067_200);
}

#[test]
fn test_update_circulating_supply_excluded_balance_rejects_missing_or_extra_atas() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let owner_a = Pubkey::new_unique();
    let owner_b = Pubkey::new_unique();
    let mut owners = [Pubkey::default(); 20];
    owners[0] = owner_a;
    owners[1] = owner_b;

    let set_ix = build_set_circulating_supply_excluded_accounts_ix(&boss, &owners);
    send_tx(&mut svm, &[set_ix], &[&payer]).unwrap();

    let ata_a = create_token_account(&mut svm, &onyc_mint, &owner_a, 1);
    let ata_b = create_token_account(&mut svm, &onyc_mint, &owner_b, 2);
    let extra = Pubkey::new_unique();

    let missing_ix = build_update_circulating_supply_excluded_balance_ix(
        &boss,
        &onyc_mint,
        &[ata_a],
        &TOKEN_PROGRAM_ID,
    );
    assert!(
        send_tx(&mut svm, &[missing_ix], &[&payer]).is_err(),
        "missing configured ATA should fail"
    );

    let extra_ix = build_update_circulating_supply_excluded_balance_ix(
        &boss,
        &onyc_mint,
        &[ata_a, ata_b, extra],
        &TOKEN_PROGRAM_ID,
    );
    assert!(
        send_tx(&mut svm, &[extra_ix], &[&payer]).is_err(),
        "extra remaining ATA should fail"
    );
}

#[test]
fn test_v2_market_info_uses_cached_excluded_balance() {
    let (mut svm, payer, token_in, onyc_mint) =
        setup_onyc_offer_with_supply(0, 1_000_000_000, 86_400, 1_000_000_000_000, 0);
    let boss = payer.pubkey();
    let owner = Pubkey::new_unique();
    let mut owners = [Pubkey::default(); 20];
    owners[0] = owner;

    let set_ix = build_set_circulating_supply_excluded_accounts_ix(&boss, &owners);
    send_tx(&mut svm, &[set_ix], &[&payer]).unwrap();

    let excluded_ata = create_token_account(&mut svm, &onyc_mint, &owner, 300_000_000_000);
    set_mint_supply(&mut svm, &onyc_mint, 1_000_000_000_000);
    let update_ix = build_update_circulating_supply_excluded_balance_ix(
        &boss,
        &onyc_mint,
        &[excluded_ata],
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[update_ix], &[&payer]).unwrap();

    let supply_ix = build_get_circulating_supply_v2_ix(&onyc_mint);
    let supply_result = send_tx(&mut svm, &[supply_ix], &[&payer]).unwrap();
    assert_eq!(get_return_u64(&supply_result), 700_000_000_000);

    let tvl_ix = build_get_tvl_v2_ix(&token_in, &onyc_mint);
    let tvl_result = send_tx(&mut svm, &[tvl_ix], &[&payer]).unwrap();
    assert_eq!(get_return_u64(&tvl_result), 700_000_000_000);

    let refresh_ix = build_refresh_market_stats_v2_ix(&boss, &token_in, &onyc_mint);
    send_tx(&mut svm, &[refresh_ix], &[&payer]).unwrap();
    let market_stats = read_market_stats(&svm);
    assert_eq!(market_stats.circulating_supply, 700_000_000_000);
    assert_eq!(market_stats.tvl, 700_000_000_000);
}

// ---------------------------------------------------------------------------
// Additional NAV tests
// ---------------------------------------------------------------------------

#[test]
fn test_get_nav_multiple_vectors_uses_most_recent() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);

    // Vector 1: base_price = 1.0, starts now
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Vector 2: base_price = 2.0, starts later
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 100,
        2_000_000_000,
        73_000,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Advance to make vector 2 active
    advance_clock_by(&mut svm, 101);

    let ix = build_get_nav_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let nav = get_return_u64(&result);

    // Should use vector 2 (base_price=2.0, APR=7.3%)
    // step=0, interval=86400, price = 2.0 * (1 + 73000 * 86400 / (1e6 * 31536000))
    // = 2.0 * (1 + 0.0002) = 2_000_400_000
    assert_eq!(nav, 2_000_400_000);
}

// ---------------------------------------------------------------------------
// Additional APY tests
// ---------------------------------------------------------------------------

#[test]
fn test_get_apy_10_percent() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time,
        1_000_000_000,
        100_000,
        86400, // 10% APR
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    let ix = build_get_apy_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let apy = get_return_u64(&result);

    // 10% APR -> ~10.52% APY with daily compounding.
    assert_eq!(apy, 105_156);
}

#[test]
fn test_get_apy_25_percent() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time,
        1_000_000_000,
        250_000,
        86400, // 25% APR
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    let ix = build_get_apy_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let apy = get_return_u64(&result);

    // 25% APR -> ~28.4% APY with daily compounding.
    assert_eq!(apy, 283_916);
}

#[test]
fn test_get_apy_multiple_vectors_uses_most_recent() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);

    // Vector 1: 3.65% APR
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Vector 2: 10% APR, starts later
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 100,
        1_000_000_000,
        100_000,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    advance_clock_by(&mut svm, 101);

    let ix = build_get_apy_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let apy = get_return_u64(&result);

    // Should use vector 2 (10% APR -> ~10.52% APY).
    assert_eq!(apy, 105_156);
}

// ---------------------------------------------------------------------------
// Additional TVL tests
// ---------------------------------------------------------------------------

#[test]
fn test_get_tvl_different_price() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time,
        2_000_000_000,
        0,
        86400, // price = 2.0
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    // Set token_out supply to 1000 tokens
    let mut mint_data = svm.get_account(&token_out).unwrap();
    mint_data.data[36..44].copy_from_slice(&1_000_000_000_000u64.to_le_bytes());
    svm.set_account(token_out, mint_data).unwrap();

    let ix = build_get_tvl_ix(&payer.pubkey(), &token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let tvl = get_return_u64(&result);

    // TVL = supply * price / 10^9 = 1000e9 * 2e9 / 1e9 = 2000e9
    assert_eq!(tvl, 2_000_000_000_000);
}

#[test]
fn test_get_tvl_after_time_advancement() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    // Set supply
    let mut mint_data = svm.get_account(&token_out).unwrap();
    mint_data.data[36..44].copy_from_slice(&1_000_000_000_000u64.to_le_bytes());
    svm.set_account(token_out, mint_data).unwrap();

    let ix = build_get_tvl_ix(&payer.pubkey(), &token_in, &token_out);
    let result1 = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let tvl1 = get_return_u64(&result1);

    // Advance 1 day
    advance_clock_by(&mut svm, 86400);

    let ix = build_get_tvl_ix(&payer.pubkey(), &token_in, &token_out);
    let result2 = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let tvl2 = get_return_u64(&result2);

    assert_eq!(tvl1, 1_000_100_000_000);
    assert_eq!(tvl2, 1_000_200_010_000);
}

// ---------------------------------------------------------------------------
// Additional NAV Adjustment tests
// ---------------------------------------------------------------------------

#[test]
fn test_get_nav_adjustment_negative() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);

    // Vector 1: base_price = 2.0
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time,
        2_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Vector 2: base_price = 1.0 (decrease)
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 100,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    advance_clock_by(&mut svm, 101);

    let ix = build_get_nav_adjustment_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let adjustment = get_return_i64(&result);

    // Adjustment = current_price - previous_price = 1.0 - 2.0 = -1.0
    assert!(
        adjustment < 0,
        "adjustment should be negative when price decreases"
    );
}

#[test]
fn test_get_nav_adjustment_time_progression() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    // Get adjustment at time 1
    let ix = build_get_nav_adjustment_ix(&token_in, &token_out);
    let result1 = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let adj1 = get_return_i64(&result1);

    // Advance within same interval - should be same
    advance_clock_by(&mut svm, 30_000);

    let ix = build_get_nav_adjustment_ix(&token_in, &token_out);
    let result2 = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let adj2 = get_return_i64(&result2);

    // Same vector, same interval → same adjustment
    assert_eq!(adj1, adj2, "adjustment should be same within same interval");
}

// ===========================================================================
// Token-2022 Tests
// ===========================================================================

#[test]
fn test_get_nav_token2022_offer() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint_2022(&mut svm, &payer, 9, &boss);
    let token_out = create_mint_2022(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    let ix = build_get_nav_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let nav = get_return_u64(&result);

    // Same as SPL Token test: price = 1_000_100_000
    assert_eq!(nav, 1_000_100_000);
}

#[test]
fn test_get_circulating_supply_token2022() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token2022_mint = create_mint_2022(&mut svm, &payer, 9, &boss);

    // Set as onyc_mint
    let ix = build_set_onyc_mint_ix(&boss, &token2022_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Set supply on the Token-2022 mint
    let mut mint_data = svm.get_account(&token2022_mint).unwrap();
    mint_data.data[36..44].copy_from_slice(&500_000_000_000u64.to_le_bytes());
    svm.set_account(token2022_mint, mint_data).unwrap();

    let ix = build_get_circulating_supply_ix_with_token_program(
        &payer.pubkey(),
        &token2022_mint,
        &TOKEN_2022_PROGRAM_ID,
    );
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let supply = get_return_u64(&result);

    // No vault, so circulating = total
    assert_eq!(supply, 500_000_000_000);
}

// ---------------------------------------------------------------------------
// Additional APY tests (matching TS coverage)
// ---------------------------------------------------------------------------

#[test]
fn test_get_apy_3_65_percent() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(
        36_500, // 3.65% APR
        1_000_000_000,
        86400,
    );

    let ix = build_get_apy_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let apy = get_return_u64(&result);

    // 3.65% APR → ~3.72% APY with daily compounding
    assert_eq!(apy, 37_172);
}

#[test]
fn test_get_apy_small_apr() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(
        100, // 0.01% APR
        1_000_000_000,
        86400,
    );

    let ix = build_get_apy_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let apy = get_return_u64(&result);

    // Very small APR ≈ same APY
    assert_eq!(apy, 100);
}

#[test]
fn test_get_apy_fails_all_vectors_future() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 100_000,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_get_apy_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "should fail when all vectors are in the future"
    );
}

// ---------------------------------------------------------------------------
// Additional NAV tests (matching TS coverage)
// ---------------------------------------------------------------------------

#[test]
fn test_get_nav_fails_nonexistent_offer() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);
    let wrong_mint = create_mint(&mut svm, &payer, 9, &boss);

    // Create an offer for token_in/token_out
    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Try with wrong token_in
    let ix = build_get_nav_ix(&wrong_mint, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "should fail with non-existent offer (wrong token_in)"
    );

    // Try with wrong token_out
    let ix = build_get_nav_ix(&token_in, &wrong_mint);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "should fail with non-existent offer (wrong token_out)"
    );
}

// ---------------------------------------------------------------------------
// Additional NAV Adjustment tests (matching TS coverage)
// ---------------------------------------------------------------------------

#[test]
fn test_get_nav_adjustment_multiple_transitions() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);

    // Vector 1: base_price = 1.0
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time,
        1_000_000_000,
        0,
        1800,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Advance and add vector 2: base_price = 1.2
    advance_clock_by(&mut svm, 1800);
    let new_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        new_time,
        1_200_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Advance and add vector 3: base_price = 1.1 (lower than second, higher than first)
    advance_clock_by(&mut svm, 1800);
    let new_time2 = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        new_time2,
        1_100_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Adjustment should compare current (vector 3) to previous (vector 2)
    let ix = build_get_nav_adjustment_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let adjustment = get_return_i64(&result);

    // Should be negative: 1.1 - 1.2 = -0.1
    assert!(
        adjustment < 0,
        "adjustment should be negative: {}",
        adjustment
    );
}

#[test]
fn test_get_nav_adjustment_zero_apr_different_base_price() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);

    // Vector 1: base_price = 1.0, 0% APR
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time,
        1_000_000_000,
        0,
        3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    advance_clock_by(&mut svm, 3600);

    // Vector 2: base_price = 2.5, 0% APR
    let new_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        new_time,
        2_500_000_000,
        0,
        3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_get_nav_adjustment_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let adjustment = get_return_i64(&result);

    // adjustment = 2.5 - 1.0 = 1.5
    assert_eq!(adjustment, 1_500_000_000);
}

#[test]
fn test_get_nav_adjustment_fails_nonexistent_offer() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);
    let wrong_mint = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_get_nav_adjustment_ix(&wrong_mint, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "should fail with non-existent offer");
}

#[test]
fn test_get_nav_adjustment_fails_all_vectors_future() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 100_000,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_get_nav_adjustment_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "should fail when all vectors are in the future"
    );
}

// ---------------------------------------------------------------------------
// Additional TVL tests (matching TS coverage)
// ---------------------------------------------------------------------------

#[test]
fn test_get_tvl_zero_apr() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(
        0,
        3_000_000_000,
        86400, // 0% APR, price = 3.0
    );

    // Set token_out supply
    let mut mint_data = svm.get_account(&token_out).unwrap();
    mint_data.data[36..44].copy_from_slice(&1_000_000_000_000u64.to_le_bytes()); // 1000 tokens
    svm.set_account(token_out, mint_data).unwrap();

    let ix = build_get_tvl_ix(&payer.pubkey(), &token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let tvl = get_return_u64(&result);

    // TVL = 1000e9 * 3e9 / 1e9 = 3000e9
    assert_eq!(tvl, 3_000_000_000_000);
}

#[test]
fn test_get_tvl_fails_nonexistent_offer() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);
    let wrong_mint = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_get_tvl_ix(&payer.pubkey(), &wrong_mint, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "should fail with non-existent offer (wrong token_in)"
    );

    let ix = build_get_tvl_ix(&payer.pubkey(), &token_in, &wrong_mint);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "should fail with non-existent offer (wrong token_out)"
    );
}

#[test]
fn test_get_tvl_fails_all_vectors_future() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 100_000,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_get_tvl_ix(&payer.pubkey(), &token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "should fail when all vectors are in the future"
    );
}

#[test]
fn test_get_tvl_wrong_token_out_mint() {
    let (mut svm, payer, token_in, _token_out) = setup_offer_with_vector(0, 1_000_000_000, 86400);
    let boss = payer.pubkey();

    let wrong_mint = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_get_tvl_ix(&payer.pubkey(), &token_in, &wrong_mint);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "should fail with wrong token_out_mint");
}

#[test]
fn test_get_tvl_multiple_vectors_uses_most_recent() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);

    // Vector 1: price = 1.0
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time,
        1_000_000_000,
        0,
        3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    advance_clock_by(&mut svm, 1800);

    // Vector 2: price = 5.0
    let new_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        new_time,
        5_000_000_000,
        0,
        1800,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Set supply
    let mut mint_data = svm.get_account(&token_out).unwrap();
    mint_data.data[36..44].copy_from_slice(&1_000_000_000_000u64.to_le_bytes());
    svm.set_account(token_out, mint_data).unwrap();

    let ix = build_get_tvl_ix(&payer.pubkey(), &token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let tvl = get_return_u64(&result);

    // Should use vector 2: TVL = 1000e9 * 5e9 / 1e9 = 5000e9
    assert_eq!(tvl, 5_000_000_000_000);
}

#[test]
fn test_get_tvl_token2022() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint_2022(&mut svm, &payer, 6, &boss);
    let token_out = create_mint_2022(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time,
        2_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    // Set supply
    let mut mint_data = svm.get_account(&token_out).unwrap();
    mint_data.data[36..44].copy_from_slice(&1_000_000_000_000u64.to_le_bytes());
    svm.set_account(token_out, mint_data).unwrap();

    let ix = build_get_tvl_ix_with_token_program(
        &payer.pubkey(),
        &token_in,
        &token_out,
        &TOKEN_2022_PROGRAM_ID,
    );
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let tvl = get_return_u64(&result);

    // TVL is based on the first stepped price for base price 2.0 and 3.65% APR.
    assert_eq!(tvl, 2_000_200_000_000);
}

// ---------------------------------------------------------------------------
// Additional tests (matching TS coverage)
// ---------------------------------------------------------------------------

#[test]
fn test_get_apy_fails_nonexistent_offer() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);
    let wrong_mint = create_mint(&mut svm, &payer, 9, &boss);

    // Create an offer for token_in/token_out
    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Try with wrong mints (no offer exists for wrong_mint/token_out)
    let ix = build_get_apy_ix(&wrong_mint, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "should fail with non-existent offer (wrong token_in)"
    );

    let ix = build_get_apy_ix(&token_in, &wrong_mint);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "should fail with non-existent offer (wrong token_out)"
    );
}

#[test]
fn test_get_apy_consistent_results() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(
        100_000, // 10% APR
        1_000_000_000,
        86400,
    );

    let ix = build_get_apy_ix(&token_in, &token_out);
    let result1 = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let apy1 = get_return_u64(&result1);

    advance_slot(&mut svm);

    let ix = build_get_apy_ix(&token_in, &token_out);
    let result2 = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let apy2 = get_return_u64(&result2);

    assert_eq!(apy1, apy2, "APY should be identical on consecutive calls");
}

#[test]
fn test_get_nav_consistent_results() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(
        36_500, // 3.65% APR
        1_000_000_000,
        86400,
    );

    let ix = build_get_nav_ix(&token_in, &token_out);
    let result1 = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let nav1 = get_return_u64(&result1);

    advance_slot(&mut svm);

    let ix = build_get_nav_ix(&token_in, &token_out);
    let result2 = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let nav2 = get_return_u64(&result2);

    assert_eq!(
        nav1, nav2,
        "NAV should be identical on consecutive calls at the same time"
    );
}

#[test]
fn test_get_nav_adjustment_zero_price_change() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);

    // Vector 1: base_price = 1.0, 0% APR
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Vector 2: same base_price = 1.0, 0% APR, starts later
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 100,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Advance to make vector 2 active
    advance_clock_by(&mut svm, 101);

    let ix = build_get_nav_adjustment_ix(&token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let adjustment = get_return_i64(&result);

    // Both vectors have the same price (1.0) and 0 APR, so adjustment = 0
    assert_eq!(
        adjustment, 0,
        "adjustment should be 0 when prices are identical"
    );
}

#[test]
fn test_get_nav_adjustment_consistent_results() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in,
        &token_out,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);

    // Vector 1: base_price = 1.0
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Vector 2: base_price = 1.5, starts later
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 100,
        1_500_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    advance_clock_by(&mut svm, 101);

    let ix = build_get_nav_adjustment_ix(&token_in, &token_out);
    let result1 = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let adj1 = get_return_i64(&result1);

    advance_slot(&mut svm);

    let ix = build_get_nav_adjustment_ix(&token_in, &token_out);
    let result2 = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let adj2 = get_return_i64(&result2);

    assert_eq!(
        adj1, adj2,
        "NAV adjustment should be identical on consecutive calls"
    );
}

#[test]
fn test_get_tvl_large_supply() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(
        0,
        1_000_000_000,
        86400, // 0% APR, price = 1.0
    );

    // Set token_out supply to a very large value: 1_000_000_000_000_000 (1 billion tokens with 6 decimals)
    let large_supply: u64 = 1_000_000_000_000_000;
    let mut mint_data = svm.get_account(&token_out).unwrap();
    mint_data.data[36..44].copy_from_slice(&large_supply.to_le_bytes());
    svm.set_account(token_out, mint_data).unwrap();

    let ix = build_get_tvl_ix(&payer.pubkey(), &token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let tvl = get_return_u64(&result);

    // TVL = supply * price / 10^9 = 1_000_000_000_000_000 * 1_000_000_000 / 1_000_000_000
    //     = 1_000_000_000_000_000
    assert_eq!(
        tvl, large_supply,
        "TVL should handle large supply correctly"
    );
}

#[test]
fn test_get_tvl_consistent_results() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(0, 1_000_000_000, 86400);

    // Set token_out supply
    let mut mint_data = svm.get_account(&token_out).unwrap();
    mint_data.data[36..44].copy_from_slice(&1_000_000_000_000u64.to_le_bytes());
    svm.set_account(token_out, mint_data).unwrap();

    let ix = build_get_tvl_ix(&payer.pubkey(), &token_in, &token_out);
    let result1 = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let tvl1 = get_return_u64(&result1);

    advance_slot(&mut svm);

    let ix = build_get_tvl_ix(&payer.pubkey(), &token_in, &token_out);
    let result2 = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let tvl2 = get_return_u64(&result2);

    assert_eq!(
        tvl1, tvl2,
        "TVL should be identical on consecutive calls (read-only)"
    );
}

#[test]
fn test_get_circulating_supply_zero_vault_balance() {
    let (mut svm, payer, onyc_mint) = setup_initialized();

    // Set supply on the onyc mint
    let total_supply: u64 = 500_000_000_000; // 500 tokens
    let mut mint_data = svm.get_account(&onyc_mint).unwrap();
    mint_data.data[36..44].copy_from_slice(&total_supply.to_le_bytes());
    svm.set_account(onyc_mint, mint_data).unwrap();

    // Create vault ATA with 0 balance so the account exists but has no tokens
    let (vault_authority_pda, _) = find_offer_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &vault_authority_pda, 0);

    let ix = build_get_circulating_supply_ix(&payer.pubkey(), &onyc_mint);
    let result = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let supply = get_return_u64(&result);

    // Vault has 0 balance, so circulating = total supply
    assert_eq!(
        supply, total_supply,
        "circulating supply should equal total supply when vault balance is 0"
    );
}

#[test]
fn test_get_circulating_supply_consistent_results() {
    let (mut svm, payer, onyc_mint) = setup_initialized();

    // Set supply on the onyc mint
    let mut mint_data = svm.get_account(&onyc_mint).unwrap();
    mint_data.data[36..44].copy_from_slice(&500_000_000_000u64.to_le_bytes());
    svm.set_account(onyc_mint, mint_data).unwrap();

    let ix = build_get_circulating_supply_ix(&payer.pubkey(), &onyc_mint);
    let result1 = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let supply1 = get_return_u64(&result1);

    advance_slot(&mut svm);

    let ix = build_get_circulating_supply_ix(&payer.pubkey(), &onyc_mint);
    let result2 = send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let supply2 = get_return_u64(&result2);

    assert_eq!(
        supply1, supply2,
        "circulating supply should be identical on consecutive calls"
    );
}
