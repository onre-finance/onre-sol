//! Boss-only enable flags and post-creation updates of modular components.
use super::mint_multisig::{expect_error, send};
use super::*;

/// The five components with an `enabled` flag checked by `take_offer`.
const COMPONENTS: [&str; 5] = [
    "managed_token",
    "pricer",
    "quoter",
    "fee_config",
    "offer_config",
];

fn toggle_ix(f: &Fixture, component: &str, boss: Pubkey, enabled: bool) -> Instruction {
    let state = find_state_pda().0;
    match component {
        "managed_token" => ix(
            accounts::SetManagedTokenEnabled {
                managed_token: managed(f.mint),
                state,
                boss,
            },
            instruction::SetManagedTokenEnabled { enabled },
        ),
        "pricer" => ix(
            accounts::SetPricerEnabled {
                pricer: price(f.mint, 0),
                state,
                boss,
            },
            instruction::SetPricerEnabled { enabled },
        ),
        "quoter" => ix(
            accounts::SetQuoterEnabled {
                quoter: f.quoter,
                state,
                boss,
            },
            instruction::SetQuoterEnabled { enabled },
        ),
        "fee_config" => ix(
            accounts::SetFeeConfigEnabled {
                fee_config: f.fee,
                state,
                boss,
            },
            instruction::SetFeeConfigEnabled { enabled },
        ),
        _ => ix(
            accounts::SetOfferConfigEnabled {
                offer_config: f.offer,
                state,
                boss,
            },
            instruction::SetOfferConfigEnabled { enabled },
        ),
    }
}

pub(super) fn boss_send(
    f: &mut Fixture,
    instruction: Instruction,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let boss = f.boss.insecure_clone();
    send(&mut f.svm, &[instruction], &[&boss])
}

#[test]
fn boss_disables_and_reenables_each_component() {
    let mut f = Fixture::new(false, false);
    let rogue = Keypair::new();
    f.svm.airdrop(&rogue.pubkey(), INITIAL_LAMPORTS).unwrap();
    for component in COMPONENTS {
        let instruction = toggle_ix(&f, component, rogue.pubkey(), false);
        expect_error(
            send(&mut f.svm, &[instruction], &[&rogue]),
            OnreError::InvalidBoss,
        );
        let instruction = toggle_ix(&f, component, f.boss.pubkey(), true);
        expect_error(boss_send(&mut f, instruction), OnreError::NoChange);

        // Disabled: execution fails closed.
        let instruction = toggle_ix(&f, component, f.boss.pubkey(), false);
        boss_send(&mut f, instruction).unwrap();
        let instruction = f.take(1_000_000, 0);
        assert_error(f.send(instruction), OnreError::ConfigurationDisabled);

        // Re-enabled: execution works again.
        let instruction = toggle_ix(&f, component, f.boss.pubkey(), true);
        boss_send(&mut f, instruction).unwrap();
        let instruction = f.take(1_000_000, 495_000_000);
        f.send(instruction).unwrap();
    }
}

fn update_fee_ix(
    f: &Fixture,
    boss: Pubkey,
    fee_vault: Pubkey,
    bps: u16,
    minimum: u64,
) -> Instruction {
    ix(
        accounts::UpdateFeeConfig {
            fee_config: f.fee,
            fee_vault,
            state: find_state_pda().0,
            boss,
        },
        instruction::UpdateFeeConfig {
            basis_points: bps,
            minimum_fee_amount: minimum,
        },
    )
}

