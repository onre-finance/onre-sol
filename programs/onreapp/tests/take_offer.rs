mod common;

use common::*;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

/// Standard take-offer test setup:
///   - Initialized state
///   - USDC (token_in, 6 decimals) and ONyc (token_out, 9 decimals)
///   - Offer created with 0% fee
///   - Vault funded with 10,000 token_out (10_000e9)
///   - User funded with 10,000 token_in (10_000e6)
///   - Boss token_in account created
struct TakeOfferCtx {
    svm: litesvm::LiteSVM,
    payer: Keypair,
    usdc_mint: Pubkey,
    onyc_mint: Pubkey,
    user: Keypair,
}

fn setup_take_offer() -> TakeOfferCtx {
    setup_take_offer_with_fee(0)
}

fn setup_take_offer_with_fee(fee_bps: u16) -> TakeOfferCtx {
    let (mut svm, payer, _original_onyc) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);
    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, fee_bps, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Create vault accounts (pre-funded)
    let (vault_authority, _) = find_offer_vault_authority_pda();
    create_token_account(&mut svm, &usdc_mint, &vault_authority, 0);
    create_token_account(&mut svm, &onyc_mint, &vault_authority, 10_000_000_000_000); // 10,000 ONyc

    // Create user
    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 10_000_000_000); // 10,000 USDC

    // Boss token_in account
    create_token_account(&mut svm, &usdc_mint, &boss, 0);

    TakeOfferCtx {
        svm,
        payer,
        usdc_mint,
        onyc_mint,
        user,
    }
}

// ===========================================================================
// Price Calculation Tests
// ===========================================================================

#[test]
fn test_price_first_interval() {
    let mut ctx = setup_take_offer();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    // base_price = 1.0 (1e9), APR = 3.65% (36500), duration = 1 day
    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    // Price in first interval: 1.0 * (1 + 0.0365 * 86400/31536000) = 1.0001
    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_100, // 1.0001 USDC
        None,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();

    let user_onyc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.onyc_mint),
    );
    assert_eq!(user_onyc, 1_000_000_000); // 1 ONyc
}

#[test]
fn test_price_with_fee() {
    let mut ctx = setup_take_offer_with_fee(100); // 1% fee
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_100,
        None,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();

    let user_onyc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.onyc_mint),
    );
    // After 1% fee: net = 1_000_100 - ceil(1_000_100 * 100 / 10000) = 1_000_100 - 10_001 = 990_099
    // token_out = 990_099 * 1e9 / (1.0001 * 1e6) = 990_099_000 / 1_000_100 ≈ 990_000
    assert_eq!(user_onyc, 990_000_000); // 0.99 ONyc
}

#[test]
fn test_ceiling_fee_small_amount() {
    let mut ctx = setup_take_offer_with_fee(50); // 0.5% fee
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
        86400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    // 199 * 50 = 9950, floor = 0, ceiling = 1
    let boss_usdc_before = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&boss, &ctx.usdc_mint),
    );

    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        199,
        None,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();

    let boss_usdc_after = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&boss, &ctx.usdc_mint),
    );
    assert_eq!(boss_usdc_after - boss_usdc_before, 199);

    let user_onyc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.onyc_mint),
    );
    // fee = ceil(199*50/10000) = 1, net = 198, token_out = 198 * 1e9 / 1e6 = 198_000
    assert_eq!(user_onyc, 198_000);
}

#[test]
fn test_price_same_interval() {
    let mut ctx = setup_take_offer();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    // First trade
    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_100,
        None,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();

    // Advance within same interval
    advance_clock_by(&mut ctx.svm, 30_000);

    // Second user
    let user2 = Keypair::new();
    ctx.svm
        .airdrop(&user2.pubkey(), 10 * INITIAL_LAMPORTS)
        .unwrap();
    create_token_account(
        &mut ctx.svm,
        &ctx.usdc_mint,
        &user2.pubkey(),
        10_000_000_000,
    );

    let ix = build_take_offer_ix(
        &user2.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_100,
        None,
    );
    send_tx(&mut ctx.svm, &[ix], &[&user2]).unwrap();

    let user1_onyc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.onyc_mint),
    );
    let user2_onyc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&user2.pubkey(), &ctx.onyc_mint),
    );
    assert_eq!(user1_onyc, 1_000_000_000);
    assert_eq!(user2_onyc, 1_000_000_000);
}

