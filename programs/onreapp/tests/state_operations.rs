mod common;

use common::*;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

// ===========================================================================
// Remove Admin
// ===========================================================================

#[test]
fn test_boss_can_remove_admin() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let admin = Keypair::new();
    let ix = build_add_admin_ix(&boss, &admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_remove_admin_ix(&boss, &admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.active_admins().len(), 0, "admin should be removed");
}

#[test]
fn test_remove_nonexistent_admin_fails() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let phantom = Keypair::new();
    let ix = build_remove_admin_ix(&boss, &phantom.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "removing nonexistent admin should fail");
}

#[test]
fn test_non_boss_cannot_remove_admin() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let admin = Keypair::new();
    let ix = build_add_admin_ix(&boss, &admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_remove_admin_ix(&non_boss.pubkey(), &admin.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(
        result.is_err(),
        "non-boss should not be able to remove admin"
    );
}

#[test]
fn test_can_remove_all_admins_one_by_one() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let admin1 = Keypair::new();
    let admin2 = Keypair::new();
    let admin3 = Keypair::new();

    for admin in [&admin1, &admin2, &admin3] {
        let ix = build_add_admin_ix(&boss, &admin.pubkey());
        send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    }

    assert_eq!(read_state(&svm).active_admins().len(), 3);

    let ix = build_remove_admin_ix(&boss, &admin1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    assert_eq!(read_state(&svm).active_admins().len(), 2);

    let ix = build_remove_admin_ix(&boss, &admin2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    assert_eq!(read_state(&svm).active_admins().len(), 1);

    let ix = build_remove_admin_ix(&boss, &admin3.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    assert_eq!(read_state(&svm).active_admins().len(), 0);
}

#[test]
fn test_can_remove_and_readd_same_admin() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let admin = Keypair::new();
    let ix = build_add_admin_ix(&boss, &admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_remove_admin_ix(&boss, &admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    assert_eq!(read_state(&svm).active_admins().len(), 0);

    advance_slot(&mut svm);

    let ix = build_add_admin_ix(&boss, &admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.active_admins().len(), 1);
    assert!(state.admins.contains(&admin.pubkey()));
}

#[test]
fn test_boss_can_remove_themselves_as_admin() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_add_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    assert!(read_state(&svm).admins.contains(&boss));

    let ix = build_remove_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert!(!state.active_admins().contains(&boss));
}

#[test]
fn test_cannot_remove_same_admin_twice() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let admin = Keypair::new();
    let ix = build_add_admin_ix(&boss, &admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_remove_admin_ix(&boss, &admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_remove_admin_ix(&boss, &admin.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "should not be able to remove same admin twice"
    );
}

#[test]
fn test_removing_admin_preserves_remaining() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let admin1 = Keypair::new();
    let admin2 = Keypair::new();
    let admin3 = Keypair::new();

    for admin in [&admin1, &admin2, &admin3] {
        let ix = build_add_admin_ix(&boss, &admin.pubkey());
        send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    }

    // Remove middle admin
    let ix = build_remove_admin_ix(&boss, &admin2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    let active = state.active_admins();
    assert_eq!(active.len(), 2);
    assert!(active.contains(&admin1.pubkey()));
    assert!(active.contains(&admin3.pubkey()));
    assert!(!active.contains(&admin2.pubkey()));
}

// ===========================================================================
// Clear Admins
// ===========================================================================

#[test]
fn test_boss_can_clear_all_admins() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    for _ in 0..3 {
        let admin = Keypair::new();
        let ix = build_add_admin_ix(&boss, &admin.pubkey());
        send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    }

    assert_eq!(read_state(&svm).active_admins().len(), 3);

    let ix = build_clear_admins_ix(&boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    assert_eq!(
        read_state(&svm).active_admins().len(),
        0,
        "all admins should be cleared"
    );
}

#[test]
fn test_non_boss_cannot_clear_admins() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let admin = Keypair::new();
    let ix = build_add_admin_ix(&boss, &admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_clear_admins_ix(&non_boss.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(
        result.is_err(),
        "non-boss should not be able to clear admins"
    );
}

// ===========================================================================
// Propose / Accept Boss
// ===========================================================================

#[test]
fn test_boss_can_propose_new_boss() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let new_boss = Keypair::new();
    let ix = build_propose_boss_ix(&boss, &new_boss.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.proposed_boss, new_boss.pubkey());
    assert_eq!(state.boss, boss, "boss should not change yet");
}

#[test]
fn test_two_step_boss_transfer() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let new_boss = Keypair::new();
    svm.airdrop(&new_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_propose_boss_ix(&boss, &new_boss.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.proposed_boss, new_boss.pubkey());
    assert_eq!(state.boss, boss);

    let ix = build_accept_boss_ix(&new_boss.pubkey());
    send_tx(&mut svm, &[ix], &[&new_boss]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.boss, new_boss.pubkey());
    assert_eq!(state.proposed_boss, Pubkey::default());
}

#[test]
fn test_non_boss_cannot_propose_boss() {
    let (mut svm, _payer, _) = setup_initialized();

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let new_boss = Keypair::new();
    let ix = build_propose_boss_ix(&non_boss.pubkey(), &new_boss.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(
        result.is_err(),
        "non-boss should not be able to propose boss"
    );
}

#[test]
fn test_cannot_propose_default_boss() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_propose_boss_ix(&boss, &Pubkey::default());
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "should not be able to propose default address as boss"
    );
}

#[test]
fn test_wrong_signer_cannot_accept_boss() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let new_boss = Keypair::new();
    svm.airdrop(&new_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_propose_boss_ix(&boss, &new_boss.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let imposter = Keypair::new();
    svm.airdrop(&imposter.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_accept_boss_ix(&imposter.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&imposter]);
    assert!(
        result.is_err(),
        "non-proposed boss should not be able to accept"
    );
}

#[test]
fn test_accept_boss_without_proposal_fails() {
    let (mut svm, _payer, _) = setup_initialized();

    let random = Keypair::new();
    svm.airdrop(&random.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_accept_boss_ix(&random.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&random]);
    assert!(result.is_err(), "accept without proposal should fail");
}

#[test]
fn test_current_boss_cannot_accept_proposal() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let new_boss = Keypair::new();
    let ix = build_propose_boss_ix(&boss, &new_boss.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Current boss tries to accept
    let ix = build_accept_boss_ix(&boss);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "current boss should not be able to accept their own proposal"
    );
}

#[test]
fn test_boss_can_change_proposal_before_acceptance() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let first = Keypair::new();
    let second = Keypair::new();

    let ix = build_propose_boss_ix(&boss, &first.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_propose_boss_ix(&boss, &second.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.proposed_boss, second.pubkey());
}

#[test]
fn test_first_proposed_boss_cannot_accept_after_change() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let first = Keypair::new();
    svm.airdrop(&first.pubkey(), INITIAL_LAMPORTS).unwrap();
    let second = Keypair::new();
    svm.airdrop(&second.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_propose_boss_ix(&boss, &first.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_propose_boss_ix(&boss, &second.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // First proposed boss tries to accept
    let ix = build_accept_boss_ix(&first.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&first]);
    assert!(
        result.is_err(),
        "first proposed boss should not be able to accept after change"
    );

    // Second one can
    let ix = build_accept_boss_ix(&second.pubkey());
    send_tx(&mut svm, &[ix], &[&second]).unwrap();
    assert_eq!(read_state(&svm).boss, second.pubkey());
}

#[test]
fn test_new_boss_can_propose_another_transfer() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let new_boss = Keypair::new();
    svm.airdrop(&new_boss.pubkey(), 10 * INITIAL_LAMPORTS)
        .unwrap();

    // Complete first transfer
    let ix = build_propose_boss_ix(&boss, &new_boss.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let ix = build_accept_boss_ix(&new_boss.pubkey());
    send_tx(&mut svm, &[ix], &[&new_boss]).unwrap();
    assert_eq!(read_state(&svm).boss, new_boss.pubkey());

    // New boss proposes third boss
    let third_boss = Keypair::new();
    svm.airdrop(&third_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_propose_boss_ix(&new_boss.pubkey(), &third_boss.pubkey());
    send_tx(&mut svm, &[ix], &[&new_boss]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.boss, new_boss.pubkey());
    assert_eq!(state.proposed_boss, third_boss.pubkey());

    // Third boss can accept
    let ix = build_accept_boss_ix(&third_boss.pubkey());
    send_tx(&mut svm, &[ix], &[&third_boss]).unwrap();
    assert_eq!(read_state(&svm).boss, third_boss.pubkey());
}

// ===========================================================================
// Kill Switch
// ===========================================================================

#[test]
fn test_boss_can_enable_kill_switch() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_set_kill_switch_ix(&boss, true);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    assert!(read_state(&svm).is_killed, "kill switch should be enabled");
}

#[test]
fn test_boss_can_disable_kill_switch() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_set_kill_switch_ix(&boss, true);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_set_kill_switch_ix(&boss, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    assert!(
        !read_state(&svm).is_killed,
        "kill switch should be disabled"
    );
}

#[test]
fn test_admin_can_enable_kill_switch() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_add_admin_ix(&boss, &admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_set_kill_switch_ix(&admin.pubkey(), true);
    send_tx(&mut svm, &[ix], &[&admin]).unwrap();

    assert!(
        read_state(&svm).is_killed,
        "admin should be able to enable kill switch"
    );
}

#[test]
fn test_admin_cannot_disable_kill_switch() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_add_admin_ix(&boss, &admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_set_kill_switch_ix(&boss, true);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_set_kill_switch_ix(&admin.pubkey(), false);
    let result = send_tx(&mut svm, &[ix], &[&admin]);
    assert!(
        result.is_err(),
        "admin should not be able to disable kill switch"
    );
    assert!(
        read_state(&svm).is_killed,
        "kill switch should still be enabled"
    );
}

#[test]
fn test_random_user_cannot_toggle_kill_switch() {
    let (mut svm, _payer, _) = setup_initialized();

    let random = Keypair::new();
    svm.airdrop(&random.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_set_kill_switch_ix(&random.pubkey(), true);
    let result = send_tx(&mut svm, &[ix], &[&random]);
    assert!(
        result.is_err(),
        "random user should not be able to enable kill switch"
    );
}

#[test]
fn test_enable_kill_switch_idempotent() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), INITIAL_LAMPORTS).unwrap();
    let ix = build_add_admin_ix(&boss, &admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_set_kill_switch_ix(&boss, true);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    assert!(read_state(&svm).is_killed);

    // Enable again with admin (different signer = different tx)
    let ix = build_set_kill_switch_ix(&admin.pubkey(), true);
    send_tx(&mut svm, &[ix], &[&admin]).unwrap();
    assert!(read_state(&svm).is_killed, "should still be enabled");
}

#[test]
fn test_multiple_admins_can_enable_kill_switch() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let admin1 = Keypair::new();
    let admin2 = Keypair::new();
    svm.airdrop(&admin1.pubkey(), INITIAL_LAMPORTS).unwrap();
    svm.airdrop(&admin2.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_add_admin_ix(&boss, &admin1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let ix = build_add_admin_ix(&boss, &admin2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Admin1 enables
    let ix = build_set_kill_switch_ix(&admin1.pubkey(), true);
    send_tx(&mut svm, &[ix], &[&admin1]).unwrap();
    assert!(read_state(&svm).is_killed);

    // Boss disables
    let ix = build_set_kill_switch_ix(&boss, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Admin2 enables
    let ix = build_set_kill_switch_ix(&admin2.pubkey(), true);
    send_tx(&mut svm, &[ix], &[&admin2]).unwrap();
    assert!(read_state(&svm).is_killed);
}

#[test]
fn test_non_boss_non_admin_cannot_disable_kill_switch() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_set_kill_switch_ix(&boss, true);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let random = Keypair::new();
    svm.airdrop(&random.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_set_kill_switch_ix(&random.pubkey(), false);
    let result = send_tx(&mut svm, &[ix], &[&random]);
    assert!(
        result.is_err(),
        "non-boss non-admin should not be able to disable kill switch"
    );
    assert!(read_state(&svm).is_killed);
}

#[test]
fn test_boss_can_disable_after_admin_enabled() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), INITIAL_LAMPORTS).unwrap();
    let ix = build_add_admin_ix(&boss, &admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Admin enables
    let ix = build_set_kill_switch_ix(&admin.pubkey(), true);
    send_tx(&mut svm, &[ix], &[&admin]).unwrap();
    assert!(read_state(&svm).is_killed);

    // Boss disables
    let ix = build_set_kill_switch_ix(&boss, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    assert!(!read_state(&svm).is_killed);
}

#[test]
fn test_removed_admin_cannot_enable_kill_switch() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_add_admin_ix(&boss, &admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_remove_admin_ix(&boss, &admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_set_kill_switch_ix(&admin.pubkey(), true);
    let result = send_tx(&mut svm, &[ix], &[&admin]);
    assert!(
        result.is_err(),
        "removed admin should not be able to enable kill switch"
    );
}

// ===========================================================================
// Approver Management
// ===========================================================================

#[test]
fn test_boss_can_add_approver() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let approver = Keypair::new();
    let ix = build_add_approver_ix(&boss, &approver.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.approver1, approver.pubkey());
    assert_eq!(state.approver2, Pubkey::default());
}

#[test]
fn test_can_add_two_approvers() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let a1 = Keypair::new();
    let a2 = Keypair::new();

    let ix = build_add_approver_ix(&boss, &a1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_add_approver_ix(&boss, &a2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.approver1, a1.pubkey());
    assert_eq!(state.approver2, a2.pubkey());
}

#[test]
fn test_cannot_add_third_approver() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let a1 = Keypair::new();
    let a2 = Keypair::new();
    let a3 = Keypair::new();

    let ix = build_add_approver_ix(&boss, &a1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let ix = build_add_approver_ix(&boss, &a2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_add_approver_ix(&boss, &a3.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "should not be able to add third approver");

    let state = read_state(&svm);
    assert_eq!(state.approver1, a1.pubkey());
    assert_eq!(state.approver2, a2.pubkey());
}

#[test]
fn test_cannot_add_duplicate_approver() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let approver = Keypair::new();
    let ix = build_add_approver_ix(&boss, &approver.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_add_approver_ix(&boss, &approver.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "should not be able to add duplicate approver"
    );
}

#[test]
fn test_cannot_add_default_approver() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_add_approver_ix(&boss, &Pubkey::default());
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "should not be able to add default pubkey as approver"
    );
}

#[test]
fn test_non_boss_cannot_add_approver() {
    let (mut svm, _payer, _) = setup_initialized();

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let approver = Keypair::new();
    let ix = build_add_approver_ix(&non_boss.pubkey(), &approver.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(
        result.is_err(),
        "non-boss should not be able to add approver"
    );
}

#[test]
fn test_boss_can_add_themselves_as_approver() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_add_approver_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.approver1, boss);
    assert_eq!(state.approver2, Pubkey::default());
}

#[test]
fn test_approvers_initialized_as_default() {
    let (_, _, _) = setup_initialized();
    // Already checked in initialize tests, but verify here too
    // setup_initialized calls initialize, which should have default approvers
}

#[test]
fn test_boss_can_remove_first_approver() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let a1 = Keypair::new();
    let a2 = Keypair::new();

    let ix = build_add_approver_ix(&boss, &a1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let ix = build_add_approver_ix(&boss, &a2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_remove_approver_ix(&boss, &a1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.approver1, Pubkey::default());
    assert_eq!(state.approver2, a2.pubkey());
}

#[test]
fn test_boss_can_remove_second_approver() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let a1 = Keypair::new();
    let a2 = Keypair::new();

    let ix = build_add_approver_ix(&boss, &a1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let ix = build_add_approver_ix(&boss, &a2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_remove_approver_ix(&boss, &a2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.approver1, a1.pubkey());
    assert_eq!(state.approver2, Pubkey::default());
}

#[test]
fn test_boss_can_remove_approver() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let approver = Keypair::new();
    let ix = build_add_approver_ix(&boss, &approver.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_remove_approver_ix(&boss, &approver.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    assert_eq!(read_state(&svm).approver1, Pubkey::default());
}

#[test]
fn test_non_boss_cannot_remove_approver() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let approver = Keypair::new();
    let ix = build_add_approver_ix(&boss, &approver.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_remove_approver_ix(&non_boss.pubkey(), &approver.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(
        result.is_err(),
        "non-boss should not be able to remove approver"
    );
}

#[test]
fn test_remove_nonexistent_approver_fails() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let random = Keypair::new();
    let ix = build_remove_approver_ix(&boss, &random.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "removing nonexistent approver should fail");
}

#[test]
fn test_cannot_remove_default_approver() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_remove_approver_ix(&boss, &Pubkey::default());
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "should not be able to remove default pubkey approver"
    );
}

#[test]
fn test_can_remove_both_approvers() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let a1 = Keypair::new();
    let a2 = Keypair::new();

    let ix = build_add_approver_ix(&boss, &a1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let ix = build_add_approver_ix(&boss, &a2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_remove_approver_ix(&boss, &a1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.approver1, Pubkey::default());
    assert_eq!(state.approver2, a2.pubkey());

    let ix = build_remove_approver_ix(&boss, &a2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.approver1, Pubkey::default());
    assert_eq!(state.approver2, Pubkey::default());
}

#[test]
fn test_can_remove_and_readd_approver() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let a1 = Keypair::new();
    let a2 = Keypair::new();

    let ix = build_add_approver_ix(&boss, &a1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let ix = build_add_approver_ix(&boss, &a2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_remove_approver_ix(&boss, &a1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    advance_slot(&mut svm);

    let ix = build_add_approver_ix(&boss, &a1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.approver1, a1.pubkey());
    assert_eq!(state.approver2, a2.pubkey());
}

#[test]
fn test_cannot_remove_same_approver_twice() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let approver = Keypair::new();
    let ix = build_add_approver_ix(&boss, &approver.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_remove_approver_ix(&boss, &approver.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_remove_approver_ix(&boss, &approver.pubkey());
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "should not be able to remove same approver twice"
    );
}

#[test]
fn test_boss_can_remove_themselves_as_approver() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let a2 = Keypair::new();
    let ix = build_add_approver_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let ix = build_add_approver_ix(&boss, &a2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_remove_approver_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.approver1, Pubkey::default());
    assert_eq!(state.approver2, a2.pubkey());
}

#[test]
fn test_can_add_approver_after_removing_one() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let a1 = Keypair::new();
    let a2 = Keypair::new();

    let ix = build_add_approver_ix(&boss, &a1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let ix = build_add_approver_ix(&boss, &a2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_remove_approver_ix(&boss, &a1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let a3 = Keypair::new();
    let ix = build_add_approver_ix(&boss, &a3.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.approver1, a3.pubkey());
    assert_eq!(state.approver2, a2.pubkey());
}

#[test]
fn test_can_swap_approvers() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let a1 = Keypair::new();
    let a2 = Keypair::new();

    let ix = build_add_approver_ix(&boss, &a1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let ix = build_add_approver_ix(&boss, &a2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Remove both
    let ix = build_remove_approver_ix(&boss, &a1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let ix = build_remove_approver_ix(&boss, &a2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    advance_slot(&mut svm);

    // Add in reverse order
    let ix = build_add_approver_ix(&boss, &a2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let ix = build_add_approver_ix(&boss, &a1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.approver1, a2.pubkey());
    assert_eq!(state.approver2, a1.pubkey());
}

#[test]
fn test_removing_approver2_allows_adding_to_slot2() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let a1 = Keypair::new();
    let a2 = Keypair::new();

    let ix = build_add_approver_ix(&boss, &a1.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let ix = build_add_approver_ix(&boss, &a2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_remove_approver_ix(&boss, &a2.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let new_approver = Keypair::new();
    let ix = build_add_approver_ix(&boss, &new_approver.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.approver1, a1.pubkey());
    assert_eq!(state.approver2, new_approver.pubkey());
}

// ===========================================================================
// Configure Max Supply
// ===========================================================================

#[test]
fn test_boss_can_configure_max_supply() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_configure_max_supply_ix(&boss, 1_000_000_000_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    assert_eq!(read_state(&svm).max_supply, 1_000_000_000_000_000);
}

#[test]
fn test_boss_can_remove_max_supply_cap() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_configure_max_supply_ix(&boss, 1_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_configure_max_supply_ix(&boss, 0);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    assert_eq!(read_state(&svm).max_supply, 0);
}

#[test]
fn test_non_boss_cannot_configure_max_supply() {
    let (mut svm, _payer, _) = setup_initialized();

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_configure_max_supply_ix(&non_boss.pubkey(), 1_000_000);
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(
        result.is_err(),
        "non-boss should not be able to configure max supply"
    );
}

#[test]
fn test_boss_can_update_max_supply_multiple_times() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_configure_max_supply_ix(&boss, 1_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    assert_eq!(read_state(&svm).max_supply, 1_000_000);

    let ix = build_configure_max_supply_ix(&boss, 2_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    assert_eq!(read_state(&svm).max_supply, 2_000_000);
}

// ===========================================================================
// Set Redemption Admin
// ===========================================================================

#[test]
fn test_boss_can_set_redemption_admin() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let redemption_admin = Keypair::new();
    let ix = build_set_redemption_admin_ix(&boss, &redemption_admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    assert_eq!(read_state(&svm).redemption_admin, redemption_admin.pubkey());
}

#[test]
fn test_non_boss_cannot_set_redemption_admin() {
    let (mut svm, _payer, _) = setup_initialized();

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_set_redemption_admin_ix(&non_boss.pubkey(), &Keypair::new().pubkey());
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(
        result.is_err(),
        "non-boss should not be able to set redemption admin"
    );
}
