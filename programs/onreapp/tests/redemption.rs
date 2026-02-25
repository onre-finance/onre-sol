mod common;

use common::*;
use litesvm::LiteSVM;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

/// Helper: creates initialized state, an offer (usdc->onyc), and a redemption offer (onyc->usdc).
/// The redemption offer is the inverse: token_in=onyc, token_out=usdc.
fn setup_redemption() -> (
    LiteSVM,
    Keypair, // payer (boss)
    Pubkey,  // usdc_mint (token_out of original offer = token_in of redemption)
    Pubkey,  // onyc_mint
    Pubkey,  // redemption token_in_mint = onyc
    Pubkey,  // redemption token_out_mint = usdc
) {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);

    // Set redemption_admin = boss for simplicity
    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Create the standard offer: usdc -> onyc (original direction)
    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Add a vector so we have pricing
    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        None,
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    // Create redemption offer: onyc -> usdc (inverse direction)
    // token_in_mint = onyc, token_out_mint = usdc
    let ix = build_make_redemption_offer_ix(&boss, &onyc_mint, &usdc_mint, 500);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    (svm, payer, usdc_mint, onyc_mint, onyc_mint, usdc_mint)
}

// ===========================================================================
// make_redemption_offer tests
// ===========================================================================

#[test]
fn test_make_redemption_offer_success() {
    let (svm, _payer, _usdc, _onyc, redemption_tin, redemption_tout) = setup_redemption();

    let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
    assert_eq!(offer_data.fee_basis_points, 500);
    assert_eq!(offer_data.request_counter, 0);
    assert_eq!(offer_data.executed_redemptions, 0);
    assert_eq!(offer_data.requested_redemptions, 0);
    assert_eq!(offer_data.token_in_mint, redemption_tin);
    assert_eq!(offer_data.token_out_mint, redemption_tout);
}

#[test]
fn test_make_redemption_offer_rejects_non_authorized() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);

    // Set redemption_admin to someone specific (not the random user)
    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_offer_ix(&boss, &usdc_mint, &_onyc_mint, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let unauthorized = Keypair::new();
    svm.airdrop(&unauthorized.pubkey(), INITIAL_LAMPORTS)
        .unwrap();

    let ix = build_make_redemption_offer_ix(&unauthorized.pubkey(), &_onyc_mint, &usdc_mint, 500);
    let result = send_tx(&mut svm, &[ix], &[&unauthorized]);
    assert!(
        result.is_err(),
        "unauthorized user should not create redemption offer"
    );
}

#[test]
fn test_make_redemption_offer_rejects_fee_over_max() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);

    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_offer_ix(&boss, &usdc_mint, &_onyc_mint, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_redemption_offer_ix(&boss, &_onyc_mint, &usdc_mint, 1001);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "fee over 1000 bps should fail");
}

