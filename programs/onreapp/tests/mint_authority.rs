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

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint, &TOKEN_PROGRAM_ID);
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

    let ix = build_transfer_mint_authority_to_program_ix(
        &non_boss.pubkey(),
        &onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
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

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &different_mint, &TOKEN_PROGRAM_ID);
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
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let (mint_authority_pda, _) = find_mint_authority_pda();
    assert_eq!(
        get_mint_authority_pubkey(&svm, &onyc_mint),
        Some(mint_authority_pda)
    );

    // Transfer back to boss
    let ix = build_transfer_mint_authority_to_boss_ix(&boss, &onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    assert_eq!(get_mint_authority_pubkey(&svm, &onyc_mint), Some(boss));
}

#[test]
fn test_transfer_mint_authority_to_boss_rejects_non_boss() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix =
        build_transfer_mint_authority_to_boss_ix(&non_boss.pubkey(), &onyc_mint, &TOKEN_PROGRAM_ID);
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

    let ix = build_transfer_mint_authority_to_boss_ix(&boss, &new_mint, &TOKEN_PROGRAM_ID);
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
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &token2_mint, &TOKEN_PROGRAM_ID);
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
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Mint tokens
    let ix = build_mint_to_ix(&boss, &onyc_mint, 1_000_000_000, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let boss_onyc_ata = get_associated_token_address(&boss, &onyc_mint);
    assert_eq!(get_token_balance(&svm, &boss_onyc_ata), 1_000_000_000);
    assert_eq!(get_mint_supply(&svm, &onyc_mint), 1_000_000_000);
}

#[test]
fn test_mint_to_rejects_non_boss() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_mint_to_ix(
        &non_boss.pubkey(),
        &onyc_mint,
        1_000_000_000,
        &TOKEN_PROGRAM_ID,
    );
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(result.is_err(), "non-boss should not be able to mint");
}

#[test]
fn test_mint_to_fails_without_program_mint_authority() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    // Try to mint without transferring mint authority first
    let ix = build_mint_to_ix(&boss, &onyc_mint, 1_000_000_000, &TOKEN_PROGRAM_ID);
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

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_mint_to_ix(&boss, &onyc_mint, 500_000_000, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_mint_to_ix(&boss, &onyc_mint, 300_000_000, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let boss_onyc_ata = get_associated_token_address(&boss, &onyc_mint);
    assert_eq!(get_token_balance(&svm, &boss_onyc_ata), 800_000_000);
    assert_eq!(get_mint_supply(&svm, &onyc_mint), 800_000_000);
}

#[test]
fn test_mint_to_creates_boss_token_account() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Boss token account doesn't exist yet
    let boss_onyc_ata = get_associated_token_address(&boss, &onyc_mint);
    assert!(svm.get_account(&boss_onyc_ata).is_none());

    // Mint should create the account
    let ix = build_mint_to_ix(&boss, &onyc_mint, 1_000_000_000, &TOKEN_PROGRAM_ID);
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
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &new_onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Mint from new onyc mint
    let ix = build_mint_to_ix(&boss, &new_onyc_mint, 500_000_000, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let boss_new_ata = get_associated_token_address(&boss, &new_onyc_mint);
    assert_eq!(get_token_balance(&svm, &boss_new_ata), 500_000_000);
}

#[test]
fn test_mint_to_fails_wrong_mint() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    // Transfer authority for the real onyc mint
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Create a different mint that is NOT stored in state
    let wrong_mint = create_mint(&mut svm, &payer, 9, &boss);
    let ix = build_transfer_mint_authority_to_program_ix(&boss, &wrong_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Try to mint from wrong mint (not the onyc_mint in state)
    let ix = build_mint_to_ix(&boss, &wrong_mint, 1_000_000_000, &TOKEN_PROGRAM_ID);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "should fail when mint doesn't match onyc_mint in state"
    );
}

#[test]
fn test_mint_to_accrues_buffer_before_mint() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);
    let ix = build_make_offer_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        0,
        false,
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let now = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        None,
        now,
        1_000_000_000,
        0,
        86_400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_mint_to_ix(&boss, &onyc_mint, 1_000_000_000, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let offer_pda = find_offer_pda(&usdc_mint, &onyc_mint).0;
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let buffer_admin = Keypair::new();
    svm.airdrop(&buffer_admin.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix =
        build_initialize_buffer_ix(&boss, &offer_pda, &onyc_mint, &buffer_admin.pubkey());
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_set_buffer_gross_yield_ix(&boss, 100_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_set_buffer_fee_config_ix(&boss, 100, 1_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_manage_buffer_ix(&buffer_admin.pubkey(), &offer_pda, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&buffer_admin]).unwrap();
    advance_slot(&mut svm);
    advance_clock_by(&mut svm, 31_536_000);

    let ix = build_mint_to_ix_for_offer(
        &boss,
        &onyc_mint,
        1_000_000_000,
        &TOKEN_PROGRAM_ID,
        &offer_pda,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let buffer_state = read_buffer_state(&svm);
    let buffer_vault = derive_ata(&find_buffer_vault_authority_pda().0, &onyc_mint, &TOKEN_PROGRAM_ID);
    let management_fee_vault = derive_ata(
        &find_management_fee_vault_authority_pda().0,
        &onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let performance_fee_vault = derive_ata(
        &find_performance_fee_vault_authority_pda().0,
        &onyc_mint,
        &TOKEN_PROGRAM_ID,
    );

    assert_eq!(get_token_balance(&svm, &buffer_vault), 81_000_000);
    assert_eq!(get_token_balance(&svm, &management_fee_vault), 10_000_000);
    assert_eq!(get_token_balance(&svm, &performance_fee_vault), 9_000_000);
    assert_eq!(
        get_token_balance(&svm, &get_associated_token_address(&boss, &onyc_mint)),
        2_000_000_000
    );
    assert_eq!(buffer_state.lowest_supply, 2_100_000_000);

    let market_stats = read_market_stats(&svm);
    let (_, market_stats_bump) = find_market_stats_pda();
    assert_eq!(market_stats.bump, market_stats_bump);
    assert_eq!(market_stats.circulating_supply, 2_100_000_000);
    assert_eq!(market_stats.nav, 1_000_000_000);
    assert_eq!(market_stats.tvl, 2_100_000_000);
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

    let ix =
        build_transfer_mint_authority_to_program_ix(&boss, &token2022_mint, &TOKEN_2022_PROGRAM_ID);
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
    let ix =
        build_transfer_mint_authority_to_program_ix(&boss, &token2022_mint, &TOKEN_2022_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let (mint_authority_pda, _) = find_mint_authority_pda();
    assert_eq!(
        get_mint_authority_pubkey(&svm, &token2022_mint),
        Some(mint_authority_pda)
    );

    // Transfer back to boss
    let ix =
        build_transfer_mint_authority_to_boss_ix(&boss, &token2022_mint, &TOKEN_2022_PROGRAM_ID);
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
    let ix =
        build_transfer_mint_authority_to_program_ix(&boss, &token2022_mint, &TOKEN_2022_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Mint tokens
    let ix = build_mint_to_ix(
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
