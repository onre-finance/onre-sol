use super::*;

fn pair(f: &Fixture) -> Pubkey {
    let offer: OfferConfig = read(&f.svm, f.offer);
    offer.pressure_state
}

fn routes(f: &mut Fixture, source: usize) -> (Pubkey, Pubkey, Pubkey) {
    let selected = match source {
        0 => {
            f.create_route(
                OfferDirection::ManagedToAsset,
                OfferFlow::Permissionless,
                QuoterKind::PropAmm,
                true,
            );
            vault(VaultKind::Liquidity)
        }
        1 => {
            let selected = find_prop_amm_proceeds_vault_pda().0;
            let setup = build_set_configurable_vault_destination_ix(
                &f.boss.pubkey(),
                &selected,
                5,
                &f.boss.pubkey(),
            );
            send_tx(&mut f.svm, &[setup], &[&f.boss]).unwrap();
            f.create_route_with_vault(
                OfferDirection::ManagedToAsset,
                OfferFlow::Permissionless,
                QuoterKind::PropAmm,
                Some(selected),
            );
            selected
        }
        _ => {
            let selected = find_redemption_vault_authority_pda().0;
            f.create_route_with_vault(
                OfferDirection::ManagedToAsset,
                OfferFlow::Permissionless,
                QuoterKind::PropAmm,
                Some(selected),
            );
            selected
        }
    };
    let sell = f.offer;
    f.create_route_with_vault(
        OfferDirection::AssetToManaged,
        OfferFlow::Permissionless,
        QuoterKind::PropAmm,
        Some(selected),
    );
    (f.offer, sell, selected)
}

pub(super) fn init_buffer(f: &mut Fixture) {
    if f.onyc {
        // Legacy MakeOffer uses one token program for both sides. ONyc's main
        // offer therefore uses an SPL asset even when the modular market uses
        // Token-2022. Canonical valuation is independent of the traded asset.
        let modular_asset = (f.asset, f.asset_program);
        if f.asset_program == TOKEN_2022_PROGRAM_ID {
            f.asset = create_mint(&mut f.svm, &f.boss, 6, &f.boss.pubkey());
            f.asset_program = TOKEN_PROGRAM_ID;
        }
        f.initialize_legacy_main_offer();
        let main_offer = read::<GlobalState>(&f.svm, find_state_pda().0).main_offer;
        (f.asset, f.asset_program) = modular_asset;
        let instruction = build_initialize_buffer_ix(&f.boss.pubkey(), &main_offer, &f.mint);
        send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    } else {
        f.initialize_managed_buffer();
    }
}

#[test]
fn one_prop_quoter_prices_both_directions_on_curve_from_every_custody_type() {
    for token_2022 in [false, true] {
        for source in 0..3 {
            for onyc in [false, true] {
                // ONyc's existing mint is SPL Token; external assets can be Token-2022.
                let mut f = Fixture::new(onyc, token_2022);
                f.seed_tokens(f.mint, f.boss.pubkey(), 10_000_000_000);
                init_buffer(&mut f);
                let (buy, sell, selected) = routes(&mut f, source);
                f.seed_tokens(f.asset, selected, 20_000_000);
                let tracker = pair(&f);
                let instruction = f.take(2_000_000, 1);
                let buy_event = executed_event(&f.send_without_approval(instruction).unwrap());
                assert!(buy_event.amount_out > 495_000_000 && buy_event.amount_out < 990_000_000);
                assert_eq!(buy_event.price, 2_000_000_000);
                assert_eq!(buy_event.proceeds_amount, 1_980_000);
                assert_eq!(buy_event.liquidity_refill_amount, 0);
                let state: PropAmmState = read(&f.svm, tracker);
                assert_eq!(state.current_buy_value_stable, 1_980_000);
                assert_eq!(state.current_sell_trade_count, 0);

                f.offer = sell;
                let instruction = f.take(495_000_000, 1);
                let sell_event = executed_event(&f.send_without_approval(instruction).unwrap());
                assert!(sell_event.amount_out > 0 && sell_event.amount_out < 980_100);
                let state: PropAmmState = read(&f.svm, tracker);
                assert_eq!(state.current_sell_value_stable, 980_100);
                assert_eq!(state.current_buy_value_stable, 1_980_000);
                assert_eq!(state.current_sell_trade_count, 1);
                assert_eq!(
                    f.balance(f.asset, selected, f.asset_program),
                    20_000_000 - sell_event.amount_out
                );
                assert_eq!(sell_event.source_amount, sell_event.amount_out);
                assert_eq!(sell_event.proceeds_amount, 0);
                assert_eq!(
                    f.balance(f.mint, f.user.pubkey(), f.token_program),
                    buy_event.amount_out - 495_000_000
                );
                let supply = get_mint_supply(&f.svm, &f.mint);
                if onyc {
                    assert_eq!(
                        read::<onreapp::instructions::BufferState>(&f.svm, f.buffer())
                            .previous_supply,
                        supply
                    );
                } else {
                    assert_eq!(
                        read::<ManagedTokenBufferState>(&f.svm, f.buffer()).previous_supply,
                        supply
                    );
                }
                assert_eq!(
                    read::<OfferConfig>(&f.svm, buy).quoter,
                    read::<OfferConfig>(&f.svm, sell).quoter
                );
            }
        }
    }
}

