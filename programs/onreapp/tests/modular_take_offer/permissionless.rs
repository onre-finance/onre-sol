use super::*;

fn assert_permissionless_error(
    result: Result<TransactionMetadata, FailedTransactionMetadata>,
    expected: OnreError,
) {
    let failure = result.unwrap_err();
    assert_eq!(
        failure.err,
        TransactionError::InstructionError(1, InstructionError::Custom(6000 + expected as u32)),
        "{:?}",
        failure.meta.logs
    );
}

fn replace_account(instruction: &mut Instruction, from: Pubkey, to: Pubkey) {
    instruction
        .accounts
        .iter_mut()
        .find(|a| a.pubkey == from)
        .unwrap()
        .pubkey = to;
}

fn permissionless_buy(f: &mut Fixture) {
    f.create_route(
        OfferDirection::AssetToManaged,
        OfferFlow::Permissionless,
        QuoterKind::NavPermissionless,
        false,
    );
}

#[test]
fn nav_requires_no_approval_and_lazily_creates_intermediary_atas_for_both_token_programs() {
    for token_2022 in [false, true] {
        for (gross, output, fee) in [(2_000_000, 990_000_000, 20_000), (100, 47_500, 5)] {
            let mut f = Fixture::new(false, token_2022);
            permissionless_buy(&mut f);
            let authority = find_permissionless_authority_pda().0;
            let input = derive_ata(&authority, &f.asset, &f.asset_program);
            let managed_output = derive_ata(&authority, &f.mint, &f.token_program);
            assert!(f.svm.get_account(&input).is_none());
            assert!(f.svm.get_account(&managed_output).is_none());
            let instruction = f.take(gross, output);
            let result = f.send_without_approval(instruction).unwrap();
            assert!(!result
                .logs
                .iter()
                .any(|log| log.contains("Ed25519SigVerify")));
            assert_eq!(
                f.balance(f.asset, f.user.pubkey(), f.asset_program),
                10_000_000 - gross
            );
            assert_eq!(
                f.balance(f.asset, vault(VaultKind::Fee), f.asset_program),
                fee
            );
            assert_eq!(
                f.balance(f.asset, vault(VaultKind::Proceeds), f.asset_program),
                gross - fee
            );
            assert_eq!(f.balance(f.mint, f.user.pubkey(), f.token_program), output);
            assert_eq!(f.balance(f.asset, authority, f.asset_program), 0);
            assert_eq!(f.balance(f.mint, authority, f.token_program), 0);
            let event = executed_event(&result);
            assert_eq!(event.flow, OfferFlow::Permissionless);
            assert_eq!(event.direction, OfferDirection::AssetToManaged);
            assert_eq!(event.amount_out, output);
            assert_eq!(event.fee_amount, fee);
            assert_eq!(event.user, f.user.pubkey());
        }
    }
}

#[test]
fn preexisting_intermediary_balances_are_not_swept_or_spent() {
    let mut f = Fixture::new(false, false);
    permissionless_buy(&mut f);
    let authority = find_permissionless_authority_pda().0;
    f.seed_tokens(f.asset, authority, 12345);
    f.seed_tokens(f.mint, authority, 67890);
    let instruction = f.take(2_000_000, 990_000_000);
    f.send_without_approval(instruction).unwrap();
    assert_eq!(f.balance(f.asset, authority, f.asset_program), 12345);
    assert_eq!(f.balance(f.mint, authority, f.token_program), 67890);
    assert_eq!(
        f.balance(f.mint, f.user.pubkey(), f.token_program),
        990_000_000
    );
}

#[test]
fn permissionless_accounts_cannot_be_omitted_or_redirected() {
    let mut f = Fixture::new(false, false);
    permissionless_buy(&mut f);
    let authority = find_permissionless_authority_pda().0;
    let input = derive_ata(&authority, &f.asset, &f.asset_program);
    let output = derive_ata(&authority, &f.mint, &f.token_program);
    for (account, expected) in [
        (input, OnreError::InvalidVaultTokenInAccount),
        (output, OnreError::InvalidPermissionlessTokenOutAccount),
    ] {
        let mut instruction = f.take(100, 0);
        omit_account(&mut instruction, account);
        assert_permissionless_error(f.send_without_approval(instruction), expected);
    }
    let mut instruction = f.take(101, 0);
    for account in [input, output, authority] {
        omit_account(&mut instruction, account);
    }
    assert_permissionless_error(
        f.send_without_approval(instruction),
        OnreError::WrongAuthority,
    );

    let impostor = Pubkey::new_unique();
    let mut instruction = f.take(102, 0);
    replace_account(&mut instruction, authority, impostor);
    replace_account(
        &mut instruction,
        input,
        derive_ata(&impostor, &f.asset, &f.asset_program),
    );
    replace_account(
        &mut instruction,
        output,
        derive_ata(&impostor, &f.mint, &f.token_program),
    );
    assert_permissionless_error(
        f.send_without_approval(instruction),
        OnreError::WrongAuthority,
    );
    assert_eq!(
        f.balance(f.asset, f.user.pubkey(), f.asset_program),
        10_000_000
    );
    assert!(f.svm.get_account(&input).is_none());
    assert!(f.svm.get_account(&output).is_none());
}