#[test]
fn test_make_redemption_offer_redemption_admin_can_create() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);
    let redemption_admin = Keypair::new();
    svm.airdrop(&redemption_admin.pubkey(), 10 * INITIAL_LAMPORTS)
        .unwrap();

    let ix = build_set_redemption_admin_ix(&boss, &redemption_admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_offer_ix(&boss, &usdc_mint, &_onyc_mint, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Redemption admin should be able to create
    let ix =
        build_make_redemption_offer_ix(&redemption_admin.pubkey(), &_onyc_mint, &usdc_mint, 500);
    send_tx(&mut svm, &[ix], &[&redemption_admin]).unwrap();
}

// ===========================================================================
// create_redemption_request tests
// ===========================================================================

#[test]
fn test_create_redemption_request_success() {
    let (mut svm, _payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();

    // Give user some onyc tokens
    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    // Need vault_token_account for redemption vault to exist
    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    let _vault_ata = create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        500_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();

    // Check redemption offer counter incremented
    let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
    assert_eq!(offer_data.request_counter, 1);
    assert_eq!(offer_data.requested_redemptions, 500_000_000);

    // User's tokens should be locked in vault
    let user_ata = get_associated_token_address(&user.pubkey(), &onyc_mint);
    assert_eq!(get_token_balance(&svm, &user_ata), 500_000_000);

    let vault_ata = get_associated_token_address(&redemption_vault_authority, &onyc_mint);
    assert_eq!(get_token_balance(&svm, &vault_ata), 500_000_000);
}

#[test]
fn test_create_multiple_redemption_requests() {
    let (mut svm, _payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();
    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 2_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    // First request
    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        500_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    // Second request
    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        300_000_000,
        1,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();

    let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
    assert_eq!(offer_data.request_counter, 2);
    assert_eq!(offer_data.requested_redemptions, 800_000_000);
}

#[test]
fn test_create_redemption_request_fails_kill_switch() {
    let (mut svm, payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    // Enable kill switch
    let ix = build_set_kill_switch_ix(&boss, true);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        500_000_000,
        0,
    );
    let result = send_tx(&mut svm, &[ix], &[&user]);
    assert!(result.is_err(), "should fail when kill switch is active");
}

// ===========================================================================
// cancel_redemption_request tests
// ===========================================================================

#[test]
fn test_cancel_redemption_request_by_redeemer() {
    let (mut svm, payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    // Create request
    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        500_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    // Cancel by redeemer (boss is redemption_admin in setup)
    let ix = build_cancel_redemption_request_ix(
        &user.pubkey(),
        &user.pubkey(),
        &boss,
        &redemption_tin,
        &redemption_tout,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();

    // Tokens should be returned
    let user_ata = get_associated_token_address(&user.pubkey(), &onyc_mint);
    assert_eq!(get_token_balance(&svm, &user_ata), 1_000_000_000);

    // Requested redemptions should be decremented
    let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
    assert_eq!(offer_data.requested_redemptions, 0);
}

#[test]
fn test_cancel_redemption_request_by_boss() {
    let (mut svm, payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        500_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    // Boss cancels (boss is also redemption_admin in this setup)
    let ix = build_cancel_redemption_request_ix(
        &boss,
        &user.pubkey(),
        &boss,
        &redemption_tin,
        &redemption_tout,
        0,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let user_ata = get_associated_token_address(&user.pubkey(), &onyc_mint);
    assert_eq!(get_token_balance(&svm, &user_ata), 1_000_000_000);
}

#[test]
fn test_cancel_redemption_request_rejects_unauthorized() {
    let (mut svm, payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        500_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    let unauthorized = Keypair::new();
    svm.airdrop(&unauthorized.pubkey(), INITIAL_LAMPORTS)
        .unwrap();

    let ix = build_cancel_redemption_request_ix(
        &unauthorized.pubkey(),
        &user.pubkey(),
        &boss,
        &redemption_tin,
        &redemption_tout,
        0,
    );
    let result = send_tx(&mut svm, &[ix], &[&unauthorized]);
    assert!(result.is_err(), "unauthorized user should not cancel");
}

#[test]
fn test_cancel_redemption_request_fails_kill_switch() {
    let (mut svm, payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        500_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    // Enable kill switch
    let ix = build_set_kill_switch_ix(&boss, true);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_cancel_redemption_request_ix(
        &user.pubkey(),
        &user.pubkey(),
        &boss,
        &redemption_tin,
        &redemption_tout,
        0,
    );
    let result = send_tx(&mut svm, &[ix], &[&user]);
    assert!(result.is_err(), "should fail when kill switch is active");
}

// ===========================================================================
// fulfill_redemption_request tests
// ===========================================================================

#[test]
fn test_fulfill_redemption_request_transfer_mode() {
    let (mut svm, payer, usdc_mint, onyc_mint, redemption_tin, redemption_tout) =
        setup_redemption();
    let boss = payer.pubkey();

    // User creates a redemption request for 1 ONyc
    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    // Setup redemption vault with onyc (for token_in)
    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);
    // Fund redemption vault with usdc (for token_out distribution)
    create_token_account(
        &mut svm,
        &usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );

    // Create boss token accounts for receiving
    create_token_account(&mut svm, &onyc_mint, &boss, 0);
    // User token_out account for receiving usdc
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 0);

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        1_000_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    // Fulfill the request (boss is redemption_admin)
    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        0,
        1_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Check that redemption offer statistics are updated
    let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
    assert_eq!(offer_data.executed_redemptions, 1_000_000_000);
    assert_eq!(offer_data.requested_redemptions, 0);

    // User should have received usdc (minus fee)
    // fee = 1_000_000_000 * 500 / 10000 = 50_000_000
    // net = 950_000_000
    // With price=1.0 and onyc=9dec, usdc=6dec:
    // token_out = net * price * 10^usdc_dec / (10^onyc_dec * 10^9)
    // = 950_000_000 * 1_000_000_000 * 1_000_000 / (1_000_000_000 * 1_000_000_000)
    // = 950_000_000 * 1_000_000 / 1_000_000_000
    // = 950_000
    let user_usdc_ata = get_associated_token_address(&user.pubkey(), &usdc_mint);
    assert_eq!(get_token_balance(&svm, &user_usdc_ata), 950_000);
}

#[test]
fn test_fulfill_redemption_request_rejects_non_admin() {
    let (mut svm, payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        500_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    let non_admin = Keypair::new();
    svm.airdrop(&non_admin.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_fulfill_redemption_request_ix(
        &non_admin.pubkey(),
        &boss,
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        0,
        500_000_000,
    );
    let result = send_tx(&mut svm, &[ix], &[&non_admin]);
    assert!(result.is_err(), "non-admin should not fulfill");
}

// ===========================================================================
// update_redemption_offer_fee tests
// ===========================================================================

#[test]
fn test_update_redemption_offer_fee_success() {
    let (mut svm, payer, _usdc, _onyc, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
    assert_eq!(offer_data.fee_basis_points, 500);

    let ix = build_update_redemption_offer_fee_ix(&boss, &redemption_tin, &redemption_tout, 800);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
    assert_eq!(offer_data.fee_basis_points, 800);
}

#[test]
fn test_update_redemption_offer_fee_to_zero() {
    let (mut svm, payer, _usdc, _onyc, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    let ix = build_update_redemption_offer_fee_ix(&boss, &redemption_tin, &redemption_tout, 0);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
    assert_eq!(offer_data.fee_basis_points, 0);
}

#[test]
fn test_update_redemption_offer_fee_rejects_over_max() {
    let (mut svm, payer, _usdc, _onyc, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    let ix = build_update_redemption_offer_fee_ix(&boss, &redemption_tin, &redemption_tout, 1001);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "fee over 1000 bps should fail");
}

#[test]
fn test_update_redemption_offer_fee_rejects_non_boss() {
    let (mut svm, _payer, _usdc, _onyc, redemption_tin, redemption_tout) = setup_redemption();

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_update_redemption_offer_fee_ix(
        &non_boss.pubkey(),
        &redemption_tin,
        &redemption_tout,
        800,
    );
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(result.is_err(), "non-boss should not update fee");
}

#[test]
fn test_update_redemption_offer_fee_rejects_same_fee() {
    let (mut svm, payer, _usdc, _onyc, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    // Current fee is 500, try to set 500 again
    let ix = build_update_redemption_offer_fee_ix(&boss, &redemption_tin, &redemption_tout, 500);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "setting same fee should fail (no-op)");
}

// ===========================================================================
// Additional make_redemption_offer tests
// ===========================================================================

#[test]
fn test_make_redemption_offer_rejects_duplicate() {
    let (mut svm, payer, _usdc, _onyc, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    // Try to create the same redemption offer again
    let ix = build_make_redemption_offer_ix(&boss, &redemption_tin, &redemption_tout, 300);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "duplicate redemption offer should fail");
}

// ===========================================================================
// Additional create_redemption_request tests
// ===========================================================================

#[test]
fn test_create_redemption_request_counter_increments() {
    let (mut svm, _payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 3_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    // Create 3 requests and verify counter increments
    for i in 0u64..3 {
        let ix = build_create_redemption_request_ix(
            &user.pubkey(),
            &redemption_tin,
            &redemption_tout,
            100_000_000,
            i,
        );
        send_tx(&mut svm, &[ix], &[&user]).unwrap();
        advance_slot(&mut svm);

        let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
        assert_eq!(offer_data.request_counter, i + 1);
    }
}

#[test]
fn test_create_redemption_request_unique_pdas() {
    let (mut svm, _payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 2_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    let (redemption_offer_pda, _) = find_redemption_offer_pda(&redemption_tin, &redemption_tout);

    // Create two requests, verify different PDAs
    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        100_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        100_000_000,
        1,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();

    let (pda0, _) = find_redemption_request_pda(&redemption_offer_pda, 0);
    let (pda1, _) = find_redemption_request_pda(&redemption_offer_pda, 1);
    assert_ne!(pda0, pda1, "different counters should give different PDAs");

    // Verify both accounts exist
    assert!(svm.get_account(&pda0).is_some());
    assert!(svm.get_account(&pda1).is_some());
}

#[test]
fn test_create_redemption_request_anyone_can_create() {
    let (mut svm, _payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();

    // Random user (not boss, not admin) can create a redemption request
    let random_user = Keypair::new();
    svm.airdrop(&random_user.pubkey(), 10 * INITIAL_LAMPORTS)
        .unwrap();
    create_token_account(&mut svm, &onyc_mint, &random_user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    let ix = build_create_redemption_request_ix(
        &random_user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        500_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&random_user]).unwrap();

    let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
    assert_eq!(offer_data.request_counter, 1);
    assert_eq!(offer_data.requested_redemptions, 500_000_000);
}

#[test]
fn test_create_redemption_request_kill_switch_deactivated_allows() {
    let (mut svm, payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    // Enable kill switch
    let ix = build_set_kill_switch_ix(&boss, true);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Should fail
    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        500_000_000,
        0,
    );
    assert!(send_tx(&mut svm, &[ix], &[&user]).is_err());
    advance_slot(&mut svm);

    // Disable kill switch
    let ix = build_set_kill_switch_ix(&boss, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Should succeed now
    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        500_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();

    let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
    assert_eq!(offer_data.request_counter, 1);
}

#[test]
fn test_create_redemption_request_same_redeemer_multiple() {
    let (mut svm, _payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 5_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    // Same user creates 3 requests
    for i in 0u64..3 {
        let ix = build_create_redemption_request_ix(
            &user.pubkey(),
            &redemption_tin,
            &redemption_tout,
            1_000_000_000,
            i,
        );
        send_tx(&mut svm, &[ix], &[&user]).unwrap();
        advance_slot(&mut svm);
    }

    let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
    assert_eq!(offer_data.request_counter, 3);
    assert_eq!(offer_data.requested_redemptions, 3_000_000_000);

    // User balance should have decreased
    let user_ata = get_associated_token_address(&user.pubkey(), &onyc_mint);
    assert_eq!(get_token_balance(&svm, &user_ata), 2_000_000_000);
}

// ===========================================================================
// Additional cancel_redemption_request tests
// ===========================================================================

#[test]
fn test_cancel_redemption_request_by_redemption_admin() {
    let (mut svm, payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    // Set a separate redemption_admin (distinct from boss)
    let redemption_admin = Keypair::new();
    svm.airdrop(&redemption_admin.pubkey(), 10 * INITIAL_LAMPORTS)
        .unwrap();

    let ix = build_set_redemption_admin_ix(&boss, &redemption_admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        500_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    // Redemption admin cancels
    let ix = build_cancel_redemption_request_ix(
        &redemption_admin.pubkey(),
        &user.pubkey(),
        &redemption_admin.pubkey(),
        &redemption_tin,
        &redemption_tout,
        0,
    );
    send_tx(&mut svm, &[ix], &[&redemption_admin]).unwrap();

    let user_ata = get_associated_token_address(&user.pubkey(), &onyc_mint);
    assert_eq!(get_token_balance(&svm, &user_ata), 1_000_000_000);
}

#[test]
fn test_cancel_redemption_request_one_while_others_active() {
    let (mut svm, payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 3_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    // Create 3 requests
    for i in 0u64..3 {
        let ix = build_create_redemption_request_ix(
            &user.pubkey(),
            &redemption_tin,
            &redemption_tout,
            500_000_000,
            i,
        );
        send_tx(&mut svm, &[ix], &[&user]).unwrap();
        advance_slot(&mut svm);
    }

    // Cancel only request #1 (middle one)
    let ix = build_cancel_redemption_request_ix(
        &user.pubkey(),
        &user.pubkey(),
        &boss,
        &redemption_tin,
        &redemption_tout,
        1,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();

    let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
    assert_eq!(offer_data.requested_redemptions, 1_000_000_000); // 2 remaining * 500M

    // User got back tokens from cancelled request
    let user_ata = get_associated_token_address(&user.pubkey(), &onyc_mint);
    assert_eq!(get_token_balance(&svm, &user_ata), 2_000_000_000); // 3B - 1.5B + 500M
}

#[test]
fn test_cancel_redemption_request_multiple_cancellations() {
    let (mut svm, payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 3_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    // Create 3 requests of 500M each
    for i in 0u64..3 {
        let ix = build_create_redemption_request_ix(
            &user.pubkey(),
            &redemption_tin,
            &redemption_tout,
            500_000_000,
            i,
        );
        send_tx(&mut svm, &[ix], &[&user]).unwrap();
        advance_slot(&mut svm);
    }

    // Cancel all 3
    for i in 0u64..3 {
        let ix = build_cancel_redemption_request_ix(
            &user.pubkey(),
            &user.pubkey(),
            &boss,
            &redemption_tin,
            &redemption_tout,
            i,
        );
        send_tx(&mut svm, &[ix], &[&user]).unwrap();
        advance_slot(&mut svm);
    }

    let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
    assert_eq!(offer_data.requested_redemptions, 0);

    let user_ata = get_associated_token_address(&user.pubkey(), &onyc_mint);
    assert_eq!(get_token_balance(&svm, &user_ata), 3_000_000_000);
}

#[test]
fn test_cancel_redemption_request_kill_switch_deactivated_allows() {
    let (mut svm, payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        500_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    // Enable kill switch
    let ix = build_set_kill_switch_ix(&boss, true);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Should fail with kill switch
    let ix = build_cancel_redemption_request_ix(
        &user.pubkey(),
        &user.pubkey(),
        &boss,
        &redemption_tin,
        &redemption_tout,
        0,
    );
    assert!(send_tx(&mut svm, &[ix], &[&user]).is_err());
    advance_slot(&mut svm);

    // Disable kill switch
    let ix = build_set_kill_switch_ix(&boss, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Should succeed now
    let ix = build_cancel_redemption_request_ix(
        &user.pubkey(),
        &user.pubkey(),
        &boss,
        &redemption_tin,
        &redemption_tout,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();

    let user_ata = get_associated_token_address(&user.pubkey(), &onyc_mint);
    assert_eq!(get_token_balance(&svm, &user_ata), 1_000_000_000);
}

// ===========================================================================
// Additional fulfill_redemption_request tests
// ===========================================================================

/// Helper to set up a fulfillable redemption request and return context
struct FulfillCtx {
    svm: LiteSVM,
    payer: Keypair,
    usdc_mint: Pubkey,
    onyc_mint: Pubkey,
    redemption_tin: Pubkey,
    redemption_tout: Pubkey,
    user: Keypair,
}

fn setup_fulfillable_request(fee_bps: u16, amount: u64) -> FulfillCtx {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);

    // Set redemption_admin = boss
    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Create offer: usdc -> onyc
    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Add a vector: base_price=1.0, apr=0, duration=86400
    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        None,
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    // Create redemption offer: onyc -> usdc
    let ix = build_make_redemption_offer_ix(&boss, &onyc_mint, &usdc_mint, fee_bps);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Setup user
    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), amount);

    // Setup vaults
    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);
    create_token_account(
        &mut svm,
        &usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );

    // Boss token accounts
    create_token_account(&mut svm, &onyc_mint, &boss, 0);
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 0);

    // Create request
    let ix = build_create_redemption_request_ix(&user.pubkey(), &onyc_mint, &usdc_mint, amount, 0);
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    FulfillCtx {
        svm,
        payer,
        usdc_mint,
        onyc_mint,
        redemption_tin: onyc_mint,
        redemption_tout: usdc_mint,
        user,
    }
}

#[test]
fn test_fulfill_redemption_request_updates_statistics() {
    let mut ctx = setup_fulfillable_request(500, 1_000_000_000);
    let boss = ctx.payer.pubkey();

    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &ctx.user.pubkey(),
        &ctx.redemption_tin,
        &ctx.redemption_tout,
        0,
        1_000_000_000,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let offer_data = read_redemption_offer(&ctx.svm, &ctx.redemption_tin, &ctx.redemption_tout);
    assert_eq!(offer_data.executed_redemptions, 1_000_000_000);
    assert_eq!(offer_data.requested_redemptions, 0);
}

#[test]
fn test_fulfill_redemption_request_accumulates_executed() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);

    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        None,
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    let ix = build_make_redemption_offer_ix(&boss, &onyc_mint, &usdc_mint, 0);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);
    create_token_account(
        &mut svm,
        &usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    create_token_account(&mut svm, &onyc_mint, &boss, 0);

    // Create and fulfill 3 requests
    for i in 0u64..3 {
        let user = Keypair::new();
        svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
        create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 500_000_000);
        create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 0);

        let ix = build_create_redemption_request_ix(
            &user.pubkey(),
            &onyc_mint,
            &usdc_mint,
            500_000_000,
            i,
        );
        send_tx(&mut svm, &[ix], &[&user]).unwrap();
        advance_slot(&mut svm);

        let ix = build_fulfill_redemption_request_ix(
            &boss,
            &boss,
            &user.pubkey(),
            &onyc_mint,
            &usdc_mint,
            i,
            500_000_000,
        );
        send_tx(&mut svm, &[ix], &[&payer]).unwrap();
        advance_slot(&mut svm);
    }

    let offer_data = read_redemption_offer(&svm, &onyc_mint, &usdc_mint);
    assert_eq!(offer_data.executed_redemptions, 1_500_000_000); // 3 * 500M
    assert_eq!(offer_data.requested_redemptions, 0);
}

#[test]
fn test_fulfill_redemption_request_zero_fee() {
    let mut ctx = setup_fulfillable_request(0, 1_000_000_000);
    let boss = ctx.payer.pubkey();

    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &ctx.user.pubkey(),
        &ctx.redemption_tin,
        &ctx.redemption_tout,
        0,
        1_000_000_000,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    // With price=1.0, zero fee, 1B onyc (9 dec) → 1M usdc (6 dec)
    // token_out = 1_000_000_000 * 1_000_000_000 * 1_000_000 / (1_000_000_000 * 1_000_000_000)
    // = 1_000_000
    let user_usdc_ata = get_associated_token_address(&ctx.user.pubkey(), &ctx.usdc_mint);
    assert_eq!(get_token_balance(&ctx.svm, &user_usdc_ata), 1_000_000);
}

#[test]
fn test_fulfill_redemption_request_kill_switch() {
    let mut ctx = setup_fulfillable_request(500, 1_000_000_000);
    let boss = ctx.payer.pubkey();

    // Enable kill switch
    let ix = build_set_kill_switch_ix(&boss, true);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    advance_slot(&mut ctx.svm);

    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &ctx.user.pubkey(),
        &ctx.redemption_tin,
        &ctx.redemption_tout,
        0,
        1_000_000_000,
    );
    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]);
    assert!(result.is_err(), "should fail when kill switch is active");
}

#[test]
fn test_fulfill_redemption_request_with_apr_growth() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);

    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // APR = 10% (10_000_000 in scale=6 where 1_000_000=1%), base_price=1.0
    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        None,
        current_time,
        1_000_000_000,
        10_000_000,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    let ix = build_make_redemption_offer_ix(&boss, &onyc_mint, &usdc_mint, 0);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);
    create_token_account(
        &mut svm,
        &usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    create_token_account(&mut svm, &onyc_mint, &boss, 0);
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 0);

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        1_000_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    // Advance to step 1: elapsed = 1 * 86400 = 86400, effective = 2 * 86400
    // Price grows with compound interest
    advance_clock_by(&mut svm, 86400);

    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        0,
        1_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Price should be > 1.0 due to APR growth, so user gets more usdc
    let user_usdc_ata = get_associated_token_address(&user.pubkey(), &usdc_mint);
    let usdc_received = get_token_balance(&svm, &user_usdc_ata);
    assert!(
        usdc_received > 1_000_000,
        "should receive more than base price: {}",
        usdc_received
    );
}

#[test]
fn test_fulfill_redemption_request_burn_and_mint() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);

    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        None,
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    let ix = build_make_redemption_offer_ix(&boss, &onyc_mint, &usdc_mint, 500);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Transfer mint authority of onyc to program's mint_authority PDA (for burn)
    let (mint_authority_pda, _) = find_mint_authority_pda();
    set_mint_authority(&mut svm, &onyc_mint, &mint_authority_pda);

    // Transfer mint authority of usdc to program's mint_authority PDA (for mint)
    set_mint_authority(&mut svm, &usdc_mint, &mint_authority_pda);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);
    // No need for usdc in vault when minting
    create_token_account(&mut svm, &usdc_mint, &redemption_vault_authority, 0);
    create_token_account(&mut svm, &onyc_mint, &boss, 0);
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 0);

    // Fix mint supply for onyc (so burn doesn't underflow)
    let mut mint_data = svm.get_account(&onyc_mint).unwrap();
    mint_data.data[36..44].copy_from_slice(&1_000_000_000u64.to_le_bytes());
    svm.set_account(onyc_mint, mint_data).unwrap();

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        1_000_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        0,
        1_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // With burn+mint mode:
    // fee=5%, net = 950_000_000 onyc
    // Net burned from vault, fee transferred to boss
    // token_out = 950_000_000 * 1.0 * 10^6 / (10^9 * 10^9) = 950_000
    // usdc minted to user
    let user_usdc_ata = get_associated_token_address(&user.pubkey(), &usdc_mint);
    assert_eq!(get_token_balance(&svm, &user_usdc_ata), 950_000);

    // Boss receives fee in onyc: 50_000_000
    let boss_onyc_ata = get_associated_token_address(&boss, &onyc_mint);
    assert_eq!(get_token_balance(&svm, &boss_onyc_ata), 50_000_000);
}

#[test]
fn test_fulfill_redemption_request_transfer_mode_fee_to_boss() {
    let mut ctx = setup_fulfillable_request(500, 1_000_000_000);
    let boss = ctx.payer.pubkey();

    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &ctx.user.pubkey(),
        &ctx.redemption_tin,
        &ctx.redemption_tout,
        0,
        1_000_000_000,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    // In transfer mode (no mint authority), boss gets full amount (net + fee)
    let boss_onyc_ata = get_associated_token_address(&boss, &ctx.onyc_mint);
    assert_eq!(get_token_balance(&ctx.svm, &boss_onyc_ata), 1_000_000_000);
}

#[test]
fn test_fulfill_redemption_request_different_price() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);

    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // base_price = 2.0 (2_000_000_000)
    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        None,
        current_time,
        2_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    let ix = build_make_redemption_offer_ix(&boss, &onyc_mint, &usdc_mint, 0);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);
    create_token_account(
        &mut svm,
        &usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    create_token_account(&mut svm, &onyc_mint, &boss, 0);
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 0);

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        1_000_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        0,
        1_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // token_out = 1_000_000_000 * 2_000_000_000 * 1_000_000 / (1_000_000_000 * 1_000_000_000)
    // = 2_000_000
    let user_usdc_ata = get_associated_token_address(&user.pubkey(), &usdc_mint);
    assert_eq!(get_token_balance(&svm, &user_usdc_ata), 2_000_000);
}

#[test]
fn test_fulfill_redemption_request_fee_with_apr() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);

    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // APR=10%, base_price=1.0
    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        None,
        current_time,
        1_000_000_000,
        10_000_000,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    // Fee = 5%
    let ix = build_make_redemption_offer_ix(&boss, &onyc_mint, &usdc_mint, 500);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);
    create_token_account(
        &mut svm,
        &usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    create_token_account(&mut svm, &onyc_mint, &boss, 0);
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 0);

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        1_000_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    // Advance time by 1 step (86400s)
    advance_clock_by(&mut svm, 86400);

    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        0,
        1_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Price > 1.0 due to APR, fee deducted from net
    // User should get more than base but less than without fee
    let user_usdc_ata = get_associated_token_address(&user.pubkey(), &usdc_mint);
    let usdc_received = get_token_balance(&svm, &user_usdc_ata);
    // With APR growth, price > 1.0, so user receives > 950_000 (base * 0.95)
    assert!(
        usdc_received > 950_000,
        "with APR growth: {}",
        usdc_received
    );
}

