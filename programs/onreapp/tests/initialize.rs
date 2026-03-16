mod common;

use common::*;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

#[test]
fn test_initialize_succeeds_with_upgrade_authority() {
    let (mut svm, payer) = setup();
    let boss = payer.pubkey();
    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_initialize_ix(&boss, &onyc_mint);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_ok(),
        "initialize should succeed: {:?}",
        result.err()
    );

    let state = read_state(&svm);
    assert_eq!(state.boss, boss, "boss should be set to payer");
    assert_eq!(state.onyc_mint, onyc_mint, "onyc_mint should be set");
    assert!(!state.is_killed, "kill switch should be off");
    assert_eq!(
        state.proposed_boss,
        Pubkey::default(),
        "proposed_boss should be default"
    );
    assert_eq!(
        state.approver1,
        Pubkey::default(),
        "approver1 should be default"
    );
    assert_eq!(
        state.approver2,
        Pubkey::default(),
        "approver2 should be default"
    );
    assert_eq!(state.max_supply, 0, "max_supply should be 0");
    assert_eq!(
        state.redemption_admin,
        Pubkey::default(),
        "redemption_admin should be default"
    );
    assert_eq!(state.active_admins().len(), 0, "no admins initially");
}

#[test]
fn test_initialize_fails_with_wrong_upgrade_authority() {
    let (mut svm, payer) = setup();

    // Create a different signer who is NOT the upgrade authority
    let wrong_boss = Keypair::new();
    svm.airdrop(&wrong_boss.pubkey(), 100 * INITIAL_LAMPORTS)
        .unwrap();

    let onyc_mint = create_mint(&mut svm, &payer, 9, &wrong_boss.pubkey());

    let ix = build_initialize_ix(&wrong_boss.pubkey(), &onyc_mint);
    let result = send_tx(&mut svm, &[ix], &[&wrong_boss]);
    assert!(
        result.is_err(),
        "initialize should fail when signer is not upgrade authority"
    );
}

#[test]
fn test_cannot_initialize_twice() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    // Try to initialize again
    let ix = build_initialize_ix(&boss, &onyc_mint);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "should not be able to initialize twice");
}