#[test]
fn buys_refill_only_the_selected_vault_before_mint_and_route_excess_to_proceeds() {
    for token_2022 in [false, true] {
        for (flow, kind) in [
            (OfferFlow::Permissioned, QuoterKind::Nav),
            (OfferFlow::Permissionless, QuoterKind::NavPermissionless),
        ] {
            for (existing, expected) in [(0, 1_980_000), (500_000, 1_500_000), (3_000_000, 0)] {
                let mut f = Fixture::new(false, token_2022);
                f.seed_tokens(f.mint, f.boss.pubkey(), 10_000_000_000);
                f.create_route(OfferDirection::AssetToManaged, flow, kind, true);
                edit::<VaultConfig>(&mut f.svm, vault(VaultKind::Liquidity), |v| {
                    v.refill_target_bps = 1000
                });
                if existing > 0 {
                    f.seed_tokens(f.asset, vault(VaultKind::Liquidity), existing);
                }
                let instruction = f.take(2_000_000, 990_000_000);
                let result = if flow == OfferFlow::Permissioned {
                    f.send(instruction)
                } else {
                    f.send_without_approval(instruction)
                }
                .unwrap();
                let event = executed_event(&result);
                assert_eq!(event.liquidity_refill_amount, expected);
                assert_eq!(event.proceeds_amount, 1_980_000 - expected);
                assert_eq!(
                    f.balance(f.asset, vault(VaultKind::Liquidity), f.asset_program),
                    existing + expected
                );
                assert_eq!(
                    f.balance(f.asset, vault(VaultKind::Proceeds), f.asset_program),
                    1_980_000 - expected
                );
                assert_eq!(
                    f.balance(f.mint, f.user.pubkey(), f.token_program),
                    990_000_000
                );
            }
        }
    }
}

#[test]
fn refill_uses_post_buffer_supply_but_does_not_count_the_pending_user_mint() {
    let mut f = Fixture::new(false, false);
    f.seed_tokens(f.mint, f.boss.pubkey(), 10_000_000_000);
    init_buffer(&mut f);
    let buffer = f.buffer();
    edit::<ManagedTokenBufferState>(&mut f.svm, buffer, |b| b.gross_apr = 100_000);
    f.create_route(
        OfferDirection::AssetToManaged,
        OfferFlow::Permissionless,
        QuoterKind::NavPermissionless,
        true,
    );
    edit::<VaultConfig>(&mut f.svm, vault(VaultKind::Liquidity), |v| {
        v.refill_target_bps = 1000
    });
    advance_clock_by(&mut f.svm, 31_536_000);
    let instruction = f.take(5_000_000, 1);
    let event = executed_event(&f.send_without_approval(instruction).unwrap());
    assert_eq!(event.liquidity_refill_amount, 2_200_000);
    assert_eq!(event.proceeds_amount, 2_750_000);
    assert_eq!(
        f.balance(f.mint, f.buffer_vaults()[0], f.token_program),
        1_000_000_000
    );
    assert_eq!(
        read::<ManagedTokenBufferState>(&f.svm, buffer).previous_supply,
        13_475_000_000
    );
}

