mod common;

use common::*;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

#[test]
fn test_boss_can_close_state() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let state = read_state(&svm);
    assert_eq!(state.boss, boss);

    let ix = build_close_state_ix(&boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // State account should no longer be owned by program
    let (state_pda, _) = find_state_pda();
    let account = svm.get_account(&state_pda);
    // After close, account should either not exist or be owned by system program
    match account {
        Some(acc) => assert_ne!(
            acc.owner, PROGRAM_ID,
            "state should no longer be owned by program"
        ),
        None => {} // Account was fully removed
    }
}

#[test]
fn test_non_boss_cannot_close_state() {
    let (mut svm, payer, _) = setup_initialized();

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_close_state_ix(&non_boss.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(
        result.is_err(),
        "non-boss should not be able to close state"
    );

    // Verify state still exists
    let state = read_state(&svm);
    assert_eq!(state.boss, payer.pubkey());
}

#[test]
fn test_can_reinitialize_after_close() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_close_state_ix(&boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Re-initialize
    let new_onyc_mint = create_mint(&mut svm, &payer, 9, &boss);
    let ix = build_initialize_ix(&boss, &new_onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.boss, boss);
    assert_eq!(state.onyc_mint, new_onyc_mint);
}

#[test]
fn test_reinitialized_state_has_defaults() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_close_state_ix(&boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let new_onyc_mint = create_mint(&mut svm, &payer, 9, &boss);
    let ix = build_initialize_ix(&boss, &new_onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.boss, boss);
    assert_eq!(state.onyc_mint, new_onyc_mint);
    assert!(!state.is_killed);
    assert_eq!(state.proposed_boss, Pubkey::default());
    assert_eq!(state.max_supply, 0);
    assert_eq!(state.active_admins().len(), 0);
    assert_eq!(state.approver1, Pubkey::default());
    assert_eq!(state.approver2, Pubkey::default());
}
