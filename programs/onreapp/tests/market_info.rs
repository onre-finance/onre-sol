mod common;

use anchor_lang::AccountDeserialize;
use common::*;
use onreapp::state::{CirculatingSupplyExcludedAccounts, CirculatingSupplyExcludedBalance};
use solana_sdk::account::Account;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::rent::Rent;
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
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = onyc_mint;

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
    let (offer_pda, _) = find_offer_pda(&token_in, &token_out);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

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

// Metrics are read from the MarketStats PDA after a refresh transaction.

#[test]
fn test_removed_market_getters_are_rejected() {
    let (mut svm, payer, _) = setup_initialized();
    for name in [
        "get_nav",
        "get_apy",
        "get_nav_adjustment",
        "get_tvl",
        "get_tvl_v2",
        "get_circulating_supply",
        "get_circulating_supply_v2",
    ] {
        let ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![],
            data: ix_discriminator(name).to_vec(),
        };
        let failure = send_tx(&mut svm, &[ix], &[&payer]).unwrap_err();
        assert!(
            failure
                .meta
                .logs
                .iter()
                .any(|line| line.contains("InstructionFallbackNotFound")),
            "{name} must be removed"
        );
    }
}

#[test]
fn test_market_stats_snapshot_stays_cached_until_refreshed_even_while_killed() {
    let (mut svm, payer, token_in, onyc_mint) =
        setup_onyc_offer_with_supply(36_500, 1_000_000_000, 86_400, 5_000_000_000, 0);
    let refresh = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &onyc_mint);
    send_tx(&mut svm, std::slice::from_ref(&refresh), &[&payer]).unwrap();
    let (pda, _) = find_market_stats_pda();
    let initial = svm.get_account(&pda).unwrap().data;
    let initial_stats = read_market_stats(&svm);

    advance_clock_by(&mut svm, 86_400);
    set_mint_supply(&mut svm, &onyc_mint, 6_000_000_000);
    assert_eq!(svm.get_account(&pda).unwrap().data, initial);

    let kill = build_set_kill_switch_ix(&payer.pubkey(), true);
    send_tx(&mut svm, &[kill, refresh], &[&payer]).unwrap();
    let updated = read_market_stats(&svm);
    assert_eq!(updated.nav, 1_000_200_010);
    assert_eq!(updated.circulating_supply, 6_000_000_000);
    assert_eq!(updated.tvl, 6_001_200_060);
    assert!(updated.last_updated_at > initial_stats.last_updated_at);
    assert!(updated.last_updated_slot > initial_stats.last_updated_slot);
}

#[test]
fn test_refresh_market_stats_rejects_an_offer_other_than_main_offer() {
    let (mut svm, payer, token_in, onyc_mint) =
        setup_onyc_offer_with_supply(0, 1_000_000_000, 86_400, 0, 0);
    let other_token_in = create_mint(&mut svm, &payer, 6, &payer.pubkey());
    let make = build_make_offer_ix(
        &payer.pubkey(),
        &other_token_in,
        &onyc_mint,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[make], &[&payer]).unwrap();
    let mut refresh = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &onyc_mint);
    refresh.accounts[0] =
        AccountMeta::new_readonly(find_offer_pda(&other_token_in, &onyc_mint).0, false);
    refresh.accounts[1] = AccountMeta::new_readonly(other_token_in, false);
    let failure = send_tx(&mut svm, &[refresh], &[&payer]).unwrap_err();
    assert!(failure
        .meta
        .logs
        .iter()
        .any(|line| line.contains("InvalidMainOffer")));
}

#[test]
fn test_refresh_market_stats_nav_success() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(
        36_500,        // 3.65% APR
        1_000_000_000, // base_price = 1.0
        86400,         // 1 day
    );

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let nav = read_market_stats(&svm).nav;

    // Step pricing: step=0, interval=(0+1)*86400, so price grows slightly from base
    // price = 1e9 * (1 + 36500 * 86400 / (1e6 * 31536000)) = 1_000_100_000
    assert_eq!(nav, 1_000_100_000);
}

