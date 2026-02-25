mod common;

use common::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

// ---------------------------------------------------------------------------
// Transfer mint authority to program
// ---------------------------------------------------------------------------

#[test]
fn test_transfer_mint_authority_to_program_success() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let (mint_authority_pda, _) = find_mint_authority_pda();

    // Boss is currently the mint authority
    assert_eq!(get_mint_authority_pubkey(&svm, &onyc_mint), Some(boss));

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Now the program PDA should be the mint authority
    assert_eq!(
        get_mint_authority_pubkey(&svm, &onyc_mint),
        Some(mint_authority_pda)
    );
}

#[test]
fn test_transfer_mint_authority_to_program_rejects_non_boss() {
    let (mut svm, _payer, onyc_mint) = setup_initialized();

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_transfer_mint_authority_to_program_ix(&non_boss.pubkey(), &onyc_mint);
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(
        result.is_err(),
        "non-boss should not transfer mint authority"
    );
}

#[test]
fn test_transfer_mint_authority_to_program_fails_boss_not_authority() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    // Create a mint with a different authority
    let other = Keypair::new();
    let different_mint = create_mint(&mut svm, &payer, 9, &other.pubkey());

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &different_mint);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "boss not being mint authority should fail");
}

// ---------------------------------------------------------------------------
// Transfer mint authority to boss
// ---------------------------------------------------------------------------

#[test]
fn test_transfer_mint_authority_to_boss_success() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    // First transfer to program
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let (mint_authority_pda, _) = find_mint_authority_pda();
    assert_eq!(
        get_mint_authority_pubkey(&svm, &onyc_mint),
        Some(mint_authority_pda)
    );

    // Transfer back to boss
    let ix = build_transfer_mint_authority_to_boss_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    assert_eq!(get_mint_authority_pubkey(&svm, &onyc_mint), Some(boss));
}

#[test]
fn test_transfer_mint_authority_to_boss_rejects_non_boss() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_transfer_mint_authority_to_boss_ix(&non_boss.pubkey(), &onyc_mint);
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(
        result.is_err(),
        "non-boss should not transfer mint authority back"
    );
}

#[test]
fn test_transfer_mint_authority_to_boss_fails_pda_not_authority() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    // Create a new mint where boss is authority (never transferred to PDA)
    let new_mint = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_transfer_mint_authority_to_boss_ix(&boss, &new_mint);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "should fail when PDA is not the mint authority"
    );
}

// ---------------------------------------------------------------------------
// Multiple tokens
// ---------------------------------------------------------------------------

#[test]
fn test_multiple_tokens_independent_authority_transfer() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token2_mint = create_mint(&mut svm, &payer, 6, &boss);
    let (mint_authority_pda, _) = find_mint_authority_pda();

    // Transfer both to program
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &token2_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    // Both should be under program PDA
    assert_eq!(
        get_mint_authority_pubkey(&svm, &onyc_mint),
        Some(mint_authority_pda)
    );
    assert_eq!(
        get_mint_authority_pubkey(&svm, &token2_mint),
        Some(mint_authority_pda)
    );
}

// ---------------------------------------------------------------------------
// Mint To
// ---------------------------------------------------------------------------

#[test]
fn test_mint_to_success() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    // Transfer mint authority to program
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Mint tokens
    let ix = build_mint_to_ix(&boss, &onyc_mint, 1_000_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let boss_onyc_ata = get_associated_token_address(&boss, &onyc_mint);
    assert_eq!(get_token_balance(&svm, &boss_onyc_ata), 1_000_000_000);
    assert_eq!(get_mint_supply(&svm, &onyc_mint), 1_000_000_000);
}

#[test]
fn test_mint_to_rejects_non_boss() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_mint_to_ix(&non_boss.pubkey(), &onyc_mint, 1_000_000_000);
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(result.is_err(), "non-boss should not be able to mint");
}

#[test]
fn test_mint_to_fails_without_program_mint_authority() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    // Try to mint without transferring mint authority first
    let ix = build_mint_to_ix(&boss, &onyc_mint, 1_000_000_000);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "should fail without program mint authority"
    );
}

