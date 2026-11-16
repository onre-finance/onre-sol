//! Escrowed fulfillment requests against Worker-execution offers.
use super::mint_multisig::{expect_error, send};
use super::*;
use onreapp::instructions::{FulfillmentRequestCancelledEvent, FulfillmentRequestFilledEvent};

/// A user holding 990_000_000 managed tokens (bought at NAV 2.0), a Worker sell
/// offer in `mode` paying from a Liquidity vault funded with 5 asset tokens, and a
/// configured worker.
fn worker_fixture(token_2022: bool, mode: OfferMode) -> (Fixture, Keypair) {
    let mut f = Fixture::new(false, token_2022);
    let instruction = f.take(2_000_000, 990_000_000);
    f.send(instruction).unwrap();
    f.create_route(OfferDirection::ManagedToAsset, mode, QuoterKind::Nav, true);
    f.seed_tokens(f.asset, vault(VaultKind::Liquidity), 5_000_000);
    let worker = Keypair::new();
    f.svm.airdrop(&worker.pubkey(), INITIAL_LAMPORTS).unwrap();
    let instruction = build_set_worker_ix(&f.boss.pubkey(), &worker.pubkey());
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    (f, worker)
}

fn request_pda(f: &Fixture, request_id: u64) -> Pubkey {
    pda(&[
        b"fulfillment_request",
        f.offer.as_ref(),
        f.user.pubkey().as_ref(),
        &request_id.to_le_bytes(),
    ])
}

fn escrow(f: &Fixture, request: Pubkey) -> Pubkey {
    derive_ata(&request, &f.mint, &f.token_program)
}

