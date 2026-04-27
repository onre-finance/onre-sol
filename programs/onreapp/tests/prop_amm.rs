mod common;

use anchor_lang::AnchorDeserialize;
use common::*;
use onreapp::instructions::prop_amm::{apply_hard_wall_reserve_curve_with_params, SwapQuote};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

struct PropAmmCtx {
    svm: litesvm::LiteSVM,
    payer: Keypair,
    usdc_mint: Pubkey,
    onyc_mint: Pubkey,
    user: Keypair,
}

fn setup_prop_amm() -> PropAmmCtx {
    let (mut svm, payer, onyc_mint) = setup_initialized();
    let boss = payer.pubkey();

    let usdc_mint = create_mint(&mut svm, &payer, 6, &boss);

    let ix = build_make_offer_ix(
        &boss,
        &usdc_mint,
        &onyc_mint,
        0,
        false,
        true,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (offer_pda, _) = find_offer_pda(&usdc_mint, &onyc_mint);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();
    let ix = build_configure_prop_amm_ix(&boss, 1_500, 2_000, 3);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (vault_authority, _) = find_offer_vault_authority_pda();
    let (permissionless_authority, _) = find_permissionless_authority_pda();
    create_token_account(&mut svm, &usdc_mint, &vault_authority, 0);
    create_token_account(&mut svm, &onyc_mint, &vault_authority, 10_000_000_000_000);
    create_token_account(&mut svm, &usdc_mint, &permissionless_authority, 0);
    create_token_account(&mut svm, &onyc_mint, &permissionless_authority, 0);

    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
    create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 10_000_000_000);
    create_token_account(&mut svm, &usdc_mint, &boss, 0);

    PropAmmCtx {
        svm,
        payer,
        usdc_mint,
        onyc_mint,
        user,
    }
}

#[test]
fn test_hard_wall_curve_is_vulnerable_to_order_splitting() {
    let hard_wall_reserve = 10_000_000;
    let one_shot = apply_hard_wall_reserve_curve_with_params(
        5_000_000,
        10_000_000,
        hard_wall_reserve,
        2_000,
        3,
    )
    .unwrap();

    let mut split_total = 0_u64;
    let mut current_liquidity = 10_000_000_u64;
    for _ in 0..5 {
        let output = apply_hard_wall_reserve_curve_with_params(
            1_000_000,
            current_liquidity,
            hard_wall_reserve,
            2_000,
            3,
        )
        .unwrap();
        split_total += output;
        current_liquidity -= output;
    }

    assert!(split_total > one_shot);
}

#[test]
fn test_hard_wall_curve_ignores_surplus_above_target_reserve() {
    let hard_wall_reserve = 5_000_000;
    let raw_sell_value = 1_000_000;
    let at_target = apply_hard_wall_reserve_curve_with_params(
        raw_sell_value,
        hard_wall_reserve,
        hard_wall_reserve,
        2_000,
        3,
    )
    .unwrap();
    let above_target = apply_hard_wall_reserve_curve_with_params(
        raw_sell_value,
        10_000_000,
        hard_wall_reserve,
        2_000,
        3,
    )
    .unwrap();

    assert_eq!(above_target, at_target);
}

#[test]
fn test_quote_swap_returns_expected_quote_data() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86_400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.usdc_mint, &ctx.onyc_mint, 1_000_000);
    let metadata = send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();
    let quote = SwapQuote::try_from_slice(get_return_data(&metadata)).unwrap();

    assert_eq!(
        quote.offer,
        find_offer_pda(&ctx.usdc_mint, &ctx.onyc_mint).0
    );
    assert_eq!(quote.token_in_amount, 1_000_000);
    assert_eq!(quote.token_in_net_amount, 1_000_000);
    assert_eq!(quote.token_in_fee_amount, 0);
    assert_eq!(quote.token_out_amount, 1_000_000_000);
    assert_eq!(quote.minimum_out, quote.token_out_amount);
}

#[test]
fn test_open_swap_enforces_minimum_out() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86_400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let quote_ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.usdc_mint, &ctx.onyc_mint, 1_000_000);
    let quote_metadata = send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).unwrap();
    let quote = SwapQuote::try_from_slice(get_return_data(&quote_metadata)).unwrap();

    let ix = build_open_swap_buy_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000,
        quote.minimum_out + 1,
        None,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]);
    assert!(result.is_err());

    let ix = build_open_swap_buy_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000,
        quote.minimum_out,
        None,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]).unwrap();

    let user_onyc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.onyc_mint),
    );
    assert_eq!(user_onyc, quote.token_out_amount);
}