#[test]
fn boss_updates_fee_config_for_subsequent_trades() {
    let mut f = Fixture::new(false, false);
    let fee_vault = vault(VaultKind::Fee);
    let rogue = Keypair::new();
    f.svm.airdrop(&rogue.pubkey(), INITIAL_LAMPORTS).unwrap();
    let instruction = update_fee_ix(&f, rogue.pubkey(), fee_vault, 200, 5);
    expect_error(
        send(&mut f.svm, &[instruction], &[&rogue]),
        OnreError::InvalidBoss,
    );
    let instruction = update_fee_ix(&f, f.boss.pubkey(), fee_vault, 100, 5);
    expect_error(boss_send(&mut f, instruction), OnreError::NoChange);
    let instruction = update_fee_ix(&f, f.boss.pubkey(), fee_vault, 1_001, 5);
    expect_error(boss_send(&mut f, instruction), OnreError::InvalidFee);
    let instruction = update_fee_ix(&f, f.boss.pubkey(), vault(VaultKind::Proceeds), 200, 5);
    expect_error(
        boss_send(&mut f, instruction),
        OnreError::InvalidConfigurableVaultKind,
    );

    let instruction = update_fee_ix(&f, f.boss.pubkey(), fee_vault, 200, 5);
    boss_send(&mut f, instruction).unwrap();
    let config: FeeConfig = read(&f.svm, f.fee);
    assert_eq!((config.basis_points, config.minimum_fee_amount), (200, 5));
    // 2% of 1 asset unit: 20_000 fee, 980_000 net at NAV 2.
    let instruction = f.take(1_000_000, 490_000_000);
    f.send(instruction).unwrap();
    assert_eq!(f.balance(f.asset, fee_vault, f.asset_program), 20_000);
    assert_eq!(
        f.balance(f.mint, f.user.pubkey(), f.token_program),
        490_000_000
    );
}

fn update_vault_ix(f: &Fixture, vault: Pubkey, destination: Pubkey, bps: u16) -> Instruction {
    ix(
        accounts::UpdateVaultConfig {
            vault,
            state: find_state_pda().0,
            boss: f.boss.pubkey(),
        },
        instruction::UpdateVaultConfig {
            withdrawal_destination: destination,
            refill_target_bps: bps,
        },
    )
}

