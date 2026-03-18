mod common;

use common::*;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

const REDEMPTION_AMOUNT: u64 = 1_000_000_000; // 1 ONyc (9 decimals)
const FEE_BASIS_POINTS: u16 = 100; // 1%
// fee = ceil(1_000_000_000 * 100 / 10_000) = 10_000_000
const EXPECTED_FEE: u64 = 10_000_000;
// token_out = net * price * 10^6 / (10^9 * 10^9) = 990_000_000 * 1e9 * 1e6 / 1e18 = 990_000
const EXPECTED_TOKEN_OUT: u64 = 990_000; // 0.99 USDC (6 decimals)

struct FeeRoutingCtx {
    svm: litesvm::LiteSVM,
    payer: Keypair, // boss
    onyc_mint: Pubkey,
    usdc_mint: Pubkey,
    user: Keypair,
}

fn setup_fee_routing() -> FeeRoutingCtx {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);

    // Set redemption_admin = boss
    let ix = build_set_redemption_admin_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Create offer: usdc -> onyc (original direction)
    let ix = build_make_offer_ix(&boss, &usdc_mint, &onyc_mint, 0, false, false, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Add a pricing vector: price=1.0, apr=0
    let current_time = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss, &usdc_mint, &onyc_mint,
        None, current_time, 1_000_000_000, 0, 86400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_clock_by(&mut svm, 1);

    // Create redemption offer: onyc -> usdc with 1% fee
    let ix = build_make_redemption_offer_ix(
        &boss, &onyc_mint, &usdc_mint, FEE_BASIS_POINTS,
        &TOKEN_PROGRAM_ID, &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Fund redeemer with ONyc (and set supply so burn checks pass)
    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &onyc_mint, &user.pubkey(), 10_000_000_000_000);
    set_mint_supply(&mut svm, &onyc_mint, 10_000_000_000_000);

    // Pre-create vault ATAs (required by fulfill, not init_if_needed)
    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut svm, &onyc_mint, &redemption_vault_authority, 0);
    create_token_account(&mut svm, &usdc_mint, &redemption_vault_authority, 0);

    // Transfer mint authority to program PDA (burn+mint mode)
    let (mint_authority_pda, _) = find_mint_authority_pda();
    set_mint_authority(&mut svm, &onyc_mint, &mint_authority_pda);
    set_mint_authority(&mut svm, &usdc_mint, &mint_authority_pda);

    FeeRoutingCtx { svm, payer, onyc_mint, usdc_mint, user }
}