#[test]
fn failed_prop_execution_rolls_back_pressure_buffer_and_refill() {
    let mut f = Fixture::new(false, false);
    f.seed_tokens(f.mint, f.boss.pubkey(), 10_000_000_000);
    init_buffer(&mut f);
    let buffer = f.buffer();
    edit::<ManagedTokenBufferState>(&mut f.svm, buffer, |b| b.gross_apr = 100_000);
    f.create_route(
        OfferDirection::AssetToManaged,
        OfferFlow::Permissionless,
        QuoterKind::PropAmm,
        true,
    );
    edit::<VaultConfig>(&mut f.svm, vault(VaultKind::Liquidity), |v| {
        v.refill_target_bps = 5000
    });
    f.seed_tokens(f.asset, vault(VaultKind::Liquidity), 6_000_000);
    advance_clock_by(&mut f.svm, 31_536_000);
    let tracker = pair(&f);
    let state_before = f.svm.get_account(&tracker).unwrap().data;
    let buffer_before = f.svm.get_account(&buffer).unwrap().data;
    // BUFFER accrual fits, but user mint exceeds the cap after asset transfers/refill.
    edit::<ManagedTokenConfig>(&mut f.svm, managed(f.mint), |m| {
        m.max_supply = 11_100_000_000
    });
    let instruction = f.take(5_000_000, 1);
    let error = f.send_without_approval(instruction).unwrap_err();
    assert_eq!(
        error.err,
        TransactionError::InstructionError(
            1,
            InstructionError::Custom(6000 + OnreError::MaxSupplyExceeded as u32)
        )
    );
    assert_eq!(f.svm.get_account(&tracker).unwrap().data, state_before);
    assert_eq!(f.svm.get_account(&buffer).unwrap().data, buffer_before);
    assert_eq!(get_mint_supply(&f.svm, &f.mint), 10_000_000_000);
    assert_eq!(
        f.balance(f.asset, f.user.pubkey(), f.asset_program),
        10_000_000
    );
    assert_eq!(
        f.balance(f.asset, vault(VaultKind::Liquidity), f.asset_program),
        6_000_000
    );
}

#[test]
fn sell_solvent_at_discount_still_rejects_insufficient_raw_liquidity_and_slippage() {
    let mut f = Fixture::new(false, false);
    f.seed_tokens(f.mint, f.user.pubkey(), 1_000_000_000);
    let (_, sell, selected) = routes(&mut f, 0);
    f.offer = sell;
    f.seed_tokens(f.asset, selected, 1_000_000);
    let tracker = pair(&f);
    let before = f.svm.get_account(&tracker).unwrap().data;
    let instruction = f.take(1_000_000_000, 0);
    assert!(f.send_without_approval(instruction).is_err());
    f.seed_tokens(f.asset, selected, 10_000_000);
    let instruction = f.take(1_000_000_000, 1_980_000);
    let error = f.send_without_approval(instruction).unwrap_err();
    assert_eq!(
        error.err,
        TransactionError::InstructionError(
            1,
            InstructionError::Custom(6000 + OnreError::MinimumOutNotMet as u32)
        )
    );
    assert_eq!(f.svm.get_account(&tracker).unwrap().data, before);
    assert_eq!(
        f.balance(f.mint, f.user.pubkey(), f.token_program),
        1_000_000_000
    );
}

