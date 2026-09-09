mod common;

use anchor_lang::AccountSerialize;
use common::*;
use onreapp::state::{ConfigurableVault, ConfigurableVaultKind};
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_sdk::account::Account;
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

    // Set worker = boss.
    let ix = build_set_worker_ix(&boss, &boss);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Create offer: usdc -> onyc (original direction)
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

    let (offer_pda, _) = find_offer_pda(&usdc_mint, &onyc_mint);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    // Add a pricing vector: price=1.0, apr=0
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

    // Create redemption offer: onyc -> usdc with 1% fee
    let ix = build_make_redemption_offer_ix(
        &boss,
        &onyc_mint,
        &usdc_mint,
        FEE_BASIS_POINTS,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
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
    create_token_account(
        &mut svm,
        &usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );

    // Transfer ONyc mint authority to the program PDA so fulfill burns token-in.
    // USDC remains externally controlled and is paid from redemption-vault liquidity.
    let (mint_authority_pda, _) = find_mint_authority_pda();
    set_mint_authority(&mut svm, &onyc_mint, &mint_authority_pda);

    FeeRoutingCtx {
        svm,
        payer,
        onyc_mint,
        usdc_mint,
        user,
    }
}

fn create_and_fulfill(ctx: &mut FeeRoutingCtx) {
    let boss = ctx.payer.pubkey();

    let request_id = "fee-routing";

    let ix = build_create_redemption_request_ix(
        &ctx.user.pubkey(),
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        REDEMPTION_AMOUNT,
        request_id,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();
    advance_slot(&mut ctx.svm);

    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &find_offer_pda(&ctx.usdc_mint, &ctx.onyc_mint).0,
        &ctx.user.pubkey(),
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        request_id,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
        REDEMPTION_AMOUNT,
    );
    let cu_ix =
        ComputeBudgetInstruction::set_compute_unit_limit(coverage_compute_unit_limit(300_000));
    send_tx(&mut ctx.svm, &[cu_ix, ix], &[&ctx.payer]).unwrap();
    advance_slot(&mut ctx.svm);
}