// ===========================================================================
// Additional update_redemption_offer_fee tests
// ===========================================================================

#[test]
fn test_update_redemption_offer_fee_to_max() {
    let (mut svm, payer, _usdc, _onyc, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    let ix = build_update_redemption_offer_fee_ix(&boss, &redemption_tin, &redemption_tout, 1000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
    assert_eq!(offer_data.fee_basis_points, 1000);
}

#[test]
fn test_update_redemption_offer_fee_multiple_times() {
    let (mut svm, payer, _usdc, _onyc, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    let fees = [100u16, 300, 0, 800, 1000];
    for fee in fees {
        let ix =
            build_update_redemption_offer_fee_ix(&boss, &redemption_tin, &redemption_tout, fee);
        send_tx(&mut svm, &[ix], &[&payer]).unwrap();
        advance_slot(&mut svm);

        let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
        assert_eq!(offer_data.fee_basis_points, fee);
    }
}

// ===========================================================================
// Token-2022 Tests
// ===========================================================================

/// Helper: sets up a Token-2022 redemption scenario with both mints as Token-2022
fn setup_redemption_token2022() -> (
    LiteSVM,
    Keypair, // payer (boss)
    Pubkey,  // usdc_mint (Token-2022)
    Pubkey,  // onyc_mint (Token-2022)
) {
    let (mut svm, payer, _original_onyc) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint_2022(&mut svm, &payer, 6, &boss);
    let onyc_mint = create_mint_2022(&mut svm, &payer, 9, &boss);

    // Set redemption_admin = boss
    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Create the standard offer: usdc -> onyc (with Token-2022 for usdc)
    let ix = build_make_offer_ix_with_programs(
        &boss,
        &usdc_mint,
        &onyc_mint,
        0,
        false,
        false,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Add a vector
    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        None,
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    // Create redemption offer: onyc -> usdc (inverse direction)
    let ix = build_make_redemption_offer_ix_with_programs(
        &boss,
        &onyc_mint,
        &usdc_mint,
        500,
        &TOKEN_2022_PROGRAM_ID,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    (svm, payer, usdc_mint, onyc_mint)
}

#[test]
fn test_fulfill_redemption_token2022_transfer_mode() {
    let (mut svm, payer, usdc_mint, onyc_mint) = setup_redemption_token2022();
    let boss = payer.pubkey();

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account_2022(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account_2022(&mut svm, &onyc_mint, &redemption_vault_authority, 0);
    create_token_account_2022(
        &mut svm,
        &usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );

    create_token_account_2022(&mut svm, &onyc_mint, &boss, 0);
    create_token_account_2022(&mut svm, &usdc_mint, &user.pubkey(), 0);

    let ix = build_create_redemption_request_ix_with_token_program(
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        1_000_000_000,
        0,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    let ix = build_fulfill_redemption_request_ix_with_programs(
        &boss,
        &boss,
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        0,
        &TOKEN_2022_PROGRAM_ID,
        &TOKEN_2022_PROGRAM_ID,
        1_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer_data = read_redemption_offer(&svm, &onyc_mint, &usdc_mint);
    assert_eq!(offer_data.executed_redemptions, 1_000_000_000);
    assert_eq!(offer_data.requested_redemptions, 0);

    // fee=5%, net=950_000_000 onyc, price=1.0
    // token_out = 950_000_000 * 1_000_000 / 1_000_000_000 = 950_000 usdc
    let user_usdc_ata = get_associated_token_address_2022(&user.pubkey(), &usdc_mint);
    assert_eq!(get_token_balance(&svm, &user_usdc_ata), 950_000);
}

#[test]
fn test_fulfill_redemption_token2022_burn_mint_mode() {
    let (mut svm, payer, usdc_mint, onyc_mint) = setup_redemption_token2022();
    let boss = payer.pubkey();

    // Transfer mint authority to program for both mints
    let (mint_authority_pda, _) = find_mint_authority_pda();
    set_mint_authority(&mut svm, &onyc_mint, &mint_authority_pda);
    set_mint_authority(&mut svm, &usdc_mint, &mint_authority_pda);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account_2022(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account_2022(&mut svm, &onyc_mint, &redemption_vault_authority, 0);
    create_token_account_2022(&mut svm, &usdc_mint, &redemption_vault_authority, 0);

    create_token_account_2022(&mut svm, &onyc_mint, &boss, 0);
    create_token_account_2022(&mut svm, &usdc_mint, &user.pubkey(), 0);

    // Fix mint supply for onyc (so burn doesn't underflow)
    let mut mint_data = svm.get_account(&onyc_mint).unwrap();
    mint_data.data[36..44].copy_from_slice(&1_000_000_000u64.to_le_bytes());
    svm.set_account(onyc_mint, mint_data).unwrap();

    let ix = build_create_redemption_request_ix_with_token_program(
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        1_000_000_000,
        0,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    let ix = build_fulfill_redemption_request_ix_with_programs(
        &boss,
        &boss,
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        0,
        &TOKEN_2022_PROGRAM_ID,
        &TOKEN_2022_PROGRAM_ID,
        1_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // fee=5%, net=950_000_000, burned from vault; fee=50_000_000 to boss
    // token_out = 950_000 usdc minted to user
    let user_usdc_ata = get_associated_token_address_2022(&user.pubkey(), &usdc_mint);
    assert_eq!(get_token_balance(&svm, &user_usdc_ata), 950_000);

    let boss_onyc_ata = get_associated_token_address_2022(&boss, &onyc_mint);
    assert_eq!(get_token_balance(&svm, &boss_onyc_ata), 50_000_000);
}

#[test]
fn test_fulfill_redemption_token2022_with_fee() {
    let (mut svm, payer, _original_onyc) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint_2022(&mut svm, &payer, 6, &boss);
    let onyc_mint = create_mint_2022(&mut svm, &payer, 9, &boss);

    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_offer_ix_with_programs(
        &boss,
        &usdc_mint,
        &onyc_mint,
        0,
        false,
        false,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        None,
        current_time,
        2_000_000_000,
        0,
        86400, // price = 2.0
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    // Redemption with 5% fee
    let ix = build_make_redemption_offer_ix_with_programs(
        &boss,
        &onyc_mint,
        &usdc_mint,
        500,
        &TOKEN_2022_PROGRAM_ID,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account_2022(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account_2022(&mut svm, &onyc_mint, &redemption_vault_authority, 0);
    create_token_account_2022(
        &mut svm,
        &usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    create_token_account_2022(&mut svm, &onyc_mint, &boss, 0);
    create_token_account_2022(&mut svm, &usdc_mint, &user.pubkey(), 0);

    let ix = build_create_redemption_request_ix_with_token_program(
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        1_000_000_000,
        0,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    let ix = build_fulfill_redemption_request_ix_with_programs(
        &boss,
        &boss,
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        0,
        &TOKEN_2022_PROGRAM_ID,
        &TOKEN_2022_PROGRAM_ID,
        1_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // fee=5%, net=950_000_000 onyc, price=2.0
    // token_out = 950_000_000 * 2_000_000_000 * 1_000_000 / (1_000_000_000 * 1_000_000_000)
    // = 1_900_000
    let user_usdc_ata = get_associated_token_address_2022(&user.pubkey(), &usdc_mint);
    assert_eq!(get_token_balance(&svm, &user_usdc_ata), 1_900_000);
}

// ===========================================================================
// Additional make_redemption_offer tests (matching TS coverage)
// ===========================================================================

#[test]
fn test_make_redemption_offer_multiple_pairs() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);
    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);
    let other_mint = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Create offers
    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_offer_ix(&boss, &usdc_mint, &other_mint, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Create two different redemption offers
    let ix = build_make_redemption_offer_ix(&boss, &onyc_mint, &usdc_mint, 500);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_redemption_offer_ix(&boss, &other_mint, &usdc_mint, 300);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer1 = read_redemption_offer(&svm, &onyc_mint, &usdc_mint);
    assert_eq!(offer1.fee_basis_points, 500);

    let offer2 = read_redemption_offer(&svm, &other_mint, &usdc_mint);
    assert_eq!(offer2.fee_basis_points, 300);
}

#[test]
fn test_make_redemption_offer_token2022() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint_2022(&mut svm, &payer, 6, &boss);
    let onyc_mint = create_mint_2022(&mut svm, &payer, 9, &boss);

    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_offer_ix_with_programs(
        &boss,
        &usdc_mint,
        &onyc_mint,
        0,
        false,
        false,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Add vector for pricing
    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        None,
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    let ix = build_make_redemption_offer_ix_with_programs(
        &boss,
        &onyc_mint,
        &usdc_mint,
        500,
        &TOKEN_2022_PROGRAM_ID,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_redemption_offer(&svm, &onyc_mint, &usdc_mint);
    assert_eq!(offer.fee_basis_points, 500);
    assert_eq!(offer.token_in_mint, onyc_mint);
    assert_eq!(offer.token_out_mint, usdc_mint);
}

// ===========================================================================
// Additional fulfill_redemption_request tests (matching TS coverage)
// ===========================================================================

#[test]
fn test_fulfill_redemption_request_rejects_already_fulfilled() {
    let mut ctx = setup_fulfillable_request(500, 1_000_000_000);
    let boss = ctx.payer.pubkey();

    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &ctx.user.pubkey(),
        &ctx.redemption_tin,
        &ctx.redemption_tout,
        0,
        1_000_000_000,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    advance_slot(&mut ctx.svm);

    // Try to fulfill again
    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &ctx.user.pubkey(),
        &ctx.redemption_tin,
        &ctx.redemption_tout,
        0,
        1_000_000_000,
    );
    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]);
    assert!(result.is_err(), "should reject already fulfilled request");
}

#[test]
fn test_fulfill_redemption_request_fails_no_active_vector() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);

    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Add vector then delete it
    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        None,
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    let ix = build_make_redemption_offer_ix(&boss, &onyc_mint, &usdc_mint, 0);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);
    create_token_account(
        &mut svm,
        &usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    create_token_account(&mut svm, &onyc_mint, &boss, 0);
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 0);

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        1_000_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    // Delete all vectors
    let ix = build_delete_all_offer_vectors_ix(&boss, &usdc_mint, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Try to fulfill without active vector
    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        0,
        1_000_000_000,
    );
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "should fail when no active vector exists");
}

#[test]
fn test_fulfill_redemption_request_price_1_003() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);

    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Price = 1.003
    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        None,
        current_time,
        1_003_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    let ix = build_make_redemption_offer_ix(&boss, &onyc_mint, &usdc_mint, 0);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);
    create_token_account(
        &mut svm,
        &usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    create_token_account(&mut svm, &onyc_mint, &boss, 0);
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 0);

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        1_000_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        0,
        1_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // token_out = 1_000_000_000 * 1_003_000_000 * 1_000_000 / (1_000_000_000 * 1_000_000_000) = 1_003_000
    let user_usdc_ata = get_associated_token_address(&user.pubkey(), &usdc_mint);
    assert_eq!(get_token_balance(&svm, &user_usdc_ata), 1_003_000);
}

#[test]
fn test_fulfill_redemption_request_price_0_5() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);

    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Price = 0.5
    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        None,
        current_time,
        500_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    let ix = build_make_redemption_offer_ix(&boss, &onyc_mint, &usdc_mint, 0);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);
    create_token_account(
        &mut svm,
        &usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    create_token_account(&mut svm, &onyc_mint, &boss, 0);
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 0);

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        1_000_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        0,
        1_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // token_out = 1_000_000_000 * 500_000_000 * 1_000_000 / (1_000_000_000 * 1_000_000_000) = 500_000
    let user_usdc_ata = get_associated_token_address(&user.pubkey(), &usdc_mint);
    assert_eq!(get_token_balance(&svm, &user_usdc_ata), 500_000);
}

#[test]
fn test_fulfill_redemption_request_price_pi() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);

    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Price = 3.141592653 (pi approximation)
    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        None,
        current_time,
        3_141_592_653,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    let ix = build_make_redemption_offer_ix(&boss, &onyc_mint, &usdc_mint, 0);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);
    create_token_account(
        &mut svm,
        &usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    create_token_account(&mut svm, &onyc_mint, &boss, 0);
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 0);

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        1_000_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        0,
        1_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // token_out = 1_000_000_000 * 3_141_592_653 * 1_000_000 / (1_000_000_000 * 1_000_000_000) = 3_141_592
    let user_usdc_ata = get_associated_token_address(&user.pubkey(), &usdc_mint);
    assert_eq!(get_token_balance(&svm, &user_usdc_ata), 3_141_592);
}