#[test]
fn independent_quoters_do_not_share_pressure_even_when_they_share_custody() {
    let mut f = Fixture::new(false, false);
    let (buy, _, selected) = routes(&mut f, 0);
    let original = pair(&f);
    // A second real quoter instance for the same pair has independent state.
    let quoter = pda(&[
        b"quoter",
        &[QuoterKind::PropAmm as u8],
        &1_u64.to_le_bytes(),
    ]);
    let second = pda(&[b"prop_amm", quoter.as_ref()]);
    let config: PropAmmState = read(&f.svm, original);
    for instruction in [
        ix(
            accounts::CreateQuoter {
                quoter,
                state: find_state_pda().0,
                boss: f.boss.pubkey(),
                system_program: SYSTEM_PROGRAM_ID,
            },
            instruction::CreateQuoter {
                kind: QuoterKind::PropAmm,
                instance_id: 1,
            },
        ),
        ix(
            accounts::CreatePropAmmState {
                quoter,
                managed_token: managed(f.mint),
                asset_mint: f.asset,
                prop_amm_state: second,
                state: find_state_pda().0,
                boss: f.boss.pubkey(),
                system_program: SYSTEM_PROGRAM_ID,
            },
            instruction::CreatePropAmmState {
                config: config.config,
            },
        ),
    ] {
        send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    }
    f.offer = buy;
    f.seed_tokens(f.asset, selected, 20_000_000);
    let instruction = f.take(2_000_000, 1);
    f.send_without_approval(instruction).unwrap();
    assert_eq!(
        read::<PropAmmState>(&f.svm, second).current_buy_value_stable,
        0
    );
    assert_eq!(
        read::<PropAmmState>(&f.svm, original).current_buy_value_stable,
        1_980_000
    );
    assert_eq!(f.balance(f.asset, selected, f.asset_program), 20_000_000);
}

fn alternate_pricer(f: &mut Fixture) -> Pubkey {
    let selected = price(f.mint, 1);
    let now = get_clock_time(&f.svm);
    for instruction in [
        ix(
            accounts::CreatePricer {
                managed_token: managed(f.mint),
                pricer: selected,
                state: find_state_pda().0,
                boss: f.boss.pubkey(),
                system_program: SYSTEM_PROGRAM_ID,
            },
            instruction::CreatePricer {
                instance_id: 1,
                denomination: PricingDenomination::Usd,
            },
        ),
        ix(
            accounts::AddPricerVector {
                pricer: selected,
                state: find_state_pda().0,
                boss: f.boss.pubkey(),
            },
            instruction::AddPricerVector {
                vector: PricingVector {
                    start_time: now,
                    base_time: now,
                    base_price: 4_000_000_000,
                    apr: 0,
                    price_fix_duration: 86400,
                },
            },
        ),
    ] {
        send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    }
    selected
}

#[test]
fn canonical_nav_controls_refill_and_sell_cap_independently_of_offer_nav() {
    let mut f = Fixture::new(false, false);
    f.seed_tokens(f.mint, f.boss.pubkey(), 10_000_000_000);
    let (_, sell, selected) = routes(&mut f, 0);
    let tracker = pair(&f);
    f.offer = pda(&[
        b"offer_config",
        f.asset.as_ref(),
        f.mint.as_ref(),
        &[OfferFlow::Permissioned as u8],
    ]);
    f.create_route(
        OfferDirection::AssetToManaged,
        OfferFlow::Permissioned,
        QuoterKind::Nav,
        true,
    );
    let buy = f.offer;
    let link = link_instruction(&f, Some(tracker));
    send_tx(&mut f.svm, &[link], &[&f.boss]).unwrap();
    edit::<VaultConfig>(&mut f.svm, selected, |v| v.refill_target_bps = 1000);
    edit::<PropAmmState>(&mut f.svm, tracker, |p| {
        p.config.curve_exponent_scaled = 20_000;
        p.config.cadence_wave_scaled = 0;
    });
    let alternate = alternate_pricer(&mut f);
    edit::<OfferConfig>(&mut f.svm, buy, |o| o.pricer = alternate);
    let mut instruction = f.take(5_000_000, 1);
    instruction.accounts[2].pubkey = alternate;
    omit_account(&mut instruction, price(f.mint, 0));
    let failed = f.send(instruction).unwrap_err();
    assert_eq!(
        failed.err,
        TransactionError::InstructionError(
            2,
            InstructionError::Custom(6000 + OnreError::IncompatiblePricer as u32)
        )
    );
    let mut instruction = f.take(5_000_000, 1);
    instruction.accounts[2].pubkey = alternate;
    let event = executed_event(&f.send(instruction).unwrap());
    assert_eq!(event.amount_out, 1_237_500_000);
    assert_eq!(event.liquidity_refill_amount, 2_000_000); // canonical NAV 2, not offer NAV 4
    f.seed_tokens(f.asset, selected, 20_000_000);
    f.offer = sell;
    edit::<OfferConfig>(&mut f.svm, sell, |o| o.pricer = alternate);
    let mut instruction = f.take(250_000_000, 1);
    instruction.accounts[2].pubkey = alternate;
    let event = executed_event(&f.send_without_approval(instruction).unwrap());
    // Raw NAV output 990,000; canonical pre-burn target 2,247,500; squared
    // utilization and a 7% peg haircut give this independently calculated vector.
    assert_eq!(event.amount_out, 976_553);
    assert_eq!(
        read::<PropAmmState>(&f.svm, tracker).current_sell_value_stable,
        990_000
    );
}