fn set_offer_fee_destination(ctx: &mut FeeRoutingCtx, destination: &Pubkey) {
    let boss = ctx.payer.pubkey();
    let (fee_vault_pda, _) = find_redemption_fee_vault_pda();
    let ix = build_set_configurable_vault_destination_ix(
        &boss,
        &fee_vault_pda,
        ConfigurableVaultKind::RedemptionFee.as_u8(),
        destination,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    advance_slot(&mut ctx.svm);
}

fn build_withdraw_offer_fee_vault_ix(
    caller: &Pubkey,
    destination: &Pubkey,
    token_in_mint: &Pubkey,
    amount: u64,
) -> solana_sdk::instruction::Instruction {
    let (fee_vault_pda, _) = find_redemption_fee_vault_pda();
    build_withdraw_configurable_vault_ix(
        caller,
        &fee_vault_pda,
        destination,
        token_in_mint,
        ConfigurableVaultKind::RedemptionFee.as_u8(),
        amount,
        &TOKEN_PROGRAM_ID,
    )
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
    let (fee_vault_pda, _) = find_redemption_fee_vault_pda();

    create_and_fulfill(&mut ctx);

    let fee_vault_ata = get_associated_token_address(&fee_vault_pda, &ctx.onyc_mint);
    assert_eq!(get_token_balance(&ctx.svm, &fee_vault_ata), EXPECTED_FEE);
}

#[test]
fn test_fee_routing_fees_go_to_custom_wallet_when_set() {
    let mut ctx = setup_fee_routing();
    let boss = ctx.payer.pubkey();

    let custom_wallet = Keypair::new();
    ctx.svm
        .airdrop(&custom_wallet.pubkey(), INITIAL_LAMPORTS)
        .unwrap();

    set_offer_fee_destination(&mut ctx, &custom_wallet.pubkey());

    create_and_fulfill(&mut ctx);

    // Fees always accrue in the configurable vault first.
    let (fee_vault_pda, _) = find_redemption_fee_vault_pda();
    let fee_vault_ata = get_associated_token_address(&fee_vault_pda, &ctx.onyc_mint);
    assert_eq!(get_token_balance(&ctx.svm, &fee_vault_ata), EXPECTED_FEE);

    let ix = build_withdraw_offer_fee_vault_ix(&boss, &custom_wallet.pubkey(), &ctx.onyc_mint, 0);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let custom_ata = get_associated_token_address(&custom_wallet.pubkey(), &ctx.onyc_mint);
    assert_eq!(get_token_balance(&ctx.svm, &custom_ata), EXPECTED_FEE);
    assert_eq!(get_balance_or_zero(&ctx.svm, &fee_vault_ata), 0);
}

#[test]
fn test_fee_routing_change_mid_stream() {
    let mut ctx = setup_fee_routing();
    let boss = ctx.payer.pubkey();
    let (fee_vault_pda, _) = find_redemption_fee_vault_pda();
    let fee_vault_ata = get_associated_token_address(&fee_vault_pda, &ctx.onyc_mint);

    // First fulfillment with default destination
    create_and_fulfill(&mut ctx);
    assert_eq!(get_token_balance(&ctx.svm, &fee_vault_ata), EXPECTED_FEE);

    // Change fee destination to a new wallet
    let custom_wallet = Keypair::new();
    ctx.svm
        .airdrop(&custom_wallet.pubkey(), INITIAL_LAMPORTS)
        .unwrap();
    set_offer_fee_destination(&mut ctx, &custom_wallet.pubkey());

    // Second fulfillment still accrues into the vault.
    create_and_fulfill(&mut ctx);

    assert_eq!(
        get_token_balance(&ctx.svm, &fee_vault_ata),
        2 * EXPECTED_FEE
    );

    let ix = build_withdraw_offer_fee_vault_ix(&boss, &custom_wallet.pubkey(), &ctx.onyc_mint, 0);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    let custom_ata = get_associated_token_address(&custom_wallet.pubkey(), &ctx.onyc_mint);
    assert_eq!(get_token_balance(&ctx.svm, &custom_ata), 2 * EXPECTED_FEE);
    assert_eq!(get_token_balance(&ctx.svm, &fee_vault_ata), 0);
}

#[test]
fn test_fee_routing_rejects_invalid_fee_destination() {
    let mut ctx = setup_fee_routing();
    let boss = ctx.payer.pubkey();

    let custom_wallet = Keypair::new();
    ctx.svm
        .airdrop(&custom_wallet.pubkey(), INITIAL_LAMPORTS)
        .unwrap();

    set_offer_fee_destination(&mut ctx, &custom_wallet.pubkey());
    create_and_fulfill(&mut ctx);

    // Withdraw to a different wallet than the configured destination.
    let wrong_wallet = Keypair::new();
    ctx.svm
        .airdrop(&wrong_wallet.pubkey(), INITIAL_LAMPORTS)
        .unwrap();
    let ix = build_withdraw_offer_fee_vault_ix(&boss, &wrong_wallet.pubkey(), &ctx.onyc_mint, 0);
    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]);
    assert!(result.is_err(), "wrong fee destination should be rejected");
}