#[test]
fn test_refresh_market_stats_nav_price_growth() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(
        36_500, // 3.65% APR
        1_000_000_000,
        86400, // 1 day
    );

    // Advance 1 day so price should have grown
    advance_clock_by(&mut svm, 86400);

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let nav = read_market_stats(&svm).nav;

    // After 1 day with 3.65% APR: price = 1.0 * (1 + 0.0365 * 86400 / 31536000) = ~1.0001
    // Step price: elapsed=86400, step = 86400/86400 = 1, interval = 2 * 86400 = 172800
    // price = 1_000_000_000 * (1 + 36500 * 172800 / (1_000_000 * 31_536_000))
    // = 1_000_000_000 * (1 + 6307200000 / 31536000000000)
    // = 1_000_000_000 * (1 + 0.0002)
    // = 1_000_200_010 after daily compounding
    assert_eq!(nav, 1_000_200_010);
}

#[test]
fn test_refresh_market_stats_nav_zero_apr() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(
        0, // 0% APR
        1_000_000_000,
        86400,
    );

    advance_clock_by(&mut svm, 86400 * 30);

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let nav = read_market_stats(&svm).nav;

    // With 0% APR, price stays the same regardless of time
    assert_eq!(nav, 1_000_000_000);
}

#[test]
fn test_refresh_market_stats_nav_fails_no_active_vector() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = onyc_mint;

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
    let (offer_pda, _) = find_offer_pda(&token_in, &token_out);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // No vectors added
    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result
        .unwrap_err()
        .meta
        .logs
        .iter()
        .any(|line| line.contains("NoActiveVector")));
}

#[test]
fn test_refresh_market_stats_nav_fails_all_vectors_future() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = onyc_mint;

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
    let (offer_pda, _) = find_offer_pda(&token_in, &token_out);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

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

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result
            .unwrap_err()
            .meta
            .logs
            .iter()
            .any(|line| line.contains("NoActiveVector")),
        "should fail when all vectors are in the future"
    );
}

#[test]
fn test_refresh_market_stats_apy_success() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(
        100_000, // 10% APR
        1_000_000_000,
        86400,
    );

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let apy = read_market_stats(&svm).apy;

    // 10% APR -> ~10.52% APY with daily compounding.
    assert_eq!(apy, 105_156);
}

#[test]
fn test_refresh_market_stats_apy_zero_apr() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(0, 1_000_000_000, 86400);

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let apy = read_market_stats(&svm).apy;

    assert_eq!(apy, 0, "0% APR should give 0% APY");
}

#[test]
fn test_refresh_market_stats_permissionless_creates_and_updates_pda() {
    let (mut svm, _payer, token_in, onyc_mint) =
        setup_onyc_offer_with_supply(36_500, 1_000_000_000, 86_400, 5_000_000_000, 2_000_000_000);
    let caller = Keypair::new();
    svm.airdrop(&caller.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_refresh_market_stats_ix(&caller.pubkey(), &token_in, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&caller]).unwrap();

    let market_stats = read_market_stats(&svm);
    assert_eq!(market_stats.bump, find_market_stats_pda().1);
    assert_eq!(market_stats.apy, 37_172);
    assert_eq!(market_stats.nav, 1_000_100_000);
    assert_eq!(market_stats.nav_adjustment, 1_000_100_000);
    assert_eq!(market_stats.circulating_supply, 5_000_000_000);
    assert_eq!(market_stats.tvl, 5_000_500_000);
    assert_eq!(market_stats.last_updated_at, 1_704_067_201);
    assert_eq!(market_stats.last_updated_slot, 3);
}

#[test]
fn test_refresh_market_stats_initializes_prefunded_pda() {
    let (mut svm, payer, token_in, onyc_mint) =
        setup_onyc_offer_with_supply(36_500, 1_000_000_000, 86_400, 5_000_000_000, 2_000_000_000);
    let (market_stats_pda, _) = find_market_stats_pda();
    let prefund_lamports = Rent::default().minimum_balance(0);
    svm.airdrop(&market_stats_pda, prefund_lamports).unwrap();

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let market_stats_account = svm.get_account(&market_stats_pda).unwrap();
    assert_eq!(market_stats_account.owner, PROGRAM_ID);
    assert!(market_stats_account.lamports > prefund_lamports);
    assert_eq!(read_market_stats(&svm).bump, find_market_stats_pda().1);
}