#[test]
fn test_open_swap_buy_refills_redemption_vault_until_target_then_overflows_to_boss() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86_400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let first_quote_ix =
        build_quote_swap_ix(&ctx.onyc_mint, &ctx.usdc_mint, &ctx.onyc_mint, 1_000_000);
    let first_quote_metadata = send_tx(&mut ctx.svm, &[first_quote_ix], &[&ctx.payer]).unwrap();
    let first_quote = SwapQuote::try_from_slice(get_return_data(&first_quote_metadata)).unwrap();

    let first_buy_ix = build_open_swap_buy_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000,
        first_quote.minimum_out,
        None,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[first_buy_ix], &[&ctx.payer, &ctx.user]).unwrap();

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    let redemption_vault_usdc = derive_ata(
        &redemption_vault_authority,
        &ctx.usdc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let boss_usdc = get_associated_token_address(&boss, &ctx.usdc_mint);
    assert_eq!(get_token_balance(&ctx.svm, &redemption_vault_usdc), 0);
    assert_eq!(get_token_balance(&ctx.svm, &boss_usdc), 1_000_000);

    let second_quote_ix =
        build_quote_swap_ix(&ctx.onyc_mint, &ctx.usdc_mint, &ctx.onyc_mint, 1_000_001);
    let second_quote_metadata = send_tx(&mut ctx.svm, &[second_quote_ix], &[&ctx.payer]).unwrap();
    let second_quote = SwapQuote::try_from_slice(get_return_data(&second_quote_metadata)).unwrap();

    let second_buy_ix = build_open_swap_buy_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_001,
        second_quote.minimum_out,
        None,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[second_buy_ix], &[&ctx.payer, &ctx.user]).unwrap();

    assert_eq!(get_token_balance(&ctx.svm, &redemption_vault_usdc), 150_000);
    assert_eq!(get_token_balance(&ctx.svm, &boss_usdc), 1_850_001);
}

#[test]
fn test_quote_and_open_swap_support_sell_side() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &ctx.onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86_400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let ix = build_make_redemption_offer_ix(
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        500,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(
        &mut ctx.svm,
        &ctx.usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    create_token_account(
        &mut ctx.svm,
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        2_000_000_000,
    );
    let ix = build_refresh_market_stats_ix(&boss, &ctx.usdc_mint, &ctx.onyc_mint);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let sell_amount = 100_000_000;
    let quote_ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.onyc_mint, &ctx.usdc_mint, sell_amount);
    let quote_metadata = send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).unwrap();
    let quote = SwapQuote::try_from_slice(get_return_data(&quote_metadata)).unwrap();

    assert_eq!(
        quote.offer,
        find_offer_pda(&ctx.usdc_mint, &ctx.onyc_mint).0
    );
    assert_eq!(quote.token_in_net_amount, 95_000_000);
    assert_eq!(quote.token_in_fee_amount, 5_000_000);
    assert!(quote.token_out_amount < 95_000);

    let supply_before = get_mint_supply(&ctx.svm, &ctx.onyc_mint);
    let vault_before = get_token_balance(
        &ctx.svm,
        &derive_ata(
            &redemption_vault_authority,
            &ctx.usdc_mint,
            &TOKEN_PROGRAM_ID,
        ),
    );

    let ix = build_open_swap_sell_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        sell_amount,
        quote.minimum_out,
        None,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]).unwrap();

    let user_usdc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.usdc_mint),
    );
    assert_eq!(user_usdc, 10_000_000_000 + quote.token_out_amount);
    assert_eq!(
        get_mint_supply(&ctx.svm, &ctx.onyc_mint),
        supply_before - 95_000_000
    );
    assert_eq!(
        get_token_balance(
            &ctx.svm,
            &derive_ata(
                &redemption_vault_authority,
                &ctx.usdc_mint,
                &TOKEN_PROGRAM_ID
            ),
        ),
        vault_before - quote.token_out_amount
    );
}

#[test]
fn test_sell_side_uses_zero_fee_when_redemption_offer_is_uninitialized() {
    let mut ctx = setup_prop_amm();
    let boss = ctx.payer.pubkey();
    let current_time = get_clock_time(&ctx.svm);

    let ix = build_transfer_mint_authority_to_program_ix(&boss, &ctx.onyc_mint, &TOKEN_PROGRAM_ID);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let ix = build_add_offer_vector_ix(
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        Some(current_time),
        current_time,
        1_000_000_000,
        0,
        86_400,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(
        &mut ctx.svm,
        &ctx.usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    create_token_account(
        &mut ctx.svm,
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        2_000_000_000,
    );
    let ix = build_refresh_market_stats_ix(&boss, &ctx.usdc_mint, &ctx.onyc_mint);
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer]).unwrap();

    let sell_amount = 100_000_000;
    let quote_ix = build_quote_swap_ix(&ctx.onyc_mint, &ctx.onyc_mint, &ctx.usdc_mint, sell_amount);
    let quote_metadata = send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).unwrap();
    let quote = SwapQuote::try_from_slice(get_return_data(&quote_metadata)).unwrap();

    assert_eq!(quote.token_in_net_amount, 100_000_000);
    assert_eq!(quote.token_in_fee_amount, 0);
    assert!(quote.token_out_amount < 100_000);

    let supply_before = get_mint_supply(&ctx.svm, &ctx.onyc_mint);
    let vault_before = get_token_balance(
        &ctx.svm,
        &derive_ata(
            &redemption_vault_authority,
            &ctx.usdc_mint,
            &TOKEN_PROGRAM_ID,
        ),
    );

    let ix = build_open_swap_sell_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        sell_amount,
        quote.minimum_out,
        None,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]).unwrap();

    let user_usdc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.usdc_mint),
    );
    assert_eq!(user_usdc, 10_000_000_000 + quote.token_out_amount);
    assert_eq!(
        get_mint_supply(&ctx.svm, &ctx.onyc_mint),
        supply_before - 100_000_000
    );
    assert_eq!(
        get_token_balance(
            &ctx.svm,
            &derive_ata(
                &redemption_vault_authority,
                &ctx.usdc_mint,
                &TOKEN_PROGRAM_ID
            ),
        ),
        vault_before - quote.token_out_amount
    );
}