#[test]
fn test_fulfill_redemption_request_very_small_amount() {
    let mut ctx = setup_fulfillable_request(0, 1_000); // very small: 1000 lamports of onyc
    let boss = ctx.payer.pubkey();

    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &ctx.user.pubkey(),
        &ctx.redemption_tin,
        &ctx.redemption_tout,
        0,
        1_000,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    // token_out = 1_000 * 1_000_000_000 / 1_000_000_000 * 1_000_000 / 1_000_000_000
    // With price 1.0, 1000 onyc lamports (9 dec) = 0.000001 usdc (6 dec) = 1 lamport
    let user_usdc_ata = get_associated_token_address(&ctx.user.pubkey(), &ctx.usdc_mint);
    let usdc_received = get_token_balance(&ctx.svm, &user_usdc_ata);
    // Very small amount rounds to 1 usdc lamport due to program rounding
    assert_eq!(
        usdc_received, 1,
        "very small amount rounds to 1 usdc lamport"
    );
}

#[test]
fn test_fulfill_redemption_request_price_0_123456789() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);

    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Price = 0.123456789
    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        None,
        current_time,
        123_456_789,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    let ix = build_make_redemption_offer_ix(&boss, &onyc_mint, &usdc_mint, 0);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);
    create_token_account(
        &mut svm,
        &usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    create_token_account(&mut svm, &onyc_mint, &boss, 0);
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 0);

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        1_000_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        0,
        1_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // token_out = 1_000_000_000 * 123_456_789 * 1_000_000 / (1_000_000_000 * 1_000_000_000) = 123_456
    let user_usdc_ata = get_associated_token_address(&user.pubkey(), &usdc_mint);
    assert_eq!(get_token_balance(&svm, &user_usdc_ata), 123_456);
}