#[test]
fn test_refresh_market_stats_initializes_fully_prefunded_pda() {
    let (mut svm, payer, token_in, onyc_mint) =
        setup_onyc_offer_with_supply(36_500, 1_000_000_000, 86_400, 5_000_000_000, 2_000_000_000);
    let (market_stats_pda, _) = find_market_stats_pda();
    svm.airdrop(&market_stats_pda, INITIAL_LAMPORTS).unwrap();

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let market_stats_account = svm.get_account(&market_stats_pda).unwrap();
    assert_eq!(market_stats_account.owner, PROGRAM_ID);
    assert_eq!(market_stats_account.lamports, INITIAL_LAMPORTS);
    assert_eq!(read_market_stats(&svm).bump, find_market_stats_pda().1);
}

#[test]
fn test_refresh_market_stats_rejects_invalid_excluded_balance_pda() {
    let (mut svm, payer, token_in, onyc_mint) =
        setup_onyc_offer_with_supply(36_500, 1_000_000_000, 86_400, 5_000_000_000, 2_000_000_000);
    let boss = payer.pubkey();

    let mut ix = build_refresh_market_stats_ix(&boss, &token_in, &onyc_mint);
    ix.accounts[4] = AccountMeta::new_readonly(Pubkey::new_unique(), false);

    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "market stats refresh should require the canonical excluded-balance PDA"
    );
}

#[test]
fn test_refresh_market_stats_succeeds_without_recent_purchases() {
    let (mut svm, payer, token_in, onyc_mint) =
        setup_onyc_offer_with_supply(0, 1_000_000_000, 86_400, 7_000_000_000, 1_500_000_000);

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &onyc_mint);
    send_tx(&mut svm, std::slice::from_ref(&ix), &[&payer]).unwrap();
    let initial = read_market_stats(&svm);

    advance_clock_by(&mut svm, 86_400);

    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let refreshed = read_market_stats(&svm);

    assert_eq!(initial.circulating_supply, 7_000_000_000);
    assert_eq!(initial.nav, 1_000_000_000);
    assert_eq!(refreshed.circulating_supply, initial.circulating_supply);
    assert_eq!(refreshed.nav, initial.nav);
    assert_eq!(initial.last_updated_at, 1_704_067_201);
    assert_eq!(initial.last_updated_slot, 3);
    assert_eq!(refreshed.last_updated_at, 1_704_153_601);
    assert_eq!(refreshed.last_updated_slot, 4);
}

#[test]
fn test_refresh_market_stats_nav_adjustment_first_vector() {
    let (mut svm, payer, token_in, token_out) =
        setup_offer_with_vector(36_500, 1_000_000_000, 86400);

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let adjustment = read_market_stats(&svm).nav_adjustment;

    // First vector: adjustment = current_price (no previous)
    // Step pricing: first interval gives slight growth from base_price
    assert_eq!(adjustment, 1_000_100_000);
}

#[test]
fn test_refresh_market_stats_nav_adjustment_positive() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = onyc_mint;

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
    let (offer_pda, _) = find_offer_pda(&token_in, &token_out);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

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

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let adjustment = read_market_stats(&svm).nav_adjustment;

    // Adjustment = 1.1 - 1.0 = 0.1 = 100_000_000
    assert_eq!(adjustment, 100_000_000);
}

#[test]
fn test_refresh_market_stats_tvl_success() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(0, 1_000_000_000, 86400);

    // Mint some token_out supply
    let mut mint_data = svm.get_account(&token_out).unwrap();
    mint_data.data[36..44].copy_from_slice(&1_000_000_000_000u64.to_le_bytes()); // 1000 tokens
    svm.set_account(token_out, mint_data).unwrap();

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let tvl = read_market_stats(&svm).tvl;

    // TVL = supply * price / 10^9 = 1000e9 * 1e9 / 1e9 = 1000e9
    assert_eq!(tvl, 1_000_000_000_000);
}

