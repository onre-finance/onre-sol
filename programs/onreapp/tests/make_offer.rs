mod common;

use common::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

#[test]
fn test_make_offer_succeeds() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(&boss, &token_in, &token_out, 500, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.token_in_mint, token_in);
    assert_eq!(offer.token_out_mint, token_out);
    assert_eq!(offer.fee_basis_points, 500);
}

#[test]
fn test_make_multiple_offers() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let t1_in = create_mint(&mut svm, &payer, 9, &boss);
    let t1_out = create_mint(&mut svm, &payer, 9, &boss);
    let ix = build_make_offer_ix(&boss, &t1_in, &t1_out, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    advance_slot(&mut svm);

    let t2_in = create_mint(&mut svm, &payer, 9, &boss);
    let t2_out = create_mint(&mut svm, &payer, 9, &boss);
    let ix = build_make_offer_ix(&boss, &t2_in, &t2_out, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer1 = read_offer(&svm, &t1_in, &t1_out);
    assert_eq!(offer1.token_in_mint, t1_in);
    assert_eq!(offer1.token_out_mint, t1_out);

    let offer2 = read_offer(&svm, &t2_in, &t2_out);
    assert_eq!(offer2.token_in_mint, t2_in);
    assert_eq!(offer2.token_out_mint, t2_out);
}

#[test]
fn test_make_offer_initializes_vault_token_in() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(&boss, &token_in, &token_out, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (vault_authority, _) = find_offer_vault_authority_pda();
    let vault_token_in_ata = get_associated_token_address(&vault_authority, &token_in);
    let account = svm.get_account(&vault_token_in_ata);
    assert!(account.is_some(), "vault token_in ATA should exist");
}

#[test]
fn test_make_offer_rejects_duplicate() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(&boss, &token_in, &token_out, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    advance_slot(&mut svm);

    let ix = build_make_offer_ix(&boss, &token_in, &token_out, 0, false, false);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "duplicate offer should fail");
}

#[test]
fn test_make_offer_rejects_non_boss() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let non_boss = Keypair::new();
    svm.airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS).unwrap();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(&non_boss.pubkey(), &token_in, &token_out, 0, false, false);
    let result = send_tx(&mut svm, &[ix], &[&non_boss]);
    assert!(result.is_err(), "non-boss should not be able to make offer");
}

#[test]
fn test_make_offer_with_permissionless_enabled() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(&boss, &token_in, &token_out, 0, false, true);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.allow_permissionless, 1);
}

#[test]
fn test_make_offer_with_permissionless_disabled() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(&boss, &token_in, &token_out, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.allow_permissionless, 0);
}

#[test]
fn test_make_offer_with_approval_required() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(&boss, &token_in, &token_out, 0, true, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.needs_approval, 1);
}

#[test]
fn test_make_offer_rejects_fee_over_max() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    // MAX_ALLOWED_FEE_BPS is 1000 (10%)
    let ix = build_make_offer_ix(&boss, &token_in, &token_out, 1001, false, false);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "fee over max should fail");
}

// ===========================================================================
// Token-2022 Tests
// ===========================================================================

#[test]
fn test_make_offer_token2022_as_token_in() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint_2022(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix_with_programs(
        &boss,
        &token_in,
        &token_out,
        500,
        false,
        false,
        &TOKEN_2022_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.token_in_mint, token_in);
    assert_eq!(offer.token_out_mint, token_out);
    assert_eq!(offer.fee_basis_points, 500);
}

#[test]
fn test_make_offer_token2022_as_token_out() {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint_2022(&mut svm, &payer, 9, &boss);

    // token_out doesn't need a special program for make_offer (no vault created for token_out)
    let ix = build_make_offer_ix(&boss, &token_in, &token_out, 500, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let offer = read_offer(&svm, &token_in, &token_out);
    assert_eq!(offer.token_in_mint, token_in);
    assert_eq!(offer.token_out_mint, token_out);
    assert_eq!(offer.fee_basis_points, 500);
}