// ===========================================================================
// Token-2022 fulfill tests with varying APR and fees (matching TS coverage)
// ===========================================================================

/// Helper for Token-2022 fulfill tests with specific APR, fee, and time period
fn fulfill_token2022_with_params(apr: u64, fee_bps: u16, advance_days: u64) {
    let (mut svm, payer, _original_onyc) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint_2022(&mut svm, &payer, 6, &boss);
    let onyc_mint = create_mint_2022(&mut svm, &payer, 9, &boss);

    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_offer_ix_with_programs(
        &boss,
        &usdc_mint,
        &onyc_mint,
        0,
        false,
        false,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        None,
        current_time,
        1_000_000_000,
        apr,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    let ix = build_make_redemption_offer_ix_with_programs(
        &boss,
        &onyc_mint,
        &usdc_mint,
        fee_bps,
        &TOKEN_2022_PROGRAM_ID,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account_2022(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account_2022(&mut svm, &onyc_mint, &redemption_vault_authority, 0);
    create_token_account_2022(
        &mut svm,
        &usdc_mint,
        &redemption_vault_authority,
        100_000_000_000,
    );
    create_token_account_2022(&mut svm, &onyc_mint, &boss, 0);
    create_token_account_2022(&mut svm, &usdc_mint, &user.pubkey(), 0);

    let ix = build_create_redemption_request_ix_with_token_program(
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        1_000_000_000,
        0,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    // Advance time
    if advance_days > 0 {
        advance_clock_by(&mut svm, advance_days * 86400);
    }

    let ix = build_fulfill_redemption_request_ix_with_programs(
        &boss,
        &boss,
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        0,
        &TOKEN_2022_PROGRAM_ID,
        &TOKEN_2022_PROGRAM_ID,
        1_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Verify fulfillment succeeded
    let offer_data = read_redemption_offer(&svm, &onyc_mint, &usdc_mint);
    assert_eq!(offer_data.executed_redemptions, 1_000_000_000);
    assert_eq!(offer_data.requested_redemptions, 0);

    // User should have received some USDC
    let user_usdc_ata = get_associated_token_address_2022(&user.pubkey(), &usdc_mint);
    let usdc_received = get_token_balance(&svm, &user_usdc_ata);
    assert!(usdc_received > 0, "user should receive some USDC");

    // With fee, user receives less than full price
    if fee_bps > 0 {
        // Net amount after fee
        let fee = 1_000_000_000u64 * fee_bps as u64 / 10_000;
        let _net = 1_000_000_000u64 - fee;
        // Boss should have received fee in onyc (transferred)
        let boss_onyc_ata = get_associated_token_address_2022(&boss, &onyc_mint);
        let boss_onyc = get_token_balance(&svm, &boss_onyc_ata);
        assert_eq!(
            boss_onyc, 1_000_000_000,
            "boss receives full onyc in transfer mode"
        );
    }
}

#[test]
fn test_fulfill_redemption_token2022_apr_5_fee_1_30days() {
    fulfill_token2022_with_params(
        50_000, // 5% APR
        100,    // 1% fee
        30,     // 30 days
    );
}

#[test]
fn test_fulfill_redemption_token2022_apr_15_fee_5_90days() {
    fulfill_token2022_with_params(
        150_000, // 15% APR
        500,     // 5% fee
        90,      // 90 days
    );
}

#[test]
fn test_fulfill_redemption_token2022_apr_25_fee_2_5_180days() {
    fulfill_token2022_with_params(
        250_000, // 25% APR
        250,     // 2.5% fee
        180,     // 180 days
    );
}

#[test]
fn test_fulfill_redemption_token2022_apr_10_fee_0_5_45days() {
    fulfill_token2022_with_params(
        100_000, // 10% APR
        50,      // 0.5% fee
        45,      // 45 days
    );
}

#[test]
fn test_fulfill_redemption_token2022_apr_30_fee_7_60days() {
    fulfill_token2022_with_params(
        300_000, // 30% APR
        700,     // 7% fee
        60,      // 60 days
    );
}

#[test]
fn test_fulfill_redemption_token2022_apr_8_fee_3_15days() {
    fulfill_token2022_with_params(
        80_000, // 8% APR
        300,    // 3% fee
        15,     // 15 days
    );
}

#[test]
fn test_fulfill_redemption_token2022_apr_12_fee_4_5_120days() {
    fulfill_token2022_with_params(
        120_000, // 12% APR
        450,     // 4.5% fee
        120,     // 120 days
    );
}

#[test]
fn test_fulfill_redemption_token2022_apr_18_fee_6_270days() {
    fulfill_token2022_with_params(
        180_000, // 18% APR
        600,     // 6% fee
        270,     // 270 days
    );
}

#[test]
fn test_fulfill_redemption_token2022_apr_22_fee_1_5_365days() {
    fulfill_token2022_with_params(
        220_000, // 22% APR
        150,     // 1.5% fee
        365,     // 365 days
    );
}

#[test]
fn test_fulfill_redemption_token2022_apr_35_fee_8_7days() {
    fulfill_token2022_with_params(
        350_000, // 35% APR
        800,     // 8% fee
        7,       // 7 days
    );
}

// ===========================================================================
// Additional update_redemption_offer_fee tests (matching TS coverage)
// ===========================================================================

#[test]
fn test_update_redemption_offer_fee_fractional_percentages() {
    let (mut svm, payer, _usdc, _onyc, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    // Test various fractional fee values
    let fees = [1u16, 10, 50, 125, 333, 999];
    for fee in fees {
        let ix =
            build_update_redemption_offer_fee_ix(&boss, &redemption_tin, &redemption_tout, fee);
        send_tx(&mut svm, &[ix], &[&payer]).unwrap();
        advance_slot(&mut svm);

        let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
        assert_eq!(
            offer_data.fee_basis_points, fee,
            "fee should be {} bps",
            fee
        );
    }
}

// ===========================================================================
// Tests matching TypeScript test suite coverage
// ===========================================================================

// --- make_redemption_offer ---

#[test]
fn test_make_redemption_offer_initializes_vault_token_in_account() {
    let (svm, _payer, _usdc, onyc_mint, redemption_tin, _redemption_tout) = setup_redemption();

    // The make_redemption_offer instruction should have created the vault token_in (onyc) ATA
    // under the redemption_vault_authority PDA via init_if_needed.
    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    let vault_token_in_ata =
        get_associated_token_address(&redemption_vault_authority, &redemption_tin);

    let account = svm.get_account(&vault_token_in_ata);
    assert!(
        account.is_some(),
        "vault token_in account should exist after make_redemption_offer"
    );

    // Verify the token account is for the correct mint (onyc) and owned by the vault authority
    let acc = account.unwrap();
    let mint_in_account = Pubkey::try_from(&acc.data[0..32]).unwrap();
    let owner_in_account = Pubkey::try_from(&acc.data[32..64]).unwrap();
    assert_eq!(
        mint_in_account, onyc_mint,
        "vault token_in mint should be onyc"
    );
    assert_eq!(
        owner_in_account, redemption_vault_authority,
        "vault token_in owner should be redemption_vault_authority"
    );
}

#[test]
fn test_make_redemption_offer_initializes_vault_token_out_account() {
    let (svm, _payer, usdc_mint, _onyc, _redemption_tin, redemption_tout) = setup_redemption();

    // The make_redemption_offer instruction should have created the vault token_out (usdc) ATA
    // under the redemption_vault_authority PDA via init_if_needed.
    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    let vault_token_out_ata =
        get_associated_token_address(&redemption_vault_authority, &redemption_tout);

    let account = svm.get_account(&vault_token_out_ata);
    assert!(
        account.is_some(),
        "vault token_out account should exist after make_redemption_offer"
    );

    // Verify the token account is for the correct mint (usdc) and owned by the vault authority
    let acc = account.unwrap();
    let mint_in_account = Pubkey::try_from(&acc.data[0..32]).unwrap();
    let owner_in_account = Pubkey::try_from(&acc.data[32..64]).unwrap();
    assert_eq!(
        mint_in_account, usdc_mint,
        "vault token_out mint should be usdc"
    );
    assert_eq!(
        owner_in_account, redemption_vault_authority,
        "vault token_out owner should be redemption_vault_authority"
    );
}

#[test]
fn test_make_redemption_offer_references_correct_offer() {
    let (svm, _payer, usdc_mint, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();

    // The redemption offer's `offer` field should match the PDA of the original offer.
    // Original offer is usdc -> onyc (token_in=usdc, token_out=onyc).
    // Redemption offer is onyc -> usdc, so the underlying offer is find_offer_pda(usdc, onyc).
    let (expected_offer_pda, _) = find_offer_pda(&usdc_mint, &onyc_mint);

    let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
    assert_eq!(
        offer_data.offer, expected_offer_pda,
        "redemption offer's `offer` field should reference the original offer PDA"
    );
}

// --- create_redemption_request ---

#[test]
fn test_create_redemption_request_updates_requested_redemptions() {
    let (mut svm, _payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 2_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    let request_amount: u64 = 750_000_000;
    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        request_amount,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();

    let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
    assert_eq!(
        offer_data.requested_redemptions, request_amount as u128,
        "requested_redemptions should equal the request amount"
    );
}

#[test]
fn test_create_redemption_request_locks_tokens_in_vault() {
    let (mut svm, _payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    let initial_balance: u64 = 1_000_000_000;
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), initial_balance);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    let request_amount: u64 = 600_000_000;
    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        request_amount,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();

    // User balance should decrease by the request amount
    let user_ata = get_associated_token_address(&user.pubkey(), &onyc_mint);
    assert_eq!(
        get_token_balance(&svm, &user_ata),
        initial_balance - request_amount,
        "user token balance should decrease by request amount"
    );

    // Vault balance should increase by the request amount
    let vault_ata = get_associated_token_address(&redemption_vault_authority, &onyc_mint);
    assert_eq!(
        get_token_balance(&svm, &vault_ata),
        request_amount,
        "vault token balance should increase by request amount"
    );
}

#[test]
fn test_create_redemption_request_redeemer_pays_rent() {
    let (mut svm, _payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();

    let user = Keypair::new();
    let airdrop_amount = 10 * INITIAL_LAMPORTS;
    svm.airdrop(&user.pubkey(), airdrop_amount).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    // Record SOL balance before creating the request
    let sol_before = svm.get_account(&user.pubkey()).unwrap().lamports;

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        500_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();

    // Record SOL balance after
    let sol_after = svm.get_account(&user.pubkey()).unwrap().lamports;

    // The redeemer's SOL balance should have decreased (paid rent for redemption request account + tx fee)
    assert!(
        sol_after < sol_before,
        "redeemer SOL should decrease after creating request: before={}, after={}",
        sol_before,
        sol_after
    );
}

// --- cancel_redemption_request ---

#[test]
fn test_cancel_redemption_request_decrements_requested_redemptions() {
    let (mut svm, payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 2_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    // Create two requests
    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        500_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        300_000_000,
        1,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    // Verify total requested = 800M
    let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
    assert_eq!(offer_data.requested_redemptions, 800_000_000);

    // Cancel request #0 (500M)
    let ix = build_cancel_redemption_request_ix(
        &user.pubkey(),
        &user.pubkey(),
        &boss,
        &redemption_tin,
        &redemption_tout,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();

    // requested_redemptions should be decremented by 500M -> 300M
    let offer_data = read_redemption_offer(&svm, &redemption_tin, &redemption_tout);
    assert_eq!(
        offer_data.requested_redemptions, 300_000_000,
        "requested_redemptions should be decremented after cancel"
    );
}

#[test]
fn test_cancel_redemption_request_returns_tokens_to_redeemer() {
    let (mut svm, payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    let initial_balance: u64 = 1_000_000_000;
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), initial_balance);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    let request_amount: u64 = 500_000_000;
    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        request_amount,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    // Verify tokens were locked
    let user_ata = get_associated_token_address(&user.pubkey(), &onyc_mint);
    assert_eq!(
        get_token_balance(&svm, &user_ata),
        initial_balance - request_amount
    );

    // Cancel the request
    let ix = build_cancel_redemption_request_ix(
        &user.pubkey(),
        &user.pubkey(),
        &boss,
        &redemption_tin,
        &redemption_tout,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();

    // Tokens should be fully restored
    assert_eq!(
        get_token_balance(&svm, &user_ata),
        initial_balance,
        "redeemer token balance should be fully restored after cancel"
    );
}

#[test]
fn test_cancel_redemption_request_closes_account() {
    let (mut svm, payer, _usdc, onyc_mint, redemption_tin, redemption_tout) = setup_redemption();
    let boss = payer.pubkey();

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);

    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &redemption_tin,
        &redemption_tout,
        500_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    // Verify the redemption request account exists
    let (redemption_offer_pda, _) = find_redemption_offer_pda(&redemption_tin, &redemption_tout);
    let (request_pda, _) = find_redemption_request_pda(&redemption_offer_pda, 0);
    assert!(
        svm.get_account(&request_pda).is_some(),
        "request account should exist before cancel"
    );

    // Cancel the request
    let ix = build_cancel_redemption_request_ix(
        &user.pubkey(),
        &user.pubkey(),
        &boss,
        &redemption_tin,
        &redemption_tout,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();

    // The redemption request account should be closed (None)
    let account = svm.get_account(&request_pda);
    assert!(
        account.is_none(),
        "redemption request account should be closed after cancel"
    );
}

// --- fulfill_redemption_request ---

#[test]
fn test_fulfill_redemption_request_decrements_requested_redemptions() {
    let mut ctx = setup_fulfillable_request(0, 1_000_000_000);
    let boss = ctx.payer.pubkey();

    // Verify requested_redemptions before fulfillment
    let offer_data = read_redemption_offer(&ctx.svm, &ctx.redemption_tin, &ctx.redemption_tout);
    assert_eq!(offer_data.requested_redemptions, 1_000_000_000);

    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &ctx.user.pubkey(),
        &ctx.redemption_tin,
        &ctx.redemption_tout,
        0,
        1_000_000_000,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    // After fulfilling, requested_redemptions should be decremented
    let offer_data = read_redemption_offer(&ctx.svm, &ctx.redemption_tin, &ctx.redemption_tout);
    assert_eq!(
        offer_data.requested_redemptions, 0,
        "requested_redemptions should be decremented by the request amount after fulfill"
    );
    assert_eq!(
        offer_data.executed_redemptions, 1_000_000_000,
        "executed_redemptions should be incremented by the request amount"
    );
}

#[test]
fn test_fulfill_redemption_request_different_decimals() {
    // Use token_in (onyc) with 9 decimals and token_out (usdc) with 6 decimals
    // to verify correct amount conversion across different decimal places.
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    // Create usdc with 6 decimals (onyc already has 9 from setup_initialized)
    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);

    // Set redemption_admin = boss
    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Create offer: usdc -> onyc (original direction)
    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Add a vector with price = 1.0 (1_000_000_000 in PRICE_DECIMALS=9)
    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        None,
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    // Create redemption offer: onyc -> usdc, 0% fee
    let ix = build_make_redemption_offer_ix(&boss, &onyc_mint, &usdc_mint, 0);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // User holds 1 ONyc token = 1_000_000_000 (9 decimals)
    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 1_000_000_000);

    // Setup vaults
    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);
    create_token_account(
        &mut svm,
        &usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );

    // Boss token account for receiving onyc
    create_token_account(&mut svm, &onyc_mint, &boss, 0);
    // User token_out account for receiving usdc
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 0);

    // Create redemption request for 1 ONyc (1_000_000_000 in 9 decimals)
    let ix = build_create_redemption_request_ix(
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        1_000_000_000,
        0,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();
    advance_slot(&mut svm);

    // Fulfill the request
    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &user.pubkey(),
        &onyc_mint,
        &usdc_mint,
        0,
        1_000_000_000,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // With price=1.0, 0% fee, 1 ONyc (9 dec) should convert to 1 USDC (6 dec)
    // token_out = 1_000_000_000 * 1_000_000_000 * 1_000_000 / (1_000_000_000 * 1_000_000_000)
    // = 1_000_000 (which is 1 USDC with 6 decimals)
    let user_usdc_ata = get_associated_token_address(&user.pubkey(), &usdc_mint);
    assert_eq!(
        get_token_balance(&svm, &user_usdc_ata),
        1_000_000,
        "1 ONyc (9 dec) at price 1.0 should give exactly 1 USDC (6 dec)"
    );
}