#[test]
fn test_price_second_interval() {
    let mut ctx = setup_take_offer();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    // Advance to second interval
    advance_clock_by(&mut ctx.svm, 86_400);

    // Price: 1.0 * (1 + 0.0365 * 2*86400/31536000) = 1.0002
    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_200,
        None,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();

    let user_onyc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.onyc_mint),
    );
    assert_eq!(user_onyc, 1_000_000_000);
}

// ===========================================================================
// Multiple Vectors
// ===========================================================================

#[test]
fn test_use_most_recent_active_vector() {
    let mut ctx = setup_take_offer();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        None,
        current_time + 1000,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    advance_slot(&mut ctx.svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        None,
        current_time + 2000,
        2_000_000_000,
        73_000,
        86400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    advance_clock_by(&mut ctx.svm, 2500);

    // Price from second vector: 2.0 * (1 + 0.073 * 86400/31536000) ≈ 2.0004
    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        2_000_400,
        None,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();

    let user_onyc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.onyc_mint),
    );
    assert_eq!(user_onyc, 1_000_000_000);
}

// ===========================================================================
// Error Cases
// ===========================================================================

#[test]
fn test_fail_no_active_vector() {
    let mut ctx = setup_take_offer();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    // Add vector in future
    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        None,
        current_time + 10000,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000,
        None,
    );
    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.user]);
    assert!(result.is_err(), "should fail with no active vector");
}

#[test]
fn test_fail_insufficient_user_balance() {
    let mut ctx = setup_take_offer();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    // User only has 10,000 USDC, try to spend 20,000
    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        20_000_000_000,
        None,
    );
    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.user]);
    assert!(result.is_err(), "should fail with insufficient balance");
}

#[test]
fn test_fail_insufficient_vault_balance() {
    let mut ctx = setup_take_offer();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    // Very low price = lots of token_out needed
    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000,
        0,
        86400, // price = 0.001
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    // 20 USDC at 0.001 price = 20,000 token_out, but vault has only 10,000
    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        20_000_000,
        None,
    );
    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.user]);
    assert!(
        result.is_err(),
        "should fail with insufficient vault balance"
    );
}

// ===========================================================================
// Token Transfer Tests
// ===========================================================================

#[test]
fn test_transfer_tokens_correctly() {
    let mut ctx = setup_take_offer();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let token_in_amount = 1_000_100u64;

    let user_usdc_before = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.usdc_mint),
    );
    let boss_usdc_before = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&boss, &ctx.usdc_mint),
    );
    let (vault_auth, _) = find_offer_vault_authority_pda();
    let vault_onyc_before = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&vault_auth, &ctx.onyc_mint),
    );

    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        token_in_amount,
        None,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();

    let user_usdc_after = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.usdc_mint),
    );
    let user_onyc_after = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.onyc_mint),
    );
    let boss_usdc_after = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&boss, &ctx.usdc_mint),
    );
    let vault_onyc_after = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&vault_auth, &ctx.onyc_mint),
    );

    assert_eq!(user_usdc_before - user_usdc_after, token_in_amount);
    assert_eq!(user_onyc_after, 1_000_000_000);
    assert_eq!(boss_usdc_after - boss_usdc_before, token_in_amount);
    assert_eq!(vault_onyc_before - vault_onyc_after, 1_000_000_000);
}

// ===========================================================================
// Edge Cases
// ===========================================================================

#[test]
fn test_wrong_token_in_mint() {
    let mut ctx = setup_take_offer();
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
        86400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let wrong_mint = create_mint(&mut ctx.svm, &ctx.payer, 6, &boss);
    create_token_account(&mut ctx.svm, &wrong_mint, &boss, 0);
    create_token_account(
        &mut ctx.svm,
        &wrong_mint,
        &ctx.user.pubkey(),
        10_000_000_000,
    );

    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &wrong_mint,
        &ctx.onyc_mint,
        1_000_000,
        None,
    );
    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.user]);
    assert!(result.is_err(), "wrong token_in should fail");
}

#[test]
fn test_wrong_token_out_mint() {
    let mut ctx = setup_take_offer();
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
        86400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let wrong_mint = create_mint(&mut ctx.svm, &ctx.payer, 9, &boss);
    create_token_account(&mut ctx.svm, &wrong_mint, &boss, 0);
    create_token_account(
        &mut ctx.svm,
        &wrong_mint,
        &ctx.user.pubkey(),
        10_000_000_000,
    );

    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &wrong_mint,
        1_000_000,
        None,
    );
    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.user]);
    assert!(result.is_err(), "wrong token_out should fail");
}