#[test]
fn boss_updates_vault_destinations_and_liquidity_targets() {
    let mut f = Fixture::new(false, false);
    f.initialize_managed_buffer();
    let liquidity = vault(VaultKind::Liquidity);
    let instruction = ix(
        accounts::CreateVaultConfig {
            vault: liquidity,
            state: find_state_pda().0,
            boss: f.boss.pubkey(),
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::CreateVaultConfig {
            kind: VaultKind::Liquidity,
            instance_id: 0,
            withdrawal_destination: f.boss.pubkey(),
            refill_target_bps: 0,
        },
    );
    boss_send(&mut f, instruction).unwrap();
    let destination = Pubkey::new_unique();
    let proceeds = vault(VaultKind::Proceeds);

    // Creation rules apply: nonzero destination, refill target only on Liquidity.
    let instruction = update_vault_ix(&f, proceeds, Pubkey::default(), 0);
    expect_error(
        boss_send(&mut f, instruction),
        OnreError::MissingConfigurableVaultDestination,
    );
    let instruction = update_vault_ix(&f, proceeds, destination, 1);
    expect_error(boss_send(&mut f, instruction), OnreError::InvalidAmount);
    let instruction = update_vault_ix(&f, liquidity, destination, 10_001);
    expect_error(boss_send(&mut f, instruction), OnreError::InvalidAmount);
    let instruction = update_vault_ix(&f, proceeds, f.boss.pubkey(), 0);
    expect_error(boss_send(&mut f, instruction), OnreError::NoChange);

    let instruction = update_vault_ix(&f, proceeds, destination, 0);
    boss_send(&mut f, instruction).unwrap();
    let instruction = update_vault_ix(&f, liquidity, destination, 2_500);
    boss_send(&mut f, instruction).unwrap();
    // BUFFER vaults, pinned to the boss at initialization, can be repointed too.
    let reserve = pda(&[b"buffer_reserve", f.mint.as_ref()]);
    let instruction = update_vault_ix(&f, reserve, destination, 0);
    boss_send(&mut f, instruction).unwrap();
    for (key, bps) in [(proceeds, 0), (liquidity, 2_500), (reserve, 0)] {
        let config: VaultConfig = read(&f.svm, key);
        assert_eq!(
            (config.withdrawal_destination, config.refill_target_bps),
            (destination, bps)
        );
    }
}

#[test]
fn boss_replaces_prop_amm_config_and_unsets_market_stats() {
    let mut f = Fixture::new(false, false);
    let quoter = pda(&[
        b"quoter",
        &[QuoterKind::PropAmm as u8],
        &0_u64.to_le_bytes(),
    ]);
    let pair = pda(&[b"prop_amm", quoter.as_ref()]);
    let config = PropAmmConfig {
        epoch_duration_seconds: 86400,
        curve_exponent_scaled: 25000,
        cadence_threshold: 20,
        cadence_wave_scaled: 10000,
        wall_sensitivity_scaled: 20000,
        curve_peg_haircut_bps: 700,
    };
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
                instance_id: 0,
            },
        ),
        ix(
            accounts::CreatePropAmmState {
                quoter,
                managed_token: managed(f.mint),
                asset_mint: f.asset,
                prop_amm_state: pair,
                state: find_state_pda().0,
                boss: f.boss.pubkey(),
                system_program: SYSTEM_PROGRAM_ID,
            },
            instruction::CreatePropAmmState { config },
        ),
    ] {
        boss_send(&mut f, instruction).unwrap();
    }
    let update = |f: &Fixture, config: PropAmmConfig| {
        ix(
            accounts::UpdatePropAmmConfig {
                prop_amm_state: pair,
                state: find_state_pda().0,
                boss: f.boss.pubkey(),
            },
            instruction::UpdatePropAmmConfig { config },
        )
    };
    let before: PropAmmState = read(&f.svm, pair);
    let instruction = update(&f, config);
    expect_error(boss_send(&mut f, instruction), OnreError::NoChange);
    let instruction = update(
        &f,
        PropAmmConfig {
            curve_exponent_scaled: 25_500,
            ..config
        },
    );
    expect_error(boss_send(&mut f, instruction), OnreError::InvalidAmount);
    let next = PropAmmConfig {
        curve_peg_haircut_bps: 500,
        epoch_duration_seconds: 3_600,
        ..config
    };
    let instruction = update(&f, next);
    boss_send(&mut f, instruction).unwrap();
    let after: PropAmmState = read(&f.svm, pair);
    assert_eq!(after.config, next);
    // The bound pair and accumulated pressure are kept.
    assert_eq!(
        (after.asset_mint, after.managed_mint, after.epoch_start),
        (before.asset_mint, before.managed_mint, before.epoch_start)
    );

    // Unsetting MarketStats stops modular reporting without touching the account.
    let stats = pda(&[b"market_stats", f.mint.as_ref()]);
    let instruction = ix(
        accounts::InitializeManagedTokenMarketStats {
            managed_token: managed(f.mint),
            market_stats: stats,
            state: find_state_pda().0,
            boss: f.boss.pubkey(),
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::InitializeManagedTokenMarketStats {},
    );
    boss_send(&mut f, instruction).unwrap();
    let unset = ix(
        accounts::UnsetManagedTokenMarketStats {
            managed_token: managed(f.mint),
            state: find_state_pda().0,
            boss: f.boss.pubkey(),
        },
        instruction::UnsetManagedTokenMarketStats {},
    );
    boss_send(&mut f, unset.clone()).unwrap();
    assert_eq!(
        read::<ManagedTokenConfig>(&f.svm, managed(f.mint)).market_stats,
        Pubkey::default()
    );
    expect_error(boss_send(&mut f, unset), OnreError::NoChange);
    let instruction = f.take(1_000_000, 495_000_000);
    f.send(instruction).unwrap();
    let report: onreapp::state::MarketStats = read(&f.svm, stats);
    assert_eq!(report.last_updated_at, 0);
}

