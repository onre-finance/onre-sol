mod common;

use common::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

#[test]
fn test_boss_can_add_admin() {
    let (mut svm, payer) = setup();
    let boss = payer.pubkey();

    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_initialize_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let new_admin = Keypair::new();
    svm.airdrop(&new_admin.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_add_admin_ix(&boss, &new_admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert!(
        state.admins.contains(&new_admin.pubkey()),
        "admin should be in list"
    );

    let active_admins = state.active_admins();
    assert_eq!(active_admins.len(), 1, "should have 1 active admin");
    assert_eq!(active_admins[0], new_admin.pubkey());
}

#[test]
fn test_non_boss_cannot_add_admin() {
    let (mut svm, payer) = setup();
    let boss = payer.pubkey();

    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_initialize_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let new_admin = Keypair::new();

    let ix = build_add_admin_ix(&non_boss.pubkey(), &new_admin.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);

    assert!(result.is_err(), "non-boss should not be able to add admin");
}

#[test]
fn test_cannot_add_same_admin_twice() {
    let (mut svm, payer) = setup();
    let boss = payer.pubkey();

    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_initialize_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let new_admin = Keypair::new();
    svm.airdrop(&new_admin.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_add_admin_ix(&boss, &new_admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_add_admin_ix(&boss, &new_admin.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&payer]);

    assert!(
        result.is_err(),
        "should not be able to add same admin twice"
    );
}

#[test]
fn test_cannot_add_more_than_20_admins() {
    let (mut svm, payer) = setup();
    let boss = payer.pubkey();

    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_initialize_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let mut admins = Vec::new();
    for _ in 0..20 {
        let admin = Keypair::new();
        svm.airdrop(&admin.pubkey(), INITIAL_LAMPORTS).unwrap();

        let ix = build_add_admin_ix(&boss, &admin.pubkey());
        send_tx(&mut svm, &[ix], &[&payer]).unwrap();

        admins.push(admin);
    }

    let extra_admin = Keypair::new();
    svm.airdrop(&extra_admin.pubkey(), INITIAL_LAMPORTS)
        .unwrap();

    let ix = build_add_admin_ix(&boss, &extra_admin.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&payer]);

    assert!(result.is_err(), "should not be able to add 21st admin");

    let state = read_state(&svm);
    let active_admins = state.active_admins();
    assert_eq!(active_admins.len(), 20, "should have exactly 20 admins");
}

#[test]
fn test_can_add_multiple_different_admins() {
    let (mut svm, payer) = setup();
    let boss = payer.pubkey();

    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_initialize_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let admin1 = Keypair::new();
    let admin2 = Keypair::new();
    let admin3 = Keypair::new();

    svm.airdrop(&admin1.pubkey(), INITIAL_LAMPORTS).unwrap();
    svm.airdrop(&admin2.pubkey(), INITIAL_LAMPORTS).unwrap();
    svm.airdrop(&admin3.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_add_admin_ix(&boss, &admin1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_add_admin_ix(&boss, &admin2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_add_admin_ix(&boss, &admin3.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);

    assert!(state.admins.contains(&admin1.pubkey()));
    assert!(state.admins.contains(&admin2.pubkey()));
    assert!(state.admins.contains(&admin3.pubkey()));

    let active_admins = state.active_admins();
    assert_eq!(active_admins.len(), 3);
}

#[test]
fn test_boss_can_add_themselves_as_admin() {
    let (mut svm, payer) = setup();
    let boss = payer.pubkey();

    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_initialize_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_add_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert!(state.admins.contains(&boss));

    let active_admins = state.active_admins();
    assert_eq!(active_admins.len(), 1);
}
