mod common;

use common::*;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

// NAV/price scale is 1e9: 1.0 NAV = 1_000_000_000.
const NAV_1_0: u64 = 1_000_000_000;

// BUFFER math reference used by tests:
//
// Accrue:
//   spread = max(0, gross_yield - current_yield)        // APR scale 1e6
//   gross_mint = lowest_supply * spread * dt / YEAR / 1e6
//
// Fee split on accrual:
//   management_slice_apr = min(spread, management_fee_apr)
//   management_fee = floor(gross_mint * management_slice_apr / spread)
//   remaining = gross_mint - management_fee
//   if current_nav > performance_hwm_nav:
//     performance_fee = floor(remaining * performance_fee_bps / 10000)
//   else:
//     performance_fee = 0
//   buffer_mint = remaining - performance_fee
//
// High-water mark:
// - HWM is tracked in NAV/price units, not vault balance.
// - Performance fees are charged only while NAV is above the stored HWM.
//
// Burn for NAV support:
//   total_assets      = circulating_supply * current_nav / 1e9
//   assets_after      = total_assets - asset_adjustment_amount
//   required_supply   = ceil(assets_after * 1e9 / quoted_nav)
//   burn_amount       = circulating_supply - required_supply
//
// Units:
// - quoted_nav/current_nav: 1e9 scale
// - asset_adjustment_amount/total_assets: token-in mint base units
//   (e.g. USDC offer => micro-USDC)
// - supply and burn amount: ONyc base units

