mod common;

use common::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

fn setup_offer() -> (litesvm::LiteSVM, Keypair, solana_sdk::pubkey::Pubkey, solana_sdk::pubkey::Pubkey) {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(&boss, &token_in, &token_out, 500, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    (svm, payer, token_in, token_out)
}

#[test]
fn test_update_fee_success() {
    let (mut svm, payer, token_in, token_out) = setup_offer();
    let boss = payer.pubkey();

    let ix = build_update_offer_fee_ix(&boss, &token_in, &token_out, 1000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.fee_basis_points, 1000);
}

#[test]
fn test_update_fee_to_zero() {
    let (mut svm, payer, token_in, token_out) = setup_offer();
    let boss = payer.pubkey();

    let ix = build_update_offer_fee_ix(&boss, &token_in, &token_out, 0);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.fee_basis_points, 0);
}

#[test]
fn test_update_fee_to_max() {
    let (mut svm, payer, token_in, token_out) = setup_offer();
    let boss = payer.pubkey();

    let ix = build_update_offer_fee_ix(&boss, &token_in, &token_out, 10000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.fee_basis_points, 10000);
}

#[test]
fn test_update_fee_rejects_nonexistent_offer() {
    let (mut svm, payer, token_in, _token_out) = setup_offer();
    let boss = payer.pubkey();

    let wrong_out = create_mint(&mut svm, &payer, 9, &boss);
    let ix = build_update_offer_fee_ix(&boss, &token_in, &wrong_out, 1000);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "should fail with wrong token_out");

    advance_slot(&mut svm);

    let wrong_in = create_mint(&mut svm, &payer, 9, &boss);
    let ix = build_update_offer_fee_ix(&boss, &wrong_in, &_token_out, 1000);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "should fail with wrong token_in");
}

#[test]
fn test_update_fee_rejects_over_max() {
    let (mut svm, payer, token_in, token_out) = setup_offer();
    let boss = payer.pubkey();

    let ix = build_update_offer_fee_ix(&boss, &token_in, &token_out, 10001);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "fee over 10000 should fail");
}

#[test]
fn test_update_fee_rejects_non_boss() {
    let (mut svm, _payer, token_in, token_out) = setup_offer();

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_update_offer_fee_ix(&non_boss.pubkey(), &token_in, &token_out, 1000);
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(result.is_err(), "non-boss should not be able to update fee");
}

#[test]
fn test_multiple_fee_updates() {
    let (mut svm, payer, token_in, token_out) = setup_offer();
    let boss = payer.pubkey();

    let ix = build_update_offer_fee_ix(&boss, &token_in, &token_out, 750);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    assert_eq!(read_offer(&svm, &token_in, &token_out).fee_basis_points, 750);

    advance_slot(&mut svm);

    let ix = build_update_offer_fee_ix(&boss, &token_in, &token_out, 250);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    assert_eq!(read_offer(&svm, &token_in, &token_out).fee_basis_points, 250);
}

#[test]
fn test_update_fee_preserves_vectors() {
    let (mut svm, payer, token_in, token_out) = setup_offer();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    let ix = build_add_offer_vector_ix(
        &boss, &token_in, &token_out,
        None, current_time + 1000, 1_000_000, 5000, 3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    advance_slot(&mut svm);

    let ix = build_update_offer_fee_ix(&boss, &token_in, &token_out, 800);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.fee_basis_points, 800);

    let active = offer.active_vectors();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].start_time, current_time + 1000);
}
