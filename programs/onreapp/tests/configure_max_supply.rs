mod common;

use common::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

// ---------------------------------------------------------------------------
// Configure Max Supply Instruction
// ---------------------------------------------------------------------------

#[test]
fn test_boss_can_configure_max_supply() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_configure_max_supply_ix(&boss, 100_000_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.max_supply, 100_000_000_000);
}

#[test]
fn test_configure_max_supply_to_zero() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    // First set to non-zero
    let ix = build_configure_max_supply_ix(&boss, 100_000_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Then set to zero (unlimited)
    let ix = build_configure_max_supply_ix(&boss, 0);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.max_supply, 0);
}

#[test]
fn test_non_boss_cannot_configure_max_supply() {
    let (mut svm, _payer, _onyc_mint) = setup_initialized();

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_configure_max_supply_ix(&non_boss.pubkey(), 100_000_000_000);
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(result.is_err(), "non-boss should not configure max supply");
}

#[test]
fn test_configure_max_supply_multiple_updates() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_configure_max_supply_ix(&boss, 100_000_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_configure_max_supply_ix(&boss, 200_000_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let state = read_state(&svm);
    assert_eq!(state.max_supply, 200_000_000_000);
}

// ---------------------------------------------------------------------------
// Mint To Enforcement
// ---------------------------------------------------------------------------

#[test]
fn test_mint_to_cannot_exceed_max_supply() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    // Set max supply to 100 tokens (9 decimals)
    let ix = build_configure_max_supply_ix(&boss, 100_000_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Transfer mint authority to program
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Try to mint 200 tokens - should fail
    let ix = build_mint_to_ix(&boss, &onyc_mint, 200_000_000_000);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "should not mint beyond max supply");
}

#[test]
fn test_mint_to_can_mint_up_to_cap() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_configure_max_supply_ix(&boss, 100_000_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Mint exactly up to cap
    let ix = build_mint_to_ix(&boss, &onyc_mint, 100_000_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let boss_ata = get_associated_token_address(&boss, &onyc_mint);
    assert_eq!(get_token_balance(&svm, &boss_ata), 100_000_000_000);
}

#[test]
fn test_mint_to_multiple_mints_within_cap() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_configure_max_supply_ix(&boss, 100_000_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // First mint: 50 tokens
    let ix = build_mint_to_ix(&boss, &onyc_mint, 50_000_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Second mint: 50 tokens (total = 100 = cap)
    let ix = build_mint_to_ix(&boss, &onyc_mint, 50_000_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Third mint: 1 token (total = 101 > cap) - should fail
    let ix = build_mint_to_ix(&boss, &onyc_mint, 1_000_000_000);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "should not mint beyond cumulative cap");
}

#[test]
fn test_mint_to_no_limit_when_zero() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    // max_supply = 0 means no limit (default)
    let state = read_state(&svm);
    assert_eq!(state.max_supply, 0);

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Mint a large amount - should succeed with no limit
    let ix = build_mint_to_ix(&boss, &onyc_mint, 1_000_000_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let boss_ata = get_associated_token_address(&boss, &onyc_mint);
    assert_eq!(get_token_balance(&svm, &boss_ata), 1_000_000_000_000);
}

// ---------------------------------------------------------------------------
// Take Offer Enforcement
// ---------------------------------------------------------------------------

#[test]
fn test_take_offer_cannot_exceed_max_supply() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);
    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);

    // Set max supply to 1 token
    let ix = build_configure_max_supply_ix(&boss, 1_000_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Transfer mint authority to program so take_offer uses minting mode
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint);
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
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Vault accounts (empty since minting mode)
    let (vault_authority, _) = find_offer_vault_authority_pda();
    create_token_account(&mut svm, &usdc_mint, &vault_authority, 0);
    create_token_account(&mut svm, &onyc_mint, &vault_authority, 0);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 10_000_000_000);
    create_token_account(&mut svm, &usdc_mint, &boss, 0);

    // Try to take 2 USDC at price 1.0 = 2 ONyc - exceeds 1 token cap
    let ix = build_take_offer_ix(
        &user.pubkey(),
        &boss,
        &usdc_mint,
        &onyc_mint,
        2_000_000,
        None,
    );
    let result = send_tx(&mut svm, &[ix], &[&user]);
    assert!(
        result.is_err(),
        "should not mint beyond max supply via take_offer"
    );
}

#[test]
fn test_take_offer_can_take_within_cap() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);
    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_configure_max_supply_ix(&boss, 10_000_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint);
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
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (vault_authority, _) = find_offer_vault_authority_pda();
    create_token_account(&mut svm, &usdc_mint, &vault_authority, 0);
    create_token_account(&mut svm, &onyc_mint, &vault_authority, 0);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 10_000_000_000);
    create_token_account(&mut svm, &usdc_mint, &boss, 0);

    // 1 USDC at price 1.0 = 1 ONyc - within 10 token cap
    let ix = build_take_offer_ix(
        &user.pubkey(),
        &boss,
        &usdc_mint,
        &onyc_mint,
        1_000_000,
        None,
    );
    send_tx(&mut svm, &[ix], &[&user]).unwrap();

    let user_onyc = get_token_balance(
        &svm,
        &get_associated_token_address(&user.pubkey(), &onyc_mint),
    );
    assert_eq!(user_onyc, 1_000_000_000);
}

