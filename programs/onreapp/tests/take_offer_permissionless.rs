mod common;

use common::*;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

/// Full setup for a permissionless take-offer test scenario:
///   - Initialized state with boss as upgrade authority
///   - USDC (token_in, 6 decimals) and ONyc (token_out, 9 decimals)
///   - Offer created with needs_approval + allow_permissionless
///   - Offer vector added (price = 1.0, no APR, 1-day price_fix_duration)
///   - Vault funded with token_out
///   - Permissionless authority intermediary accounts created
///   - Boss token_in account created
///   - Approver set
fn setup_permissionless_offer() -> PermissionlessOfferCtx {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    // Create USDC (token_in) and ONyc (token_out)
    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);
    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);

    // Make offer: 0% fee, needs_approval=true, allow_permissionless=true
    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, true, true);
    send_tx(&mut svm, &[ix], &[&payer]).expect("make_offer failed");

    // Add offer vector: start_time = current clock time, base_price = 1.0 (1_000_000_000),
    // apr = 0, price_fix_duration = 86400 (1 day)
    let current_time = 1704067200u64; // Jan 1, 2024 (matches setup clock)
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000, // base_price = 1.0
        0,             // apr = 0
        86400,         // price_fix_duration = 1 day
    );
    send_tx(&mut svm, &[ix], &[&payer]).expect("add_offer_vector failed");

    // Create vault accounts with token_out balance (for transfer mechanism)
    let (vault_authority, _) = find_offer_vault_authority_pda();
    // Vault needs token_out to transfer to users
    create_token_account(&mut svm, &onyc_mint, &vault_authority, 1_000_000_000_000);
    // Vault token_in account (for burn operations, starts empty)
    create_token_account(&mut svm, &usdc_mint, &vault_authority, 0);

    // Create permissionless authority intermediary accounts
    let (permissionless_authority, _) = find_permissionless_authority_pda();
    create_token_account(&mut svm, &usdc_mint, &permissionless_authority, 0);
    create_token_account(&mut svm, &onyc_mint, &permissionless_authority, 0);

    // Create boss token_in account
    create_token_account(&mut svm, &usdc_mint, &boss, 0);

    // Set approver
    let approver = Keypair::new();
    let ix = build_add_approver_ix(&boss, &approver.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).expect("add_approver failed");

    PermissionlessOfferCtx {
        svm,
        payer,
        usdc_mint,
        onyc_mint,
        approver,
    }
}

struct PermissionlessOfferCtx {
    svm: litesvm::LiteSVM,
    payer: Keypair,
    usdc_mint: Pubkey,
    onyc_mint: Pubkey,
    approver: Keypair,
}

// ===========================================================================
// Ed25519 Approval + Permissionless Take Offer Tests
// ===========================================================================

#[test]
fn test_take_offer_permissionless_with_valid_approval() {
    let mut ctx = setup_permissionless_offer();
    let boss = ctx.payer.pubkey();

    // Create user
    let user = Keypair::new();
    ctx.svm
        .airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS)
        .unwrap();

    // Fund user with USDC (1000 USDC = 1_000_000_000 in 6 decimals)
    let token_in_amount: u64 = 1_000_000_000; // 1000 USDC
    create_token_account(
        &mut ctx.svm,
        &ctx.usdc_mint,
        &user.pubkey(),
        token_in_amount,
    );

    // Create user_token_out account (needed for init_if_needed but let's pre-create)
    create_token_account(&mut ctx.svm, &ctx.onyc_mint, &user.pubkey(), 0);

    // Build approval message
    let expiry_unix = 1704067200u64 + 3600; // 1 hour from now
    let approval_msg_bytes = serialize_approval_message(&PROGRAM_ID, &user.pubkey(), expiry_unix);

    // Build Ed25519 verify instruction (must be right before the program instruction)
    let ed25519_ix = build_ed25519_verify_ix(&ctx.approver, &approval_msg_bytes);

    // Build take_offer_permissionless instruction
    let take_ix = build_take_offer_permissionless_ix(
        &user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        token_in_amount,
        Some(&approval_msg_bytes),
    );

    // Send both instructions in one transaction (Ed25519 verify must be before take_offer)
    let result = send_tx(&mut ctx.svm, &[ed25519_ix, take_ix], &[&user]);
    assert!(
        result.is_ok(),
        "take_offer_permissionless with valid approval should succeed: {:?}",
        result.err()
    );

    // Verify token balances
    let user_usdc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&user.pubkey(), &ctx.usdc_mint),
    );
    assert_eq!(user_usdc, 0, "user should have spent all USDC");

    let user_onyc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&user.pubkey(), &ctx.onyc_mint),
    );
    assert!(user_onyc > 0, "user should have received ONyc tokens");

    let boss_usdc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&boss, &ctx.usdc_mint),
    );
    assert_eq!(boss_usdc, token_in_amount, "boss should have received USDC");
}

