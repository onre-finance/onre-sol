use super::*;
use crate::common::send_tx;
use onreapp::{instructions::BufferState, state::MarketStats};
use solana_sdk::instruction::AccountMeta;

fn send(
    svm: &mut LiteSVM,
    instructions: &[Instruction],
    signers: &[&Keypair],
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    // Repeated settlements at one timestamp still need distinct transactions.
    advance_slot(svm);
    send_tx(svm, instructions, signers)
}

fn parts(f: &Fixture) -> accounts::ManagedBufferAccounts {
    let [reserve, management, performance] = f.buffer_vaults();
    accounts::ManagedBufferAccounts {
        state: find_state_pda().0,
        managed_token: managed(f.mint),
        managed_mint: f.mint,
        mint_authority: find_mint_authority_pda().0,
        mint_multisig: f.mint_multisig(),
        token_program: f.token_program,
        buffer_state: f.buffer(),
        buffer_pricer: Some(price(f.mint, 0)),
        reserve_token: derive_ata(&reserve, &f.mint, &f.token_program),
        management_token: derive_ata(&management, &f.mint, &f.token_program),
        performance_token: derive_ata(&performance, &f.mint, &f.token_program),
    }
}
fn settle_with_stats_ix(f: &Fixture, worker: Pubkey, market_stats: Pubkey) -> Instruction {
    ix(
        accounts::SettleManagedTokenBuffer {
            buffer_accounts: parts(f),
            worker,
            market_stats,
            legacy_market_stats: f.legacy_market_stats(),
            circulating_supply_excluded_balance: find_circulating_supply_excluded_balance_pda().0,
        },
        instruction::SettleManagedTokenBuffer {},
    )
}
fn settle_ix(f: &Fixture, worker: Pubkey) -> Instruction {
    settle_with_stats_ix(f, worker, f.market_stats())
}
fn configure_ix(f: &Fixture, boss: Pubkey, settings: BufferSettings) -> Instruction {
    ix(
        accounts::SetManagedTokenBufferSettings {
            buffer_accounts: parts(f),
            boss,
            market_stats: f.market_stats(),
            legacy_market_stats: f.legacy_market_stats(),
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
fn check_code(result: Result<TransactionMetadata, FailedTransactionMetadata>, code: u32) {
    let failed = result.unwrap_err();
    assert_eq!(
        failed.err,
        TransactionError::InstructionError(1, InstructionError::Custom(code)),
        "{:?}",
        failed.meta.logs
    );
}
fn check_error(
    result: Result<TransactionMetadata, FailedTransactionMetadata>,
    expected: OnreError,
) {
    check_code(result, 6000 + expected as u32);
}

fn per_mint_stats(f: &Fixture) -> Pubkey {
    pda(&[b"market_stats", f.mint.as_ref()])
}
fn init_stats_ix(f: &Fixture, boss: Pubkey) -> Instruction {
    ix(
        accounts::InitializeManagedTokenMarketStats {
            managed_token: managed(f.mint),
            market_stats: per_mint_stats(f),
            state: find_state_pda().0,
            boss,
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::InitializeManagedTokenMarketStats {},
    )
}
fn select_stats_ix(f: &Fixture, boss: Pubkey, market_stats: Pubkey) -> Instruction {
    ix(
        accounts::SetManagedTokenMarketStats {
            managed_token: managed(f.mint),
            market_stats,
            state: find_state_pda().0,
            boss,
        },
        instruction::SetManagedTokenMarketStats {},
    )
}
fn selected_stats(f: &Fixture) -> Pubkey {
    read::<ManagedTokenConfig>(&f.svm, managed(f.mint)).market_stats
}

fn switch_ix(f: &Fixture, boss: Pubkey) -> Instruction {
    let [reserve, management, performance] = [
        find_reserve_vault_authority_pda().0,
        find_management_fee_vault_pda().0,
        find_performance_fee_vault_pda().0,
    ]
    .map(|owner| derive_ata(&owner, &f.mint, &f.token_program));
    ix(
        accounts::SwitchOnycBufferToPerMint {
            state: find_state_pda().0,
            boss,
            managed_token: managed(f.mint),
            onyc_mint: f.mint,
            mint_authority: find_mint_authority_pda().0,
            mint_multisig: f.mint_multisig(),
            token_program: f.token_program,
            buffer_pricer: price(f.mint, 0),
            legacy_buffer_state: find_buffer_state_pda().0,
            legacy_reserve_token: reserve,
            legacy_management_token: management,
            legacy_performance_token: performance,
            buffer_state: pda(&[b"buffer", f.mint.as_ref()]),
        },
        instruction::SwitchOnycBufferToPerMint {},
    )
}

/// `supply × rate × elapsed / (year × 1e6)` with a zero Pricer APR.
fn accrual(supply: u64, rate: u64, elapsed: u64) -> u64 {
    (supply as u128 * rate as u128 * elapsed as u128 / 31_536_000_000_000) as u64
}

#[test]
fn onyc_switches_once_from_the_legacy_to_the_per_mint_buffer() {
    // Per-mint ONyc BUFFER initialized at 10%; go back to before the switch with an
    // active legacy singleton at 5%.
    let mut f = setup(true, false);
    f.set_onyc_per_mint_buffer_active(false);
    f.initialize_legacy_main_offer();
    let main = find_offer_pda(&f.asset, &f.mint).0;
    let boss = f.boss.pubkey();
    for instruction in [
        build_initialize_buffer_ix(&boss, &main, &f.mint),
        build_set_buffer_gross_yield_ix(&boss, &main, &f.mint, 50_000),
    ] {
        send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    }
    let legacy = find_buffer_state_pda().0;
    let per_mint = pda(&[b"buffer", f.mint.as_ref()]);
    let per_mint_before = f.svm.get_account(&per_mint).unwrap().data;

    // Before the switch the per-mint BUFFER cannot settle, and modular execution
    // accrues the active legacy singleton.
    let instruction = settle_ix(&f, boss);
    check_error(
        send(&mut f.svm, &[instruction], &[&f.boss]),
        OnreError::PerMintBufferInactive,
    );
    let start = get_mint_supply(&f.svm, &f.mint);
    advance_clock_by(&mut f.svm, 3_153_600);
    let instruction = f.take(2_000_000, 990_000_000);
    f.send(instruction).unwrap();
    let accrued = accrual(start, 50_000, 3_153_600);
    assert_eq!(
        get_mint_supply(&f.svm, &f.mint),
        start + accrued + 990_000_000
    );
    let state: BufferState = read(&f.svm, legacy);
    assert_eq!(state.previous_supply, get_mint_supply(&f.svm, &f.mint));
    assert_eq!(f.svm.get_account(&per_mint).unwrap().data, per_mint_before);

    // Only the boss switches; the legacy singleton accrues up to the switch and the
    // per-mint BUFFER takes over its settings from the current supply.
    let rogue = Keypair::new();
    f.svm.airdrop(&rogue.pubkey(), INITIAL_LAMPORTS).unwrap();
    let instruction = switch_ix(&f, rogue.pubkey());
    check_error(
        send(&mut f.svm, &[instruction], &[&rogue]),
        OnreError::InvalidBoss,
    );
    advance_clock_by(&mut f.svm, 3_153_600);
    let before_switch = get_mint_supply(&f.svm, &f.mint);
    let instruction = switch_ix(&f, boss);
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    let switched = get_mint_supply(&f.svm, &f.mint);
    assert_eq!(
        switched,
        before_switch + accrual(before_switch, 50_000, 3_153_600)
    );
    assert!(read::<GlobalState>(&f.svm, find_state_pda().0).onyc_per_mint_buffer_active);
    let state: BufferState = read(&f.svm, legacy);
    assert_eq!(state.previous_supply, switched);
    let taken_over: ManagedTokenBufferState = read(&f.svm, per_mint);
    assert_eq!(taken_over.previous_supply, switched);
    assert_eq!(taken_over.gross_apr, 50_000);
    assert_eq!(
        taken_over.last_accrual_timestamp,
        get_clock_time(&f.svm) as i64
    );

    // One way: no second switch, no more legacy settlement.
    let instruction = switch_ix(&f, boss);
    check_error(
        send(&mut f.svm, &[instruction], &[&f.boss]),
        OnreError::LegacyBufferRetired,
    );
    let instruction = build_settle_buffer_ix(&boss, &main, &f.mint);
    check_error(
        send(&mut f.svm, &[instruction], &[&f.boss]),
        OnreError::LegacyBufferRetired,
    );

    // Afterwards only the per-mint BUFFER accrues, at the taken-over 5%.
    let legacy_before = f.svm.get_account(&legacy).unwrap().data;
    advance_clock_by(&mut f.svm, 3_153_600);
    settle(&mut f);
    assert_eq!(
        get_mint_supply(&f.svm, &f.mint),
        switched + accrual(switched, 50_000, 3_153_600)
    );
    assert_eq!(f.svm.get_account(&legacy).unwrap().data, legacy_before);
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
fn per_mint_market_stats_are_opt_in_and_bound_to_the_selected_account() {
    for token_2022 in [false, true] {
        let mut f = setup(false, token_2022);
        assert_eq!(selected_stats(&f), Pubkey::default());
        // Unselected reporting ignores whatever account is supplied.
        advance_clock_by(&mut f.svm, 3_153_600);
        let instruction = settle_with_stats_ix(&f, f.boss.pubkey(), Pubkey::new_unique());
        send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
        assert_eq!(get_mint_supply(&f.svm, &f.mint), 10_100_000_000);

        let instruction = init_stats_ix(&f, f.boss.pubkey());
        send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
        assert_eq!(selected_stats(&f), per_mint_stats(&f));
        let stats: MarketStats = read(&f.svm, per_mint_stats(&f));
        assert_eq!((stats.nav, stats.last_updated_at), (0, 0));

        settle(&mut f);
        let stats: MarketStats = read(&f.svm, per_mint_stats(&f));
        assert_eq!(stats.nav, 2_000_000_000);
        assert_eq!(stats.apy, 0);
        assert_eq!(stats.circulating_supply, 10_100_000_000);
        assert_eq!(stats.tvl, 20_200_000_000);
        assert_eq!(stats.last_updated_at, get_clock_time(&f.svm) as i64);

        // Any other account, even a valid MarketStats, is rejected once selected.
        for substitute in [Pubkey::new_unique(), find_market_stats_pda().0] {
            let instruction = settle_with_stats_ix(&f, f.boss.pubkey(), substitute);
            check_error(
                send(&mut f.svm, &[instruction], &[&f.boss]),
                OnreError::InvalidMarketStatsPda,
            );
        }

        // Trades refresh the same selected account with post-trade supply.
        let instruction = f.take(2_000_000, 990_000_000);
        f.send(instruction).unwrap();
        let stats: MarketStats = read(&f.svm, per_mint_stats(&f));
        assert_eq!(stats.circulating_supply, get_mint_supply(&f.svm, &f.mint));
        assert_eq!(stats.circulating_supply, 11_090_000_000);
    }
}

#[test]
fn onyc_can_select_the_legacy_market_stats_shared_with_legacy_refresh() {
    let mut f = setup(true, false);
    f.initialize_legacy_main_offer(); // legacy NAV 1; canonical modular Pricer NAV 2
    let legacy_stats = find_market_stats_pda().0;
    // Any address works, but it must already deserialize as MarketStats.
    let instruction = select_stats_ix(&f, f.boss.pubkey(), legacy_stats);
    check_code(send(&mut f.svm, &[instruction], &[&f.boss]), 3012);
    let instruction = select_stats_ix(&f, f.boss.pubkey(), price(f.mint, 0));
    check_code(send(&mut f.svm, &[instruction], &[&f.boss]), 3002);

    let instruction = build_refresh_market_stats_ix(&f.boss.pubkey(), &f.asset, &f.mint);
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    assert_eq!(read_market_stats(&f.svm).nav, 1_000_000_000);
    // Accounts deserialize before constraints, so the boss check needs a valid account.
    let rogue = Keypair::new();
    f.svm.airdrop(&rogue.pubkey(), INITIAL_LAMPORTS).unwrap();
    let instruction = select_stats_ix(&f, rogue.pubkey(), legacy_stats);
    check_error(
        send(&mut f.svm, &[instruction], &[&rogue]),
        OnreError::InvalidBoss,
    );
    let instruction = select_stats_ix(&f, f.boss.pubkey(), legacy_stats);
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    assert_eq!(selected_stats(&f), legacy_stats);
    let instruction = select_stats_ix(&f, f.boss.pubkey(), legacy_stats);
    check_error(
        send(&mut f.svm, &[instruction], &[&f.boss]),
        OnreError::NoChange,
    );

    // Modular settlement writes the shared account from the canonical Pricer and the
    // ONyc excluded-balance cache; the legacy refresh later writes it from main offer.
    advance_clock_by(&mut f.svm, 3_153_600);
    settle(&mut f);
    let stats = read_market_stats(&f.svm);
    assert_eq!(stats.nav, 2_000_000_000);
    assert_eq!(stats.circulating_supply, get_mint_supply(&f.svm, &f.mint));
    let bump = stats.bump;
    let instruction = build_refresh_market_stats_ix(&f.boss.pubkey(), &f.asset, &f.mint);
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    let stats = read_market_stats(&f.svm);
    assert_eq!(stats.nav, 1_000_000_000);
    assert_eq!(stats.bump, bump);
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
        // The canonical per-mint state cannot be swapped, not even for ONyc's legacy one.
        let mut spoof = settle_ix(&f, f.boss.pubkey());
        for account in &mut spoof.accounts {
            if account.pubkey == f.buffer() {
                account.pubkey = find_buffer_state_pda().0;
            }
        }
        check_error(
            send(&mut f.svm, &[spoof], &[&f.boss]),
            OnreError::InvalidBufferStateAccount,
        );
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
}

#[test]
fn standalone_buffer_rejects_active_transfer_fees_without_changing_accounting() {
    let mut f = setup(false, true);
    let fee_mint = create_mint_2022_with_transfer_fee(
        &mut f.svm,
        &f.boss,
        9,
        &find_mint_authority_pda().0,
        100,
        1000,
    );
    // Enable fees on the registered mint while retaining its supply and identity.
    let supply = get_mint_supply(&f.svm, &f.mint);
    let mut account = f.svm.get_account(&fee_mint).unwrap();
    account.data[36..44].copy_from_slice(&supply.to_le_bytes());
    f.svm.set_account(f.mint, account).unwrap();
    advance_clock_by(&mut f.svm, 3_153_600);
    let before = f.svm.get_account(&f.buffer()).unwrap().data;
    let instruction = settle_ix(&f, f.boss.pubkey());
    check_error(
        send(&mut f.svm, &[instruction], &[&f.boss]),
        OnreError::TransferFeeNotSupported,
    );
    let mut settings = config();
    settings.gross_apr = 200_000;
    let instruction = configure_ix(&f, f.boss.pubkey(), settings);
    check_error(
        send(&mut f.svm, &[instruction], &[&f.boss]),
        OnreError::TransferFeeNotSupported,
    );
    assert_eq!(f.svm.get_account(&f.buffer()).unwrap().data, before);
    assert_eq!(get_mint_supply(&f.svm, &f.mint), supply);
}

#[test]
fn onyc_prop_amm_uses_canonical_pricer_nav_for_refill_curve_caps_and_reporting() {
    for direction in [
        OfferDirection::AssetToManaged,
        OfferDirection::ManagedToAsset,
    ] {
        let mut f = setup(true, false);
        if direction == OfferDirection::ManagedToAsset {
            f.seed_tokens(f.mint, f.user.pubkey(), 1_000_000_000);
        }
        // ONyc may also report into its own per-mint MarketStats account.
        let instruction = init_stats_ix(&f, f.boss.pubkey());
        send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
        // Canonical token-wide NAV is 2, offer quote NAV is 4.
        f.create_route(
            direction,
            OfferFlow::Permissionless,
            QuoterKind::PropAmm,
            true,
        );
        let selected = vault(VaultKind::Liquidity);
        edit::<VaultConfig>(&mut f.svm, selected, |v| v.refill_target_bps = 5000);
        f.seed_tokens(f.asset, selected, 6_000_000);
        let offer: OfferConfig = read(&f.svm, f.offer);
        edit::<PropAmmState>(&mut f.svm, offer.pressure_state, |p| {
            p.config.curve_exponent_scaled = 20_000;
            p.config.curve_peg_haircut_bps = 1000;
            p.config.wall_sensitivity_scaled = 0;
            p.config.cadence_wave_scaled = 0;
        });
        let alternate = modular_prop_amm::alternate_pricer(&mut f);
        edit::<OfferConfig>(&mut f.svm, f.offer, |o| o.pricer = alternate);
        let (gross, expected, refill) = match direction {
            // Buy: 1.98 / 6 utilization; 10% * utilization^2 haircut on 0.495 tokens.
            OfferDirection::AssetToManaged => (2_000_000, 489_609_450, 1_980_000),
            // Sell: 0.99 / 6 utilization; same haircut on 990,000 asset base units.
            OfferDirection::ManagedToAsset => (250_000_000, 987_304, 0),
        };
        let mut instruction = f.take(gross, expected);
        instruction.accounts[2].pubkey = alternate;
        let event = executed_event(&f.send_without_approval(instruction).unwrap());
        assert_eq!(event.amount_out, expected);
        assert_eq!(event.price, 4_000_000_000);
        assert_eq!(event.liquidity_refill_amount, refill);
        let stats: MarketStats = read(&f.svm, per_mint_stats(&f));
        assert_eq!(stats.nav, 2_000_000_000);
        let state: ManagedTokenBufferState = read(&f.svm, f.buffer());
        assert_eq!(state.previous_supply, get_mint_supply(&f.svm, &f.mint));
    }
}

/// Replaces the `account` meta in a built take instruction.
fn with_account(
    mut instruction: Instruction,
    account: Pubkey,
    replacement: AccountMeta,
) -> Instruction {
    let metas: Vec<_> = instruction
        .accounts
        .iter_mut()
        .filter(|meta| meta.pubkey == account)
        .collect();
    assert_eq!(metas.len(), 1);
    for meta in metas {
        *meta = replacement.clone();
    }
    instruction
}

#[test]
fn onyc_take_refreshes_selected_and_legacy_market_stats() {
    let mut f = setup(true, false);
    f.initialize_legacy_main_offer(); // legacy NAV 1; canonical modular Pricer NAV 2
    let instruction = init_stats_ix(&f, f.boss.pubkey());
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    let legacy_stats = find_market_stats_pda().0;
    assert!(f.svm.get_account(&legacy_stats).is_none());

    // ONyc cannot omit or substitute the legacy singleton.
    let missing = with_account(
        f.take(2_000_000, 0),
        legacy_stats,
        AccountMeta::new_readonly(PROGRAM_ID, false),
    );
    assert_error(f.send(missing), OnreError::LegacyMarketStatsRequired);
    let substitute = with_account(
        f.take(2_000_000, 0),
        legacy_stats,
        AccountMeta::new(Pubkey::new_unique(), false),
    );
    assert_error(f.send(substitute), OnreError::InvalidMarketStatsOwner);

    // One trade refreshes both accounts from the canonical Pricer with post-trade
    // supply; the singleton is created on first use.
    let instruction = f.take(2_000_000, 0);
    f.send(instruction).unwrap();
    let supply = get_mint_supply(&f.svm, &f.mint);
    let now = get_clock_time(&f.svm) as i64;
    for account in [per_mint_stats(&f), legacy_stats] {
        let stats: MarketStats = read(&f.svm, account);
        assert_eq!(stats.nav, 2_000_000_000);
        assert_eq!(stats.circulating_supply, supply);
        assert_eq!(stats.tvl, supply * 2);
        assert_eq!(stats.last_updated_at, now);
    }
    assert_eq!(read_market_stats(&f.svm).bump, find_market_stats_pda().1);

    // Legacy refresh keeps writing only the singleton, from the main offer.
    let instruction = build_refresh_market_stats_ix(&f.boss.pubkey(), &f.asset, &f.mint);
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    assert_eq!(read_market_stats(&f.svm).nav, 1_000_000_000);
    let per_mint: MarketStats = read(&f.svm, per_mint_stats(&f));
    assert_eq!(per_mint.nav, 2_000_000_000);

    // Selecting the singleton itself still refreshes it from the Pricer.
    let instruction = select_stats_ix(&f, f.boss.pubkey(), legacy_stats);
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    let instruction = f.take(2_000_000, 0);
    f.send(instruction).unwrap();
    let stats = read_market_stats(&f.svm);
    assert_eq!(stats.nav, 2_000_000_000);
    assert_eq!(stats.circulating_supply, get_mint_supply(&f.svm, &f.mint));
}

#[test]
fn non_onyc_take_does_not_require_or_touch_legacy_market_stats() {
    let mut f = setup(false, false);
    let instruction = init_stats_ix(&f, f.boss.pubkey());
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    let instruction = f.take(2_000_000, 0);
    f.send(instruction).unwrap();
    let stats: MarketStats = read(&f.svm, per_mint_stats(&f));
    assert_eq!(stats.circulating_supply, get_mint_supply(&f.svm, &f.mint));
    assert!(f.svm.get_account(&find_market_stats_pda().0).is_none());
}

#[test]
fn onyc_buffer_operations_refresh_legacy_market_stats_once_created() {
    let mut f = setup(true, false); // settings were configured before the singleton existed
    let instruction = init_stats_ix(&f, f.boss.pubkey());
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    let legacy_stats = find_market_stats_pda().0;
    assert!(f.svm.get_account(&legacy_stats).is_none());

    // ONyc cannot omit or substitute the legacy singleton.
    advance_clock_by(&mut f.svm, 3_153_600);
    let missing = with_account(
        settle_ix(&f, f.boss.pubkey()),
        legacy_stats,
        AccountMeta::new_readonly(PROGRAM_ID, false),
    );
    check_error(
        send(&mut f.svm, &[missing], &[&f.boss]),
        OnreError::LegacyMarketStatsRequired,
    );
    let substitute = with_account(
        configure_ix(
            &f,
            f.boss.pubkey(),
            BufferSettings {
                gross_apr: 200_000,
                ..config()
            },
        ),
        legacy_stats,
        AccountMeta::new(Pubkey::new_unique(), false),
    );
    check_error(
        send(&mut f.svm, &[substitute], &[&f.boss]),
        OnreError::InvalidMarketStatsOwner,
    );

    // Without a payer, BUFFER operations do not create the singleton.
    settle(&mut f);
    assert!(f.svm.get_account(&legacy_stats).is_none());
    let stats: MarketStats = read(&f.svm, per_mint_stats(&f));
    assert_eq!(stats.circulating_supply, get_mint_supply(&f.svm, &f.mint));

    // Once a trade creates it, settlement and settings refresh it with the accrued
    // supply, alongside the selected per-mint account.
    let instruction = f.take(2_000_000, 0);
    f.send(instruction).unwrap();
    for operation in 0..2 {
        advance_clock_by(&mut f.svm, 3_153_600);
        if operation == 0 {
            settle(&mut f);
        } else {
            configure(
                &mut f,
                BufferSettings {
                    gross_apr: 200_000,
                    ..config()
                },
            );
        }
        let supply = get_mint_supply(&f.svm, &f.mint);
        let now = get_clock_time(&f.svm) as i64;
        for account in [per_mint_stats(&f), legacy_stats] {
            let stats: MarketStats = read(&f.svm, account);
            assert_eq!(stats.nav, 2_000_000_000);
            assert_eq!(stats.circulating_supply, supply);
            assert_eq!(stats.last_updated_at, now);
        }
    }
}

/// Settle with an explicit `mint_multisig` account, overriding the configured one.
fn settle_with_mint_multisig_ix(f: &Fixture, mint_multisig: Option<Pubkey>) -> Instruction {
    let mut buffer_accounts = parts(f);
    buffer_accounts.mint_multisig = mint_multisig;
    ix(
        accounts::SettleManagedTokenBuffer {
            buffer_accounts,
            worker: f.boss.pubkey(),
            market_stats: f.market_stats(),
            legacy_market_stats: f.legacy_market_stats(),
            circulating_supply_excluded_balance: find_circulating_supply_excluded_balance_pda().0,
        },
        instruction::SettleManagedTokenBuffer {},
    )
}

#[test]
fn buffer_operations_mint_directly_or_through_the_multisig_as_configured() {
    use super::mint_multisig::{create_multisig, expect_error, switch_to_multisig, toggle_ix};
    for token_2022 in [false, true] {
        let mut f = setup(false, token_2022);
        let pda = find_mint_authority_pda().0;
        let multisig = create_multisig(f.token_program, &mut f, 1, &[Pubkey::new_unique(), pda]);
        let accrue = |f: &mut Fixture, operation: &dyn Fn(&mut Fixture)| {
            let before = get_mint_supply(&f.svm, &f.mint);
            advance_clock_by(&mut f.svm, 3_153_600);
            operation(f);
            assert!(get_mint_supply(&f.svm, &f.mint) > before);
        };

        // Direct: the PDA mints accrual, and a supplied multisig is rejected.
        let instruction = settle_with_mint_multisig_ix(&f, Some(multisig));
        expect_error(
            send(&mut f.svm, &[instruction], &[&f.boss]),
            OnreError::UnexpectedMintMultisig,
        );
        accrue(&mut f, &|f| settle(f));
        assert_eq!(get_mint_authority_pubkey(&f.svm, &f.mint), Some(pda));

        // Multisig: settlement and settings changes mint through the configured multisig,
        // which must be supplied.
        switch_to_multisig(&mut f, multisig).unwrap();
        let instruction = settle_with_mint_multisig_ix(&f, None);
        expect_error(
            send(&mut f.svm, &[instruction], &[&f.boss]),
            OnreError::MintMultisigRequired,
        );
        accrue(&mut f, &|f| settle(f));
        accrue(&mut f, &|f| {
            configure(
                f,
                BufferSettings {
                    gross_apr: 200_000,
                    ..config()
                },
            )
        });
        assert_eq!(get_mint_authority_pubkey(&f.svm, &f.mint), Some(multisig));

        // Back to direct: authority returns to the PDA, which mints accrual again.
        let instruction = toggle_ix(&f, f.boss.pubkey(), multisig, false);
        send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
        accrue(&mut f, &|f| settle(f));
        assert_eq!(get_mint_authority_pubkey(&f.svm, &f.mint), Some(pda));
    }
}

fn burn_for_nav_ix(
    f: &Fixture,
    boss: Pubkey,
    reserve_vault: Pubkey,
    adjustment: u64,
) -> Instruction {
    ix(
        accounts::BurnManagedTokenForNavIncrease {
            buffer_accounts: parts(f),
            boss,
            reserve_vault,
            market_stats: f.market_stats(),
            legacy_market_stats: f.legacy_market_stats(),
            circulating_supply_excluded_balance: find_circulating_supply_excluded_balance_pda().0,
        },
        instruction::BurnManagedTokenForNavIncrease {
            asset_adjustment_amount: adjustment,
        },
    )
}

#[test]
fn boss_burns_buffer_reserve_to_keep_nav_after_an_asset_adjustment() {
    for (onyc, token_2022) in [(false, false), (false, true), (true, false)] {
        let mut f = setup(onyc, token_2022);
        let instruction = init_stats_ix(&f, f.boss.pubkey());
        send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
        advance_clock_by(&mut f.svm, 3_153_600);
        settle(&mut f);
        let [reserve_vault, management_vault, _] = f.buffer_vaults();
        let reserve_token = derive_ata(&reserve_vault, &f.mint, &f.token_program);
        let reserve = get_token_balance(&f.svm, &reserve_token);
        let supply = get_mint_supply(&f.svm, &f.mint);
        assert!(reserve > 0);

        // Rejections: boss only, nonzero adjustment within total assets, enough
        // reserve, and the BUFFER's own reserve vault.
        let rogue = Keypair::new();
        f.svm.airdrop(&rogue.pubkey(), INITIAL_LAMPORTS).unwrap();
        let instruction = burn_for_nav_ix(&f, rogue.pubkey(), reserve_vault, 20_000_000);
        check_error(
            send(&mut f.svm, &[instruction], &[&rogue]),
            OnreError::InvalidBoss,
        );
        for (adjustment, error) in [
            (0, OnreError::InvalidAssetAdjustmentAmount),
            (supply * 2 + 1, OnreError::InvalidAssetAdjustmentAmount),
            (supply, OnreError::InsufficientCacheBalance),
        ] {
            let instruction = burn_for_nav_ix(&f, f.boss.pubkey(), reserve_vault, adjustment);
            check_error(send(&mut f.svm, &[instruction], &[&f.boss]), error);
        }
        let instruction = burn_for_nav_ix(&f, f.boss.pubkey(), management_vault, 20_000_000);
        check_error(
            send(&mut f.svm, &[instruction], &[&f.boss]),
            OnreError::InvalidConfigurableVault,
        );

        // NAV 2: removing 0.02 of assets (20_000_000 in 9-decimal units) burns
        // ceil-adjusted supply worth exactly that at the unchanged NAV.
        let total_assets = supply as u128 * 2;
        let required = (total_assets - 20_000_000).div_ceil(2) as u64;
        let expected_burn = supply - required;
        let instruction = burn_for_nav_ix(&f, f.boss.pubkey(), reserve_vault, 20_000_000);
        send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
        assert_eq!(expected_burn, 10_000_000);
        assert_eq!(get_mint_supply(&f.svm, &f.mint), supply - expected_burn);
        assert_eq!(
            get_token_balance(&f.svm, &reserve_token),
            reserve - expected_burn
        );
        let state: ManagedTokenBufferState = read(&f.svm, f.buffer());
        assert_eq!(state.previous_supply, supply - expected_burn);
        let stats: MarketStats = read(&f.svm, per_mint_stats(&f));
        assert_eq!(stats.nav, 2_000_000_000);
        assert_eq!(stats.circulating_supply, supply - expected_burn);
    }
}