#[test]
fn onyc_refill_uses_main_offer_nav_and_the_canonical_excluded_balance() {
    let mut f = Fixture::new(true, false);
    f.initialize_legacy_main_offer(); // main NAV 1; modular offer NAV 2
    let excluded = Pubkey::new_unique();
    f.seed_tokens(f.mint, f.boss.pubkey(), 8_000_000_000);
    f.seed_tokens(f.mint, excluded, 2_000_000_000);
    let instruction =
        build_set_circulating_supply_excluded_accounts_ix(&f.boss.pubkey(), &[excluded]);
    send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    let instruction = build_update_circulating_supply_excluded_balance_ix(
        &f.boss.pubkey(),
        &f.mint,
        &[derive_ata(&excluded, &f.mint, &f.token_program)],
        &f.token_program,
    );
    send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    f.create_route(
        OfferDirection::AssetToManaged,
        OfferFlow::Permissionless,
        QuoterKind::NavPermissionless,
        true,
    );
    edit::<VaultConfig>(&mut f.svm, vault(VaultKind::Liquidity), |v| {
        v.refill_target_bps = 1000
    });
    let instruction = f.take(2_000_000, 1);
    let event = executed_event(&f.send_without_approval(instruction).unwrap());
    assert_eq!(event.liquidity_refill_amount, 800_000);
    assert_eq!(event.proceeds_amount, 1_180_000);
}

fn link_instruction(f: &Fixture, tracker: Option<Pubkey>) -> Instruction {
    let offer: OfferConfig = read(&f.svm, f.offer);
    ix(
        accounts::SetOfferPressureState {
            offer_config: f.offer,
            quoter: offer.quoter,
            prop_amm_state: tracker,
            state: find_state_pda().0,
            boss: f.boss.pubkey(),
        },
        instruction::SetOfferPressureState {},
    )
}