#[test]
fn test_take_offer_permissionless_fails_without_approval() {
    let mut ctx = setup_permissionless_offer();
    let boss = ctx.payer.pubkey();

    let user = Keypair::new();
    ctx.svm
        .airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS)
        .unwrap();

    let token_in_amount: u64 = 1_000_000_000;
    create_token_account(
        &mut ctx.svm,
        &ctx.usdc_mint,
        &user.pubkey(),
        token_in_amount,
    );
    create_token_account(&mut ctx.svm, &ctx.onyc_mint, &user.pubkey(), 0);

    // No Ed25519 instruction, no approval_message
    let take_ix = build_take_offer_permissionless_ix(
        &user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        token_in_amount,
        None,
    );

    let result = send_tx(&mut ctx.svm, &[take_ix], &[&user]);
    assert!(
        result.is_err(),
        "should fail without approval when needs_approval is true"
    );
}

#[test]
fn test_take_offer_permissionless_fails_with_expired_approval() {
    let mut ctx = setup_permissionless_offer();
    let boss = ctx.payer.pubkey();

    let user = Keypair::new();
    ctx.svm
        .airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS)
        .unwrap();

    let token_in_amount: u64 = 1_000_000_000;
    create_token_account(
        &mut ctx.svm,
        &ctx.usdc_mint,
        &user.pubkey(),
        token_in_amount,
    );
    create_token_account(&mut ctx.svm, &ctx.onyc_mint, &user.pubkey(), 0);

    // Expired approval (time in the past)
    let expiry_unix = 1704067200u64 - 1; // 1 second before current clock
    let approval_msg_bytes = serialize_approval_message(&PROGRAM_ID, &user.pubkey(), expiry_unix);

    let ed25519_ix = build_ed25519_verify_ix(&ctx.approver, &approval_msg_bytes);

    let take_ix = build_take_offer_permissionless_ix(
        &user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        token_in_amount,
        Some(&approval_msg_bytes),
    );

    let result = send_tx(&mut ctx.svm, &[ed25519_ix, take_ix], &[&user]);
    assert!(result.is_err(), "should fail with expired approval message");
}

#[test]
fn test_take_offer_permissionless_fails_with_wrong_approver() {
    let mut ctx = setup_permissionless_offer();
    let boss = ctx.payer.pubkey();

    let user = Keypair::new();
    ctx.svm
        .airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS)
        .unwrap();

    let token_in_amount: u64 = 1_000_000_000;
    create_token_account(
        &mut ctx.svm,
        &ctx.usdc_mint,
        &user.pubkey(),
        token_in_amount,
    );
    create_token_account(&mut ctx.svm, &ctx.onyc_mint, &user.pubkey(), 0);

    let expiry_unix = 1704067200u64 + 3600;
    let approval_msg_bytes = serialize_approval_message(&PROGRAM_ID, &user.pubkey(), expiry_unix);

    // Sign with a WRONG keypair (not the registered approver)
    let wrong_approver = Keypair::new();
    let ed25519_ix = build_ed25519_verify_ix(&wrong_approver, &approval_msg_bytes);

    let take_ix = build_take_offer_permissionless_ix(
        &user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        token_in_amount,
        Some(&approval_msg_bytes),
    );

    let result = send_tx(&mut ctx.svm, &[ed25519_ix, take_ix], &[&user]);
    assert!(
        result.is_err(),
        "should fail when Ed25519 is signed by wrong approver"
    );
}

