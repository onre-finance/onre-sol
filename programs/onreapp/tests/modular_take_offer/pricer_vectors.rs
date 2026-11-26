//! Modular Pricer vector lifecycle: eviction shared with legacy offers and EVM,
//! the one-day step limit, and boss-only deletion.
use super::mint_multisig::{expect_error, send};
use super::*;

fn vector(start_time: u64, base_price: u64, price_fix_duration: u64) -> PricingVector {
    PricingVector {
        start_time,
        base_time: start_time,
        base_price,
        apr: 0,
        price_fix_duration,
    }
}

fn add_ix(f: &Fixture, boss: Pubkey, vector: PricingVector) -> Instruction {
    ix(
        accounts::AddPricerVector {
            pricer: price(f.mint, 0),
            state: find_state_pda().0,
            boss,
        },
        instruction::AddPricerVector { vector },
    )
}

fn add(
    f: &mut Fixture,
    vector: PricingVector,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let instruction = add_ix(f, f.boss.pubkey(), vector);
    let boss = f.boss.insecure_clone();
    send(&mut f.svm, &[instruction], &[&boss])
}

fn delete_accounts(f: &Fixture, boss: Pubkey) -> accounts::DeletePricerVector {
    accounts::DeletePricerVector {
        pricer: price(f.mint, 0),
        state: find_state_pda().0,
        boss,
    }
}

/// Stored vector start times in ascending order.
fn stored(f: &Fixture) -> Vec<u64> {
    let pricer: Pricer = read(&f.svm, price(f.mint, 0));
    let mut starts: Vec<u64> = pricer
        .vectors
        .iter()
        .map(|vector| vector.start_time)
        .filter(|start| *start != 0)
        .collect();
    starts.sort_unstable();
    starts
}

#[test]
fn adding_vectors_evicts_history_but_keeps_active_previous_and_future() {
    let mut f = Fixture::new(false, false);
    // Twelve successive immediately-active vectors never fill the ten slots:
    // each addition keeps only itself and the previously active vector.
    let mut history = Vec::new();
    for i in 0..12 {
        advance_clock_by(&mut f.svm, 1);
        let now = get_clock_time(&f.svm);
        add(&mut f, vector(now, 2_000_000_000 + i, 86_400)).unwrap();
        history.push(now);
        assert!(stored(&f).len() <= 3);
    }
    let [.., previous, active] = history[..] else {
        unreachable!()
    };
    assert_eq!(stored(&f), vec![previous, active]);

    // Future vectors are kept; once one activates, the next addition evicts
    // everything older than the vector active before it.
    let now = get_clock_time(&f.svm);
    for offset in [100, 200] {
        add(&mut f, vector(now + offset, 2_000_000_000, 86_400)).unwrap();
    }
    assert_eq!(stored(&f), vec![previous, active, now + 100, now + 200]);
    advance_clock_by(&mut f.svm, 150);
    add(&mut f, vector(now + 300, 2_000_000_000, 86_400)).unwrap();
    assert_eq!(stored(&f), vec![active, now + 100, now + 200, now + 300]);

    // Future vectors alone can still fill every slot.
    for i in 4..10 {
        add(&mut f, vector(now + 300 + 100 * i, 2_000_000_000, 86_400)).unwrap();
    }
    assert_eq!(stored(&f).len(), 10);
    expect_error(
        add(&mut f, vector(now + 10_000, 2_000_000_000, 86_400)),
        OnreError::TooManyVectors,
    );
}

#[test]
fn price_fix_duration_is_limited_to_one_day() {
    let mut f = Fixture::new(false, false);
    let start = get_clock_time(&f.svm) + 10;
    expect_error(
        add(&mut f, vector(start, 2_000_000_000, 86_401)),
        OnreError::InvalidPriceFixDuration,
    );
    add(&mut f, vector(start, 2_000_000_000, 86_400)).unwrap();
}

#[test]
fn apr_is_limited_to_one_hundred_percent() {
    let mut f = Fixture::new(false, false);
    let start = get_clock_time(&f.svm) + 10;
    let with_apr = |apr| PricingVector {
        apr,
        ..vector(start, 2_000_000_000, 86_400)
    };
    expect_error(
        add(&mut f, with_apr(1_000_001)),
        OnreError::PricerAprTooHigh,
    );
    add(&mut f, with_apr(1_000_000)).unwrap();
}

#[test]
fn boss_deletes_future_vectors_or_all_vectors() {
    let mut f = Fixture::new(false, false);
    let now = get_clock_time(&f.svm);
    let active = stored(&f)[0];
    add(&mut f, vector(now + 100, 3_000_000_000, 86_400)).unwrap();
    add(&mut f, vector(now + 200, 4_000_000_000, 86_400)).unwrap();

    let delete = |f: &Fixture, boss: Pubkey, start: u64| {
        ix(
            delete_accounts(f, boss),
            instruction::DeletePricerVector {
                vector_start_time: start,
            },
        )
    };
    let rogue = Keypair::new();
    f.svm.airdrop(&rogue.pubkey(), INITIAL_LAMPORTS).unwrap();
    let instruction = delete(&f, rogue.pubkey(), now + 100);
    expect_error(
        send(&mut f.svm, &[instruction], &[&rogue]),
        OnreError::InvalidBoss,
    );
    // Started vectors cannot be deleted individually; unknown ones are rejected.
    let instruction = delete(&f, f.boss.pubkey(), active);
    expect_error(
        send(&mut f.svm, &[instruction], &[&f.boss]),
        OnreError::StartTimeInPast,
    );
    let instruction = delete(&f, f.boss.pubkey(), now + 150);
    expect_error(
        send(&mut f.svm, &[instruction], &[&f.boss]),
        OnreError::VectorNotFound,
    );
    let instruction = delete(&f, f.boss.pubkey(), now + 100);
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    assert_eq!(stored(&f), vec![active, now + 200]);

    // Deleting everything stops pricing until a new vector is added.
    let instruction = ix(
        delete_accounts(&f, rogue.pubkey()),
        instruction::DeleteAllPricerVectors {},
    );
    expect_error(
        send(&mut f.svm, &[instruction], &[&rogue]),
        OnreError::InvalidBoss,
    );
    let instruction = ix(
        delete_accounts(&f, f.boss.pubkey()),
        instruction::DeleteAllPricerVectors {},
    );
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    assert!(stored(&f).is_empty());
    let instruction = f.take(2_000_000, 0);
    assert_error(f.send(instruction), OnreError::NoActiveVector);

    let now = get_clock_time(&f.svm);
    add(&mut f, vector(now, 2_000_000_000, 86_400)).unwrap();
    let instruction = f.take(2_000_000, 990_000_000);
    f.send(instruction).unwrap();
}
