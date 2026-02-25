mod common;

use common::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

#[test]
fn test_delete_existing_vector() {
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

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.active_vectors().len(), 1);

    advance_slot(&mut svm);

    let ix = build_delete_offer_vector_ix(&boss, &token_in, &token_out, current_time + 1000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.active_vectors().len(), 0);
}

#[test]
fn test_delete_fails_incorrect_mints() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();

    let wrong_mint = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_delete_offer_vector_ix(&boss, &token_in, &wrong_mint, 1000);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "wrong token_out should fail");

    advance_slot(&mut svm);

    let ix = build_delete_offer_vector_ix(&boss, &wrong_mint, &token_out, 1000);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "wrong token_in should fail");
}

#[test]
fn test_delete_fails_zero_start_time() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();

    let ix = build_delete_offer_vector_ix(&boss, &token_in, &token_out, 0);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "zero start_time should fail");
}

#[test]
fn test_delete_fails_nonexistent_vector() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    let ix = build_delete_offer_vector_ix(&boss, &token_in, &token_out, current_time + 999);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "nonexistent vector should fail");
}

#[test]
fn test_delete_specific_keeps_others() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    for i in 1..=3 {
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
    assert_eq!(offer.active_vectors().len(), 3);

    // Delete the middle vector
    let ix = build_delete_offer_vector_ix(&boss, &token_in, &token_out, current_time + 2000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    let active = offer.active_vectors();
    assert_eq!(active.len(), 2);

    let mut start_times: Vec<u64> = active.iter().map(|v| v.start_time).collect();
    start_times.sort();
    assert_eq!(start_times, vec![current_time + 1000, current_time + 3000]);

    let prices: Vec<u64> = active.iter().map(|v| v.base_price).collect();
    assert!(prices.contains(&1_000_000));
    assert!(prices.contains(&3_000_000));
    assert!(!prices.contains(&2_000_000));
}

#[test]
fn test_delete_rejects_non_boss() {
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

    let ix = build_delete_offer_vector_ix(
        &non_boss.pubkey(),
        &token_in,
        &token_out,
        current_time + 1000,
    );
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(
        result.is_err(),
        "non-boss should not be able to delete vector"
    );
}

#[test]
fn test_delete_rejects_past_vector() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

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

    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 200,
        2_000_000,
        7500,
        3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Advance past both vectors
    advance_clock_by(&mut svm, 250);

    // Try to delete past vector - should fail
    let ix = build_delete_offer_vector_ix(&boss, &token_in, &token_out, current_time + 100);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "past vector deletion should fail");
}

#[test]
fn test_delete_rejects_current_active_vector() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 10,
        1_000_000,
        5000,
        3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    advance_slot(&mut svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 20,
        2_000_000,
        7500,
        3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Advance past second vector
    advance_clock_by(&mut svm, 25);

    // Try to delete current active vector - should fail
    let ix = build_delete_offer_vector_ix(&boss, &token_in, &token_out, current_time + 20);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "current active vector deletion should fail"
    );
}

#[test]
fn test_delete_allows_future_vector() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    for i in 1..=3 {
        let ix = build_add_offer_vector_ix(
            &boss,
            &token_in,
            &token_out,
            None,
            current_time + (i * 10),
            i as u64 * 1_000_000,
            5000,
            3600,
        );
        send_tx(&mut svm, &[ix], &[&payer]).unwrap();
        advance_slot(&mut svm);
    }

    // Advance so vector 2 is active, vector 3 is future
    advance_clock_by(&mut svm, 25);

    let ix = build_delete_offer_vector_ix(&boss, &token_in, &token_out, current_time + 30);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    let active = offer.active_vectors();
    assert_eq!(active.len(), 2);

    let mut start_times: Vec<u64> = active.iter().map(|v| v.start_time).collect();
    start_times.sort();
    assert_eq!(start_times, vec![current_time + 10, current_time + 20]);
}

#[test]
fn test_delete_allows_when_all_future() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

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

    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 200,
        2_000_000,
        7500,
        3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    advance_slot(&mut svm);

    // Delete first future vector - should succeed
    let ix = build_delete_offer_vector_ix(&boss, &token_in, &token_out, current_time + 100);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    let active = offer.active_vectors();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].start_time, current_time + 200);
}