#[test]
fn test_take_offer_permissionless_fails_with_wrong_user_in_approval() {
    let mut ctx = setup_permissionless_offer();
    let boss = ctx.payer.pubkey();

    let user = Keypair::new();
    ctx.svm
        .airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS)
        .unwrap();

    let token_in_amount: u64 = 1_000_000_000;
    create_token_account(
        &mut ctx.svm,
        &ctx.usdc_mint,
        &user.pubkey(),
        token_in_amount,
    );
    create_token_account(&mut ctx.svm, &ctx.onyc_mint, &user.pubkey(), 0);

    // Approval message for a DIFFERENT user
    let wrong_user = Keypair::new();
    let expiry_unix = 1704067200u64 + 3600;
    let approval_msg_bytes =
        serialize_approval_message(&PROGRAM_ID, &wrong_user.pubkey(), expiry_unix);

    let ed25519_ix = build_ed25519_verify_ix(&ctx.approver, &approval_msg_bytes);

    let take_ix = build_take_offer_permissionless_ix(
        &user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        token_in_amount,
        Some(&approval_msg_bytes),
    );

    let result = send_tx(&mut ctx.svm, &[ed25519_ix, take_ix], &[&user]);
    assert!(
        result.is_err(),
        "should fail when approval message is for a different user"
    );
}

#[test]
fn test_take_offer_permissionless_fails_with_wrong_program_in_approval() {
    let mut ctx = setup_permissionless_offer();
    let boss = ctx.payer.pubkey();

    let user = Keypair::new();
    ctx.svm
        .airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS)
        .unwrap();

    let token_in_amount: u64 = 1_000_000_000;
    create_token_account(
        &mut ctx.svm,
        &ctx.usdc_mint,
        &user.pubkey(),
        token_in_amount,
    );
    create_token_account(&mut ctx.svm, &ctx.onyc_mint, &user.pubkey(), 0);

    // Approval message with wrong program_id
    let wrong_program = Pubkey::new_unique();
    let expiry_unix = 1704067200u64 + 3600;
    let approval_msg_bytes =
        serialize_approval_message(&wrong_program, &user.pubkey(), expiry_unix);

    let ed25519_ix = build_ed25519_verify_ix(&ctx.approver, &approval_msg_bytes);

    let take_ix = build_take_offer_permissionless_ix(
        &user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        token_in_amount,
        Some(&approval_msg_bytes),
    );

    let result = send_tx(&mut ctx.svm, &[ed25519_ix, take_ix], &[&user]);
    assert!(
        result.is_err(),
        "should fail when approval message has wrong program_id"
    );
}

// ===========================================================================
// Permissionless flow without approval (needs_approval=false)
// ===========================================================================

struct PermissionlessNoApprovalCtx {
    svm: litesvm::LiteSVM,
    payer: Keypair,
    usdc_mint: Pubkey,
    onyc_mint: Pubkey,
    user: Keypair,
}

fn setup_permissionless_no_approval() -> PermissionlessNoApprovalCtx {
    setup_permissionless_no_approval_with_fee(0)
}

fn setup_permissionless_no_approval_with_fee(fee_bps: u16) -> PermissionlessNoApprovalCtx {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);
    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);

    // needs_approval=false, allow_permissionless=true
    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, fee_bps, false, true);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (vault_authority, _) = find_offer_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &vault_authority, 1_000_000_000_000);
    create_token_account(&mut svm, &usdc_mint, &vault_authority, 0);

    let (permissionless_authority, _) = find_permissionless_authority_pda();
    create_token_account(&mut svm, &usdc_mint, &permissionless_authority, 0);
    create_token_account(&mut svm, &onyc_mint, &permissionless_authority, 0);

    create_token_account(&mut svm, &usdc_mint, &boss, 0);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 10_000_000_000);
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 0);

    PermissionlessNoApprovalCtx {
        svm,
        payer,
        usdc_mint,
        onyc_mint,
        user,
    }
}

// ===========================================================================
// Basic Flow Tests
// ===========================================================================

#[test]
fn test_permissionless_basic_success() {
    let mut ctx = setup_permissionless_no_approval();
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

    // 1.0001 USDC at price 1.0001 = 1 ONyc
    let ix = build_take_offer_permissionless_ix(
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

    let user_usdc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.usdc_mint),
    );
    assert_eq!(user_usdc, 10_000_000_000 - 1_000_100);
}