#[test]
fn test_fulfill_rejects_configurable_vault_with_invalid_discriminator() {
    let mut ctx = setup_fee_routing();
    let boss = ctx.payer.pubkey();
    let (fee_vault_pda, bump) = find_redemption_fee_vault_pda();
    let vault = ConfigurableVault {
        kind: ConfigurableVaultKind::RedemptionFee.as_u8(),
        withdrawal_destination: Pubkey::default(),
        bump,
        reserved: [0; 31],
    };
    let mut data = Vec::new();
    vault.try_serialize(&mut data).unwrap();
    data[0] ^= u8::MAX;
    ctx.svm
        .set_account(
            fee_vault_pda,
            Account {
                lamports: INITIAL_LAMPORTS,
                data,
                owner: PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();

    let request_id = "invalid-fee-vault";
    let ix = build_create_redemption_request_ix(
        &ctx.user.pubkey(),
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        REDEMPTION_AMOUNT,
        request_id,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.user]).unwrap();
    advance_slot(&mut ctx.svm);

    let ix = build_fulfill_redemption_request_ix(
        &boss,
        &boss,
        &find_offer_pda(&ctx.usdc_mint, &ctx.onyc_mint).0,
        &ctx.user.pubkey(),
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        request_id,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
        REDEMPTION_AMOUNT,
    );

    assert!(
        send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).is_err(),
        "a program-owned configurable vault with an invalid discriminator should fail"
    );
}

#[test]
fn test_set_fee_destination_rejects_non_boss() {
    let mut ctx = setup_fee_routing();
    let destination = Keypair::new();
    let non_boss = Keypair::new();
    ctx.svm
        .airdrop(&destination.pubkey(), INITIAL_LAMPORTS)
        .unwrap();
    ctx.svm
        .airdrop(&non_boss.pubkey(), INITIAL_LAMPORTS)
        .unwrap();

    let (fee_vault_pda, _) = find_redemption_fee_vault_pda();
    let ix = build_set_configurable_vault_destination_ix(
        &non_boss.pubkey(),
        &fee_vault_pda,
        ConfigurableVaultKind::RedemptionFee.as_u8(),
        &destination.pubkey(),
    );
    let result = send_tx(&mut ctx.svm, &[ix], &[&non_boss]);
    assert!(
        result.is_err(),
        "non-boss should not set configurable vault destination"
    );
}

#[test]
fn test_set_fee_destination_rejects_no_change() {
    let mut ctx = setup_fee_routing();
    let destination = Keypair::new();
    ctx.svm
        .airdrop(&destination.pubkey(), INITIAL_LAMPORTS)
        .unwrap();
    set_offer_fee_destination(&mut ctx, &destination.pubkey());

    let boss = ctx.payer.pubkey();
    let (fee_vault_pda, _) = find_redemption_fee_vault_pda();
    let ix = build_set_configurable_vault_destination_ix(
        &boss,
        &fee_vault_pda,
        ConfigurableVaultKind::RedemptionFee.as_u8(),
        &destination.pubkey(),
    );
    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]);
    assert!(
        result.is_err(),
        "setting the same configurable vault destination should fail"
    );
}

#[test]
fn test_withdraw_offer_fees_rejects_missing_destination() {
    let mut ctx = setup_fee_routing();
    let boss = ctx.payer.pubkey();
    create_and_fulfill(&mut ctx);

    let ix =
        build_withdraw_offer_fee_vault_ix(&boss, &Pubkey::default(), &ctx.onyc_mint, EXPECTED_FEE);
    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]);
    assert!(
        result.is_err(),
        "withdrawal should fail before a destination is configured"
    );
}

#[test]
fn test_withdraw_offer_fees_rejects_kind_mismatch() {
    let mut ctx = setup_fee_routing();
    let boss = ctx.payer.pubkey();
    let destination = Keypair::new();
    ctx.svm
        .airdrop(&destination.pubkey(), INITIAL_LAMPORTS)
        .unwrap();
    set_offer_fee_destination(&mut ctx, &destination.pubkey());
    create_and_fulfill(&mut ctx);

    let (fee_vault_pda, _) = find_redemption_fee_vault_pda();
    let ix = build_withdraw_configurable_vault_ix(
        &boss,
        &fee_vault_pda,
        &destination.pubkey(),
        &ctx.onyc_mint,
        ConfigurableVaultKind::ManagementFee.as_u8(),
        EXPECTED_FEE,
        &TOKEN_PROGRAM_ID,
    );
    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]);
    assert!(
        result.is_err(),
        "withdrawal should reject a vault PDA/kind mismatch"
    );
}