#[test]
fn boss_replaces_offer_components_including_the_pricer_instance() {
    let mut f = Fixture::new(false, false);
    let now = get_clock_time(&f.svm);
    let state = find_state_pda().0;
    let boss = f.boss.pubkey();
    let pricer = price(f.mint, 1);
    let fee = pda(&[b"fee_config", &1_u64.to_le_bytes()]);
    let prop_amm = pda(&[
        b"quoter",
        &[QuoterKind::PropAmm as u8],
        &0_u64.to_le_bytes(),
    ]);
    for instruction in [
        ix(
            accounts::CreatePricer {
                managed_token: managed(f.mint),
                pricer,
                state,
                boss,
                system_program: SYSTEM_PROGRAM_ID,
            },
            instruction::CreatePricer {
                instance_id: 1,
                denomination: PricingDenomination::Usd,
            },
        ),
        ix(
            accounts::AddPricerVector {
                pricer,
                state,
                boss,
            },
            instruction::AddPricerVector {
                vector: PricingVector {
                    start_time: now,
                    base_time: now,
                    base_price: 4_000_000_000,
                    apr: 0,
                    price_fix_duration: 86_400,
                },
            },
        ),
        ix(
            accounts::CreateFeeConfig {
                fee_vault: vault(VaultKind::Fee),
                fee_config: fee,
                state,
                boss,
                system_program: SYSTEM_PROGRAM_ID,
            },
            instruction::CreateFeeConfig {
                instance_id: 1,
                token_decimals: 6,
                basis_points: 300,
                minimum_fee_amount: 5,
            },
        ),
        ix(
            accounts::CreateQuoter {
                quoter: prop_amm,
                state,
                boss,
                system_program: SYSTEM_PROGRAM_ID,
            },
            instruction::CreateQuoter {
                kind: QuoterKind::PropAmm,
                instance_id: 0,
            },
        ),
    ] {
        boss_send(&mut f, instruction).unwrap();
    }
    let update = |f: &Fixture, boss: Pubkey, pricer: Pubkey, quoter: Pubkey, fee: Pubkey| {
        ix(
            accounts::UpdateOfferConfig {
                offer_config: f.offer,
                pricer,
                quoter,
                prop_amm_state: None,
                fee_config: fee,
                proceeds_vault: vault(VaultKind::Proceeds),
                source_vault: None,
                destination_vault: None,
                haircut_vault: None,
                state: find_state_pda().0,
                boss,
            },
            instruction::UpdateOfferConfig {},
        )
    };

    let rogue = Keypair::new();
    f.svm.airdrop(&rogue.pubkey(), INITIAL_LAMPORTS).unwrap();
    let instruction = update(&f, rogue.pubkey(), pricer, f.quoter, fee);
    expect_error(
        send(&mut f.svm, &[instruction], &[&rogue]),
        OnreError::InvalidBoss,
    );
    let instruction = update(&f, boss, price(f.mint, 0), f.quoter, f.fee);
    expect_error(boss_send(&mut f, instruction), OnreError::NoChange);
    // A Prop AMM quoter needs its pair's pressure state.
    let instruction = update(&f, boss, pricer, prop_amm, fee);
    expect_error(
        boss_send(&mut f, instruction),
        OnreError::IncompatibleQuoter,
    );
    // Disabled components are rejected like at creation.
    let disable = ix(
        accounts::SetPricerEnabled {
            pricer,
            state,
            boss,
        },
        instruction::SetPricerEnabled { enabled: false },
    );
    boss_send(&mut f, disable).unwrap();
    let instruction = update(&f, boss, pricer, f.quoter, fee);
    expect_error(
        boss_send(&mut f, instruction),
        OnreError::ConfigurationDisabled,
    );
    let enable = ix(
        accounts::SetPricerEnabled {
            pricer,
            state,
            boss,
        },
        instruction::SetPricerEnabled { enabled: true },
    );
    boss_send(&mut f, enable).unwrap();

    // Switch to Pricer instance 1 (NAV 4) and a 3% fee.
    let instruction = update(&f, boss, pricer, f.quoter, fee);
    boss_send(&mut f, instruction).unwrap();
    let config: OfferConfig = read(&f.svm, f.offer);
    assert_eq!((config.pricer, config.fee_config), (pricer, fee));
    assert_eq!(OfferMode::of(&config), OfferMode::Permissioned);
    // 1_000_000 gross, 30_000 fee, 970_000 net at NAV 4. The offer's Pricer is the
    // first canonical-Pricer account in the take instruction; the BUFFER Pricer
    // later in the list stays canonical.
    let mut instruction = f.take(1_000_000, 242_500_000);
    let offer_pricer = instruction
        .accounts
        .iter()
        .position(|meta| meta.pubkey == price(f.mint, 0))
        .unwrap();
    instruction.accounts[offer_pricer].pubkey = pricer;
    for meta in instruction
        .accounts
        .iter_mut()
        .filter(|meta| meta.pubkey == f.fee)
    {
        meta.pubkey = fee;
    }
    f.send(instruction).unwrap();
    assert_eq!(
        f.balance(f.mint, f.user.pubkey(), f.token_program),
        242_500_000
    );
    assert_eq!(
        f.balance(f.asset, vault(VaultKind::Fee), f.asset_program),
        30_000
    );
}