#[test]
fn test_zero_apr_fixed_price() {
    let mut ctx = setup_take_offer();
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
        86400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    // Advance 10 days
    advance_clock_by(&mut ctx.svm, 86_401 * 10);

    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000, // exactly 1.0 USDC
        None,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();

    let user_onyc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.onyc_mint),
    );
    assert_eq!(user_onyc, 1_000_000_000);
}

#[test]
fn test_high_apr_long_period() {
    let mut ctx = setup_take_offer();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        365_000,
        86400, // 36.5% APR
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    // Advance 1 year
    advance_clock_by(&mut ctx.svm, 86400 * 365);

    // After 1 year: price = 1.0 * (1 + 0.365 * 366/365) ≈ 1.366
    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_366_000,
        None,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();

    let user_onyc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.onyc_mint),
    );
    assert_eq!(user_onyc, 1_000_000_000);
}

// ===========================================================================
// Vault Transfer (No Mint Authority)
// ===========================================================================

#[test]
fn test_vault_transfer_token_out_no_mint_authority() {
    let mut ctx = setup_take_offer();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let supply_before = get_mint_supply(&ctx.svm, &ctx.usdc_mint);
    let (vault_auth, _) = find_offer_vault_authority_pda();
    let vault_onyc_before = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&vault_auth, &ctx.onyc_mint),
    );

    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_100,
        None,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();

    let supply_after = get_mint_supply(&ctx.svm, &ctx.usdc_mint);
    let user_onyc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.onyc_mint),
    );
    let vault_onyc_after = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&vault_auth, &ctx.onyc_mint),
    );

    assert_eq!(supply_before, supply_after); // No supply burned
    assert_eq!(user_onyc, 1_000_000_000);
    assert_eq!(vault_onyc_before - vault_onyc_after, 1_000_000_000);
}

#[test]
fn test_user_to_boss_transfer_no_mint_authority() {
    let mut ctx = setup_take_offer();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let token_in_amount = 1_000_100u64;

    let user_usdc_before = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.usdc_mint),
    );
    let boss_usdc_before = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&boss, &ctx.usdc_mint),
    );
    let supply_before = get_mint_supply(&ctx.svm, &ctx.usdc_mint);

    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        token_in_amount,
        None,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();

    let user_usdc_after = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.usdc_mint),
    );
    let boss_usdc_after = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&boss, &ctx.usdc_mint),
    );
    let supply_after = get_mint_supply(&ctx.svm, &ctx.usdc_mint);

    assert_eq!(supply_before, supply_after); // No burning
    assert_eq!(user_usdc_before - user_usdc_after, token_in_amount);
    assert_eq!(boss_usdc_after - boss_usdc_before, token_in_amount);
}

// ===========================================================================
// Kill Switch Tests
// ===========================================================================

#[test]
fn test_kill_switch_rejects_take_offer() {
    let mut ctx = setup_take_offer();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let admin = Keypair::new();
    ctx.svm.airdrop(&admin.pubkey(), INITIAL_LAMPORTS).unwrap();
    let ix = build_add_admin_ix(&boss, &admin.pubkey());
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    advance_slot(&mut ctx.svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    advance_slot(&mut ctx.svm);

    // Enable kill switch
    let ix = build_set_kill_switch_ix(&admin.pubkey(), true);
    send_tx(&mut ctx.svm, &[ix], &[&admin]).unwrap();

    let state = read_state(&ctx.svm);
    assert!(state.is_killed);

    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_100,
        None,
    );
    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.user]);
    assert!(result.is_err(), "kill switch should block take_offer");
}

#[test]
fn test_kill_switch_disabled_allows_take_offer() {
    let mut ctx = setup_take_offer();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let admin = Keypair::new();
    ctx.svm.airdrop(&admin.pubkey(), INITIAL_LAMPORTS).unwrap();
    let ix = build_add_admin_ix(&boss, &admin.pubkey());
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    advance_slot(&mut ctx.svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    advance_slot(&mut ctx.svm);

    // Enable then disable
    let ix = build_set_kill_switch_ix(&admin.pubkey(), true);
    send_tx(&mut ctx.svm, &[ix], &[&admin]).unwrap();

    advance_slot(&mut ctx.svm);

    let ix = build_set_kill_switch_ix(&boss, false);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let state = read_state(&ctx.svm);
    assert!(!state.is_killed);

    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_100,
        None,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();

    let user_onyc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.onyc_mint),
    );
    assert_eq!(user_onyc, 1_000_000_000);
}