fn create_and_fulfill(ctx: &mut FeeRoutingCtx, fee_destination: &Pubkey) {
    let boss = ctx.payer.pubkey();

    let offer_data = read_redemption_offer(&ctx.svm, &ctx.onyc_mint, &ctx.usdc_mint);
    let counter = offer_data.request_counter;

    let ix = build_create_redemption_request_ix(
        &ctx.user.pubkey(), &ctx.onyc_mint, &ctx.usdc_mint,
        REDEMPTION_AMOUNT, counter, &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();
    advance_slot(&mut ctx.svm);

    let ix = build_fulfill_redemption_request_with_fee_dest_ix(
        &boss, &boss, &ctx.user.pubkey(),
        &ctx.onyc_mint, &ctx.usdc_mint, counter,
        &TOKEN_PROGRAM_ID, &TOKEN_PROGRAM_ID,
        REDEMPTION_AMOUNT, fee_destination,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    advance_slot(&mut ctx.svm);
}

fn get_balance_or_zero(svm: &litesvm::LiteSVM, ata: &Pubkey) -> u64 {
    if svm.get_account(ata).is_some() {
        get_token_balance(svm, ata)
    } else {
        0
    }
}

// ===========================================================================
// fee routing tests
// ===========================================================================

#[test]
fn test_fee_routing_fees_accumulate_in_vault_when_default() {
    let mut ctx = setup_fee_routing();
    let (fee_vault_pda, _) = find_redemption_fee_vault_authority_pda();

    create_and_fulfill(&mut ctx, &fee_vault_pda);

    let fee_vault_ata = get_associated_token_address(&fee_vault_pda, &ctx.onyc_mint);
    assert_eq!(get_token_balance(&ctx.svm, &fee_vault_ata), EXPECTED_FEE);
}

#[test]
fn test_fee_routing_fees_go_to_custom_wallet_when_set() {
    let mut ctx = setup_fee_routing();
    let boss = ctx.payer.pubkey();

    let custom_wallet = Keypair::new();
    ctx.svm.airdrop(&custom_wallet.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_set_redemption_fee_destination_ix(&boss, &custom_wallet.pubkey());
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    advance_slot(&mut ctx.svm);

    create_and_fulfill(&mut ctx, &custom_wallet.pubkey());

    // Custom wallet ATA should have the fee
    let custom_ata = get_associated_token_address(&custom_wallet.pubkey(), &ctx.onyc_mint);
    assert_eq!(get_token_balance(&ctx.svm, &custom_ata), EXPECTED_FEE);

    // Fee vault PDA ATA should be empty or non-existent (no fees routed there)
    let (fee_vault_pda, _) = find_redemption_fee_vault_authority_pda();
    let fee_vault_ata = get_associated_token_address(&fee_vault_pda, &ctx.onyc_mint);
    assert_eq!(get_balance_or_zero(&ctx.svm, &fee_vault_ata), 0);
}

#[test]
fn test_fee_routing_change_mid_stream() {
    let mut ctx = setup_fee_routing();
    let boss = ctx.payer.pubkey();
    let (fee_vault_pda, _) = find_redemption_fee_vault_authority_pda();
    let fee_vault_ata = get_associated_token_address(&fee_vault_pda, &ctx.onyc_mint);

    // First fulfillment with default destination
    create_and_fulfill(&mut ctx, &fee_vault_pda);
    assert_eq!(get_token_balance(&ctx.svm, &fee_vault_ata), EXPECTED_FEE);

    // Change fee destination to a new wallet
    let custom_wallet = Keypair::new();
    ctx.svm.airdrop(&custom_wallet.pubkey(), INITIAL_LAMPORTS).unwrap();
    let ix = build_set_redemption_fee_destination_ix(&boss, &custom_wallet.pubkey());
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    advance_slot(&mut ctx.svm);

    // Second fulfillment with custom destination
    create_and_fulfill(&mut ctx, &custom_wallet.pubkey());

    // Vault PDA still has only the first fee
    assert_eq!(get_token_balance(&ctx.svm, &fee_vault_ata), EXPECTED_FEE);

    // Custom wallet has the second fee
    let custom_ata = get_associated_token_address(&custom_wallet.pubkey(), &ctx.onyc_mint);
    assert_eq!(get_token_balance(&ctx.svm, &custom_ata), EXPECTED_FEE);
}

#[test]
fn test_fee_routing_rejects_invalid_fee_destination() {
    let mut ctx = setup_fee_routing();
    let boss = ctx.payer.pubkey();

    let custom_wallet = Keypair::new();
    ctx.svm.airdrop(&custom_wallet.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_set_redemption_fee_destination_ix(&boss, &custom_wallet.pubkey());
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    advance_slot(&mut ctx.svm);

    let offer_data = read_redemption_offer(&ctx.svm, &ctx.onyc_mint, &ctx.usdc_mint);
    let counter = offer_data.request_counter;

    let ix = build_create_redemption_request_ix(
        &ctx.user.pubkey(), &ctx.onyc_mint, &ctx.usdc_mint,
        REDEMPTION_AMOUNT, counter, &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();
    advance_slot(&mut ctx.svm);

    // Pass a different wallet as fee destination — should be rejected
    let wrong_wallet = Keypair::new();
    let ix = build_fulfill_redemption_request_with_fee_dest_ix(
        &boss, &boss, &ctx.user.pubkey(),
        &ctx.onyc_mint, &ctx.usdc_mint, counter,
        &TOKEN_PROGRAM_ID, &TOKEN_PROGRAM_ID,
        REDEMPTION_AMOUNT, &wrong_wallet.pubkey(),
    );
    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]);
    assert!(result.is_err(), "wrong fee destination should be rejected");
}

#[test]
fn test_fee_routing_no_fee_transfer_when_zero_bps() {
    let mut ctx = setup_fee_routing();
    let boss = ctx.payer.pubkey();

    // Reset fee to 0
    let ix = build_update_redemption_offer_fee_ix(&boss, &ctx.onyc_mint, &ctx.usdc_mint, 0);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    advance_slot(&mut ctx.svm);

    let (fee_vault_pda, _) = find_redemption_fee_vault_authority_pda();
    create_and_fulfill(&mut ctx, &fee_vault_pda);

    let fee_vault_ata = get_associated_token_address(&fee_vault_pda, &ctx.onyc_mint);
    assert_eq!(get_balance_or_zero(&ctx.svm, &fee_vault_ata), 0);
}

#[test]
fn test_fee_routing_fee_math_correctness() {
    let mut ctx = setup_fee_routing();
    let (fee_vault_pda, _) = find_redemption_fee_vault_authority_pda();

    create_and_fulfill(&mut ctx, &fee_vault_pda);

    // Fee vault PDA ATA should have exactly EXPECTED_FEE
    let fee_vault_ata = get_associated_token_address(&fee_vault_pda, &ctx.onyc_mint);
    assert_eq!(get_token_balance(&ctx.svm, &fee_vault_ata), EXPECTED_FEE);

    // User USDC ATA should have EXPECTED_TOKEN_OUT
    let user_usdc_ata = get_associated_token_address(&ctx.user.pubkey(), &ctx.usdc_mint);
    assert_eq!(get_token_balance(&ctx.svm, &user_usdc_ata), EXPECTED_TOKEN_OUT);
}