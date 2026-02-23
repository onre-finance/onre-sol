mod common;

use common::*;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use litesvm::LiteSVM;

// ---------------------------------------------------------------------------
// Shared setup
// ---------------------------------------------------------------------------

/// Fully set-up context for partial-fulfillment tests.
///
/// Returns (svm, payer/boss, onyc_mint, usdc_mint, redemption_offer_pda,
///          redemption_request_pda, user)
///
/// Balances after setup:
/// - User holds 10 ONyc = 10_000_000_000 (9 dec)
/// - Redemption request created for REDEMPTION_AMOUNT
/// - Redemption vault pre-funded with 100 USDC for transfer-mode payout
///   (program does NOT have mint authority – simplest mode)
fn setup_partial(
    fee_bps: u16,
    redemption_amount: u64,
) -> (
    LiteSVM,
    Keypair,  // payer (== boss == redemption_admin)
    Pubkey,   // onyc_mint
    Pubkey,   // usdc_mint
    Pubkey,   // redemption_offer_pda
    Pubkey,   // redemption_request_pda (request_id = 0)
    Keypair,  // user / redeemer
) {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);

    // Set boss as redemption_admin
    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Create underlying offer: usdc -> onyc
    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Price vector: 1.0, 0% APR
    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss, &usdc_mint, &onyc_mint,
        None, current_time, 1_000_000_000, 0, 86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    // Create redemption offer: onyc -> usdc
    let ix = build_make_redemption_offer_ix(&boss, &onyc_mint, &usdc_mint, fee_bps);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();

    // Vault for locked onyc (token_in)
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);
    // Vault for usdc payout (token_out): pre-funded; program does NOT have mint authority
    create_token_account(&mut svm, &usdc_mint, &redemption_vault_authority, 100_000_000);

    // Boss token accounts required by the instruction
    create_token_account(&mut svm, &onyc_mint, &boss, 0);

    // User (redeemer)
    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 10_000_000_000);
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 0);

    // Create redemption request (locks onyc in vault)
    let ix = build_create_redemption_request_ix(
        &user.pubkey(), &onyc_mint, &usdc_mint, redemption_amount, 0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    let (redemption_offer_pda, _) = find_redemption_offer_pda(&onyc_mint, &usdc_mint);
    let (redemption_request_pda, _) = find_redemption_request_pda(&redemption_offer_pda, 0);

    (svm, payer, onyc_mint, usdc_mint, redemption_offer_pda, redemption_request_pda, user)
}

// ---------------------------------------------------------------------------
// fulfilled_amount tracking
// ---------------------------------------------------------------------------

