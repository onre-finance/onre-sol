mod common;

use common::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

#[test]
fn test_add_vector_to_offer() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);
    let base_time = current_time + 3600;

    let ix = build_add_offer_vector_ix(
        &boss, &token_in, &token_out, None, base_time, 1_000_000, 5000, 3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    let v = &offer.vectors[0];
    assert_eq!(v.base_time, base_time);
    assert_eq!(v.start_time, base_time); // start_time = base_time when in future
    assert_eq!(v.base_price, 1_000_000);
    assert_eq!(v.apr, 5000);
    assert_eq!(v.price_fix_duration, 3600);
}

#[test]
fn test_start_time_current_when_base_time_past() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time - 3600,
        1_000_000,
        250_000,
        1000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    let v = &offer.vectors[0];
    assert_eq!(v.base_time, current_time - 3600);
    assert_eq!(v.start_time, current_time);
}

#[test]
fn test_add_multiple_vectors() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    for (i, (bt, bp, apr, pfd)) in [
        (current_time + 1000, 1_000_000u64, 5000u64, 3600u64),
        (current_time + 3000, 2_000_000, 7500, 1800),
        (current_time + 5000, 3_000_000, 1000, 900),
    ]
    .iter()
    .enumerate()
    {
        let ix =
            build_add_offer_vector_ix(&boss, &token_in, &token_out, None, *bt, *bp, *apr, *pfd);
        send_tx(&mut svm, &[ix], &[&payer]).unwrap();
        if i < 2 {
            advance_slot(&mut svm);
        }
    }

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.vectors[0].start_time, current_time + 1000);
    assert_eq!(offer.vectors[1].start_time, current_time + 3000);
    assert_eq!(offer.vectors[2].start_time, current_time + 5000);
}

#[test]
fn test_reject_wrong_token_mints() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    let wrong_mint = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_add_offer_vector_ix(
        &boss,
        &wrong_mint,
        &token_out,
        None,
        current_time + 1000,
        1_000_000,
        5000,
        3600,
    );
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "wrong token_in_mint should fail");

    advance_slot(&mut svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &wrong_mint,
        None,
        current_time + 1000,
        1_000_000,
        5000,
        3600,
    );
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "wrong token_out_mint should fail");
}

#[test]
fn test_reject_zero_base_time() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();

    let ix =
        build_add_offer_vector_ix(&boss, &token_in, &token_out, None, 0, 1_000_000, 5000, 3600);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "zero base_time should fail");
}

#[test]
fn test_reject_zero_base_price() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time,
        0,
        5000,
        3600,
    );
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "zero base_price should fail");
}

#[test]
fn test_reject_zero_price_fix_duration() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time,
        1_000_000,
        5000,
        0,
    );
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "zero price_fix_duration should fail");
}

#[test]
fn test_allow_zero_apr() {
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
        0,
        3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.vectors[0].apr, 0);
}

#[test]
fn test_reject_start_time_before_latest() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 2000,
        1_000_000,
        5000,
        3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    advance_slot(&mut svm);

    // Try to add vector with earlier base_time
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 1000,
        2_000_000,
        7500,
        1800,
    );
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "start_time before latest should fail");
}

#[test]
fn test_reject_duplicate_start_time() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);
    let base_time = current_time + 2000;

    let ix = build_add_offer_vector_ix(
        &boss, &token_in, &token_out, None, base_time, 1_000_000, 5000, 3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    advance_slot(&mut svm);

    let ix = build_add_offer_vector_ix(
        &boss, &token_in, &token_out, None, base_time, 2_000_000, 7500, 1800,
    );
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "duplicate start_time should fail");
}

#[test]
fn test_allow_base_time_before_last_vector_after_advance() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);
    let first_base_time = current_time + 1000;

    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        first_base_time,
        1_000_000,
        5000,
        3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Advance past the first vector
    advance_clock_by(&mut svm, 5000);
    let new_time = get_clock_time(&svm);

    // Add second vector with base_time before first vector's start_time
    // but start_time = max(base_time, current_time) = current_time (which is after first)
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        first_base_time - 1000,
        2_000_000,
        7500,
        1800,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    let active = offer.active_vectors();
    assert_eq!(active.len(), 2);
    assert_eq!(active[0].start_time, first_base_time);
    assert_eq!(active[1].start_time, new_time);
    assert_eq!(active[1].base_time, first_base_time - 1000);
}

#[test]
fn test_explicit_start_time_in_future() {
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

    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        Some(current_time + 3000),
        current_time + 2000,
        2_000_000,
        7500,
        1800,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    let active = offer.active_vectors();
    assert_eq!(active.len(), 2);
    assert_eq!(active[0].start_time, current_time + 1000);
    assert_eq!(active[1].start_time, current_time + 3000);
}

#[test]
fn test_reject_explicit_start_time_duplicate() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        Some(current_time + 2000),
        current_time + 2000,
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
        Some(current_time + 2000),
        current_time + 1000,
        2_000_000,
        7500,
        1800,
    );
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "duplicate explicit start_time should fail");
}

#[test]
fn test_reject_explicit_start_time_before_existing() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        Some(current_time + 3000),
        current_time + 3000,
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
        Some(current_time + 2000),
        current_time + 1000,
        2_000_000,
        7500,
        1800,
    );
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "explicit start_time before existing should fail"
    );
}

#[test]
fn test_reject_explicit_start_time_zero() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        Some(0),
        current_time + 1000,
        1_000_000,
        5000,
        3600,
    );
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "explicit start_time of 0 should fail");
}

