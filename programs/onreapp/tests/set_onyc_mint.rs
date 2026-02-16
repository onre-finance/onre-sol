mod common;

use common::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

#[test]
fn test_boss_can_update_onyc_mint() {
    let (mut svm, payer, _original_mint) = setup_initialized();
    let boss = payer.pubkey();

    let new_mint = create_mint(&mut svm, &payer, 9, &boss);
    let ix = build_set_onyc_mint_ix(&boss, &new_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.onyc_mint, new_mint);
    assert_eq!(state.boss, boss);
}

#[test]
fn test_non_boss_cannot_update_onyc_mint() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let new_mint = create_mint(&mut svm, &payer, 9, &boss);
    let ix = build_set_onyc_mint_ix(&non_boss.pubkey(), &new_mint);
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(result.is_err(), "non-boss should not be able to update onyc mint");
}

#[test]
fn test_set_onyc_mint_preserves_other_state() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    // Add an admin to verify it's preserved
    let admin = Keypair::new();
    let ix = build_add_admin_ix(&boss, &admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state_before = read_state(&svm);

    // Set new mint
    let new_mint = create_mint(&mut svm, &payer, 9, &boss);
    let ix = build_set_onyc_mint_ix(&boss, &new_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state_after = read_state(&svm);
    assert_eq!(state_after.boss, state_before.boss);
    assert_eq!(state_after.is_killed, state_before.is_killed);
    assert_eq!(state_after.active_admins(), state_before.active_admins());
    assert_eq!(state_after.onyc_mint, new_mint);
}

#[test]
fn test_multiple_onyc_mint_updates() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let mint1 = create_mint(&mut svm, &payer, 6, &boss);
    let mint2 = create_mint(&mut svm, &payer, 9, &boss);
    let mint3 = create_mint(&mut svm, &payer, 18, &boss);

    let ix = build_set_onyc_mint_ix(&boss, &mint1);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    assert_eq!(read_state(&svm).onyc_mint, mint1);

    let ix = build_set_onyc_mint_ix(&boss, &mint2);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    assert_eq!(read_state(&svm).onyc_mint, mint2);

    let ix = build_set_onyc_mint_ix(&boss, &mint3);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    assert_eq!(read_state(&svm).onyc_mint, mint3);
}

#[test]
fn test_set_onyc_mint_after_kill_switch_operations() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), INITIAL_LAMPORTS).unwrap();
    let ix = build_add_admin_ix(&boss, &admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Enable kill switch (admin)
    let ix = build_set_kill_switch_ix(&admin.pubkey(), true);
    send_tx(&mut svm, &[ix], &[&admin]).unwrap();

    // Disable kill switch (boss)
    let ix = build_set_kill_switch_ix(&boss, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Set new mint
    let new_mint = create_mint(&mut svm, &payer, 9, &boss);
    let ix = build_set_onyc_mint_ix(&boss, &new_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.onyc_mint, new_mint);
    assert!(!state.is_killed);
}

#[test]
fn test_set_onyc_mint_state_size_consistent() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let (state_pda, _) = find_state_pda();
    let state_size_before = svm.get_account(&state_pda).unwrap().data.len();

    let new_mint = create_mint(&mut svm, &payer, 9, &boss);
    let ix = build_set_onyc_mint_ix(&boss, &new_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state_size_after = svm.get_account(&state_pda).unwrap().data.len();
    assert_eq!(state_size_before, state_size_after, "state account size should not change");
}

// ===========================================================================
// Token-2022 Tests
// ===========================================================================

#[test]
fn test_set_onyc_mint_token2022() {
    let (mut svm, payer, _original_mint) = setup_initialized();
    let boss = payer.pubkey();

    let new_mint = create_mint_2022(&mut svm, &payer, 9, &boss);
    let ix = build_set_onyc_mint_ix(&boss, &new_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.onyc_mint, new_mint);
    assert_eq!(state.boss, boss);
}