fn create_ix(
    f: &Fixture,
    request_id: u64,
    input_amount: u64,
    approval: Option<ApprovalMessage>,
) -> Instruction {
    let request = request_pda(f, request_id);
    ix(
        accounts::CreateFulfillmentRequest {
            config: f.config_accounts(),
            token_in_mint: f.mint,
            user_token_in: derive_ata(&f.user.pubkey(), &f.mint, &f.token_program),
            fulfillment_request: request,
            escrow_token_in: escrow(f, request),
            state: find_state_pda().0,
            instructions_sysvar: SYSVAR_INSTRUCTIONS_ID,
            user: f.user.pubkey(),
            token_in_program: f.token_program,
            associated_token_program: ATA_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::CreateFulfillmentRequest {
            request_id,
            input_amount,
            approval_message: approval,
        },
    )
}

/// Creates a request on a permissioned Worker offer, with the approval.
fn create(f: &mut Fixture, request_id: u64, input_amount: u64) -> Pubkey {
    let instruction = create_ix(f, request_id, input_amount, Some(f.approval()));
    f.send(instruction).unwrap();
    request_pda(f, request_id)
}

fn fill_ix(f: &Fixture, worker: Pubkey, request: Pubkey, input_amount: u64) -> Instruction {
    let (token_in, token_out, in_program, out_program) = f.pair(OfferDirection::ManagedToAsset);
    ix(
        accounts::FulfillFulfillmentRequest {
            config: f.config_accounts(),
            fulfillment_request: request,
            user: f.user.pubkey(),
            escrow_token_in: escrow(f, request),
            token_in_mint: token_in,
            token_out_mint: token_out,
            user_token_out: derive_ata(&f.user.pubkey(), &token_out, &out_program),
            fee_token_in: derive_ata(&vault(VaultKind::Fee), &token_in, &in_program),
            mint_authority: find_mint_authority_pda().0,
            state: find_state_pda().0,
            buffer: f.buffer_accounts(f.mint_multisig()),
            worker,
            token_in_program: in_program,
            token_out_program: out_program,
            associated_token_program: ATA_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
            custody: f.custody_accounts(),
        },
        instruction::FulfillFulfillmentRequest { input_amount },
    )
}

fn fill(
    f: &mut Fixture,
    worker: &Keypair,
    request: Pubkey,
    input_amount: u64,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let instruction = fill_ix(f, worker.pubkey(), request, input_amount);
    f.send_v0_signed(instruction, None, worker)
}

fn cancel_ix(f: &Fixture, signer: Pubkey, request: Pubkey) -> Instruction {
    ix(
        accounts::CancelFulfillmentRequest {
            fulfillment_request: request,
            offer_config: f.offer,
            user: f.user.pubkey(),
            signer,
            state: find_state_pda().0,
            token_in_mint: f.mint,
            escrow_token_in: escrow(f, request),
            user_token_in: derive_ata(&f.user.pubkey(), &f.mint, &f.token_program),
            token_in_program: f.token_program,
            associated_token_program: ATA_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::CancelFulfillmentRequest {},
    )
}

fn lamports(f: &Fixture, key: Pubkey) -> u64 {
    f.svm.get_account(&key).map_or(0, |a| a.lamports)
}

#[test]
fn worker_fills_partially_at_each_fills_price_and_a_full_fill_closes_the_request() {
    for token_2022 in [false, true] {
        let (mut f, worker) = worker_fixture(token_2022, OfferMode::Worker);
        let request = create(&mut f, 1, 900_000_000);
        let stored: FulfillmentRequest = read(&f.svm, request);
        assert_eq!(
            (stored.offer_config, stored.user, stored.request_id),
            (f.offer, f.user.pubkey(), 1)
        );
        assert_eq!(
            (stored.input_amount, stored.fulfilled_input_amount),
            (900_000_000, 0)
        );
        assert_eq!(
            f.balance(f.mint, f.user.pubkey(), f.token_program),
            90_000_000
        );
        assert_eq!(get_token_balance(&f.svm, &escrow(&f, request)), 900_000_000);

        // 1% fee on the fill; the net input is burned and paid at NAV 2.0.
        let result = fill(&mut f, &worker, request, 400_000_000).unwrap();
        let executed = executed_event(&result);
        assert_eq!(executed.user, f.user.pubkey());
        assert_eq!(
            (executed.access, executed.execution),
            (OfferAccess::Permissioned, OfferExecution::Worker)
        );
        assert_eq!(
            (executed.gross_input_amount, executed.fee_amount),
            (400_000_000, 4_000_000)
        );
        let filled: FulfillmentRequestFilledEvent = emitted(&result);
        assert_eq!(
            (filled.input_amount, filled.fulfilled_input_amount),
            (400_000_000, 400_000_000)
        );
        assert_eq!((filled.amount_out, filled.price), (792_000, 2_000_000_000));
        assert!(!filled.fully_fulfilled);
        assert_eq!(
            f.balance(f.asset, f.user.pubkey(), f.asset_program),
            8_792_000
        );
        assert_eq!(get_token_balance(&f.svm, &escrow(&f, request)), 500_000_000);
        assert_eq!(get_mint_supply(&f.svm, &f.mint), 594_000_000);
        let stored: FulfillmentRequest = read(&f.svm, request);
        assert_eq!(stored.fulfilled_input_amount, 400_000_000);

        // The next fill uses the price and fee current at that time.
        let now = get_clock_time(&f.svm);
        let instruction = ix(
            accounts::AddPricerVector {
                pricer: price(f.mint, 0),
                state: find_state_pda().0,
                boss: f.boss.pubkey(),
            },
            instruction::AddPricerVector {
                vector: PricingVector {
                    start_time: now + 100,
                    base_time: now + 100,
                    base_price: 3_000_000_000,
                    apr: 0,
                    price_fix_duration: 86_400,
                },
            },
        );
        send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
        advance_clock_by(&mut f.svm, 100);
        let rent = lamports(&f, request) + lamports(&f, escrow(&f, request));
        let user_lamports = lamports(&f, f.user.pubkey());
        let result = fill(&mut f, &worker, request, 500_000_000).unwrap();
        let filled: FulfillmentRequestFilledEvent = emitted(&result);
        assert_eq!(
            (filled.fulfilled_input_amount, filled.fee_amount),
            (900_000_000, 5_000_000)
        );
        assert_eq!(
            (filled.amount_out, filled.price),
            (1_485_000, 3_000_000_000)
        );
        assert!(filled.fully_fulfilled);
        assert_eq!(
            f.balance(f.asset, f.user.pubkey(), f.asset_program),
            10_277_000
        );
        assert_eq!(
            f.balance(f.mint, vault(VaultKind::Fee), f.token_program),
            9_000_000
        );
        assert_eq!(get_mint_supply(&f.svm, &f.mint), 99_000_000);
        // Request and escrow are closed; the user, who paid their rent, gets it back.
        assert!(f.svm.get_account(&request).is_none());
        assert!(f.svm.get_account(&escrow(&f, request)).is_none());
        assert_eq!(lamports(&f, f.user.pubkey()), user_lamports + rent);
        // The request ID is free again.
        create(&mut f, 1, 1_000_000);
    }
}

#[test]
fn request_access_follows_the_offer_and_ids_are_unique_per_open_request() {
    let (mut f, _) = worker_fixture(false, OfferMode::Worker);
    let instruction = create_ix(&f, 1, 1_000_000, None);
    expect_error(
        f.send_without_approval(instruction),
        OnreError::ApprovalRequired,
    );
    create(&mut f, 1, 1_000_000);
    let instruction = create_ix(&f, 1, 1_000_000, Some(f.approval()));
    assert!(f.send(instruction).is_err());
    let instruction = create_ix(&f, 2, 0, Some(f.approval()));
    expect_error(f.send(instruction), OnreError::InvalidAmount);
    // take_offer cannot execute a Worker offer.
    let instruction = f.take(1_000_000, 0);
    expect_error(f.send(instruction), OnreError::UnsupportedOfferExecution);

    // Permissionless Worker offers take requests without, and never with, approval.
    f.create_route(
        OfferDirection::ManagedToAsset,
        OfferMode::WorkerPermissionless,
        QuoterKind::Nav,
        true,
    );
    let instruction = create_ix(&f, 1, 1_000_000, Some(f.approval()));
    expect_error(f.send(instruction), OnreError::UnexpectedApproval);
    let instruction = create_ix(&f, 1, 1_000_000, None);
    f.send_without_approval(instruction).unwrap();
    assert_eq!(
        read::<FulfillmentRequest>(&f.svm, request_pda(&f, 1)).input_amount,
        1_000_000
    );

    // Instant offers do not take requests.
    f.create_route(
        OfferDirection::ManagedToAsset,
        OfferMode::Permissioned,
        QuoterKind::Nav,
        true,
    );
    let instruction = create_ix(&f, 1, 1_000_000, Some(f.approval()));
    expect_error(f.send(instruction), OnreError::UnsupportedOfferExecution);
}

#[test]
fn fills_are_worker_only_and_bounded_and_the_kill_switch_stops_them_but_not_cancels() {
    let (mut f, worker) = worker_fixture(false, OfferMode::Worker);
    let request = create(&mut f, 1, 900_000_000);
    let user = f.user.insecure_clone();
    expect_error(fill(&mut f, &user, request, 1), OnreError::InvalidWorker);
    expect_error(fill(&mut f, &worker, request, 0), OnreError::InvalidAmount);
    expect_error(
        fill(&mut f, &worker, request, 900_000_001),
        OnreError::FulfillmentAmountExceedsRemaining,
    );
    fill(&mut f, &worker, request, 400_000_000).unwrap();

    let stranger = Keypair::new();
    f.svm.airdrop(&stranger.pubkey(), INITIAL_LAMPORTS).unwrap();
    let instruction = cancel_ix(&f, stranger.pubkey(), request);
    expect_error(
        send(&mut f.svm, &[instruction], &[&stranger]),
        OnreError::Unauthorized,
    );

    let instruction = build_set_kill_switch_ix(&f.boss.pubkey(), true);
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    expect_error(
        fill(&mut f, &worker, request, 1),
        OnreError::KillSwitchActivated,
    );
    let instruction = create_ix(&f, 2, 1, Some(f.approval()));
    expect_error(f.send(instruction), OnreError::KillSwitchActivated);

    // The worker cancels the unfilled remainder back to the user.
    let rent = lamports(&f, request) + lamports(&f, escrow(&f, request));
    let user_lamports = lamports(&f, f.user.pubkey());
    let instruction = cancel_ix(&f, worker.pubkey(), request);
    let result = send(&mut f.svm, &[instruction], &[&worker]).unwrap();
    let cancelled: FulfillmentRequestCancelledEvent = emitted(&result);
    assert_eq!(
        (cancelled.returned_amount, cancelled.cancelled_by),
        (500_000_000, worker.pubkey())
    );
    assert_eq!(
        f.balance(f.mint, f.user.pubkey(), f.token_program),
        590_000_000
    );
    assert!(f.svm.get_account(&request).is_none());
    assert!(f.svm.get_account(&escrow(&f, request)).is_none());
    assert_eq!(lamports(&f, f.user.pubkey()), user_lamports + rent);
}

#[test]
fn the_user_cancels_and_a_full_fill_sweeps_unsolicited_escrow_surplus_to_the_fee_vault() {
    let (mut f, worker) = worker_fixture(false, OfferMode::Worker);
    let request = create(&mut f, 1, 100_000_000);
    let user = f.user.insecure_clone();
    let instruction = cancel_ix(&f, user.pubkey(), request);
    send(&mut f.svm, &[instruction], &[&user]).unwrap();
    assert!(f.svm.get_account(&request).is_none());
    assert_eq!(
        f.balance(f.mint, f.user.pubkey(), f.token_program),
        990_000_000
    );

    // A donation to the escrow would otherwise block closing it on the final fill.
    let request = create(&mut f, 2, 100_000_000);
    f.seed_tokens(f.mint, request, 100_001_000);
    fill(&mut f, &worker, request, 100_000_000).unwrap();
    assert!(f.svm.get_account(&request).is_none());
    assert!(f.svm.get_account(&escrow(&f, request)).is_none());
    assert_eq!(
        f.balance(f.mint, vault(VaultKind::Fee), f.token_program),
        1_000_000 + 1_000
    );
}
