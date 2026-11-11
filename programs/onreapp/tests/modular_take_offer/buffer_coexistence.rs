use super::*;
use onreapp::instructions::BufferState;

fn send(
    svm: &mut LiteSVM,
    instructions: &[Instruction],
    signers: &[&Keypair],
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    // Repeated settlements at one timestamp still need distinct transactions.
    advance_slot(svm);
    common::send_tx(svm, instructions, signers)
}

fn parts(f: &Fixture) -> accounts::ManagedBufferAccounts {
    let [reserve, management, performance] = f.buffer_vaults();
    let legacy =
        f.onyc && !read::<GlobalState>(&f.svm, find_state_pda().0).onyc_buffer_cutover_completed;
    accounts::ManagedBufferAccounts {
        state: find_state_pda().0,
        managed_token: managed(f.mint),
        managed_mint: f.mint,
        mint_authority: find_mint_authority_pda().0,
        token_program: f.token_program,
        buffer_state: f.buffer(),
        buffer_pricer: (!legacy).then(|| price(f.mint, 0)),
        main_offer: legacy.then(|| read::<GlobalState>(&f.svm, find_state_pda().0).main_offer),
        reserve_token: derive_ata(&reserve, &f.mint, &f.token_program),
        management_token: derive_ata(&management, &f.mint, &f.token_program),
        performance_token: derive_ata(&performance, &f.mint, &f.token_program),
    }
}
fn settle_ix(f: &Fixture, worker: Pubkey) -> Instruction {
    ix(
        accounts::SettleManagedTokenBuffer {
            buffer_accounts: parts(f),
            worker,
            system_program: SYSTEM_PROGRAM_ID,
            market_stats: find_market_stats_pda().0,
            circulating_supply_excluded_balance: find_circulating_supply_excluded_balance_pda().0,
        },
        instruction::SettleManagedTokenBuffer {},
    )
}
fn configure_ix(f: &Fixture, boss: Pubkey, settings: BufferSettings) -> Instruction {
    ix(
        accounts::SetManagedTokenBufferSettings {
            buffer_accounts: parts(f),
            boss,
            system_program: SYSTEM_PROGRAM_ID,
            market_stats: find_market_stats_pda().0,
            circulating_supply_excluded_balance: find_circulating_supply_excluded_balance_pda().0,
        },
        instruction::SetManagedTokenBufferSettings { settings },
    )
}
fn config() -> BufferSettings {
    BufferSettings {
        gross_apr: 100_000,
        management_fee_basis_points: 100,
        performance_fee_basis_points: 200,
        performance_fee_high_watermark_enabled: true,
    }
}
fn configure(f: &mut Fixture, settings: BufferSettings) {
    let instruction = configure_ix(f, f.boss.pubkey(), settings);
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
}
fn settle(f: &mut Fixture) {
    let instruction = settle_ix(f, f.boss.pubkey());
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
}
fn setup(onyc: bool, token_2022: bool) -> Fixture {
    let mut f = Fixture::new(onyc, token_2022);
    f.seed_tokens(f.mint, f.boss.pubkey(), 10_000_000_000);
    modular_prop_amm::init_buffer(&mut f);
    let instruction = build_set_worker_ix(&f.boss.pubkey(), &f.boss.pubkey());
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    configure(&mut f, config());
    f
}
fn check_error(
    result: Result<TransactionMetadata, FailedTransactionMetadata>,
    expected: OnreError,
) {
    let failed = result.unwrap_err();
    assert_eq!(
        failed.err,
        TransactionError::InstructionError(1, InstructionError::Custom(6000 + expected as u32)),
        "{:?}",
        failed.meta.logs
    );
}
fn native(f: &Fixture) -> (Pubkey, [Pubkey; 3]) {
    (
        pda(&[b"buffer", f.mint.as_ref()]),
        [
            pda(&[b"buffer_reserve", f.mint.as_ref()]),
            pda(&[b"buffer_management", f.mint.as_ref()]),
            pda(&[b"buffer_performance", f.mint.as_ref()]),
        ],
    )
}
fn prepare_native(f: &mut Fixture) {
    let (buffer, [reserve, management, performance]) = native(f);
    let instruction = ix(
        accounts::InitializeManagedTokenBuffer {
            managed_token: managed(f.mint),
            managed_mint: f.mint,
            pricer: price(f.mint, 0),
            buffer_state: buffer,
            reserve_vault: reserve,
            management_fee_vault: management,
            performance_fee_vault: performance,
            state: find_state_pda().0,
            boss: f.boss.pubkey(),
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::InitializeManagedTokenBuffer {},
    );
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
}
fn match_pricer(f: &mut Fixture) {
    edit::<Pricer>(&mut f.svm, price(f.mint, 0), |p| {
        p.vectors[0].base_price = 1_000_000_000
    });
}
fn cutover_ix(f: &Fixture, boss: Pubkey) -> Instruction {
    let (buffer, [reserve, management, performance]) = native(f);
    let main = read::<GlobalState>(&f.svm, find_state_pda().0).main_offer;
    ix(
        accounts::CutOverOnycBuffer {
            state: find_state_pda().0,
            managed_token: managed(f.mint),
            managed_mint: f.mint,
            mint_authority: find_mint_authority_pda().0,
            legacy_buffer_state: find_buffer_state_pda().0,
            buffer_state: buffer,
            main_offer: main,
            pricer: price(f.mint, 0),
            legacy_reserve_authority: find_reserve_vault_authority_pda().0,
            legacy_management_vault: find_management_fee_vault_pda().0,
            legacy_performance_vault: find_performance_fee_vault_pda().0,
            legacy_reserve_token: derive_ata(
                &find_reserve_vault_authority_pda().0,
                &f.mint,
                &f.token_program,
            ),
            legacy_management_token: derive_ata(
                &find_management_fee_vault_pda().0,
                &f.mint,
                &f.token_program,
            ),
            legacy_performance_token: derive_ata(
                &find_performance_fee_vault_pda().0,
                &f.mint,
                &f.token_program,
            ),
            reserve_vault: reserve,
            management_fee_vault: management,
            performance_fee_vault: performance,
            reserve_token: derive_ata(&reserve, &f.mint, &f.token_program),
            management_token: derive_ata(&management, &f.mint, &f.token_program),
            performance_token: derive_ata(&performance, &f.mint, &f.token_program),
            boss,
            token_program: f.token_program,
            associated_token_program: ATA_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
            market_stats: find_market_stats_pda().0,
            circulating_supply_excluded_balance: find_circulating_supply_excluded_balance_pda().0,
        },
        instruction::CutOverOnycBuffer {},
    )
}
fn cutover(f: &mut Fixture) {
    let instruction = cutover_ix(f, f.boss.pubkey());
    let budget = solana_compute_budget_interface::ComputeBudgetInstruction::set_compute_unit_limit(
        1_400_000,
    );
    let message =
        solana_sdk::message::Message::new(&[budget, instruction.clone()], Some(&f.boss.pubkey()));
    assert!(message.account_keys.len() <= 64);
    assert!(65 + VersionedMessage::Legacy(message).serialize().len() <= 1232);
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
}

#[test]
fn legacy_and_modular_operations_share_one_clock_settings_and_vaults() {
    let mut f = setup(true, false);
    prepare_native(&mut f);
    let (inactive, _) = native(&f);
    let inactive_before = f.svm.get_account(&inactive).unwrap().data;
    advance_clock_by(&mut f.svm, 3_153_600);
    settle(&mut f);
    assert_eq!(get_mint_supply(&f.svm, &f.mint), 10_100_000_000);
    let main = read::<GlobalState>(&f.svm, find_state_pda().0).main_offer;
    let instruction = build_settle_buffer_ix(&f.boss.pubkey(), &main, &f.mint);
    let before = f.svm.get_account(&find_buffer_state_pda().0).unwrap().data;
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    assert_eq!(
        f.svm.get_account(&find_buffer_state_pda().0).unwrap().data,
        before
    );
    advance_clock_by(&mut f.svm, 3_153_600);
    let instruction = build_settle_buffer_ix(&f.boss.pubkey(), &main, &f.mint);
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    let supply = get_mint_supply(&f.svm, &f.mint);
    settle(&mut f);
    assert_eq!(get_mint_supply(&f.svm, &f.mint), supply);
    assert_eq!(supply, 10_201_000_000);
    let instruction = build_set_buffer_gross_yield_ix(&f.boss.pubkey(), &main, &f.mint, 150_000);
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    let mut settings = config();
    settings.management_fee_basis_points = 200;
    settings.gross_apr = 150_000;
    configure(&mut f, settings);
    let state: BufferState = read(&f.svm, find_buffer_state_pda().0);
    assert_eq!(
        (state.gross_apr, state.management_fee_basis_points),
        (150_000, 200)
    );
    assert_eq!(state.previous_supply, supply);
    assert_eq!(f.svm.get_account(&inactive).unwrap().data, inactive_before);
    let mut spoof = settle_ix(&f, f.boss.pubkey());
    for account in &mut spoof.accounts {
        if account.pubkey == find_buffer_state_pda().0 {
            account.pubkey = inactive;
        }
    }
    check_error(
        send(&mut f.svm, &[spoof], &[&f.boss]),
        OnreError::InvalidBufferStateAccount,
    );
}

#[test]
fn native_buffer_settlement_and_policy_changes_preserve_supply_and_watermark() {
    for token_2022 in [false, true] {
        let mut f = setup(false, token_2022);
        advance_clock_by(&mut f.svm, 3_153_600);
        settle(&mut f);
        assert_eq!(get_mint_supply(&f.svm, &f.mint), 10_100_000_000);
        let [reserve, management, performance] = f.buffer_vaults();
        assert_eq!(f.balance(f.mint, reserve, f.token_program), 88_200_000);
        assert_eq!(f.balance(f.mint, management, f.token_program), 10_000_000);
        assert_eq!(f.balance(f.mint, performance, f.token_program), 1_800_000);
        settle(&mut f);
        assert_eq!(get_mint_supply(&f.svm, &f.mint), 10_100_000_000);
        advance_clock_by(&mut f.svm, 3_153_600);
        let mut settings = config();
        settings.gross_apr = 200_000;
        configure(&mut f, settings);
        assert_eq!(get_mint_supply(&f.svm, &f.mint), 10_201_000_000);
        let state: ManagedTokenBufferState = read(&f.svm, f.buffer());
        assert_eq!(state.performance_fee_high_watermark, 2_000_000_000);
        assert_eq!(state.gross_apr, 200_000);
    }
}

#[test]
fn atomic_cutover_moves_all_balances_and_preserves_final_legacy_accounting() {
    let mut f = setup(true, false);
    prepare_native(&mut f);
    match_pricer(&mut f);
    // Preexisting reserve and fees, as well as the final unpaid accrual, must move.
    for (vault, amount) in f
        .buffer_vaults()
        .into_iter()
        .zip([1_000_000, 2_000_000, 3_000_000])
    {
        f.seed_tokens(f.mint, vault, amount);
    }
    let before = get_mint_supply(&f.svm, &f.mint);
    edit::<BufferState>(&mut f.svm, find_buffer_state_pda().0, |b| {
        b.previous_supply = before
    });
    advance_clock_by(&mut f.svm, 3_153_600);
    let accrued = before / 100;
    cutover(&mut f);
    let old: BufferState = read(&f.svm, find_buffer_state_pda().0);
    let current: ManagedTokenBufferState = read(&f.svm, f.buffer());
    assert!(read::<GlobalState>(&f.svm, find_state_pda().0).onyc_buffer_cutover_completed);
    assert!(old.retired);
    assert_eq!(current.previous_supply, before + accrued);
    assert_eq!(current.last_accrual_timestamp, old.last_accrual_timestamp);
    assert_eq!(
        current.performance_fee_high_watermark,
        old.performance_fee_high_watermark
    );
    assert_eq!(
        (
            current.gross_apr,
            current.management_fee_basis_points,
            current.performance_fee_basis_points,
            current.performance_fee_high_watermark_enabled
        ),
        (
            old.gross_apr,
            old.management_fee_basis_points,
            old.performance_fee_basis_points,
            old.performance_fee_high_watermark_enabled
        )
    );
    let old_vaults = [
        find_reserve_vault_authority_pda().0,
        find_management_fee_vault_pda().0,
        find_performance_fee_vault_pda().0,
    ];
    let management = accrued / 10;
    let performance = (accrued - management) * 200 / 10000;
    for ((vault, prior), added) in f
        .buffer_vaults()
        .into_iter()
        .zip([1_000_000, 2_000_000, 3_000_000])
        .zip([accrued - management - performance, management, performance])
    {
        assert_eq!(f.balance(f.mint, vault, f.token_program), prior + added);
    }
    for vault in old_vaults {
        assert_eq!(f.balance(f.mint, vault, f.token_program), 0);
    }
    let after = get_mint_supply(&f.svm, &f.mint);
    settle(&mut f);
    assert_eq!(get_mint_supply(&f.svm, &f.mint), after);
    let main = read::<GlobalState>(&f.svm, find_state_pda().0).main_offer;
    let legacy = build_settle_buffer_ix(&f.boss.pubkey(), &main, &f.mint);
    check_error(
        send(&mut f.svm, &[legacy], &[&f.boss]),
        OnreError::LegacyBufferRetired,
    );
    let again = cutover_ix(&f, f.boss.pubkey());
    check_error(
        send(&mut f.svm, &[again], &[&f.boss]),
        OnreError::LegacyBufferRetired,
    );
    // After cutover, native Pricer drives BUFFER, TVL/reporting, while legacy
    // state and vaults stay frozen. Existing per-offer pricing remains independent.
    edit::<Pricer>(&mut f.svm, price(f.mint, 0), |p| {
        p.vectors[0].base_price = 1_100_000_000
    });
    advance_clock_by(&mut f.svm, 3_153_600);
    let instruction = f.take(100_000, 1);
    f.send(instruction).unwrap();
    let current: ManagedTokenBufferState = read(&f.svm, f.buffer());
    assert_eq!(current.previous_supply, get_mint_supply(&f.svm, &f.mint));
    assert_eq!(read_market_stats(&f.svm).nav, 1_100_000_000);
    let frozen = f.svm.get_account(&find_buffer_state_pda().0).unwrap().data;
    settle(&mut f);
    assert_eq!(
        f.svm.get_account(&find_buffer_state_pda().0).unwrap().data,
        frozen
    );
    let mut skipped = settle_ix(&f, f.boss.pubkey());
    for account in &mut skipped.accounts {
        if account.pubkey == f.buffer() {
            account.pubkey = find_buffer_state_pda().0;
        }
    }
    check_error(
        send(&mut f.svm, &[skipped], &[&f.boss]),
        OnreError::InvalidBufferStateAccount,
    );
}

#[test]
fn failed_cutover_rolls_back_accrual_transfers_ata_creation_and_route_switch() {
    let mut f = setup(true, false);
    prepare_native(&mut f);
    match_pricer(&mut f);
    advance_clock_by(&mut f.svm, 3_153_600);
    let before = f.svm.clone();
    let old = f.svm.get_account(&find_buffer_state_pda().0).unwrap().data;
    // Invalid reporting account fails after the state copy and transfers.
    let mut instruction = cutover_ix(&f, f.boss.pubkey());
    for account in &mut instruction.accounts {
        if account.pubkey == find_market_stats_pda().0 {
            account.pubkey = Pubkey::new_unique();
        }
    }
    assert!(send(&mut f.svm, &[instruction], &[&f.boss]).is_err());
    assert_eq!(
        f.svm.get_account(&find_buffer_state_pda().0).unwrap().data,
        old
    );
    assert_eq!(get_mint_supply(&f.svm, &f.mint), 10_000_000_000);
    assert!(!read::<GlobalState>(&f.svm, find_state_pda().0).onyc_buffer_cutover_completed);
    for vault in native(&f).1 {
        assert!(f
            .svm
            .get_account(&derive_ata(&vault, &f.mint, &f.token_program))
            .is_none());
    }
    f.svm = before;
    edit::<Pricer>(&mut f.svm, price(f.mint, 0), |p| {
        p.vectors[0].base_price = 2_000_000_000
    });
    let instruction = cutover_ix(&f, f.boss.pubkey());
    check_error(
        send(&mut f.svm, &[instruction], &[&f.boss]),
        OnreError::IncompatiblePricer,
    );
}

#[test]
fn buffer_operations_enforce_authority_kill_switch_identity_and_aggregate_caps() {
    for onyc in [false, true] {
        let mut f = setup(onyc, false);
        let rogue = Keypair::new();
        f.svm.airdrop(&rogue.pubkey(), INITIAL_LAMPORTS).unwrap();
        let instruction = settle_ix(&f, rogue.pubkey());
        check_error(
            send(&mut f.svm, &[instruction], &[&rogue]),
            OnreError::InvalidWorker,
        );
        let mut settings = config();
        settings.gross_apr = 200_000;
        let instruction = configure_ix(&f, rogue.pubkey(), settings);
        check_error(
            send(&mut f.svm, &[instruction], &[&rogue]),
            OnreError::InvalidBoss,
        );
        let mut invalid = settings;
        invalid.gross_apr = 1_000_001;
        let instruction = configure_ix(&f, f.boss.pubkey(), invalid);
        check_error(
            send(&mut f.svm, &[instruction], &[&f.boss]),
            OnreError::InvalidAPR,
        );
        invalid = settings;
        invalid.management_fee_basis_points = 10001;
        let instruction = configure_ix(&f, f.boss.pubkey(), invalid);
        check_error(
            send(&mut f.svm, &[instruction], &[&f.boss]),
            OnreError::InvalidFee,
        );
        let instruction = configure_ix(&f, f.boss.pubkey(), config());
        check_error(
            send(&mut f.svm, &[instruction], &[&f.boss]),
            OnreError::NoChange,
        );
        edit::<GlobalState>(&mut f.svm, find_state_pda().0, |s| s.is_killed = true);
        let instruction = settle_ix(&f, f.boss.pubkey());
        check_error(
            send(&mut f.svm, &[instruction], &[&f.boss]),
            OnreError::KillSwitchActivated,
        );
        edit::<GlobalState>(&mut f.svm, find_state_pda().0, |s| s.is_killed = false);
        advance_clock_by(&mut f.svm, 3_153_600);
        let before = f.svm.get_account(&f.buffer()).unwrap().data;
        edit::<ManagedTokenConfig>(&mut f.svm, managed(f.mint), |m| {
            m.max_mint_amount = 99_999_999
        });
        let instruction = settle_ix(&f, f.boss.pubkey());
        check_error(
            send(&mut f.svm, &[instruction], &[&f.boss]),
            OnreError::MaxMintAmountExceeded,
        );
        assert_eq!(f.svm.get_account(&f.buffer()).unwrap().data, before);
        assert_eq!(get_mint_supply(&f.svm, &f.mint), 10_000_000_000);
    }
    let mut f = setup(true, false);
    prepare_native(&mut f);
    match_pricer(&mut f);
    let rogue = Keypair::new();
    f.svm.airdrop(&rogue.pubkey(), INITIAL_LAMPORTS).unwrap();
    let instruction = cutover_ix(&f, rogue.pubkey());
    check_error(
        send(&mut f.svm, &[instruction], &[&rogue]),
        OnreError::InvalidBoss,
    );
}