#[test]
fn test_mint_to_multiple_times_accumulates() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_mint_to_ix(&boss, &onyc_mint, 500_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_mint_to_ix(&boss, &onyc_mint, 300_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let boss_onyc_ata = get_associated_token_address(&boss, &onyc_mint);
    assert_eq!(get_token_balance(&svm, &boss_onyc_ata), 800_000_000);
    assert_eq!(get_mint_supply(&svm, &onyc_mint), 800_000_000);
}

#[test]
fn test_mint_to_creates_boss_token_account() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Boss token account doesn't exist yet
    let boss_onyc_ata = get_associated_token_address(&boss, &onyc_mint);
    assert!(svm.get_account(&boss_onyc_ata).is_none());

    // Mint should create the account
    let ix = build_mint_to_ix(&boss, &onyc_mint, 1_000_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    assert_eq!(get_token_balance(&svm, &boss_onyc_ata), 1_000_000_000);
}

#[test]
fn test_mint_to_works_with_updated_onyc_mint() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    // Create new onyc mint and update state
    let new_onyc_mint = create_mint(&mut svm, &payer, 9, &boss);
    let ix = build_set_onyc_mint_ix(&boss, &new_onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Transfer authority for new mint
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &new_onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Mint from new onyc mint
    let ix = build_mint_to_ix(&boss, &new_onyc_mint, 500_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let boss_new_ata = get_associated_token_address(&boss, &new_onyc_mint);
    assert_eq!(get_token_balance(&svm, &boss_new_ata), 500_000_000);
}

#[test]
fn test_mint_to_fails_wrong_mint() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    // Transfer authority for the real onyc mint
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Create a different mint that is NOT stored in state
    let wrong_mint = create_mint(&mut svm, &payer, 9, &boss);
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &wrong_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Try to mint from wrong mint (not the onyc_mint in state)
    let ix = build_mint_to_ix(&boss, &wrong_mint, 1_000_000_000);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "should fail when mint doesn't match onyc_mint in state"
    );
}

// ===========================================================================
// Token-2022 Tests
// ===========================================================================

#[test]
fn test_transfer_mint_authority_to_program_token2022() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token2022_mint = create_mint_2022(&mut svm, &payer, 9, &boss);

    // Update onyc_mint to the Token-2022 mint
    let ix = build_set_onyc_mint_ix(&boss, &token2022_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let (mint_authority_pda, _) = find_mint_authority_pda();
    assert_eq!(get_mint_authority_pubkey(&svm, &token2022_mint), Some(boss));

    let ix = build_transfer_mint_authority_to_program_ix_with_token_program(
        &boss,
        &token2022_mint,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    assert_eq!(
        get_mint_authority_pubkey(&svm, &token2022_mint),
        Some(mint_authority_pda)
    );
}

#[test]
fn test_transfer_mint_authority_round_trip_token2022() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token2022_mint = create_mint_2022(&mut svm, &payer, 9, &boss);

    let ix = build_set_onyc_mint_ix(&boss, &token2022_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Transfer to program
    let ix = build_transfer_mint_authority_to_program_ix_with_token_program(
        &boss,
        &token2022_mint,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let (mint_authority_pda, _) = find_mint_authority_pda();
    assert_eq!(
        get_mint_authority_pubkey(&svm, &token2022_mint),
        Some(mint_authority_pda)
    );

    // Transfer back to boss
    let ix = build_transfer_mint_authority_to_boss_ix_with_token_program(
        &boss,
        &token2022_mint,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    assert_eq!(get_mint_authority_pubkey(&svm, &token2022_mint), Some(boss));
}

#[test]
fn test_mint_to_token2022() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let token2022_mint = create_mint_2022(&mut svm, &payer, 9, &boss);

    let ix = build_set_onyc_mint_ix(&boss, &token2022_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Transfer mint authority to program
    let ix = build_transfer_mint_authority_to_program_ix_with_token_program(
        &boss,
        &token2022_mint,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Mint tokens
    let ix = build_mint_to_ix_with_token_program(
        &boss,
        &token2022_mint,
        1_000_000_000,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let boss_ata = get_associated_token_address_2022(&boss, &token2022_mint);
    assert_eq!(get_token_balance(&svm, &boss_ata), 1_000_000_000);
    assert_eq!(get_mint_supply(&svm, &token2022_mint), 1_000_000_000);
}