// ===========================================================================
// Approval Tests
// ===========================================================================

#[test]
fn test_take_offer_with_approval_required_fails_without_approval() {
    let (mut svm, payer, _original_onyc) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);
    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);

    // Create offer with needs_approval = true
    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, true, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (vault_authority, _) = find_offer_vault_authority_pda();
    create_token_account(&mut svm, &usdc_mint, &vault_authority, 0);
    create_token_account(&mut svm, &onyc_mint, &vault_authority, 10_000_000_000_000);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 10_000_000_000);
    create_token_account(&mut svm, &usdc_mint, &boss, 0);

    let approver = Keypair::new();
    let ix = build_add_approver_ix(&boss, &approver.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Try without approval
    let ix = build_take_offer_ix(
        &user.pubkey(),
        &boss,
        &usdc_mint,
        &onyc_mint,
        1_000_100,
        None,
    );
    let result = send_tx(&mut svm, &[ix], &[&user]);
    assert!(
        result.is_err(),
        "should fail without approval when required"
    );
}

#[test]
fn test_take_offer_with_valid_approval() {
    let (mut svm, payer, _original_onyc) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);
    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, true, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (vault_authority, _) = find_offer_vault_authority_pda();
    create_token_account(&mut svm, &usdc_mint, &vault_authority, 0);
    create_token_account(&mut svm, &onyc_mint, &vault_authority, 10_000_000_000_000);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 10_000_000_000);
    create_token_account(&mut svm, &usdc_mint, &boss, 0);

    let approver = Keypair::new();
    let ix = build_add_approver_ix(&boss, &approver.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Build approval
    let expiry_unix = current_time + 3600;
    let approval_msg = serialize_approval_message(&PROGRAM_ID, &user.pubkey(), expiry_unix);
    let ed25519_ix = build_ed25519_verify_ix(&approver, &approval_msg);

    let take_ix = build_take_offer_ix(
        &user.pubkey(),
        &boss,
        &usdc_mint,
        &onyc_mint,
        1_000_100,
        Some(&approval_msg),
    );
    send_tx(&mut svm, &[ed25519_ix, take_ix], &[&user]).unwrap();

    let user_onyc = get_token_balance(
        &svm,
        &get_associated_token_address(&user.pubkey(), &onyc_mint),
    );
    assert_eq!(user_onyc, 1_000_000_000);
}

// ===========================================================================
// Mint Authority Tests (program mints/burns instead of vault transfer)
// ===========================================================================

#[test]
fn test_mint_token_out_with_program_mint_authority() {
    let mut ctx = setup_take_offer();
    let boss = ctx.payer.pubkey();

    // Transfer mint authority for onyc to program
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &ctx.onyc_mint);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    advance_slot(&mut ctx.svm);

    let current_time = get_clock_time(&ctx.svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let (vault_auth, _) = find_offer_vault_authority_pda();
    let vault_onyc_before = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&vault_auth, &ctx.onyc_mint),
    );
    let supply_before = get_mint_supply(&ctx.svm, &ctx.onyc_mint);

    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_100,
        None,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();

    let user_onyc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.onyc_mint),
    );
    let vault_onyc_after = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&vault_auth, &ctx.onyc_mint),
    );
    let supply_after = get_mint_supply(&ctx.svm, &ctx.onyc_mint);

    assert_eq!(user_onyc, 1_000_000_000);
    assert_eq!(
        vault_onyc_before, vault_onyc_after,
        "vault unchanged (tokens minted)"
    );
    assert_eq!(
        supply_after - supply_before,
        1_000_000_000,
        "supply increased by mint"
    );
}