#[test]
fn test_take_offer_multiple_users_until_cap() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);
    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);

    // Cap at 2 tokens
    let ix = build_configure_max_supply_ix(&boss, 2_000_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint);
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
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (vault_authority, _) = find_offer_vault_authority_pda();
    create_token_account(&mut svm, &usdc_mint, &vault_authority, 0);
    create_token_account(&mut svm, &onyc_mint, &vault_authority, 0);
    create_token_account(&mut svm, &usdc_mint, &boss, 0);

    // User 1: take 1 ONyc
    let user1 = Keypair::new();
    svm.airdrop(&user1.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &usdc_mint, &user1.pubkey(), 10_000_000_000);

    let ix = build_take_offer_ix(
        &user1.pubkey(),
        &boss,
        &usdc_mint,
        &onyc_mint,
        1_000_000,
        None,
    );
    send_tx(&mut svm, &[ix], &[&user1]).unwrap();
    advance_slot(&mut svm);

    // User 2: take 1 ONyc (total = 2 = cap)
    let user2 = Keypair::new();
    svm.airdrop(&user2.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &usdc_mint, &user2.pubkey(), 10_000_000_000);

    let ix = build_take_offer_ix(
        &user2.pubkey(),
        &boss,
        &usdc_mint,
        &onyc_mint,
        1_000_000,
        None,
    );
    send_tx(&mut svm, &[ix], &[&user2]).unwrap();
    advance_slot(&mut svm);

    // User 3: take 1 ONyc (total = 3 > cap) - should fail
    let user3 = Keypair::new();
    svm.airdrop(&user3.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &usdc_mint, &user3.pubkey(), 10_000_000_000);

    let ix = build_take_offer_ix(
        &user3.pubkey(),
        &boss,
        &usdc_mint,
        &onyc_mint,
        1_000_000,
        None,
    );
    let result = send_tx(&mut svm, &[ix], &[&user3]);
    assert!(result.is_err(), "third user should hit max supply cap");
}

// ---------------------------------------------------------------------------
// Take Offer Permissionless Enforcement
// ---------------------------------------------------------------------------

#[test]
fn test_take_offer_permissionless_cannot_exceed_max_supply() {
    let (mut svm, payer, _original_onyc) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);
    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_configure_max_supply_ix(&boss, 1_000_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Permissionless offer, no approval
    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, false, true);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (vault_authority, _) = find_offer_vault_authority_pda();
    create_token_account(&mut svm, &usdc_mint, &vault_authority, 0);
    create_token_account(&mut svm, &onyc_mint, &vault_authority, 0);

    let (permissionless_authority, _) = find_permissionless_authority_pda();
    create_token_account(&mut svm, &usdc_mint, &permissionless_authority, 0);
    create_token_account(&mut svm, &onyc_mint, &permissionless_authority, 0);

    create_token_account(&mut svm, &usdc_mint, &boss, 0);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 10_000_000_000);
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 0);

    // 2 USDC at price 1.0 = 2 ONyc > 1 token cap
    let ix = build_take_offer_permissionless_ix(
        &user.pubkey(),
        &boss,
        &usdc_mint,
        &onyc_mint,
        2_000_000,
        None,
    );
    let result = send_tx(&mut svm, &[ix], &[&user]);
    assert!(
        result.is_err(),
        "should not mint beyond max supply via permissionless"
    );
}

#[test]
fn test_take_offer_permissionless_respects_cumulative_supply() {
    let (mut svm, payer, _original_onyc) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);
    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);

    // Cap at 2 tokens
    let ix = build_configure_max_supply_ix(&boss, 2_000_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, false, true);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (vault_authority, _) = find_offer_vault_authority_pda();
    create_token_account(&mut svm, &usdc_mint, &vault_authority, 0);
    create_token_account(&mut svm, &onyc_mint, &vault_authority, 0);

    let (permissionless_authority, _) = find_permissionless_authority_pda();
    create_token_account(&mut svm, &usdc_mint, &permissionless_authority, 0);
    create_token_account(&mut svm, &onyc_mint, &permissionless_authority, 0);

    create_token_account(&mut svm, &usdc_mint, &boss, 0);

    // User 1: take 1 ONyc
    let user1 = Keypair::new();
    svm.airdrop(&user1.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &usdc_mint, &user1.pubkey(), 10_000_000_000);
    create_token_account(&mut svm, &onyc_mint, &user1.pubkey(), 0);

    let ix = build_take_offer_permissionless_ix(
        &user1.pubkey(),
        &boss,
        &usdc_mint,
        &onyc_mint,
        1_000_000,
        None,
    );
    send_tx(&mut svm, &[ix], &[&user1]).unwrap();
    advance_slot(&mut svm);

    // User 2: take 1.5 ONyc (total = 2.5 > 2 cap) - should fail
    let user2 = Keypair::new();
    svm.airdrop(&user2.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &usdc_mint, &user2.pubkey(), 10_000_000_000);
    create_token_account(&mut svm, &onyc_mint, &user2.pubkey(), 0);

    let ix = build_take_offer_permissionless_ix(
        &user2.pubkey(),
        &boss,
        &usdc_mint,
        &onyc_mint,
        1_500_000,
        None,
    );
    let result = send_tx(&mut svm, &[ix], &[&user2]);
    assert!(result.is_err(), "cumulative supply should not exceed max");
}