#[test]
fn fee_configs_bind_input_decimals_and_are_shared_across_mints() {
    let mut f = Fixture::new(false, false);
    let boss = f.boss.pubkey();
    let state = find_state_pda().0;
    let buy_fee = f.fee;
    let create_fee = |id: u64, token_decimals: u8| {
        ix(
            accounts::CreateFeeConfig {
                fee_vault: vault(VaultKind::Fee),
                fee_config: pda(&[b"fee_config", &id.to_le_bytes()]),
                state,
                boss,
                system_program: SYSTEM_PROGRAM_ID,
            },
            instruction::CreateFeeConfig {
                instance_id: id,
                token_decimals,
                basis_points: 100,
                minimum_fee_amount: 5,
            },
        )
    };
    let instruction = create_fee(40, 19);
    expect_error(boss_send(&mut f, instruction), OnreError::DecimalsExceedMax);
    // A 9-decimal fee config, as a managed-token sell would use.
    let instruction = create_fee(41, 9);
    boss_send(&mut f, instruction).unwrap();
    let sell_fee = pda(&[b"fee_config", &41_u64.to_le_bytes()]);
    assert_eq!(read::<FeeConfig>(&f.svm, buy_fee).token_decimals, 6);
    assert_eq!(read::<FeeConfig>(&f.svm, sell_fee).token_decimals, 9);

    // The 6-decimal buy cannot switch to the 9-decimal fee config.
    let update = |f: &Fixture, fee_config: Pubkey| {
        ix(
            accounts::UpdateOfferConfig {
                offer_config: f.offer,
                pricer: price(f.mint, 0),
                quoter: f.quoter,
                prop_amm_state: None,
                fee_config,
                proceeds_vault: vault(VaultKind::Proceeds),
                source_vault: None,
                destination_vault: None,
                haircut_vault: None,
                state,
                boss,
            },
            instruction::UpdateOfferConfig {},
        )
    };
    let instruction = update(&f, sell_fee);
    expect_error(
        boss_send(&mut f, instruction),
        OnreError::FeeConfigDecimalsMismatch,
    );
    // Execution re-checks the binding even if an offer referenced a mismatched
    // fee config.
    let original = f.svm.clone();
    edit::<OfferConfig>(&mut f.svm, f.offer, |c| c.fee_config = sell_fee);
    f.fee = sell_fee;
    let instruction = f.take(1_000_000, 0);
    expect_error(f.send(instruction), OnreError::FeeConfigDecimalsMismatch);
    f.svm = original;
    f.fee = buy_fee;

    // USDC and USDG (both 6 decimals) share one fee config and its minimum.
    let usdc_offer = f.offer;
    let usdg = create_mint(&mut f.svm, &f.boss, 6, &boss);
    f.asset = usdg;
    f.seed_tokens(usdg, f.user.pubkey(), 10_000_000);
    f.create_route(
        OfferDirection::AssetToManaged,
        OfferMode::Permissioned,
        QuoterKind::Nav,
        false,
    );
    assert_ne!(f.offer, usdc_offer);
    for offer in [usdc_offer, f.offer] {
        assert_eq!(read::<OfferConfig>(&f.svm, offer).fee_config, buy_fee);
    }
    // 100 gross: the shared 5-unit minimum exceeds 1% of the input.
    let instruction = f.take(100, 0);
    let event = executed_event(&f.send(instruction).unwrap());
    assert_eq!(event.fee_amount, 5);
    assert_eq!(f.balance(usdg, vault(VaultKind::Fee), f.asset_program), 5);
}