#[test]
fn test_burn_token_in_with_program_mint_authority() {
    let mut ctx = setup_take_offer();
    let boss = ctx.payer.pubkey();

    // Transfer mint authority for USDC to program (so it burns token_in)
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &ctx.usdc_mint);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    advance_slot(&mut ctx.svm);

    // Fix mint supply to match existing token balances (create_token_account doesn't update supply)
    let mut mint_data = ctx.svm.get_account(&ctx.usdc_mint).unwrap();
    mint_data.data[36..44].copy_from_slice(&10_000_000_000u64.to_le_bytes());
    ctx.svm.set_account(ctx.usdc_mint, mint_data).unwrap();

    let current_time = get_clock_time(&ctx.svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let token_in_amount = 1_000_000u64;
    let supply_before = get_mint_supply(&ctx.svm, &ctx.usdc_mint);
    let boss_usdc_before = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&boss, &ctx.usdc_mint),
    );

    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        token_in_amount,
        None,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();

    let supply_after = get_mint_supply(&ctx.svm, &ctx.usdc_mint);
    let boss_usdc_after = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&boss, &ctx.usdc_mint),
    );

    assert_eq!(
        supply_before - supply_after,
        token_in_amount,
        "supply decreased (burned)"
    );
    assert_eq!(
        boss_usdc_after, boss_usdc_before,
        "boss receives nothing (tokens burned)"
    );
}

#[test]
fn test_fee_collection_with_mint_authority_burn() {
    let mut ctx = setup_take_offer_with_fee(500); // 5% fee
    let boss = ctx.payer.pubkey();

    // Transfer mint authority for onyc to program
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &ctx.onyc_mint);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    advance_slot(&mut ctx.svm);

    let current_time = get_clock_time(&ctx.svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let token_in_amount = 1_000_000u64;

    let ix = build_take_offer_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        token_in_amount,
        None,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();

    // Boss receives full amount
    let boss_usdc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&boss, &ctx.usdc_mint),
    );
    assert_eq!(boss_usdc, token_in_amount);

    // fee = ceil(1_000_000 * 500 / 10000) = ceil(50000) = 50_000
    // net = 1_000_000 - 50_000 = 950_000
    // token_out = 950_000 * 1e9 / 1e6 = 950_000_000
    let user_onyc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.onyc_mint),
    );
    assert_eq!(user_onyc, 950_000_000);
}

// ===========================================================================
// Token-2022 Tests
// ===========================================================================