#[test]
fn test_reject_ordering_violations() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        Some(current_time + 1000),
        current_time + 1000,
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
        Some(current_time + 2000),
        current_time + 2000,
        2_000_000,
        7500,
        3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    advance_slot(&mut svm);

    // Try to insert between two existing vectors
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        Some(current_time + 1500),
        current_time + 500,
        1_500_000,
        6000,
        3600,
    );
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "inserting between existing vectors should fail"
    );
}

#[test]
fn test_reject_max_vectors_exceeded() {
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
            1_000_000,
            5000,
            3600,
        );
        send_tx(&mut svm, &[ix], &[&payer]).unwrap();
        advance_slot(&mut svm);
    }

    // Try to add one more
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + ((MAX_VECTORS as u64 + 1) * 1000),
        1_000_000,
        5000,
        3600,
    );
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "exceeding max vectors should fail");
}

#[test]
fn test_large_values() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    let large_price = 999_999_999_999_999_999u64;
    let large_apr = 999_999u64;

    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 1000,
        large_price,
        large_apr,
        3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.vectors[0].base_price, large_price);
    assert_eq!(offer.vectors[0].apr, large_apr);
}

#[test]
fn test_minimum_valid_values() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    let ix = build_add_offer_vector_ix(&boss, &token_in, &token_out, None, 1, 1, 0, 1);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    let v = &offer.vectors[0];
    assert_eq!(v.base_time, 1);
    assert!(v.start_time >= current_time);
    assert_eq!(v.base_price, 1);
    assert_eq!(v.apr, 0);
    assert_eq!(v.price_fix_duration, 1);
}

#[test]
fn test_reject_non_boss() {
    let (mut svm, _payer, token_in, token_out) = setup_offer_with_mints();
    let current_time = get_clock_time(&svm);

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_add_offer_vector_ix(
        &non_boss.pubkey(),
        &token_in,
        &token_out,
        None,
        current_time + 1000,
        1_000_000,
        5000,
        3600,
    );
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(result.is_err(), "non-boss should not be able to add vector");
}

#[test]
fn test_vectors_on_multiple_offers() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    // Create second offer
    let t2_in = create_mint(&mut svm, &payer, 9, &boss);
    let t2_out = create_mint(&mut svm, &payer, 9, &boss);
    let ix = build_make_offer_ix(&boss, &t2_in, &t2_out, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    advance_slot(&mut svm);

    // Add vectors to both offers
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

    let ix = build_add_offer_vector_ix(
        &boss,
        &t2_in,
        &t2_out,
        None,
        current_time + 1000,
        3_000_000,
        7500,
        1800,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    advance_slot(&mut svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 3000,
        2_000_000,
        2500,
        3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer1 = read_offer(&svm, &token_in, &token_out);
    let offer2 = read_offer(&svm, &t2_in, &t2_out);

    assert_eq!(offer1.vectors[0].start_time, current_time + 1000);
    assert_eq!(offer1.vectors[1].start_time, current_time + 3000);
    assert_eq!(offer1.vectors[0].base_price, 1_000_000);
    assert_eq!(offer2.vectors[0].base_price, 3_000_000);
    assert_eq!(offer1.vectors[0].apr, 5000);
    assert_eq!(offer2.vectors[0].apr, 7500);
}

#[test]
fn test_clean_old_past_vectors() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    // Add 5 vectors all in the future
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

    // Advance so 4th vector is active
    advance_clock_by(&mut svm, 4500);

    // Add 6th vector to trigger cleanup
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        current_time + 6000,
        6_000_000,
        5000,
        3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    let active = offer.active_vectors();

    // Should have: vector 3 (prev active), vector 4 (active), vector 5 (future), vector 6 (new)
    assert_eq!(active.len(), 4);

    let mut start_times: Vec<u64> = active.iter().map(|v| v.start_time).collect();
    start_times.sort();
    assert_eq!(
        start_times,
        vec![
            current_time + 3000,
            current_time + 4000,
            current_time + 5000,
            current_time + 6000,
        ]
    );

    let prices: Vec<u64> = active.iter().map(|v| v.base_price).collect();
    assert!(prices.contains(&3_000_000));
    assert!(prices.contains(&4_000_000));
    assert!(prices.contains(&5_000_000));
    assert!(prices.contains(&6_000_000));
    assert!(!prices.contains(&1_000_000));
    assert!(!prices.contains(&2_000_000));
}

#[test]
fn test_add_when_full_with_cleanup() {
    let (mut svm, payer, token_in, token_out) = setup_offer_with_mints();
    let boss = payer.pubkey();
    let current_time = get_clock_time(&svm);

    // Fill all MAX_VECTORS slots
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

    let last_vector_start = current_time + (MAX_VECTORS as u64 * 1000);

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.active_vectors().len(), MAX_VECTORS);

    // Advance far into the future so all vectors are past
    advance_clock_by(&mut svm, 100_000);
    let new_time = get_clock_time(&svm);

    // Add new vector - should succeed because cleanup frees space
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in,
        &token_out,
        None,
        new_time - 1000,
        11_000_000,
        5000,
        3600,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    let active = offer.active_vectors();
    assert_eq!(active.len(), 2);
    assert_eq!(active[0].start_time, new_time);
    assert_eq!(active[0].base_time, new_time - 1000);
    assert_eq!(active[1].start_time, last_vector_start);
}