// ===========================================================================
// Price Calculation Tests
// ===========================================================================

#[test]
fn test_permissionless_price_first_interval() {
    let mut ctx = setup_permissionless_no_approval();
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

    let ix = build_take_offer_permissionless_ix(
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

#[test]
fn test_permissionless_price_with_fee() {
    let mut ctx = setup_permissionless_no_approval_with_fee(100); // 1% fee
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

    let ix = build_take_offer_permissionless_ix(
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
    // fee = ceil(1_000_100 * 100 / 10000) = 10_001, net = 990_099
    // token_out = 990_099 * 1e9 / (1.0001 * 1e6) = 990_000_000
    assert_eq!(user_onyc, 990_000_000);
}

#[test]
fn test_permissionless_price_second_interval() {
    let mut ctx = setup_permissionless_no_approval();
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

    let ix = build_take_offer_permissionless_ix(
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

#[test]
fn test_permissionless_most_recent_active_vector() {
    let mut ctx = setup_permissionless_no_approval();
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
    let ix = build_take_offer_permissionless_ix(
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
// Error Handling Tests
// ===========================================================================

#[test]
fn test_permissionless_fail_no_active_vector() {
    let mut ctx = setup_permissionless_no_approval();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    // Add vector in the future
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

    let ix = build_take_offer_permissionless_ix(
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
fn test_permissionless_fail_insufficient_user_balance() {
    let mut ctx = setup_permissionless_no_approval();
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

    // User has 10,000 USDC, try 20,000
    let ix = build_take_offer_permissionless_ix(
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
fn test_permissionless_fail_insufficient_vault_balance() {
    let mut ctx = setup_permissionless_no_approval();
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

    // 20 USDC at 0.001 price = 20,000 token_out, but vault has only 1,000
    let ix = build_take_offer_permissionless_ix(
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
// Edge Cases
// ===========================================================================

#[test]
fn test_permissionless_zero_apr_fixed_price() {
    let mut ctx = setup_permissionless_no_approval();
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

    advance_clock_by(&mut ctx.svm, 86_401 * 10);

    let ix = build_take_offer_permissionless_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000,
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
fn test_permissionless_high_apr_long_period() {
    let mut ctx = setup_permissionless_no_approval();
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

    // After 1 year: price ≈ 1.366
    let ix = build_take_offer_permissionless_ix(
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
// Token Transfer Tests (vault mode, no mint authority)
// ===========================================================================

#[test]
fn test_permissionless_vault_transfer_token_out() {
    let mut ctx = setup_permissionless_no_approval();
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

    let (vault_auth, _) = find_offer_vault_authority_pda();
    let supply_before = get_mint_supply(&ctx.svm, &ctx.onyc_mint);
    let vault_onyc_before = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&vault_auth, &ctx.onyc_mint),
    );

    let ix = build_take_offer_permissionless_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_100,
        None,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();

    let supply_after = get_mint_supply(&ctx.svm, &ctx.onyc_mint);
    let user_onyc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.onyc_mint),
    );
    let vault_onyc_after = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&vault_auth, &ctx.onyc_mint),
    );

    assert_eq!(
        supply_before, supply_after,
        "no supply change (not minting)"
    );
    assert_eq!(user_onyc, 1_000_000_000);
    assert_eq!(vault_onyc_before - vault_onyc_after, 1_000_000_000);
}

#[test]
fn test_permissionless_user_to_boss_transfer() {
    let mut ctx = setup_permissionless_no_approval();
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

    let ix = build_take_offer_permissionless_ix(
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

    assert_eq!(
        supply_before, supply_after,
        "no supply change (not burning)"
    );
    assert_eq!(user_usdc_before - user_usdc_after, token_in_amount);
    assert_eq!(boss_usdc_after - boss_usdc_before, token_in_amount);

    // Verify intermediary accounts are empty
    let (permissionless_auth, _) = find_permissionless_authority_pda();
    let intermediary_usdc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&permissionless_auth, &ctx.usdc_mint),
    );
    let intermediary_onyc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&permissionless_auth, &ctx.onyc_mint),
    );
    assert_eq!(intermediary_usdc, 0, "no residual token_in in intermediary");
    assert_eq!(
        intermediary_onyc, 0,
        "no residual token_out in intermediary"
    );
}

// ===========================================================================
// Mint Authority Tests
// ===========================================================================

#[test]
fn test_permissionless_mint_token_out_with_mint_authority() {
    let mut ctx = setup_permissionless_no_approval();
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

    let ix = build_take_offer_permissionless_ix(
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

    assert_eq!(user_onyc, 1_000_000_000);
    assert_eq!(
        vault_onyc_before, vault_onyc_after,
        "vault unchanged (tokens were minted)"
    );
}

#[test]
fn test_permissionless_burn_token_in_with_mint_authority() {
    let mut ctx = setup_permissionless_no_approval();
    let boss = ctx.payer.pubkey();

    // Transfer mint authority for USDC to program (so it burns token_in)
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &ctx.usdc_mint);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    advance_slot(&mut ctx.svm);

    // Fix mint supply to match token balances (create_token_account doesn't update supply)
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

    let ix = build_take_offer_permissionless_ix(
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
        "supply should decrease (burned)"
    );
    assert_eq!(
        boss_usdc_after, boss_usdc_before,
        "boss receives nothing (tokens burned)"
    );
}

#[test]
fn test_permissionless_fee_calculations_when_minting() {
    let mut ctx = setup_permissionless_no_approval_with_fee(500); // 5% fee
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

    let token_in_amount = 1_050_000u64; // 1.05 USDC

    let ix = build_take_offer_permissionless_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        token_in_amount,
        None,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();

    // Boss receives full amount (fee included in transfer)
    let boss_usdc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&boss, &ctx.usdc_mint),
    );
    assert_eq!(boss_usdc, token_in_amount);

    // User receives token_out based on net amount after fee
    // fee = ceil(1_050_000 * 500 / 10000) = ceil(52500) = 52_500
    // net = 1_050_000 - 52_500 = 997_500
    // token_out = 997_500 * 1e9 / 1e6 = 997_500_000
    let user_onyc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.onyc_mint),
    );
    assert_eq!(user_onyc, 997_500_000);
}

// ===========================================================================
// Kill Switch Tests
// ===========================================================================

#[test]
fn test_permissionless_kill_switch_rejects() {
    let mut ctx = setup_permissionless_no_approval();
    let boss = ctx.payer.pubkey();

    let admin = Keypair::new();
    ctx.svm.airdrop(&admin.pubkey(), INITIAL_LAMPORTS).unwrap();
    let ix = build_add_admin_ix(&boss, &admin.pubkey());
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
    advance_slot(&mut ctx.svm);

    let ix = build_set_kill_switch_ix(&admin.pubkey(), true);
    send_tx(&mut ctx.svm, &[ix], &[&admin]).unwrap();

    let ix = build_take_offer_permissionless_ix(
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_100,
        None,
    );
    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.user]);
    assert!(
        result.is_err(),
        "kill switch should block permissionless take"
    );
}

#[test]
fn test_permissionless_not_allowed_rejects() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);
    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);

    // allow_permissionless=false
    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (vault_authority, _) = find_offer_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &vault_authority, 1_000_000_000_000);
    create_token_account(&mut svm, &usdc_mint, &vault_authority, 0);

    let (permissionless_authority, _) = find_permissionless_authority_pda();
    create_token_account(&mut svm, &usdc_mint, &permissionless_authority, 0);
    create_token_account(&mut svm, &onyc_mint, &permissionless_authority, 0);

    create_token_account(&mut svm, &usdc_mint, &boss, 0);

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

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 10_000_000_000);
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 0);

    let ix = build_take_offer_permissionless_ix(
        &user.pubkey(),
        &boss,
        &usdc_mint,
        &onyc_mint,
        1_000_000,
        None,
    );
    let result = send_tx(&mut svm, &[ix], &[&user]);
    assert!(
        result.is_err(),
        "should fail when permissionless not allowed"
    );
}