#[test]
fn test_set_circulating_supply_excluded_accounts_boss_only_and_stores_owners() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let owner_a = Pubkey::new_unique();
    let owner_b = Pubkey::new_unique();
    let mut owners = [Pubkey::default(); 20];
    owners[0] = owner_a;
    owners[1] = owner_b;

    let ix = build_set_circulating_supply_excluded_accounts_ix(&boss, &[owner_a, owner_b]);
    assert_eq!(ix.data.len(), 12 + 2 * 32);
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
fn test_excluded_owners_vec_limits_and_replacement() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let owners: Vec<Pubkey> = (0..20).map(|_| Pubkey::new_unique()).collect();
    let ix = build_set_circulating_supply_excluded_accounts_ix(&boss, &owners);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let (pda, _) = find_circulating_supply_excluded_accounts_pda();
    let before = svm.get_account(&pda).unwrap();
    assert_eq!(
        read_circulating_supply_excluded_accounts(&svm)
            .owners
            .as_slice(),
        owners.as_slice()
    );

    let mut too_many = owners.clone();
    too_many.push(Pubkey::new_unique());
    let ix = build_set_circulating_supply_excluded_accounts_ix(&boss, &too_many);
    let failure = send_tx(&mut svm, &[ix], &[&payer]).unwrap_err();
    assert!(failure
        .meta
        .logs
        .iter()
        .any(|line| line.contains("InvalidCirculatingSupplyExcludedAccounts")));
    assert_eq!(svm.get_account(&pda).unwrap().data, before.data);

    let replacement = Pubkey::new_unique();
    let ix = build_set_circulating_supply_excluded_accounts_ix(&boss, &[replacement]);
    assert_eq!(ix.data.len(), 44);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let stored = read_circulating_supply_excluded_accounts(&svm);
    assert_eq!(stored.owners[0], replacement);
    assert!(stored.owners[1..]
        .iter()
        .all(|owner| *owner == Pubkey::default()));
    assert_eq!(svm.get_account(&pda).unwrap().data.len(), before.data.len());

    let ix = build_set_circulating_supply_excluded_accounts_ix(&boss, &[]);
    assert_eq!(ix.data.len(), 12);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    assert_eq!(
        read_circulating_supply_excluded_accounts(&svm).owners,
        [Pubkey::default(); 20]
    );
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
fn test_update_circulating_supply_excluded_balance_rejects_swapped_ata_order() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let owner_a = Pubkey::new_unique();
    let owner_b = Pubkey::new_unique();
    let mut owners = [Pubkey::default(); 20];
    owners[0] = owner_a;
    owners[1] = owner_b;

    let set_ix = build_set_circulating_supply_excluded_accounts_ix(&boss, &owners);
    send_tx(&mut svm, &[set_ix], &[&payer]).unwrap();

    let ata_a = create_token_account(&mut svm, &onyc_mint, &owner_a, 10);
    let ata_b = create_token_account(&mut svm, &onyc_mint, &owner_b, 20);
    let update_ix = build_update_circulating_supply_excluded_balance_ix(
        &boss,
        &onyc_mint,
        &[ata_a, ata_b],
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[update_ix], &[&payer]).unwrap();
    assert_eq!(read_circulating_supply_excluded_balance(&svm).amount, 30);

    let swapped_ix = build_update_circulating_supply_excluded_balance_ix(
        &boss,
        &onyc_mint,
        &[ata_b, ata_a],
        &TOKEN_PROGRAM_ID,
    );
    assert!(
        send_tx(&mut svm, &[swapped_ix], &[&payer]).is_err(),
        "swapped configured ATAs should fail"
    );
    assert_eq!(read_circulating_supply_excluded_balance(&svm).amount, 30);
}

