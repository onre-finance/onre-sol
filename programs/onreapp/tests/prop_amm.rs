mod common;

use anchor_lang::AnchorDeserialize;
use common::*;
use onreapp::instructions::prop_amm::SwapQuote;
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
        false,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (offer_pda, _) = find_offer_pda(&usdc_mint, &onyc_mint);
    let ix = build_set_main_offer_ix(&boss, &offer_pda);
    send_tx(&mut svm, &[ix], &[&payer]).unwrap();

    let (vault_authority, _) = find_offer_vault_authority_pda();
    create_token_account(&mut svm, &usdc_mint, &vault_authority, 0);
    create_token_account(&mut svm, &onyc_mint, &vault_authority, 10_000_000_000_000);

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

    let quote_expiry = current_time as i64 + 60;
    let ix = build_quote_swap_ix(
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000,
        quote_expiry,
    );
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
    assert_eq!(quote.quote_expiry, quote_expiry);
}

#[test]
fn test_open_swap_enforces_minimum_out_and_expiry() {
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

    let quote_expiry = current_time as i64 + 60;
    let quote_ix = build_quote_swap_ix(
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000,
        quote_expiry,
    );
    let quote_metadata = send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).unwrap();
    let quote = SwapQuote::try_from_slice(get_return_data(&quote_metadata)).unwrap();

    let ix = build_open_swap_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000,
        quote.minimum_out + 1,
        quote.quote_expiry,
        None,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]);
    assert!(result.is_err());

    let ix = build_open_swap_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000,
        quote.minimum_out,
        quote.quote_expiry,
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

    let late_expiry = get_clock_time(&ctx.svm) as i64 + 1;
    let quote_ix = build_quote_swap_ix(
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000,
        late_expiry,
    );
    let quote_metadata = send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).unwrap();
    let quote = SwapQuote::try_from_slice(get_return_data(&quote_metadata)).unwrap();

    advance_clock_by(&mut ctx.svm, 2);

    let ix = build_open_swap_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.usdc_mint,
        &ctx.onyc_mint,
        1_000_000,
        quote.minimum_out,
        quote.quote_expiry,
        None,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    let result = send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]);
    assert!(result.is_err());
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

    let (redemption_vault_authority, _) = find_redemption_vault_authority_pda();
    create_token_account(&mut ctx.svm, &ctx.onyc_mint, &redemption_vault_authority, 0);
    create_token_account(
        &mut ctx.svm,
        &ctx.usdc_mint,
        &redemption_vault_authority,
        10_000_000_000,
    );
    create_token_account(&mut ctx.svm, &ctx.onyc_mint, &ctx.user.pubkey(), 2_000_000_000);

    let quote_expiry = current_time as i64 + 60;
    let quote_ix = build_quote_swap_ix(
        &ctx.onyc_mint,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        1_000_000_000,
        quote_expiry,
    );
    let quote_metadata = send_tx(&mut ctx.svm, &[quote_ix], &[&ctx.payer]).unwrap();
    let quote = SwapQuote::try_from_slice(get_return_data(&quote_metadata)).unwrap();

    assert_eq!(
        quote.offer,
        find_offer_pda(&ctx.usdc_mint, &ctx.onyc_mint).0
    );
    assert_eq!(quote.token_out_amount, 1_000_000);

    let supply_before = get_mint_supply(&ctx.svm, &ctx.onyc_mint);
    let vault_before = get_token_balance(
        &ctx.svm,
        &derive_ata(
            &redemption_vault_authority,
            &ctx.usdc_mint,
            &TOKEN_PROGRAM_ID,
        ),
    );

    let ix = build_open_swap_ix(
        &ctx.onyc_mint,
        &ctx.user.pubkey(),
        &boss,
        &ctx.onyc_mint,
        &ctx.usdc_mint,
        1_000_000_000,
        quote.minimum_out,
        quote.quote_expiry,
        None,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut ctx.svm, &[ix], &[&ctx.payer, &ctx.user]).unwrap();

    let user_usdc = get_token_balance(
        &ctx.svm,
        &get_associated_token_address(&ctx.user.pubkey(), &ctx.usdc_mint),
    );
    assert_eq!(user_usdc, 10_001_000_000);
    assert_eq!(
        get_mint_supply(&ctx.svm, &ctx.onyc_mint),
        supply_before - 1_000_000_000
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
        vault_before - 1_000_000
    );
}