#[test]
fn nav_link_is_explicit_mandatory_and_relieves_the_configured_curve() {
    let mut f = Fixture::new(false, false);
    f.seed_tokens(f.mint, f.user.pubkey(), 2_000_000_000);
    let (_, sell, selected) = routes(&mut f, 0);
    f.seed_tokens(f.asset, selected, 20_000_000);
    let tracker = pair(&f);
    edit::<PropAmmState>(&mut f.svm, tracker, |s| {
        s.current_sell_value_stable = 10_000_000;
        s.config.cadence_wave_scaled = 0;
    });
    // Align the seeded pressure with the runtime epoch.
    let now = get_clock_time(&f.svm) as i64;
    edit::<PropAmmState>(&mut f.svm, tracker, |s| s.epoch_start = now);
    f.offer = sell;
    let before = f.svm.clone();
    let instruction = f.take(250_000_000, 1);
    let pressured = executed_event(&f.send_without_approval(instruction).unwrap()).amount_out;
    f.svm = before;
    // The initial NAV offer uses no liquidity and is unlinked by default.
    f.offer = pda(&[
        b"offer_config",
        f.asset.as_ref(),
        f.mint.as_ref(),
        &[OfferFlow::Permissioned as u8],
    ]);
    let instruction = f.take(2_000_000, 990_000_000);
    let snapshot = f.svm.get_account(&tracker).unwrap().data;
    f.send(instruction).unwrap();
    assert_eq!(f.svm.get_account(&tracker).unwrap().data, snapshot);
    let link = link_instruction(&f, Some(tracker));
    send_tx(&mut f.svm, &[link], &[&f.boss]).unwrap();
    let mut instruction = f.take(2_000_000, 990_000_000);
    omit_account(&mut instruction, tracker);
    assert_error(f.send(instruction), OnreError::IncompatibleQuoter);
    let instruction = f.take(2_000_000, 990_000_000);
    let event = executed_event(&f.send(instruction).unwrap());
    assert_eq!(event.amount_out, 990_000_000);
    let state: PropAmmState = read(&f.svm, tracker);
    assert_eq!(state.current_buy_value_stable, 1_980_000);
    assert_eq!(state.current_buy_trade_count, 1);
    f.offer = sell;
    let instruction = f.take(250_000_000, 1);
    let relieved = executed_event(&f.send_without_approval(instruction).unwrap()).amount_out;
    assert!(relieved > pressured);
}

#[test]
fn buy_curve_requires_existing_liquidity_and_honors_slippage() {
    let mut f = Fixture::new(false, false);
    let (_, _, selected) = routes(&mut f, 0);
    let instruction = f.take(2_000_000, 0);
    assert!(f.send_without_approval(instruction).is_err());
    f.seed_tokens(f.asset, selected, 20_000_000);
    let tracker = pair(&f);
    let before = f.svm.get_account(&tracker).unwrap().data;
    let instruction = f.take(2_000_000, 990_000_000);
    assert!(f.send_without_approval(instruction).is_err());
    assert_eq!(f.svm.get_account(&tracker).unwrap().data, before);
    let initial = f.svm.clone();
    let instruction = f.take(2_000_000, 1);
    let normal = executed_event(&f.send_without_approval(instruction).unwrap()).amount_out;
    f.svm = initial;
    let now = get_clock_time(&f.svm) as i64;
    edit::<PropAmmState>(&mut f.svm, tracker, |s| {
        s.epoch_start = now;
        s.current_buy_value_stable = 10_000_000;
    });
    let instruction = f.take(2_000_000, 1);
    let pressured = executed_event(&f.send_without_approval(instruction).unwrap()).amount_out;
    assert!(pressured < normal && normal < 990_000_000);
}