#[test]
fn test_update_circulating_supply_excluded_balance_rejects_malformed_ata_data() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let owner = Pubkey::new_unique();
    let mut owners = [Pubkey::default(); 20];
    owners[0] = owner;

    let set_ix = build_set_circulating_supply_excluded_accounts_ix(&boss, &owners);
    send_tx(&mut svm, &[set_ix], &[&payer]).unwrap();

    let ata = create_token_account(&mut svm, &onyc_mint, &owner, 30);
    let update_ix = build_update_circulating_supply_excluded_balance_ix(
        &boss,
        &onyc_mint,
        &[ata],
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[update_ix], &[&payer]).unwrap();
    assert_eq!(read_circulating_supply_excluded_balance(&svm).amount, 30);

    let wrong_mint = create_mint(&mut svm, &payer, 9, &boss);
    let mut account = svm.get_account(&ata).unwrap();
    account.data[0..32].copy_from_slice(wrong_mint.as_ref());
    svm.set_account(ata, account).unwrap();
    let update_ix = build_update_circulating_supply_excluded_balance_ix(
        &boss,
        &onyc_mint,
        &[ata],
        &TOKEN_PROGRAM_ID,
    );
    assert!(
        send_tx(&mut svm, &[update_ix], &[&payer]).is_err(),
        "ATA with wrong mint should fail"
    );
    assert_eq!(read_circulating_supply_excluded_balance(&svm).amount, 30);

    let mut account = svm.get_account(&ata).unwrap();
    account.data[0..32].copy_from_slice(onyc_mint.as_ref());
    account.data[32..64].copy_from_slice(Pubkey::new_unique().as_ref());
    svm.set_account(ata, account).unwrap();
    let update_ix = build_update_circulating_supply_excluded_balance_ix(
        &boss,
        &onyc_mint,
        &[ata],
        &TOKEN_PROGRAM_ID,
    );
    assert!(
        send_tx(&mut svm, &[update_ix], &[&payer]).is_err(),
        "ATA with wrong owner should fail"
    );
    assert_eq!(read_circulating_supply_excluded_balance(&svm).amount, 30);

    svm.set_account(
        ata,
        Account {
            executable: false,
            data: Vec::new(),
            lamports: INITIAL_LAMPORTS,
            owner: SYSTEM_PROGRAM_ID,
            rent_epoch: 0,
        },
    )
    .unwrap();
    let update_ix = build_update_circulating_supply_excluded_balance_ix(
        &boss,
        &onyc_mint,
        &[ata],
        &TOKEN_PROGRAM_ID,
    );
    assert!(
        send_tx(&mut svm, &[update_ix], &[&payer]).is_err(),
        "uninitialized ATA should fail"
    );
    assert_eq!(read_circulating_supply_excluded_balance(&svm).amount, 30);
}

#[test]
fn test_market_info_uses_cached_excluded_balance() {
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

    let refresh_ix = build_refresh_market_stats_ix(&boss, &token_in, &onyc_mint);
    send_tx(&mut svm, &[refresh_ix], &[&payer]).unwrap();
    let market_stats = read_market_stats(&svm);
    assert_eq!(market_stats.circulating_supply, 700_000_000_000);
    assert_eq!(market_stats.tvl, 700_000_000_000);

    // A market refresh alone still uses the old excluded-balance snapshot.
    advance_slot(&mut svm);
    let mut ata = svm.get_account(&excluded_ata).unwrap();
    ata.data[64..72].copy_from_slice(&400_000_000_000u64.to_le_bytes());
    svm.set_account(excluded_ata, ata).unwrap();
    let refresh_ix = build_refresh_market_stats_ix(&boss, &token_in, &onyc_mint);
    send_tx(&mut svm, std::slice::from_ref(&refresh_ix), &[&payer]).unwrap();
    assert_eq!(read_market_stats(&svm).circulating_supply, 700_000_000_000);

    // Update exclusions before the market snapshot, atomically in one transaction.
    advance_slot(&mut svm);
    let update_ix = build_update_circulating_supply_excluded_balance_ix(
        &boss,
        &onyc_mint,
        &[excluded_ata],
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[update_ix, refresh_ix], &[&payer]).unwrap();
    let updated = read_market_stats(&svm);
    assert_eq!(updated.circulating_supply, 600_000_000_000);
    assert_eq!(updated.tvl, 600_000_000_000);
}

#[test]
fn test_refresh_market_stats_nav_multiple_vectors_uses_most_recent() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = onyc_mint;

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
    let (offer_pda, _) = find_offer_pda(&token_in, &token_out);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

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

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let nav = read_market_stats(&svm).nav;

    // Should use vector 2 (base_price=2.0, APR=7.3%)
    // step=0, interval=86400, price = 2.0 * (1 + 73000 * 86400 / (1e6 * 31536000))
    // = 2.0 * (1 + 0.0002) = 2_000_400_000
    assert_eq!(nav, 2_000_400_000);
}