fn unregister_ix(f: &Fixture, boss: Pubkey) -> Instruction {
    ix(
        accounts::UnregisterManagedToken {
            managed_token: managed(f.mint),
            buffer_state: pda(&[b"buffer", f.mint.as_ref()]),
            state: find_state_pda().0,
            boss,
        },
        instruction::UnregisterManagedToken {},
    )
}

fn disable_managed_token(f: &mut Fixture) {
    let instruction = toggle_ix(f, "managed_token", f.boss.pubkey(), false);
    let boss = f.boss.insecure_clone();
    send(&mut f.svm, &[instruction], &[&boss]).unwrap();
}

#[test]
fn boss_unregisters_a_disabled_token_that_never_went_live() {
    let mut f = Fixture::new(false, false);
    let state = find_state_pda().0;
    let boss = f.boss.pubkey();

    // Enabled, or with market stats selected: still in use.
    let instruction = unregister_ix(&f, f.boss.pubkey());
    expect_error(
        boss_send(&mut f, instruction),
        OnreError::ManagedTokenStillInUse,
    );
    disable_managed_token(&mut f);
    let instruction = ix(
        accounts::InitializeManagedTokenMarketStats {
            managed_token: managed(f.mint),
            market_stats: pda(&[b"market_stats", f.mint.as_ref()]),
            state,
            boss,
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::InitializeManagedTokenMarketStats {},
    );
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    let instruction = unregister_ix(&f, f.boss.pubkey());
    expect_error(
        boss_send(&mut f, instruction),
        OnreError::ManagedTokenStillInUse,
    );
    let instruction = ix(
        accounts::UnsetManagedTokenMarketStats {
            managed_token: managed(f.mint),
            state,
            boss,
        },
        instruction::UnsetManagedTokenMarketStats {},
    );
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();

    // Boss only; then the registration closes and its rent goes to the boss.
    let rogue = Keypair::new();
    f.svm.airdrop(&rogue.pubkey(), INITIAL_LAMPORTS).unwrap();
    let instruction = unregister_ix(&f, rogue.pubkey());
    expect_error(
        send(&mut f.svm, &[instruction], &[&rogue]),
        OnreError::InvalidBoss,
    );
    let rent = f.svm.get_account(&managed(f.mint)).unwrap().lamports;
    let before = f.svm.get_account(&boss).unwrap().lamports;
    let instruction = unregister_ix(&f, f.boss.pubkey());
    boss_send(&mut f, instruction).unwrap();
    assert!(f
        .svm
        .get_account(&managed(f.mint))
        .is_none_or(|a| a.lamports == 0));
    assert!(f.svm.get_account(&boss).unwrap().lamports + 10_000 >= before + rent);

    // The mint can be registered again.
    let instruction = ix(
        accounts::RegisterManagedToken {
            managed_mint: f.mint,
            managed_token: managed(f.mint),
            state,
            boss,
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::RegisterManagedToken {},
    );
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
}

#[test]
fn a_token_with_a_buffer_and_onyc_cannot_be_unregistered() {
    let mut f = Fixture::new(false, false);
    super::modular_prop_amm::init_buffer(&mut f);
    disable_managed_token(&mut f);
    let instruction = unregister_ix(&f, f.boss.pubkey());
    expect_error(
        boss_send(&mut f, instruction),
        OnreError::ManagedTokenStillInUse,
    );

    let mut f = Fixture::new(true, false);
    disable_managed_token(&mut f);
    let instruction = unregister_ix(&f, f.boss.pubkey());
    expect_error(
        boss_send(&mut f, instruction),
        OnreError::CannotUnregisterOnyc,
    );
}