#[test]
fn execution_rejects_cross_pair_and_unconfigured_pressure_accounts() {
    let mut f = Fixture::new(false, false);
    let nav_buy = f.offer;
    let (_, _, selected) = routes(&mut f, 0);
    let correct = pair(&f);
    let config = read::<PropAmmState>(&f.svm, correct).config;
    f.seed_tokens(f.asset, selected, 20_000_000);
    let other_asset = create_mint(&mut f.svm, &f.boss, 6, &f.boss.pubkey());
    let other_managed = create_mint(&mut f.svm, &f.boss, 9, &find_mint_authority_pda().0);
    let register = ix(
        accounts::RegisterManagedToken {
            managed_mint: other_managed,
            managed_token: managed(other_managed),
            state: find_state_pda().0,
            boss: f.boss.pubkey(),
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::RegisterManagedToken {},
    );
    send_tx(&mut f.svm, &[register], &[&f.boss]).unwrap();
    let mut trackers = Vec::new();
    // Include another state with the correct pair: even that cannot substitute
    // for the boss-selected tracker at execution time.
    for (id, asset, mint) in [
        (1_u64, other_asset, f.mint),
        (2, f.asset, other_managed),
        (3, f.asset, f.mint),
    ] {
        let quoter = pda(&[b"quoter", &[QuoterKind::PropAmm as u8], &id.to_le_bytes()]);
        let tracker = pda(&[b"prop_amm", quoter.as_ref()]);
        for instruction in [
            ix(
                accounts::CreateQuoter {
                    quoter,
                    state: find_state_pda().0,
                    boss: f.boss.pubkey(),
                    system_program: SYSTEM_PROGRAM_ID,
                },
                instruction::CreateQuoter {
                    kind: QuoterKind::PropAmm,
                    instance_id: id,
                },
            ),
            ix(
                accounts::CreatePropAmmState {
                    quoter,
                    managed_token: managed(mint),
                    asset_mint: asset,
                    prop_amm_state: tracker,
                    state: find_state_pda().0,
                    boss: f.boss.pubkey(),
                    system_program: SYSTEM_PROGRAM_ID,
                },
                instruction::CreatePropAmmState { config },
            ),
        ] {
            send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
        }
        trackers.push(tracker);
    }
    f.offer = nav_buy;
    let link = link_instruction(&f, Some(correct));
    send_tx(&mut f.svm, &[link], &[&f.boss]).unwrap();
    let before = f.svm.get_account(&correct).unwrap().data;
    for substitute in trackers {
        let mut instruction = f.take(2_000_000, 1);
        for account in &mut instruction.accounts {
            if account.pubkey == correct {
                account.pubkey = substitute;
            }
        }
        assert_error(f.send(instruction), OnreError::IncompatibleQuoter);
    }
    assert_eq!(f.svm.get_account(&correct).unwrap().data, before);
    let unsolicited = f.take(2_000_000, 1);
    let unlink = link_instruction(&f, None);
    send_tx(&mut f.svm, &[unlink], &[&f.boss]).unwrap();
    assert_error(f.send(unsolicited), OnreError::IncompatibleQuoter);
}

#[test]
fn curve_buy_prices_before_refill_and_records_full_net_input() {
    for token_2022 in [false, true] {
        let mut f = Fixture::new(false, token_2022);
        f.seed_tokens(f.mint, f.boss.pubkey(), 10_000_000_000);
        let (_, _, selected) = routes(&mut f, 0);
        f.seed_tokens(f.asset, selected, 9_000_000);
        edit::<VaultConfig>(&mut f.svm, selected, |v| v.refill_target_bps = 5000);
        let tracker = pair(&f);
        let instruction = f.take(2_000_000, 1);
        let event = executed_event(&f.send_without_approval(instruction).unwrap());
        assert!(event.amount_out > 0 && event.amount_out < 990_000_000);
        assert_eq!(event.liquidity_refill_amount, 1_000_000);
        assert_eq!(event.proceeds_amount, 980_000);
        assert_eq!(f.balance(f.asset, selected, f.asset_program), 10_000_000);
        let state: PropAmmState = read(&f.svm, tracker);
        assert_eq!(state.current_buy_value_stable, 1_980_000);
        assert_eq!(state.current_buy_trade_count, 1);
    }
}

#[test]
fn standalone_prop_offer_executes_and_pressure_expires_without_reverse_offer() {
    for direction in [
        OfferDirection::ManagedToAsset,
        OfferDirection::AssetToManaged,
    ] {
        let mut f = Fixture::new(false, false);
        // A fresh asset ensures the fixture's initial NAV offer is unrelated.
        f.asset = create_mint(&mut f.svm, &f.boss, 6, &f.boss.pubkey());
        f.seed_tokens(f.mint, f.user.pubkey(), 5_000_000_000);
        f.seed_tokens(f.asset, f.user.pubkey(), 10_000_000);
        f.create_route(
            direction,
            OfferFlow::Permissionless,
            QuoterKind::PropAmm,
            true,
        );
        f.seed_tokens(f.asset, vault(VaultKind::Liquidity), 20_000_000);
        let tracker = pair(&f);
        let original: PropAmmState = read(&f.svm, tracker);
        let (gross, value, raw_output, reverse_in, reverse_out) = match direction {
            OfferDirection::ManagedToAsset => (250_000_000, 495_000_u128, 495_000, f.asset, f.mint),
            OfferDirection::AssetToManaged => {
                (1_000_000, 990_000_u128, 495_000_000, f.mint, f.asset)
            }
        };
        for flow in [
            OfferFlow::Permissioned,
            OfferFlow::Permissionless,
            OfferFlow::Worker,
        ] {
            let reverse = pda(&[
                b"offer_config",
                reverse_in.as_ref(),
                reverse_out.as_ref(),
                &[flow as u8],
            ]);
            assert!(f.svm.get_account(&reverse).is_none());
        }
        for delay in [0, original.config.epoch_duration_seconds * 2] {
            if delay > 0 {
                advance_clock_by(&mut f.svm, delay);
            }
            let instruction = f.take(gross, 1);
            let event = executed_event(&f.send_without_approval(instruction).unwrap());
            assert!(event.amount_out > 0 && event.amount_out < raw_output);
            let state: PropAmmState = read(&f.svm, tracker);
            assert_eq!(state.previous_net_buy_value_stable, 0);
            assert_eq!(state.previous_net_sell_value_stable, 0);
            let (same_volume, opposite_volume, same_count, opposite_count) = match direction {
                OfferDirection::ManagedToAsset => (
                    state.current_sell_value_stable,
                    state.current_buy_value_stable,
                    state.current_sell_trade_count,
                    state.current_buy_trade_count,
                ),
                OfferDirection::AssetToManaged => (
                    state.current_buy_value_stable,
                    state.current_sell_value_stable,
                    state.current_buy_trade_count,
                    state.current_sell_trade_count,
                ),
            };
            assert_eq!(
                (same_volume, opposite_volume, same_count, opposite_count),
                (value, 0, 1, 0)
            );
        }
    }
}

#[test]
fn production_usdg_profile_matches_mainnet_simulation_vectors() {
    // Finalized production snapshot 467467063, 2026-11-09 21:23 UTC.
    // Three quote_swap_sell simulations at timestamp 1794259397. Keep the
    // observed production profile, without requiring live RPC in CI.
    let mut f = Fixture::new(true, true);
    init_buffer(&mut f);
    f.seed_tokens(f.mint, f.user.pubkey(), 200_000_000_000_000);
    let (_, sell, selected) = routes(&mut f, 2);
    f.seed_tokens(f.asset, selected, 27_661_097_835_196);
    f.offer = sell;
    let tracker = pair(&f);
    let now = get_clock_time(&f.svm) as i64;
    edit::<Pricer>(&mut f.svm, price(f.mint, 0), |p| {
        p.vectors[0].base_price = 1_164_144_940
    });
    edit::<FeeConfig>(&mut f.svm, f.fee, |fee| {
        fee.basis_points = 25;
        fee.minimum_fee_amount = 1_500_000_000;
    });
    edit::<PropAmmState>(&mut f.svm, tracker, |s| {
        s.config = PropAmmConfig {
            epoch_duration_seconds: 86_400,
            curve_exponent_scaled: 21_000,
            cadence_threshold: 150,
            cadence_wave_scaled: 2_000,
            wall_sensitivity_scaled: 20_000,
            curve_peg_haircut_bps: 3_000,
        };
        s.current_sell_value_stable = 161_057_168_028;
        s.previous_net_sell_value_stable = 293_328_564_770;
        s.current_sell_trade_count = 87;
        s.epoch_start = now - 80_454;
    });
    let initial = f.svm.clone();
    for (gross, expected) in [
        (1_813_443_530_854, 2_105_783_113),
        (12_598_660_319_631, 14_627_581_755),
        (100_000_000_000_000, 115_973_906_313),
    ] {
        f.svm = initial.clone();
        let instruction = f.take(gross, expected);
        let event = executed_event(&f.send_without_approval(instruction).unwrap());
        assert_eq!(event.amount_out, expected);
        assert_eq!(event.price, 1_164_144_940);
        assert_eq!(
            f.balance(f.asset, selected, f.asset_program),
            27_661_097_835_196 - expected
        );
        let state: PropAmmState = read(&f.svm, tracker);
        assert_eq!(state.current_sell_trade_count, 88);
        assert_eq!(state.current_buy_value_stable, 0);
    }
}