#[test]
fn test_refresh_market_stats_apy_10_percent() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = onyc_mint;

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
    let (offer_pda, _) = find_offer_pda(&token_in, &token_out);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

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

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let apy = read_market_stats(&svm).apy;

    // 10% APR -> ~10.52% APY with daily compounding.
    assert_eq!(apy, 105_156);
}

#[test]
fn test_refresh_market_stats_apy_25_percent() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = onyc_mint;

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
    let (offer_pda, _) = find_offer_pda(&token_in, &token_out);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

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

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let apy = read_market_stats(&svm).apy;

    // 25% APR -> ~28.4% APY with daily compounding.
    assert_eq!(apy, 283_916);
}

#[test]
fn test_refresh_market_stats_apy_multiple_vectors_uses_most_recent() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = onyc_mint;

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
    let (offer_pda, _) = find_offer_pda(&token_in, &token_out);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

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

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let apy = read_market_stats(&svm).apy;

    // Should use vector 2 (10% APR -> ~10.52% APY).
    assert_eq!(apy, 105_156);
}

#[test]
fn test_refresh_market_stats_tvl_different_price() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = onyc_mint;

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
    let (offer_pda, _) = find_offer_pda(&token_in, &token_out);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

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

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let tvl = read_market_stats(&svm).tvl;

    // TVL = supply * price / 10^9 = 1000e9 * 2e9 / 1e9 = 2000e9
    assert_eq!(tvl, 2_000_000_000_000);
}

#[test]
fn test_refresh_market_stats_tvl_after_time_advancement() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = onyc_mint;

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
    let (offer_pda, _) = find_offer_pda(&token_in, &token_out);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

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

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let tvl1 = read_market_stats(&svm).tvl;

    // Advance 1 day
    advance_clock_by(&mut svm, 86400);

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let tvl2 = read_market_stats(&svm).tvl;

    assert_eq!(tvl1, 1_000_100_000_000);
    assert_eq!(tvl2, 1_000_200_010_000);
}

#[test]
fn test_refresh_market_stats_nav_adjustment_negative() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = onyc_mint;

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
    let (offer_pda, _) = find_offer_pda(&token_in, &token_out);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

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

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let adjustment = read_market_stats(&svm).nav_adjustment;

    // Adjustment = current_price - previous_price = 1.0 - 2.0 = -1.0
    assert!(
        adjustment < 0,
        "adjustment should be negative when price decreases"
    );
}

#[test]
fn test_refresh_market_stats_nav_adjustment_time_progression() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = onyc_mint;

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
    let (offer_pda, _) = find_offer_pda(&token_in, &token_out);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

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
    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let adj1 = read_market_stats(&svm).nav_adjustment;

    // Advance within same interval - should be same
    advance_clock_by(&mut svm, 30_000);

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let adj2 = read_market_stats(&svm).nav_adjustment;

    // Same vector, same interval → same adjustment
    assert_eq!(adj1, adj2, "adjustment should be same within same interval");
}

#[test]
fn test_refresh_market_stats_apy_3_65_percent() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(
        36_500, // 3.65% APR
        1_000_000_000,
        86400,
    );

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let apy = read_market_stats(&svm).apy;

    // 3.65% APR → ~3.72% APY with daily compounding
    assert_eq!(apy, 37_172);
}

#[test]
fn test_refresh_market_stats_apy_small_apr() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(
        100, // 0.01% APR
        1_000_000_000,
        86400,
    );

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let apy = read_market_stats(&svm).apy;

    // Very small APR ≈ same APY
    assert_eq!(apy, 100);
}

#[test]
fn test_refresh_market_stats_nav_adjustment_multiple_transitions() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = onyc_mint;

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
    let (offer_pda, _) = find_offer_pda(&token_in, &token_out);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

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
    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let adjustment = read_market_stats(&svm).nav_adjustment;

    // Should be negative: 1.1 - 1.2 = -0.1
    assert!(
        adjustment < 0,
        "adjustment should be negative: {}",
        adjustment
    );
}