#[test]
fn test_withdraw_offer_fees_rejects_transfer_fee_mint() {
    let (mut svm, payer, _onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let mint = create_mint_2022_with_transfer_fee(&mut svm, &payer, 6, &boss, 100, 1_000_000);
    let destination = Keypair::new();
    svm.airdrop(&destination.pubkey(), INITIAL_LAMPORTS)
        .unwrap();

    let (fee_vault_pda, _) = find_redemption_fee_vault_pda();
    let ix = build_set_configurable_vault_destination_ix(
        &boss,
        &fee_vault_pda,
        ConfigurableVaultKind::RedemptionFee.as_u8(),
        &destination.pubkey(),
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let vault_token_account = create_token_account_2022(&mut svm, &mint, &fee_vault_pda, 1_000_000);
    let ix = build_withdraw_configurable_vault_ix(
        &boss,
        &fee_vault_pda,
        &destination.pubkey(),
        &mint,
        ConfigurableVaultKind::RedemptionFee.as_u8(),
        100_000,
        &TOKEN_2022_PROGRAM_ID,
    );
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "configurable vault withdrawals should reject transfer-fee mints"
    );
    assert_eq!(get_token_balance(&svm, &vault_token_account), 1_000_000);
}

#[test]
fn test_fee_routing_no_fee_transfer_when_zero_bps() {
    let mut ctx = setup_fee_routing();
    let boss = ctx.payer.pubkey();

    // Reset fee to 0
    let ix = build_update_redemption_offer_fee_ix(&boss, &ctx.onyc_mint, &ctx.usdc_mint, 0);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    advance_slot(&mut ctx.svm);

    let (fee_vault_pda, _) = find_redemption_fee_vault_pda();
    create_and_fulfill(&mut ctx);

    let fee_vault_ata = get_associated_token_address(&fee_vault_pda, &ctx.onyc_mint);
    assert_eq!(get_balance_or_zero(&ctx.svm, &fee_vault_ata), 0);
}

#[test]
fn test_fee_routing_fee_math_correctness() {
    let mut ctx = setup_fee_routing();
    let (fee_vault_pda, _) = find_redemption_fee_vault_pda();

    create_and_fulfill(&mut ctx);

    // Fee vault PDA ATA should have exactly EXPECTED_FEE
    let fee_vault_ata = get_associated_token_address(&fee_vault_pda, &ctx.onyc_mint);
    assert_eq!(get_token_balance(&ctx.svm, &fee_vault_ata), EXPECTED_FEE);

    // User USDC ATA should have EXPECTED_TOKEN_OUT
    let user_usdc_ata = get_associated_token_address(&ctx.user.pubkey(), &ctx.usdc_mint);
    assert_eq!(
        get_token_balance(&ctx.svm, &user_usdc_ata),
        EXPECTED_TOKEN_OUT
    );
}

#[test]
fn test_withdraw_offer_fees_withdraws_full_balance_when_amount_is_zero() {
    let mut ctx = setup_fee_routing();
    let boss = ctx.payer.pubkey();
    let (fee_vault_pda, _) = find_redemption_fee_vault_pda();
    create_and_fulfill(&mut ctx);

    let fee_vault_ata = get_associated_token_address(&fee_vault_pda, &ctx.onyc_mint);
    assert_eq!(get_token_balance(&ctx.svm, &fee_vault_ata), EXPECTED_FEE);

    let destination = Keypair::new();
    ctx.svm
        .airdrop(&destination.pubkey(), INITIAL_LAMPORTS)
        .unwrap();
    set_offer_fee_destination(&mut ctx, &destination.pubkey());

    let ix = build_withdraw_offer_fee_vault_ix(&boss, &destination.pubkey(), &ctx.onyc_mint, 0);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let destination_ata = get_associated_token_address(&destination.pubkey(), &ctx.onyc_mint);
    assert_eq!(get_token_balance(&ctx.svm, &destination_ata), EXPECTED_FEE);
    assert_eq!(get_token_balance(&ctx.svm, &fee_vault_ata), 0);
}

#[test]
fn test_withdraw_offer_fees_supports_partial_withdrawal() {
    let mut ctx = setup_fee_routing();
    let boss = ctx.payer.pubkey();
    let (fee_vault_pda, _) = find_redemption_fee_vault_pda();
    create_and_fulfill(&mut ctx);

    let destination = Keypair::new();
    ctx.svm
        .airdrop(&destination.pubkey(), INITIAL_LAMPORTS)
        .unwrap();
    set_offer_fee_destination(&mut ctx, &destination.pubkey());
    let withdraw_amount = EXPECTED_FEE / 2;

    let ix = build_withdraw_offer_fee_vault_ix(
        &boss,
        &destination.pubkey(),
        &ctx.onyc_mint,
        withdraw_amount,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let fee_vault_ata = get_associated_token_address(&fee_vault_pda, &ctx.onyc_mint);
    let destination_ata = get_associated_token_address(&destination.pubkey(), &ctx.onyc_mint);
    assert_eq!(
        get_token_balance(&ctx.svm, &destination_ata),
        withdraw_amount
    );
    assert_eq!(
        get_token_balance(&ctx.svm, &fee_vault_ata),
        EXPECTED_FEE - withdraw_amount
    );
}

#[test]
fn test_withdraw_offer_fees_rejects_zero_balance() {
    let mut ctx = setup_fee_routing();
    let boss = ctx.payer.pubkey();
    let destination = Keypair::new();
    ctx.svm
        .airdrop(&destination.pubkey(), INITIAL_LAMPORTS)
        .unwrap();
    set_offer_fee_destination(&mut ctx, &destination.pubkey());

    let ix = build_withdraw_offer_fee_vault_ix(&boss, &destination.pubkey(), &ctx.onyc_mint, 0);
    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]);
    assert!(
        result.is_err(),
        "empty fee vault should reject full-balance withdrawal"
    );
}