fn setup_buffer_context(
    gross_yield: u64,
    current_yield: u64,
    management_fee_basis_points: u16,
    performance_fee_basis_points: u16,
) -> (litesvm::LiteSVM, Keypair, Pubkey, Pubkey, Keypair) {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let token_in_mint = create_mint(&mut svm, &payer, 6, &boss);
    let yield_token_in_mint = create_mint(&mut svm, &payer, 6, &boss);
    let caller = Keypair::new();
    svm.airdrop(&caller.pubkey(), INITIAL_LAMPORTS).unwrap();

    let ix = build_make_offer_ix(
        &boss,
        &token_in_mint,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let now = get_clock_time(&svm);
    let ix = build_add_offer_vector_ix(
        &boss,
        &token_in_mint,
        &onyc_mint,
        Some(now),
        now,
        NAV_1_0,
        0,
        86_400,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_make_offer_ix(
        &boss,
        &yield_token_in_mint,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);
    let (offer_pda, _) = find_offer_pda(&yield_token_in_mint, &onyc_mint);

    let ix = build_add_offer_vector_ix(
        &boss,
        &yield_token_in_mint,
        &onyc_mint,
        Some(now),
        now,
        NAV_1_0,
        current_yield,
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

    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_initialize_buffer_ix(&boss, &offer_pda, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_set_buffer_gross_yield_ix(&boss, gross_yield);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    if management_fee_basis_points != 0 || performance_fee_basis_points != 0 {
        let ix = build_set_buffer_fee_config_ix(
            &boss,
            management_fee_basis_points,
            performance_fee_basis_points,
        );
        send_tx(&mut svm, &[ix], &[&payer]).unwrap();
        advance_slot(&mut svm);
    }

    (svm, payer, token_in_mint, onyc_mint, caller)
}

#[test]
fn test_initialize_buffer_success() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let token_in_mint = create_mint(&mut svm, &payer, 6, &boss);
    let ix = build_make_offer_ix(
        &boss,
        &token_in_mint,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let (offer_pda, _) = find_offer_pda(&token_in_mint, &onyc_mint);

    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_initialize_buffer_ix(&boss, &offer_pda, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let buffer_state = read_buffer_state(&svm);
    assert_eq!(buffer_state.onyc_mint, onyc_mint);
    assert_eq!(buffer_state.gross_yield, 0);
    assert_eq!(buffer_state.previous_supply, 0);
    assert_eq!(buffer_state.management_fee_basis_points, 0);
    assert_eq!(buffer_state.performance_fee_basis_points, 0);
    assert_eq!(buffer_state.performance_fee_high_watermark, 0);

    let (reserve_vault_authority_pda, _) = find_reserve_vault_authority_pda();
    let buffer_vault_ata = derive_ata(&reserve_vault_authority_pda, &onyc_mint, &TOKEN_PROGRAM_ID);
    let (management_fee_vault_authority_pda, _) = find_management_fee_vault_authority_pda();
    let management_fee_vault_ata = derive_ata(
        &management_fee_vault_authority_pda,
        &onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let (performance_fee_vault_authority_pda, _) = find_performance_fee_vault_authority_pda();
    let performance_fee_vault_ata = derive_ata(
        &performance_fee_vault_authority_pda,
        &onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    assert!(svm.get_account(&buffer_vault_ata).is_some());
    assert!(svm.get_account(&management_fee_vault_ata).is_some());
    assert!(svm.get_account(&performance_fee_vault_ata).is_some());
}

#[test]
fn test_initialize_buffer_requires_state_main_offer() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let token_in_mint = create_mint(&mut svm, &payer, 6, &boss);
    let ix = build_make_offer_ix(
        &boss,
        &token_in_mint,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let (offer_pda, _) = find_offer_pda(&token_in_mint, &onyc_mint);

    let ix = build_initialize_buffer_ix(&boss, &offer_pda, &onyc_mint);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "initialize_buffer should require state.main_offer"
    );
}

#[test]
fn test_set_main_offer_updates_program_state() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let token_in_mint_a = create_mint(&mut svm, &payer, 6, &boss);
    let token_in_mint_b = create_mint(&mut svm, &payer, 6, &boss);
    let ix = build_make_offer_ix(
        &boss,
        &token_in_mint_a,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let ix = build_make_offer_ix(
        &boss,
        &token_in_mint_b,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (offer_a_pda, _) = find_offer_pda(&token_in_mint_a, &onyc_mint);
    let (offer_b_pda, _) = find_offer_pda(&token_in_mint_b, &onyc_mint);

    let ix = build_set_main_offer_ix(&boss, &offer_a_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_initialize_buffer_ix(&boss, &offer_a_pda, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_set_main_offer_ix(&boss, &offer_b_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    assert_eq!(read_state(&svm).main_offer, offer_b_pda);
}

#[test]
fn test_set_main_offer_rejects_offer_with_wrong_token_out_mint() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let token_in_mint = create_mint(&mut svm, &payer, 6, &boss);
    let wrong_token_out_mint = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in_mint,
        &wrong_token_out_mint,
        0,
        false,
        true,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (offer_pda, _) = find_offer_pda(&token_in_mint, &wrong_token_out_mint);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    let result = send_tx(&mut svm, &[ix], &[&payer]);

    assert!(
        result.is_err(),
        "set_main_offer should reject offers whose token_out_mint is not state.onyc_mint"
    );
    assert_eq!(read_state(&svm).main_offer, Pubkey::default());
    assert_ne!(wrong_token_out_mint, onyc_mint);
}

#[test]
fn test_mint_to_rejects_noncanonical_buffer_state_account() {
    let (mut svm, payer, _token_in_mint, onyc_mint, _caller) =
        setup_buffer_context(100_000, 0, 0, 0);
    let boss = payer.pubkey();
    let main_offer = read_state(&svm).main_offer;

    let mut ix =
        build_mint_to_ix_for_offer(&boss, &onyc_mint, 1_000_000, &TOKEN_PROGRAM_ID, &main_offer);
    ix.accounts[9].pubkey = Pubkey::new_unique();

    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(
        result.is_err(),
        "mint_to should reject a non-canonical buffer_state account instead of skipping accrual"
    );
}

#[test]
fn test_deposit_reserve_vault_allows_any_depositor() {
    let (mut svm, _payer, _token_in_mint, onyc_mint, caller) = setup_buffer_context(1, 0, 0, 0);
    let deposit_amount = 250_000_000;
    let caller_onyc_ata =
        create_token_account(&mut svm, &onyc_mint, &caller.pubkey(), deposit_amount);
    let reserve_vault_onyc_ata = derive_ata(
        &find_reserve_vault_authority_pda().0,
        &onyc_mint,
        &TOKEN_PROGRAM_ID,
    );

    let ix = build_deposit_reserve_vault_ix(&caller.pubkey(), &onyc_mint, deposit_amount);
    send_tx(&mut svm, &[ix], &[&caller]).unwrap();

    assert_eq!(get_token_balance(&svm, &caller_onyc_ata), 0);
    assert_eq!(
        get_token_balance(&svm, &reserve_vault_onyc_ata),
        deposit_amount
    );
}

#[test]
fn test_withdraw_reserve_vault_allows_boss() {
    let (mut svm, payer, _token_in_mint, onyc_mint, caller) = setup_buffer_context(1, 0, 0, 0);
    let deposit_amount = 300_000_000;
    let withdraw_amount = 120_000_000;
    create_token_account(&mut svm, &onyc_mint, &caller.pubkey(), deposit_amount);
    let reserve_vault_onyc_ata = derive_ata(
        &find_reserve_vault_authority_pda().0,
        &onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let boss_onyc_ata = derive_ata(&payer.pubkey(), &onyc_mint, &TOKEN_PROGRAM_ID);

    let deposit_ix = build_deposit_reserve_vault_ix(&caller.pubkey(), &onyc_mint, deposit_amount);
    send_tx(&mut svm, &[deposit_ix], &[&caller]).unwrap();

    let withdraw_ix = build_withdraw_reserve_vault_ix(&payer.pubkey(), &onyc_mint, withdraw_amount);
    send_tx(&mut svm, &[withdraw_ix], &[&payer]).unwrap();

    assert_eq!(
        get_token_balance(&svm, &reserve_vault_onyc_ata),
        deposit_amount - withdraw_amount
    );
    assert_eq!(
        get_token_balance(&svm, &boss_onyc_ata),
        1_000_000_000 + withdraw_amount
    );
}

#[test]
fn test_withdraw_reserve_vault_rejects_non_boss() {
    let (mut svm, _payer, _token_in_mint, onyc_mint, caller) = setup_buffer_context(1, 0, 0, 0);
    let deposit_amount = 150_000_000;
    create_token_account(&mut svm, &onyc_mint, &caller.pubkey(), deposit_amount);

    let deposit_ix = build_deposit_reserve_vault_ix(&caller.pubkey(), &onyc_mint, deposit_amount);
    send_tx(&mut svm, &[deposit_ix], &[&caller]).unwrap();

    let withdraw_ix = build_withdraw_reserve_vault_ix(&caller.pubkey(), &onyc_mint, 1);
    let result = send_tx(&mut svm, &[withdraw_ix], &[&caller]);
    assert!(result.is_err(), "non-boss withdrawal should fail");
}

#[test]
fn test_set_main_offer_rejects_no_change() {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();
    let token_in_mint = create_mint(&mut svm, &payer, 6, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &token_in_mint,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (offer_pda, _) = find_offer_pda(&token_in_mint, &onyc_mint);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    let result = send_tx(&mut svm, &[ix], &[&payer]);

    assert!(
        result.is_err(),
        "set_main_offer should reject no-op updates"
    );
}

#[test]
fn test_set_buffer_gross_yield_rejects_no_change() {
    let (mut svm, payer, _token_in_mint, _onyc_mint, _caller) =
        setup_buffer_context(150_000, 50_000, 0, 0);
    let boss = payer.pubkey();

    let ix = build_set_buffer_gross_yield_ix(&boss, 150_000);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "setting same gross yield should fail");
}

#[test]
fn test_set_buffer_fee_config_rejects_no_change() {
    let (mut svm, payer, _token_in_mint, _onyc_mint, _caller) =
        setup_buffer_context(150_000, 50_000, 100, 1_000);
    let boss = payer.pubkey();

    let ix = build_set_buffer_fee_config_ix(&boss, 100, 1_000);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "setting same fee config should fail");
}

#[test]
fn test_burn_for_nav_increase_uses_circulating_supply_basis() {
    let (mut svm, payer, _token_in_mint, onyc_mint, _caller) = setup_buffer_context(1, 0, 0, 0);
    let boss = payer.pubkey();
    let state = read_state(&svm);
    let main_offer = state.main_offer;
    assert_ne!(main_offer, Pubkey::default());

    let boss_onyc_ata = derive_ata(&boss, &onyc_mint, &TOKEN_PROGRAM_ID);
    let offer_vault_onyc_ata = derive_ata(
        &find_offer_vault_authority_pda().0,
        &onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let reserve_vault_onyc_ata = derive_ata(
        &find_reserve_vault_authority_pda().0,
        &onyc_mint,
        &TOKEN_PROGRAM_ID,
    );

    let ix = build_offer_vault_deposit_ix(&boss, &onyc_mint, 100_000_000, &TOKEN_PROGRAM_ID);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_deposit_reserve_vault_ix(&boss, &onyc_mint, 300_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    assert_eq!(get_token_balance(&svm, &offer_vault_onyc_ata), 100_000_000);
    assert_eq!(get_token_balance(&svm, &reserve_vault_onyc_ata), 300_000_000);
    assert_eq!(get_mint_supply(&svm, &onyc_mint), 1_000_000_000);

    let ix = build_burn_for_nav_increase_ix(&boss, &main_offer, &onyc_mint, 100_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    assert_eq!(get_token_balance(&svm, &offer_vault_onyc_ata), 100_000_000);
    assert_eq!(get_token_balance(&svm, &reserve_vault_onyc_ata), 200_000_000);
    assert_eq!(get_token_balance(&svm, &boss_onyc_ata), 600_000_000);
    assert_eq!(get_mint_supply(&svm, &onyc_mint), 900_000_000);

    let buffer_state = read_buffer_state(&svm);
    assert_eq!(buffer_state.previous_supply, 900_000_000);

    let circulating_supply =
        get_mint_supply(&svm, &onyc_mint) - get_token_balance(&svm, &offer_vault_onyc_ata);
    assert_eq!(circulating_supply, 800_000_000);
}

#[test]
fn test_burn_for_nav_increase_rejects_when_no_burn_is_needed() {
    let (mut svm, payer, _token_in_mint, onyc_mint, _caller) = setup_buffer_context(1, 0, 0, 0);
    let boss = payer.pubkey();
    let main_offer = read_state(&svm).main_offer;

    let ix = build_deposit_reserve_vault_ix(&boss, &onyc_mint, 100_000_000);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    advance_slot(&mut svm);

    let ix = build_burn_for_nav_increase_ix(&boss, &main_offer, &onyc_mint, 0);
    let result = send_tx(&mut svm, &[ix], &[&payer]);
    assert!(result.is_err(), "zero asset adjustment should require no burn");

    let reserve_vault_onyc_ata = derive_ata(
        &find_reserve_vault_authority_pda().0,
        &onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    assert_eq!(get_token_balance(&svm, &reserve_vault_onyc_ata), 100_000_000);
    assert_eq!(get_mint_supply(&svm, &onyc_mint), 1_000_000_000);
}