#[test]
fn test_refresh_market_stats_nav_adjustment_zero_apr_different_base_price() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = onyc_mint;

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
    let (offer_pda, _) = find_offer_pda(&token_in, &token_out);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

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

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let adjustment = read_market_stats(&svm).nav_adjustment;

    // adjustment = 2.5 - 1.0 = 1.5
    assert_eq!(adjustment, 1_500_000_000);
}

#[test]
fn test_refresh_market_stats_tvl_zero_apr() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(
        0,
        3_000_000_000,
        86400, // 0% APR, price = 3.0
    );

    // Set token_out supply
    let mut mint_data = svm.get_account(&token_out).unwrap();
    mint_data.data[36..44].copy_from_slice(&1_000_000_000_000u64.to_le_bytes()); // 1000 tokens
    svm.set_account(token_out, mint_data).unwrap();

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let tvl = read_market_stats(&svm).tvl;

    // TVL = 1000e9 * 3e9 / 1e9 = 3000e9
    assert_eq!(tvl, 3_000_000_000_000);
}

#[test]
fn test_refresh_market_stats_tvl_multiple_vectors_uses_most_recent() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = onyc_mint;

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
    let (offer_pda, _) = find_offer_pda(&token_in, &token_out);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

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

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let tvl = read_market_stats(&svm).tvl;

    // Should use vector 2: TVL = 1000e9 * 5e9 / 1e9 = 5000e9
    assert_eq!(tvl, 5_000_000_000_000);
}

#[test]
fn test_refresh_market_stats_apy_consistent_results() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(
        100_000, // 10% APR
        1_000_000_000,
        86400,
    );

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let apy1 = read_market_stats(&svm).apy;

    advance_slot(&mut svm);

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let apy2 = read_market_stats(&svm).apy;

    assert_eq!(apy1, apy2, "APY should be identical on consecutive calls");
}

#[test]
fn test_refresh_market_stats_nav_consistent_results() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(
        36_500, // 3.65% APR
        1_000_000_000,
        86400,
    );

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let nav1 = read_market_stats(&svm).nav;

    advance_slot(&mut svm);

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let nav2 = read_market_stats(&svm).nav;

    assert_eq!(
        nav1, nav2,
        "NAV should be identical on consecutive calls at the same time"
    );
}

#[test]
fn test_refresh_market_stats_nav_adjustment_zero_price_change() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = onyc_mint;

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
    let (offer_pda, _) = find_offer_pda(&token_in, &token_out);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

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

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let adjustment = read_market_stats(&svm).nav_adjustment;

    // Both vectors have the same price (1.0) and 0 APR, so adjustment = 0
    assert_eq!(
        adjustment, 0,
        "adjustment should be 0 when prices are identical"
    );
}

#[test]
fn test_refresh_market_stats_nav_adjustment_consistent_results() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = onyc_mint;

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
    let (offer_pda, _) = find_offer_pda(&token_in, &token_out);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

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

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let adj1 = read_market_stats(&svm).nav_adjustment;

    advance_slot(&mut svm);

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let adj2 = read_market_stats(&svm).nav_adjustment;

    assert_eq!(
        adj1, adj2,
        "NAV adjustment should be identical on consecutive calls"
    );
}

#[test]
fn test_refresh_market_stats_tvl_large_supply() {
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

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let tvl = read_market_stats(&svm).tvl;

    // TVL = supply * price / 10^9 = 1_000_000_000_000_000 * 1_000_000_000 / 1_000_000_000
    //     = 1_000_000_000_000_000
    assert_eq!(
        tvl, large_supply,
        "TVL should handle large supply correctly"
    );
}

#[test]
fn test_refresh_market_stats_tvl_consistent_results() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_vector(0, 1_000_000_000, 86400);

    // Set token_out supply
    let mut mint_data = svm.get_account(&token_out).unwrap();
    mint_data.data[36..44].copy_from_slice(&1_000_000_000_000u64.to_le_bytes());
    svm.set_account(token_out, mint_data).unwrap();

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let tvl1 = read_market_stats(&svm).tvl;

    advance_slot(&mut svm);

    let ix = build_refresh_market_stats_ix(&payer.pubkey(), &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let tvl2 = read_market_stats(&svm).tvl;

    assert_eq!(
        tvl1, tvl2,
        "TVL should be identical on consecutive calls (read-only)"
    );
}