#[test]
fn test_take_offer_token2022_transfers() {
    let (mut svm, payer, _original_onyc) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint_2022(&mut svm, &payer, 6, &boss);
    let onyc_mint = create_mint_2022(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix_with_programs(
        &boss,
        &usdc_mint,
        &onyc_mint,
        0,
        false,
        false,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (vault_authority, _) = find_offer_vault_authority_pda();
    create_token_account_2022(&mut svm, &usdc_mint, &vault_authority, 0);
    create_token_account_2022(&mut svm, &onyc_mint, &vault_authority, 10_000_000_000_000);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account_2022(&mut svm, &usdc_mint, &user.pubkey(), 10_000_000_000);
    create_token_account_2022(&mut svm, &usdc_mint, &boss, 0);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        36_500,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // User token_out account
    create_token_account_2022(&mut svm, &onyc_mint, &user.pubkey(), 0);

    let token_in_amount = 1_000_100u64;

    let user_usdc_before = get_token_balance(
        &svm,
        &get_associated_token_address_2022(&user.pubkey(), &usdc_mint),
    );

    let ix = build_take_offer_ix_with_programs(
        &user.pubkey(),
        &boss,
        &usdc_mint,
        &onyc_mint,
        token_in_amount,
        None,
        &TOKEN_2022_PROGRAM_ID,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();

    let user_usdc_after = get_token_balance(
        &svm,
        &get_associated_token_address_2022(&user.pubkey(), &usdc_mint),
    );
    let user_onyc = get_token_balance(
        &svm,
        &get_associated_token_address_2022(&user.pubkey(), &onyc_mint),
    );
    let boss_usdc = get_token_balance(&svm, &get_associated_token_address_2022(&boss, &usdc_mint));

    assert_eq!(user_usdc_before - user_usdc_after, token_in_amount);
    assert_eq!(user_onyc, 1_000_000_000);
    assert_eq!(boss_usdc, token_in_amount);
}

#[test]
fn test_take_offer_token2022_zero_transfer_fee_accepted() {
    let (mut svm, payer, _original_onyc) = setup_initialized();
    let boss = payer.pubkey();

    // Token-2022 mint with TransferFeeConfig but 0% fee
    let usdc_mint = create_mint_2022_with_transfer_fee(&mut svm, &payer, 6, &boss, 0, 0);
    let onyc_mint = create_mint_2022_with_transfer_fee(&mut svm, &payer, 9, &boss, 0, 0);

    let ix = build_make_offer_ix_with_programs(
        &boss,
        &usdc_mint,
        &onyc_mint,
        0,
        false,
        false,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (vault_authority, _) = find_offer_vault_authority_pda();
    create_token_account_2022(&mut svm, &usdc_mint, &vault_authority, 0);
    create_token_account_2022(&mut svm, &onyc_mint, &vault_authority, 10_000_000_000_000);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account_2022(&mut svm, &usdc_mint, &user.pubkey(), 10_000_000_000);
    create_token_account_2022(&mut svm, &usdc_mint, &boss, 0);
    create_token_account_2022(&mut svm, &onyc_mint, &user.pubkey(), 0);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_take_offer_ix_with_programs(
        &user.pubkey(),
        &boss,
        &usdc_mint,
        &onyc_mint,
        1_000_000,
        None,
        &TOKEN_2022_PROGRAM_ID,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();

    let user_onyc = get_token_balance(
        &svm,
        &get_associated_token_address_2022(&user.pubkey(), &onyc_mint),
    );
    assert_eq!(user_onyc, 1_000_000_000);
}

#[test]
fn test_take_offer_token2022_rejects_token_in_transfer_fee() {
    let (mut svm, payer, _original_onyc) = setup_initialized();
    let boss = payer.pubkey();

    // token_in with non-zero transfer fee
    let usdc_mint = create_mint_2022_with_transfer_fee(&mut svm, &payer, 6, &boss, 500, 1_000_000);
    let onyc_mint = create_mint_2022(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix_with_programs(
        &boss,
        &usdc_mint,
        &onyc_mint,
        0,
        false,
        false,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (vault_authority, _) = find_offer_vault_authority_pda();
    create_token_account_2022(&mut svm, &usdc_mint, &vault_authority, 0);
    create_token_account_2022(&mut svm, &onyc_mint, &vault_authority, 10_000_000_000_000);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account_2022(&mut svm, &usdc_mint, &user.pubkey(), 10_000_000_000);
    create_token_account_2022(&mut svm, &usdc_mint, &boss, 0);
    create_token_account_2022(&mut svm, &onyc_mint, &user.pubkey(), 0);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_take_offer_ix_with_programs(
        &user.pubkey(),
        &boss,
        &usdc_mint,
        &onyc_mint,
        1_000_000,
        None,
        &TOKEN_2022_PROGRAM_ID,
        &TOKEN_2022_PROGRAM_ID,
    );
    let result = send_tx(&mut svm, &[ix], &[&user]);
    assert!(
        result.is_err(),
        "token_in with transfer fee should be rejected"
    );
}

#[test]
fn test_take_offer_token2022_rejects_token_out_transfer_fee() {
    let (mut svm, payer, _original_onyc) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint_2022(&mut svm, &payer, 6, &boss);
    // token_out with non-zero transfer fee
    let onyc_mint = create_mint_2022_with_transfer_fee(&mut svm, &payer, 9, &boss, 500, 1_000_000);

    let ix = build_make_offer_ix_with_programs(
        &boss,
        &usdc_mint,
        &onyc_mint,
        0,
        false,
        false,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (vault_authority, _) = find_offer_vault_authority_pda();
    create_token_account_2022(&mut svm, &usdc_mint, &vault_authority, 0);
    create_token_account_2022(&mut svm, &onyc_mint, &vault_authority, 10_000_000_000_000);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account_2022(&mut svm, &usdc_mint, &user.pubkey(), 10_000_000_000);
    create_token_account_2022(&mut svm, &usdc_mint, &boss, 0);
    create_token_account_2022(&mut svm, &onyc_mint, &user.pubkey(), 0);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_take_offer_ix_with_programs(
        &user.pubkey(),
        &boss,
        &usdc_mint,
        &onyc_mint,
        1_000_000,
        None,
        &TOKEN_2022_PROGRAM_ID,
        &TOKEN_2022_PROGRAM_ID,
    );
    let result = send_tx(&mut svm, &[ix], &[&user]);
    assert!(
        result.is_err(),
        "token_out with transfer fee should be rejected"
    );
}
