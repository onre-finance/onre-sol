mod common;

use common::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

#[test]
fn test_delete_all_multiple_vectors() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    for i in 1..=5 {
        let ix = build_add_offer_vector_ix(
            &boss,
            &token_in,
            &token_out,
            None,
            current_time + (i * 1000),
            i as u64 * 1_000_000,
            5000,
            3600,
        );
        send_tx(&mut svm, &[ix], &[&payer]).unwrap();
        advance_slot(&mut svm);
    }

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.active_vectors().len(), 5);

    let ix = build_delete_all_offer_vectors_ix(&boss, &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.active_vectors().len(), 0);
}

#[test]
fn test_delete_all_succeeds_no_vectors() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.active_vectors().len(), 0);

    let ix = build_delete_all_offer_vectors_ix(&boss, &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.active_vectors().len(), 0);
}

#[test]
fn test_delete_all_past_active_and_future() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    // Vector 1: will become past
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 100,
        1_000_000,
        5000,
        3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Vector 2: will become active
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 200,
        2_000_000,
        5000,
        3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Vector 3: will remain future
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 500,
        3_000_000,
        5000,
        3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Advance to make first two past/active
    advance_clock_by(&mut svm, 250);

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.active_vectors().len(), 3);

    let ix = build_delete_all_offer_vectors_ix(&boss, &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.active_vectors().len(), 0);
}

#[test]
fn test_delete_all_max_vectors() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    for i in 1..=MAX_VECTORS {
        let ix = build_add_offer_vector_ix(
            &boss,
            &token_in,
            &token_out,
            None,
            current_time + (i as u64 * 1000),
            i as u64 * 1_000_000,
            5000,
            3600,
        );
        send_tx(&mut svm, &[ix], &[&payer]).unwrap();
        advance_slot(&mut svm);
    }

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.active_vectors().len(), MAX_VECTORS);

    let ix = build_delete_all_offer_vectors_ix(&boss, &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.active_vectors().len(), 0);
}

#[test]
fn test_delete_all_fails_incorrect_mints() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();

    let wrong_mint = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_delete_all_offer_vectors_ix(&boss, &token_in, &wrong_mint);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "wrong token_out should fail");

    advance_slot(&mut svm);

    let ix = build_delete_all_offer_vectors_ix(&boss, &wrong_mint, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "wrong token_in should fail");
}

#[test]
fn test_delete_all_rejects_non_boss() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 1000,
        1_000_000,
        5000,
        3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_delete_all_offer_vectors_ix(&non_boss.pubkey(), &token_in, &token_out);
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(
        result.is_err(),
        "non-boss should not be able to delete all vectors"
    );
}

#[test]
fn test_add_vectors_after_delete_all() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 1000,
        1_000_000,
        5000,
        3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    advance_slot(&mut svm);

    let ix = build_delete_all_offer_vectors_ix(&boss, &token_in, &token_out);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    advance_slot(&mut svm);

    // Add new vector after deleting all
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 2000,
        2_000_000,
        7500,
        1800,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    let active = offer.active_vectors();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].base_price, 2_000_000);
}