#[test]
fn malformed_intermediary_ata_authority_is_rejected() {
    let mut f = Fixture::new(false, false);
    permissionless_buy(&mut f);
    let authority = find_permissionless_authority_pda().0;
    f.seed_tokens(f.asset, authority, 12345);
    let input = derive_ata(&authority, &f.asset, &f.asset_program);
    let mut account = f.svm.get_account(&input).unwrap();
    account.data[32..64].copy_from_slice(f.user.pubkey().as_ref());
    f.svm.set_account(input, account).unwrap();
    let instruction = f.take(100, 0);
    assert!(f.send_without_approval(instruction).is_err());
    assert_eq!(get_token_balance(&f.svm, &input), 12345);
    assert_eq!(
        f.balance(f.asset, f.user.pubkey(), f.asset_program),
        10_000_000
    );
}

#[test]
fn permissioned_authorization_cannot_be_bypassed_with_intermediary_accounts() {
    let mut f = Fixture::new(false, false);
    let permissioned = f.offer;
    let quoter = f.quoter;
    permissionless_buy(&mut f);
    let mut instruction = f.take(100, 0);
    replace_account(&mut instruction, f.offer, permissioned);
    let permissionless_quoter = read::<OfferConfig>(&f.svm, f.offer).quoter;
    replace_account(&mut instruction, permissionless_quoter, quoter);
    assert_permissionless_error(
        f.send_without_approval(instruction),
        OnreError::PermissionlessNotAllowed,
    );
    f.offer = permissioned;
    let mut instruction = f.take(101, 0);
    instruction.data = instruction::TakeOffer {
        gross_input_amount: 101,
        minimum_amount_out: 0,
        deadline: get_clock_time(&f.svm) + 100,
        approval_message: None,
    }
    .data();
    assert_permissionless_error(
        f.send_without_approval(instruction),
        OnreError::ApprovalRequired,
    );
}

#[test]
fn permissionless_rejects_approval_and_preserves_deadline_slippage_and_kill_controls() {
    let mut f = Fixture::new(false, false);
    permissionless_buy(&mut f);
    let mut instruction = f.take(100, 0);
    instruction.data = instruction::TakeOffer {
        gross_input_amount: 100,
        minimum_amount_out: 0,
        deadline: get_clock_time(&f.svm) + 100,
        approval_message: Some(f.approval()),
    }
    .data();
    assert_permissionless_error(
        f.send_without_approval(instruction),
        OnreError::UnexpectedApproval,
    );
    let mut instruction = f.take(101, 0);
    instruction.data = instruction::TakeOffer {
        gross_input_amount: 101,
        minimum_amount_out: 0,
        deadline: get_clock_time(&f.svm) - 1,
        approval_message: None,
    }
    .data();
    assert_permissionless_error(f.send_without_approval(instruction), OnreError::Expired);
    let instruction = f.take(100, 47_501);
    assert_permissionless_error(
        f.send_without_approval(instruction),
        OnreError::MinimumOutNotMet,
    );
    send_tx(
        &mut f.svm,
        &[build_set_kill_switch_ix(&f.boss.pubkey(), true)],
        &[&f.boss],
    )
    .unwrap();
    let instruction = f.take(102, 0);
    assert_permissionless_error(
        f.send_without_approval(instruction),
        OnreError::KillSwitchActivated,
    );
}

#[test]
fn late_failure_rolls_back_routing_accrual_and_all_lazy_ata_creation() {
    let mut f = Fixture::new(false, false);
    f.seed_tokens(f.mint, f.boss.pubkey(), 1_000_000_000);
    f.initialize_managed_buffer();
    let buffer = f.buffer();
    edit::<ManagedTokenBufferState>(&mut f.svm, buffer, |b| b.gross_apr = 100_000);
    permissionless_buy(&mut f);
    edit::<ManagedTokenConfig>(&mut f.svm, managed(f.mint), |m| {
        m.max_supply = 1_500_000_000
    });
    advance_clock_by(&mut f.svm, 31_536_000);
    let before = f.svm.get_account(&buffer).unwrap().data;
    let instruction = f.take(2_000_000, 0);
    assert_permissionless_error(
        f.send_without_approval(instruction),
        OnreError::MaxSupplyExceeded,
    );
    assert_eq!(f.svm.get_account(&buffer).unwrap().data, before);
    assert_eq!(get_mint_supply(&f.svm, &f.mint), 1_000_000_000);
    assert_eq!(
        f.balance(f.asset, f.user.pubkey(), f.asset_program),
        10_000_000
    );
    let authority = find_permissionless_authority_pda().0;
    for (mint, owner, program) in [
        (f.asset, authority, f.asset_program),
        (f.mint, authority, f.token_program),
        (f.asset, vault(VaultKind::Fee), f.asset_program),
        (f.asset, vault(VaultKind::Proceeds), f.asset_program),
        (f.mint, f.user.pubkey(), f.token_program),
    ] {
        assert!(f
            .svm
            .get_account(&derive_ata(&owner, &mint, &program))
            .is_none());
    }
    assert_eq!(f.balance(f.mint, f.buffer_vaults()[0], f.token_program), 0);
}