#[test]
fn test_permissionless_kill_switch_disabled_allows() {
    let mut ctx = setup_permissionless_no_approval();
    let boss = ctx.payer.pubkey();

    let admin = Keypair::new();
    ctx.svm.airdrop(&admin.pubkey(), INITIAL_LAMPORTS).unwrap();
    let ix = build_add_admin_ix(&boss, &admin.pubkey());
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
    advance_slot(&mut ctx.svm);

    // Enable then disable
    let ix = build_set_kill_switch_ix(&admin.pubkey(), true);
    send_tx(&mut ctx.svm, &[ix], &[&admin]).unwrap();
    advance_slot(&mut ctx.svm);

    let ix = build_set_kill_switch_ix(&boss, false);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let ix = build_take_offer_permissionless_ix(
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
// Token-2022 Tests
// ===========================================================================

#[test]
fn test_permissionless_token2022_basic_success() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint_2022(&mut svm, &payer, 6, &boss);
    let onyc_mint = create_mint_2022(&mut svm, &payer, 9, &boss);

    // needs_approval=false, allow_permissionless=true
    let ix = build_make_offer_ix_with_programs(
        &boss,
        &usdc_mint,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (vault_authority, _) = find_offer_vault_authority_pda();
    create_token_account_2022(&mut svm, &onyc_mint, &vault_authority, 1_000_000_000_000);
    create_token_account_2022(&mut svm, &usdc_mint, &vault_authority, 0);

    let (permissionless_authority, _) = find_permissionless_authority_pda();
    create_token_account_2022(&mut svm, &usdc_mint, &permissionless_authority, 0);
    create_token_account_2022(&mut svm, &onyc_mint, &permissionless_authority, 0);

    create_token_account_2022(&mut svm, &usdc_mint, &boss, 0);

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

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account_2022(&mut svm, &usdc_mint, &user.pubkey(), 10_000_000_000);
    create_token_account_2022(&mut svm, &onyc_mint, &user.pubkey(), 0);

    let ix = build_take_offer_permissionless_ix_with_programs(
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

    let boss_usdc = get_token_balance(&svm, &get_associated_token_address_2022(&boss, &usdc_mint));
    assert_eq!(boss_usdc, 1_000_000);
}

#[test]
fn test_permissionless_token2022_rejects_token_in_transfer_fee() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint_2022_with_transfer_fee(&mut svm, &payer, 6, &boss, 500, 1_000_000);
    let onyc_mint = create_mint_2022(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix_with_programs(
        &boss,
        &usdc_mint,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (vault_authority, _) = find_offer_vault_authority_pda();
    create_token_account_2022(&mut svm, &onyc_mint, &vault_authority, 1_000_000_000_000);
    create_token_account_2022(&mut svm, &usdc_mint, &vault_authority, 0);

    let (permissionless_authority, _) = find_permissionless_authority_pda();
    create_token_account_2022(&mut svm, &usdc_mint, &permissionless_authority, 0);
    create_token_account_2022(&mut svm, &onyc_mint, &permissionless_authority, 0);

    create_token_account_2022(&mut svm, &usdc_mint, &boss, 0);

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

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account_2022(&mut svm, &usdc_mint, &user.pubkey(), 10_000_000_000);
    create_token_account_2022(&mut svm, &onyc_mint, &user.pubkey(), 0);

    let ix = build_take_offer_permissionless_ix_with_programs(
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
fn test_permissionless_token2022_rejects_token_out_transfer_fee() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint_2022(&mut svm, &payer, 6, &boss);
    let onyc_mint = create_mint_2022_with_transfer_fee(&mut svm, &payer, 9, &boss, 500, 1_000_000);

    let ix = build_make_offer_ix_with_programs(
        &boss,
        &usdc_mint,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (vault_authority, _) = find_offer_vault_authority_pda();
    create_token_account_2022(&mut svm, &onyc_mint, &vault_authority, 1_000_000_000_000);
    create_token_account_2022(&mut svm, &usdc_mint, &vault_authority, 0);

    let (permissionless_authority, _) = find_permissionless_authority_pda();
    create_token_account_2022(&mut svm, &usdc_mint, &permissionless_authority, 0);
    create_token_account_2022(&mut svm, &onyc_mint, &permissionless_authority, 0);

    create_token_account_2022(&mut svm, &usdc_mint, &boss, 0);

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

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account_2022(&mut svm, &usdc_mint, &user.pubkey(), 10_000_000_000);
    create_token_account_2022(&mut svm, &onyc_mint, &user.pubkey(), 0);

    let ix = build_take_offer_permissionless_ix_with_programs(
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