#[test]
fn test_withdraw_offer_fees_rejects_amount_above_balance() {
    let mut ctx = setup_fee_routing();
    let boss = ctx.payer.pubkey();
    create_and_fulfill(&mut ctx);

    let destination = Keypair::new();
    ctx.svm
        .airdrop(&destination.pubkey(), INITIAL_LAMPORTS)
        .unwrap();
    set_offer_fee_destination(&mut ctx, &destination.pubkey());

    let ix = build_withdraw_offer_fee_vault_ix(
        &boss,
        &destination.pubkey(),
        &ctx.onyc_mint,
        EXPECTED_FEE + 1,
    );
    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]);
    assert!(
        result.is_err(),
        "withdrawal above vault balance should fail"
    );
}

#[test]
fn test_withdraw_offer_fees_allows_non_boss_to_configured_destination() {
    let mut ctx = setup_fee_routing();
    create_and_fulfill(&mut ctx);

    let caller = Keypair::new();
    let destination = Keypair::new();
    ctx.svm.airdrop(&caller.pubkey(), INITIAL_LAMPORTS).unwrap();
    ctx.svm
        .airdrop(&destination.pubkey(), INITIAL_LAMPORTS)
        .unwrap();
    set_offer_fee_destination(&mut ctx, &destination.pubkey());

    let ix = build_withdraw_offer_fee_vault_ix(
        &caller.pubkey(),
        &destination.pubkey(),
        &ctx.onyc_mint,
        EXPECTED_FEE,
    );
    send_tx(&mut ctx.svm, &[ix], &[&caller]).unwrap();

    let destination_ata = get_associated_token_address(&destination.pubkey(), &ctx.onyc_mint);
    assert_eq!(get_token_balance(&ctx.svm, &destination_ata), EXPECTED_FEE);
}