#[test]
fn test_partial_fulfill_updates_fulfilled_amount() {
    // 9 ONyc total; fulfill 2 ONyc first
    let (mut svm, payer, onyc_mint, usdc_mint, redemption_offer_pda, redemption_request_pda, user) =
        setup_partial(0, 9_000_000_000);
    let boss = payer.pubkey();

    let ix = build_fulfill_redemption_request_ix(
        &boss, &boss, &user.pubkey(),
        &onyc_mint, &usdc_mint, 0,
        2_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let req = read_redemption_request(&svm, &redemption_offer_pda, 0);
    assert_eq!(req.fulfilled_amount, 2_000_000_000, "fulfilled_amount should equal partial");
    assert_eq!(req.amount, 9_000_000_000, "total amount unchanged");
}

#[test]
fn test_partial_fulfill_account_stays_open() {
    let (mut svm, payer, onyc_mint, usdc_mint, redemption_offer_pda, redemption_request_pda, user) =
        setup_partial(0, 9_000_000_000);
    let boss = payer.pubkey();

    let ix = build_fulfill_redemption_request_ix(
        &boss, &boss, &user.pubkey(),
        &onyc_mint, &usdc_mint, 0,
        3_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Account must still be present
    assert!(svm.get_account(&redemption_request_pda).is_some(),
        "account should stay open after partial fulfillment");
}

#[test]
fn test_partial_fulfill_account_closed_after_final_call() {
    // Use three distinct amounts to avoid LiteSVM deduplication
    let (mut svm, payer, onyc_mint, usdc_mint, redemption_offer_pda, redemption_request_pda, user) =
        setup_partial(0, 9_000_000_000);
    let boss = payer.pubkey();

    for amount in [2_000_000_000u64, 3_000_000_000, 4_000_000_000] {
        let ix = build_fulfill_redemption_request_ix(
            &boss, &boss, &user.pubkey(),
            &onyc_mint, &usdc_mint, 0,
            amount,
        );
        send_tx(&mut svm, &[ix], &[&payer]).unwrap();
        advance_slot(&mut svm);
    }

    assert!(svm.get_account(&redemption_request_pda).is_none(),
        "account should be closed once fully fulfilled");
}

// ---------------------------------------------------------------------------
// Offer-level counter updates
// ---------------------------------------------------------------------------

#[test]
fn test_partial_fulfill_decrements_requested_redemptions_per_call() {
    let (mut svm, payer, onyc_mint, usdc_mint, redemption_offer_pda, _, user) =
        setup_partial(0, 9_000_000_000);
    let boss = payer.pubkey();

    let before = read_redemption_offer(&svm, &onyc_mint, &usdc_mint).requested_redemptions;

    let ix = build_fulfill_redemption_request_ix(
        &boss, &boss, &user.pubkey(),
        &onyc_mint, &usdc_mint, 0,
        2_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let after = read_redemption_offer(&svm, &onyc_mint, &usdc_mint).requested_redemptions;
    assert_eq!(after, before - 2_000_000_000,
        "requested_redemptions should decrease by exactly the partial amount");
}

#[test]
fn test_partial_fulfill_increments_executed_redemptions_per_call() {
    let (mut svm, payer, onyc_mint, usdc_mint, _, _, user) =
        setup_partial(0, 9_000_000_000);
    let boss = payer.pubkey();

    let ix = build_fulfill_redemption_request_ix(
        &boss, &boss, &user.pubkey(),
        &onyc_mint, &usdc_mint, 0,
        2_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_redemption_offer(&svm, &onyc_mint, &usdc_mint);
    assert_eq!(offer.executed_redemptions, 2_000_000_000,
        "executed_redemptions should increase by the partial amount");
}

#[test]
fn test_full_3_call_redemption_correct_offer_stats() {
    let (mut svm, payer, onyc_mint, usdc_mint, _, _, user) =
        setup_partial(0, 9_000_000_000);
    let boss = payer.pubkey();

    for amount in [2_000_000_000u64, 3_000_000_000, 4_000_000_000] {
        let ix = build_fulfill_redemption_request_ix(
            &boss, &boss, &user.pubkey(),
            &onyc_mint, &usdc_mint, 0,
            amount,
        );
        send_tx(&mut svm, &[ix], &[&payer]).unwrap();
        advance_slot(&mut svm);
    }

    let offer = read_redemption_offer(&svm, &onyc_mint, &usdc_mint);
    assert_eq!(offer.executed_redemptions, 9_000_000_000, "should match total redeemed");
    assert_eq!(offer.requested_redemptions, 0, "nothing left pending");
}

// ---------------------------------------------------------------------------
// Token payout
// ---------------------------------------------------------------------------

#[test]
fn test_partial_fulfill_cumulative_usdc_received() {
    // Price 1.0, 0% fee, 9 ONyc total in three distinct partial calls
    let (mut svm, payer, onyc_mint, usdc_mint, _, _, user) =
        setup_partial(0, 9_000_000_000);
    let boss = payer.pubkey();

    let user_usdc_ata = get_associated_token_address(&user.pubkey(), &usdc_mint);

    // 2 ONyc → 2 USDC = 2_000_000 (6 dec)
    let ix = build_fulfill_redemption_request_ix(
        &boss, &boss, &user.pubkey(),
        &onyc_mint, &usdc_mint, 0,
        2_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    assert_eq!(get_token_balance(&svm, &user_usdc_ata), 2_000_000, "after 1st partial");

    // 3 ONyc → 3 USDC cumulative 5 USDC = 5_000_000
    let ix = build_fulfill_redemption_request_ix(
        &boss, &boss, &user.pubkey(),
        &onyc_mint, &usdc_mint, 0,
        3_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    assert_eq!(get_token_balance(&svm, &user_usdc_ata), 5_000_000, "after 2nd partial");

    // 4 ONyc → 4 USDC cumulative 9 USDC = 9_000_000
    let ix = build_fulfill_redemption_request_ix(
        &boss, &boss, &user.pubkey(),
        &onyc_mint, &usdc_mint, 0,
        4_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    assert_eq!(get_token_balance(&svm, &user_usdc_ata), 9_000_000, "after full fulfillment");
}

// ---------------------------------------------------------------------------
// Cancellation of partially fulfilled request
// ---------------------------------------------------------------------------

#[test]
fn test_cancel_after_partial_returns_remaining() {
    // 9 ONyc total, fulfill 3 ONyc, then cancel → redeemer gets back 6 ONyc
    let (mut svm, payer, onyc_mint, usdc_mint, redemption_offer_pda, redemption_request_pda, user) =
        setup_partial(0, 9_000_000_000);
    let boss = payer.pubkey();

    let user_onyc_ata = get_associated_token_address(&user.pubkey(), &onyc_mint);
    let balance_after_lock = get_token_balance(&svm, &user_onyc_ata);

    // Partially fulfill 3 ONyc (burned from vault)
    let ix = build_fulfill_redemption_request_ix(
        &boss, &boss, &user.pubkey(),
        &onyc_mint, &usdc_mint, 0,
        3_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Cancel – should return only the unfulfilled 6 ONyc
    let ix = build_cancel_redemption_request_ix(
        &user.pubkey(), &user.pubkey(), &boss,
        &onyc_mint, &usdc_mint, 0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();

    let balance_after_cancel = get_token_balance(&svm, &user_onyc_ata);
    // balance_after_lock + returned(6 ONyc) = balance_after_lock + 6_000_000_000
    assert_eq!(
        balance_after_cancel,
        balance_after_lock + 6_000_000_000,
        "only unfulfilled ONyc should be returned",
    );

    assert!(svm.get_account(&redemption_request_pda).is_none(),
        "account closed after cancel");
}

#[test]
fn test_cancel_after_partial_requested_redemptions_zero() {
    let (mut svm, payer, onyc_mint, usdc_mint, _, _, user) =
        setup_partial(0, 9_000_000_000);
    let boss = payer.pubkey();

    let ix = build_fulfill_redemption_request_ix(
        &boss, &boss, &user.pubkey(),
        &onyc_mint, &usdc_mint, 0,
        3_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_cancel_redemption_request_ix(
        &user.pubkey(), &user.pubkey(), &boss,
        &onyc_mint, &usdc_mint, 0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();

    let offer = read_redemption_offer(&svm, &onyc_mint, &usdc_mint);
    assert_eq!(offer.requested_redemptions, 0,
        "requested_redemptions should be zero after cancel of partial request");
}

// ---------------------------------------------------------------------------
// Validation errors
// ---------------------------------------------------------------------------

#[test]
fn test_partial_fulfill_rejects_zero_amount() {
    let (mut svm, payer, onyc_mint, usdc_mint, _, _, user) =
        setup_partial(0, 9_000_000_000);
    let boss = payer.pubkey();

    let ix = build_fulfill_redemption_request_ix(
        &boss, &boss, &user.pubkey(),
        &onyc_mint, &usdc_mint, 0,
        0,
    );
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "zero amount should be rejected");
}

#[test]
fn test_partial_fulfill_rejects_amount_exceeding_remaining() {
    let (mut svm, payer, onyc_mint, usdc_mint, _, _, user) =
        setup_partial(0, 9_000_000_000);
    let boss = payer.pubkey();

    let ix = build_fulfill_redemption_request_ix(
        &boss, &boss, &user.pubkey(),
        &onyc_mint, &usdc_mint, 0,
        9_000_000_001, // one more than total
    );
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "amount exceeding request should be rejected");
}

#[test]
fn test_partial_fulfill_rejects_overfill_after_first_partial() {
    let (mut svm, payer, onyc_mint, usdc_mint, _, _, user) =
        setup_partial(0, 9_000_000_000);
    let boss = payer.pubkey();

    // Fulfill 5 ONyc → 4 ONyc remaining
    let ix = build_fulfill_redemption_request_ix(
        &boss, &boss, &user.pubkey(),
        &onyc_mint, &usdc_mint, 0,
        5_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Try to fulfill 5 ONyc again (more than remaining 4)
    let ix = build_fulfill_redemption_request_ix(
        &boss, &boss, &user.pubkey(),
        &onyc_mint, &usdc_mint, 0,
        5_000_000_000,
    );
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "over-fill after partial should be rejected");
}