#[test]
fn managed_to_asset_burns_from_the_intermediary_and_routes_liquidity_output_back() {
    let mut f = Fixture::new(false, true);
    f.seed_tokens(f.mint, f.user.pubkey(), 1_000_000_000);
    let authority = find_permissionless_authority_pda().0;
    f.seed_tokens(f.mint, authority, 77);
    f.seed_tokens(f.asset, authority, 88);
    f.initialize_managed_buffer();
    f.create_route(
        OfferDirection::ManagedToAsset,
        OfferFlow::Permissionless,
        QuoterKind::NavPermissionless,
        true,
    );
    f.seed_tokens(f.asset, vault(VaultKind::Liquidity), 2_000_000);
    let instruction = f.take(1_000_000_000, 1_980_000);
    let result = f.send_without_approval(instruction).unwrap();
    assert_eq!(f.balance(f.mint, f.user.pubkey(), f.token_program), 0);
    assert_eq!(
        f.balance(f.asset, f.user.pubkey(), f.asset_program),
        11_980_000
    );
    assert_eq!(f.balance(f.mint, authority, f.token_program), 77);
    assert_eq!(f.balance(f.asset, authority, f.asset_program), 88);
    assert_eq!(
        f.balance(f.mint, vault(VaultKind::Fee), f.token_program),
        10_000_000
    );
    assert_eq!(
        read::<ManagedTokenBufferState>(&f.svm, f.buffer()).previous_supply,
        10_000_077
    );
    assert_eq!(executed_event(&result).liquidity_amount, 1_980_000);
}

#[test]
fn usdc_and_usdg_share_onyc_buffer_and_coexist_with_legacy_execution() {
    let mut f = Fixture::new(true, false);
    f.initialize_legacy_main_offer();
    let main_offer = find_offer_pda(&f.asset, &f.mint).0;
    send_tx(
        &mut f.svm,
        &[build_initialize_buffer_ix(
            &f.boss.pubkey(),
            &main_offer,
            &f.mint,
        )],
        &[&f.boss],
    )
    .unwrap();
    let usdc = f.asset;
    permissionless_buy(&mut f);
    let instruction = f.take(2_000_000, 990_000_000);
    f.send_without_approval(instruction).unwrap();
    let buffer = f.buffer();
    assert_eq!(
        read::<onreapp::instructions::BufferState>(&f.svm, buffer).previous_supply,
        990_000_000
    );
    edit::<onreapp::instructions::BufferState>(&mut f.svm, buffer, |b| b.gross_apr = 100_000);

    // Representative stable assets: six-decimal SPL USDC and Token-2022 USDG,
    // sharing one ONyc mint and the original USDC main offer for token-wide accrual.
    let usdg = create_mint_2022(&mut f.svm, &f.boss, 6, &f.boss.pubkey());
    f.seed_tokens(usdg, f.user.pubkey(), 10_000_000);
    f.asset = usdg;
    f.asset_program = TOKEN_2022_PROGRAM_ID;
    permissionless_buy(&mut f);
    advance_clock_by(&mut f.svm, 31_536_000);
    let instruction = f.take(2_000_000, 990_000_000);
    f.send_without_approval(instruction).unwrap();
    let after = read::<onreapp::instructions::BufferState>(&f.svm, buffer);
    assert_eq!(after.previous_supply, 2_079_000_000);
    assert_eq!(
        f.balance(f.mint, f.buffer_vaults()[0], f.token_program),
        99_000_000
    );

    let vault = find_offer_vault_authority_pda().0;
    f.seed_tokens(usdc, vault, 0);
    f.seed_tokens(f.mint, vault, 0);
    let instruction = build_take_offer_v2_ix(
        &f.user.pubkey(),
        &f.boss.pubkey(),
        &usdc,
        &f.mint,
        2_000_000,
        None,
        &TOKEN_PROGRAM_ID,
        &f.token_program,
    );
    send_tx(&mut f.svm, &[instruction], &[&f.user]).unwrap();
    let after_legacy = read::<onreapp::instructions::BufferState>(&f.svm, buffer);
    assert_eq!(after_legacy.previous_supply, 4_079_000_000);
    assert_eq!(
        after_legacy.last_accrual_timestamp,
        after.last_accrual_timestamp
    );
    assert_eq!(
        f.balance(f.mint, f.buffer_vaults()[0], f.token_program),
        99_000_000
    );
}
