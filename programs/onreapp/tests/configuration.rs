mod common;

use anchor_lang::{
    AccountDeserialize, AccountSerialize, AnchorDeserialize, AnchorSerialize, Discriminator, Event,
    InstructionData, ToAccountMetas,
};
use common::*;
use litesvm::LiteSVM;
use onreapp::instructions::configuration::*;
use onreapp::{accounts, instruction, model, OnreError};
use solana_sdk::{
    account::Account, instruction::Instruction, pubkey::Pubkey, signature::Keypair, signer::Signer,
};

fn pda(seeds: &[&[u8]]) -> Pubkey {
    Pubkey::find_program_address(seeds, &PROGRAM_ID).0
}

fn ix(a: impl ToAccountMetas, d: impl InstructionData) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: a.to_account_metas(None),
        data: d.data(),
    }
}

fn read<T: AccountDeserialize>(svm: &LiteSVM, key: Pubkey) -> T {
    T::try_deserialize(&mut svm.get_account(&key).unwrap().data.as_slice()).unwrap()
}

fn write<T: AccountSerialize>(svm: &mut LiteSVM, key: Pubkey, value: &T) {
    let mut a = svm.get_account(&key).unwrap();
    a.data.clear();
    value.try_serialize(&mut a.data).unwrap();
    svm.set_account(key, a).unwrap();
}

fn creation_events(meta: &litesvm::types::TransactionMetadata) -> Vec<Vec<u8>> {
    use anchor_lang::__private::base64::{engine::general_purpose::STANDARD, Engine};
    meta.logs
        .iter()
        .filter_map(|log| log.strip_prefix("Program data: "))
        .map(|data| STANDARD.decode(data).unwrap())
        .collect()
}

/// Verify the public event payload against the actual committed account, including
/// its actor, PDA, defaults, references, and runtime timestamp.
fn assert_creation_event<E: Event, T: AccountDeserialize + AnchorSerialize>(
    svm: &LiteSVM,
    data: &[u8],
    key: Pubkey,
    boss: Pubkey,
) {
    let mut expected = E::DISCRIMINATOR.to_vec();
    boss.serialize(&mut expected).unwrap();
    key.serialize(&mut expected).unwrap();
    read::<T>(svm, key).serialize(&mut expected).unwrap();
    (get_clock_time(svm) as i64)
        .serialize(&mut expected)
        .unwrap();
    assert_eq!(data, expected);
    let mut body = &data[E::DISCRIMINATOR.len()..];
    E::deserialize(&mut body).unwrap();
    assert!(body.is_empty());
}

fn assert_error(
    result: Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata>,
    error: OnreError,
) {
    use solana_sdk::{instruction::InstructionError, transaction::TransactionError};
    let failed = result.unwrap_err();
    assert!(creation_events(&failed.meta).is_empty());
    assert_eq!(
        failed.err,
        TransactionError::InstructionError(1, InstructionError::Custom(6000 + error as u32))
    );
}

#[test]
fn registered_components_emit_complete_creation_events() {
    let (mut svm, boss, mint) = setup_initialized();
    let key = boss.pubkey();
    macro_rules! check {
        ($instruction:expr, $event:ty, $account:ty, $address:expr) => {{
            let meta = send_tx(&mut svm, &[$instruction], &[&boss]).unwrap();
            let events = creation_events(&meta);
            assert_eq!(events.len(), 1);
            assert_creation_event::<$event, $account>(&svm, &events[0], $address, key);
        }};
    }
    check!(
        register_managed(key, mint),
        ManagedTokenRegisteredEvent,
        model::ManagedTokenConfig,
        managed(mint)
    );
    check!(
        create_pricer(key, mint, 0),
        PricerCreatedEvent,
        model::Pricer,
        pricer(mint, 0)
    );
    check!(
        create_quoter(key, model::QuoterKind::Nav, 7),
        QuoterCreatedEvent,
        model::Quoter,
        quoter(model::QuoterKind::Nav, 7)
    );
    check!(
        create_vault(key, model::VaultKind::Fee, 8, 0),
        VaultConfigCreatedEvent,
        model::VaultConfig,
        vault(model::VaultKind::Fee, 8)
    );
    check!(
        create_vault(key, model::VaultKind::Liquidity, 9, 625),
        VaultConfigCreatedEvent,
        model::VaultConfig,
        vault(model::VaultKind::Liquidity, 9)
    );
    check!(
        create_fee(key, 10, vault(model::VaultKind::Fee, 8), 75),
        FeeConfigCreatedEvent,
        model::FeeConfig,
        fee(10)
    );
}

fn managed(mint: Pubkey) -> Pubkey {
    pda(&[b"managed_token", mint.as_ref()])
}
fn pricer(mint: Pubkey, id: u64) -> Pubkey {
    pda(&[b"pricer", mint.as_ref(), &[0], &id.to_le_bytes()])
}
fn quoter(kind: model::QuoterKind, id: u64) -> Pubkey {
    pda(&[b"quoter", &[kind as u8], &id.to_le_bytes()])
}
fn vault(kind: model::VaultKind, id: u64) -> Pubkey {
    pda(&[b"vault", &[kind as u8], &id.to_le_bytes()])
}
fn fee(id: u64) -> Pubkey {
    pda(&[b"fee_config", &id.to_le_bytes()])
}

fn register_managed(boss: Pubkey, mint: Pubkey) -> Instruction {
    ix(
        accounts::RegisterManagedToken {
            managed_mint: mint,
            managed_token: managed(mint),
            state: find_state_pda().0,
            boss,
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::RegisterManagedToken {},
    )
}

fn create_pricer(boss: Pubkey, mint: Pubkey, id: u64) -> Instruction {
    ix(
        accounts::CreatePricer {
            managed_token: managed(mint),
            pricer: pricer(mint, id),
            state: find_state_pda().0,
            boss,
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::CreatePricer {
            instance_id: id,
            denomination: model::PricingDenomination::Usd,
        },
    )
}

fn create_quoter(boss: Pubkey, kind: model::QuoterKind, id: u64) -> Instruction {
    ix(
        accounts::CreateQuoter {
            quoter: quoter(kind, id),
            state: find_state_pda().0,
            boss,
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::CreateQuoter {
            kind,
            instance_id: id,
        },
    )
}

fn create_vault(boss: Pubkey, kind: model::VaultKind, id: u64, target: u16) -> Instruction {
    ix(
        accounts::CreateVaultConfig {
            vault: vault(kind, id),
            state: find_state_pda().0,
            boss,
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::CreateVaultConfig {
            kind,
            instance_id: id,
            withdrawal_destination: boss,
            refill_target_bps: target,
        },
    )
}

fn create_fee(boss: Pubkey, id: u64, fee_vault: Pubkey, bps: u16) -> Instruction {
    ix(
        accounts::CreateFeeConfig {
            fee_vault,
            fee_config: fee(id),
            state: find_state_pda().0,
            boss,
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::CreateFeeConfig {
            instance_id: id,
            basis_points: bps,
            minimum_fee_amount: 5,
        },
    )
}

fn initialize_buffer(boss: Pubkey, mint: Pubkey) -> Instruction {
    ix(
        accounts::InitializeManagedTokenBuffer {
            managed_token: managed(mint),
            managed_mint: mint,
            pricer: pricer(mint, 0),
            buffer_state: pda(&[b"buffer", mint.as_ref()]),
            reserve_vault: pda(&[b"buffer_reserve", mint.as_ref()]),
            management_fee_vault: pda(&[b"buffer_management", mint.as_ref()]),
            performance_fee_vault: pda(&[b"buffer_performance", mint.as_ref()]),
            state: find_state_pda().0,
            boss,
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::InitializeManagedTokenBuffer {},
    )
}

fn base_curve() -> model::PropAmmConfig {
    model::PropAmmConfig {
        epoch_duration_seconds: 100,
        curve_exponent_scaled: 25_000,
        cadence_threshold: 20,
        cadence_wave_scaled: 10_000,
        wall_sensitivity_scaled: 20_000,
        curve_peg_haircut_bps: 700,
    }
}

struct Fixture {
    svm: LiteSVM,
    boss: Keypair,
    mint: Pubkey,
    asset: Pubkey,
    selected_pricer: Pubkey,
    selected_quoter: Pubkey,
    pair: Option<Pubkey>,
    liquidity: Option<Pubkey>,
}

impl Fixture {
    fn new() -> Self {
        let (mut svm, boss, mint) = setup_initialized();
        let key = boss.pubkey();
        let asset = create_mint(&mut svm, &boss, 6, &key);
        for instruction in [
            register_managed(key, mint),
            create_pricer(key, mint, 0),
            create_quoter(key, model::QuoterKind::Nav, 0),
            create_vault(key, model::VaultKind::Fee, 0, 0),
            create_vault(key, model::VaultKind::Proceeds, 0, 0),
            create_vault(key, model::VaultKind::Liquidity, 0, 500),
            create_fee(key, 0, vault(model::VaultKind::Fee, 0), 30),
        ] {
            send_tx(&mut svm, &[instruction], &[&boss]).unwrap();
        }
        Self {
            svm,
            boss,
            mint,
            asset,
            selected_pricer: pricer(mint, 0),
            selected_quoter: quoter(model::QuoterKind::Nav, 0),
            pair: None,
            liquidity: Some(vault(model::VaultKind::Liquidity, 0)),
        }
    }

    fn offer_key(&self, flow: model::OfferFlow, reverse: bool) -> Pubkey {
        let (input, output) = if reverse {
            (self.mint, self.asset)
        } else {
            (self.asset, self.mint)
        };
        pda(&[
            b"offer_config",
            input.as_ref(),
            output.as_ref(),
            &[flow as u8],
        ])
    }

    fn offer_ix(&self, flow: model::OfferFlow, reverse: bool) -> Instruction {
        let (token_in_mint, token_out_mint) = if reverse {
            (self.mint, self.asset)
        } else {
            (self.asset, self.mint)
        };
        ix(
            accounts::CreateOfferConfig {
                token_in_mint,
                token_out_mint,
                managed_token: managed(self.mint),
                asset_managed_registration: managed(self.asset),
                pricer: self.selected_pricer,
                quoter: self.selected_quoter,
                prop_amm_state: self.pair,
                fee_config: fee(0),
                proceeds_vault: vault(model::VaultKind::Proceeds, 0),
                liquidity_vault: self.liquidity,
                offer_config: self.offer_key(flow, reverse),
                state: find_state_pda().0,
                boss: self.boss.pubkey(),
                system_program: SYSTEM_PROGRAM_ID,
            },
            instruction::CreateOfferConfig { flow },
        )
    }

    fn pair_ix(
        &self,
        key: Pubkey,
        asset_mint: Pubkey,
        config: model::PropAmmConfig,
    ) -> Instruction {
        ix(
            accounts::CreatePropAmmState {
                quoter: key,
                managed_token: managed(self.mint),
                asset_mint,
                prop_amm_state: pda(&[b"prop_amm", key.as_ref()]),
                state: find_state_pda().0,
                boss: self.boss.pubkey(),
                system_program: SYSTEM_PROGRAM_ID,
            },
            instruction::CreatePropAmmState { config },
        )
    }
}

#[test]
fn creates_reusable_components_and_both_nav_directions() {
    let mut f = Fixture::new();
    let global_before = f.svm.get_account(&find_state_pda().0).unwrap();
    let mint_before = f.svm.get_account(&f.mint).unwrap();
    let price: model::Pricer = read(&f.svm, f.selected_pricer);
    assert!(price.enabled);
    assert!(price.vectors.iter().all(|vector| vector.start_time == 0));
    for reverse in [false, true] {
        let instruction = f.offer_ix(model::OfferFlow::Permissioned, reverse);
        let meta = send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
        let events = creation_events(&meta);
        assert_eq!(events.len(), 1);
        assert_creation_event::<OfferConfigCreatedEvent, model::OfferConfig>(
            &f.svm,
            &events[0],
            f.offer_key(model::OfferFlow::Permissioned, reverse),
            f.boss.pubkey(),
        );
        let offer: model::OfferConfig =
            read(&f.svm, f.offer_key(model::OfferFlow::Permissioned, reverse));
        assert_eq!(offer.pricer, f.selected_pricer);
        assert_eq!(offer.quoter, f.selected_quoter);
        assert_eq!(
            offer.direction,
            if reverse {
                model::OfferDirection::ManagedToAsset
            } else {
                model::OfferDirection::AssetToManaged
            }
        );
        assert_eq!(
            (offer.token_in_decimals, offer.token_out_decimals),
            if reverse { (9, 6) } else { (6, 9) }
        );
    }
    assert_eq!(
        f.svm.get_account(&find_state_pda().0).unwrap(),
        global_before
    );
    assert_eq!(f.svm.get_account(&f.mint).unwrap(), mint_before);
}

#[test]
fn offers_reject_two_managed_mints_and_spoofed_registration_pdas() {
    let mut f = Fixture::new();
    let boss = f.boss.pubkey();
    send_tx(&mut f.svm, &[register_managed(boss, f.asset)], &[&f.boss]).unwrap();
    for reverse in [false, true] {
        let instruction = f.offer_ix(model::OfferFlow::Permissioned, reverse);
        assert_error(
            send_tx(&mut f.svm, &[instruction.clone()], &[&f.boss]),
            OnreError::TokenRoleConflict,
        );
        let mut spoofed = instruction;
        for account in &mut spoofed.accounts {
            if account.pubkey == managed(f.asset) {
                account.pubkey = Pubkey::new_unique();
            }
        }
        assert_error(
            send_tx(&mut f.svm, &[spoofed], &[&f.boss]),
            OnreError::InvalidMint,
        );
        assert!(f
            .svm
            .get_account(&f.offer_key(model::OfferFlow::Permissioned, reverse))
            .is_none());
    }
}

#[test]
fn offer_rejects_a_self_pair() {
    let mut f = Fixture::new();
    f.asset = f.mint;
    let instruction = f.offer_ix(model::OfferFlow::Permissioned, false);
    assert_error(
        send_tx(&mut f.svm, &[instruction], &[&f.boss]),
        OnreError::InvalidOfferDirection,
    );
    assert!(f
        .svm
        .get_account(&f.offer_key(model::OfferFlow::Permissioned, false))
        .is_none());
}

#[test]
fn offer_requires_the_selected_managed_mint_in_the_pair() {
    let mut f = Fixture::new();
    let boss = f.boss.pubkey();
    let other = create_mint(&mut f.svm, &f.boss, 9, &boss);
    send_tx(&mut f.svm, &[register_managed(boss, other)], &[&f.boss]).unwrap();
    let mut instruction = f.offer_ix(model::OfferFlow::Permissioned, false);
    for account in &mut instruction.accounts {
        if account.pubkey == managed(f.mint) {
            account.pubkey = managed(other);
        }
    }
    assert_error(
        send_tx(&mut f.svm, &[instruction], &[&f.boss]),
        OnreError::InvalidOfferDirection,
    );
}

#[test]
fn offer_validates_external_mint_decimals_without_registration() {
    let mut f = Fixture::new();
    f.asset = create_mint(&mut f.svm, &f.boss, 19, &f.boss.pubkey());
    for reverse in [false, true] {
        let instruction = f.offer_ix(model::OfferFlow::Permissioned, reverse);
        assert_error(
            send_tx(&mut f.svm, &[instruction], &[&f.boss]),
            OnreError::DecimalsExceedMax,
        );
    }
}

#[test]
fn boss_can_choose_an_unregistered_token_2022_asset_in_both_directions() {
    let mut f = Fixture::new();
    f.asset = create_mint_2022(&mut f.svm, &f.boss, 6, &f.boss.pubkey());
    for reverse in [false, true] {
        let instruction = f.offer_ix(model::OfferFlow::Permissioned, reverse);
        let meta = send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
        let events = creation_events(&meta);
        assert_eq!(events.len(), 1);
        assert_creation_event::<OfferConfigCreatedEvent, model::OfferConfig>(
            &f.svm,
            &events[0],
            f.offer_key(model::OfferFlow::Permissioned, reverse),
            f.boss.pubkey(),
        );
        let offer: model::OfferConfig =
            read(&f.svm, f.offer_key(model::OfferFlow::Permissioned, reverse));
        let price: model::Pricer = read(&f.svm, offer.pricer);
        assert_eq!(price.denomination, model::PricingDenomination::Usd);
    }
}

#[test]
fn every_creator_requires_boss_and_rolls_back_allocations() {
    let mut f = Fixture::new();
    let rogue = Keypair::new();
    f.svm
        .airdrop(&rogue.pubkey(), 10 * INITIAL_LAMPORTS)
        .unwrap();
    let mint = create_mint(&mut f.svm, &f.boss, 9, &f.boss.pubkey());
    let curve_key = quoter(model::QuoterKind::PropAmm, 90);
    send_tx(
        &mut f.svm,
        &[create_quoter(
            f.boss.pubkey(),
            model::QuoterKind::PropAmm,
            90,
        )],
        &[&f.boss],
    )
    .unwrap();
    let instructions = [
        register_managed(rogue.pubkey(), mint),
        create_pricer(rogue.pubkey(), f.mint, 90),
        create_quoter(rogue.pubkey(), model::QuoterKind::Nav, 90),
        create_vault(rogue.pubkey(), model::VaultKind::Fee, 90, 0),
        create_fee(rogue.pubkey(), 90, vault(model::VaultKind::Fee, 0), 10),
        f.pair_ix(curve_key, f.asset, base_curve()),
        f.offer_ix(model::OfferFlow::Permissioned, false),
        initialize_buffer(rogue.pubkey(), f.mint),
    ];
    for mut instruction in instructions {
        for account in &mut instruction.accounts {
            if account.pubkey == f.boss.pubkey() {
                account.pubkey = rogue.pubkey();
            }
        }
        assert_error(
            send_tx(&mut f.svm, &[instruction], &[&rogue]),
            OnreError::InvalidBoss,
        );
    }
    assert!(f.svm.get_account(&pricer(f.mint, 90)).is_none());
    assert!(f
        .svm
        .get_account(&pda(&[b"buffer", f.mint.as_ref()]))
        .is_none());
}

#[test]
fn managed_registration_rejects_unsupported_decimals() {
    let mut f = Fixture::new();
    let boss = f.boss.pubkey();
    let mint = create_mint(&mut f.svm, &f.boss, 19, &boss);
    assert_error(
        send_tx(&mut f.svm, &[register_managed(boss, mint)], &[&f.boss]),
        OnreError::DecimalsExceedMax,
    );
}

#[test]
fn duplicate_creation_preserves_existing_pricer_vectors() {
    let mut f = Fixture::new();
    let mut price: model::Pricer = read(&f.svm, f.selected_pricer);
    price.vectors[0].start_time = 1;
    price.vectors[0].base_price = 1_234_567_890;
    write(&mut f.svm, f.selected_pricer, &price);
    let before = f.svm.get_account(&f.selected_pricer).unwrap();
    f.svm.expire_blockhash();
    let failed = send_tx(
        &mut f.svm,
        &[create_pricer(f.boss.pubkey(), f.mint, 0)],
        &[&f.boss],
    )
    .unwrap_err();
    assert!(creation_events(&failed.meta).is_empty());
    assert_eq!(f.svm.get_account(&f.selected_pricer).unwrap(), before);
}

#[test]
fn pricer_vector_insertion_uses_empty_slots_and_checks_all_stored_start_times() {
    let mut f = Fixture::new();
    let boss = f.boss.pubkey();
    let selected_pricer = f.selected_pricer;
    let now = get_clock_time(&f.svm);
    let vector = |start_time| model::PricingVector {
        start_time,
        base_time: now,
        base_price: 1_000_000_000,
        apr: 0,
        price_fix_duration: 86400,
    };
    let append = |vector| {
        ix(
            accounts::AddPricerVector {
                pricer: selected_pricer,
                state: find_state_pda().0,
                boss,
            },
            instruction::AddPricerVector { vector },
        )
    };
    let mut price: model::Pricer = read(&f.svm, selected_pricer);
    price.vectors[3] = vector(now);
    price.vectors[8] = vector(now + 20);
    write(&mut f.svm, selected_pricer, &price);

    assert_error(
        send_tx(&mut f.svm, &[append(vector(now + 20))], &[&f.boss]),
        OnreError::DuplicateStartTime,
    );
    assert_error(
        send_tx(&mut f.svm, &[append(vector(now + 10))], &[&f.boss]),
        OnreError::InvalidTimeRange,
    );
    send_tx(&mut f.svm, &[append(vector(now + 30))], &[&f.boss]).unwrap();
    let mut price: model::Pricer = read(&f.svm, selected_pricer);
    assert_eq!(price.vectors[0], vector(now + 30));
    assert_eq!(price.vectors[3], vector(now));
    assert_eq!(price.vectors[8], vector(now + 20));

    for (index, slot) in price.vectors.iter_mut().enumerate() {
        if slot.start_time == 0 {
            *slot = vector(now + index as u64 + 1);
        }
    }
    write(&mut f.svm, selected_pricer, &price);
    assert_error(
        send_tx(&mut f.svm, &[append(vector(now + 40))], &[&f.boss]),
        OnreError::TooManyVectors,
    );
}

#[test]
fn failed_transaction_can_contain_creation_logs_without_committing_accounts() {
    let mut f = Fixture::new();
    let boss = f.boss.pubkey();
    let target = quoter(model::QuoterKind::Nav, 50);
    let failed = send_tx(
        &mut f.svm,
        &[
            create_quoter(boss, model::QuoterKind::Nav, 50),
            create_fee(boss, 50, vault(model::VaultKind::Fee, 0), 1001),
        ],
        &[&f.boss],
    )
    .unwrap_err();
    let events = creation_events(&failed.meta);
    assert_eq!(events.len(), 1);
    assert_eq!(&events[0][..8], QuoterCreatedEvent::DISCRIMINATOR);
    let event = QuoterCreatedEvent::deserialize(&mut &events[0][8..]).unwrap();
    assert_eq!(event.boss, boss);
    assert_eq!(event.quoter, target);
    assert_eq!(event.configuration.instance_id, 50);
    assert!(f.svm.get_account(&target).is_none());
    assert!(f.svm.get_account(&fee(50)).is_none());
}

#[test]
fn vault_and_fee_creation_validate_purpose_and_bounds() {
    let mut f = Fixture::new();
    let boss = f.boss.pubkey();
    assert_error(
        send_tx(
            &mut f.svm,
            &[create_fee(
                boss,
                8,
                vault(model::VaultKind::Proceeds, 0),
                10,
            )],
            &[&f.boss],
        ),
        OnreError::InvalidConfigurableVaultKind,
    );
    assert_error(
        send_tx(
            &mut f.svm,
            &[create_fee(boss, 9, vault(model::VaultKind::Fee, 0), 1001)],
            &[&f.boss],
        ),
        OnreError::InvalidFee,
    );
    assert_error(
        send_tx(
            &mut f.svm,
            &[create_vault(boss, model::VaultKind::Fee, 8, 1)],
            &[&f.boss],
        ),
        OnreError::InvalidAmount,
    );
    assert_error(
        send_tx(
            &mut f.svm,
            &[create_vault(boss, model::VaultKind::Liquidity, 8, 10_001)],
            &[&f.boss],
        ),
        OnreError::InvalidAmount,
    );
    // A prefunded target must not prevent its legitimate initialization.
    let target = vault(model::VaultKind::Fee, 19);
    f.svm.airdrop(&target, 1_000_000).unwrap();
    send_tx(
        &mut f.svm,
        &[create_vault(boss, model::VaultKind::Fee, 19, 0)],
        &[&f.boss],
    )
    .unwrap();
}

#[test]
fn offer_requires_compatible_pricer_and_enabled_dependencies() {
    let mut f = Fixture::new();
    let boss = f.boss.pubkey();
    let other = create_mint(&mut f.svm, &f.boss, 9, &boss);
    send_tx(&mut f.svm, &[register_managed(boss, other)], &[&f.boss]).unwrap();
    send_tx(&mut f.svm, &[create_pricer(boss, other, 0)], &[&f.boss]).unwrap();
    f.selected_pricer = pricer(other, 0);
    let instruction = f.offer_ix(model::OfferFlow::Permissioned, false);
    assert_error(
        send_tx(&mut f.svm, &[instruction], &[&f.boss]),
        OnreError::IncompatiblePricer,
    );
    f.selected_pricer = pricer(f.mint, 0);
    let mut price: model::Pricer = read(&f.svm, f.selected_pricer);
    price.enabled = false;
    write(&mut f.svm, f.selected_pricer, &price);
    let instruction = f.offer_ix(model::OfferFlow::Permissioned, false);
    assert_error(
        send_tx(&mut f.svm, &[instruction], &[&f.boss]),
        OnreError::ConfigurationDisabled,
    );
    send_tx(&mut f.svm, &[create_pricer(boss, f.mint, 1)], &[&f.boss]).unwrap();
    f.selected_pricer = pricer(f.mint, 1);
    let instruction = f.offer_ix(model::OfferFlow::Permissioned, false);
    send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    let offer: model::OfferConfig =
        read(&f.svm, f.offer_key(model::OfferFlow::Permissioned, false));
    assert_eq!(offer.pricer, pricer(f.mint, 1));
}

#[test]
fn offer_checks_flow_and_optional_liquidity() {
    let mut f = Fixture::new();
    for (flow, reverse, error) in [
        (
            model::OfferFlow::Worker,
            false,
            OnreError::IncompatibleQuoter,
        ),
        (
            model::OfferFlow::Permissionless,
            false,
            OnreError::IncompatibleQuoter,
        ),
    ] {
        let instruction = f.offer_ix(flow, reverse);
        assert_error(send_tx(&mut f.svm, &[instruction], &[&f.boss]), error);
    }
    f.liquidity = None;
    let instruction = f.offer_ix(model::OfferFlow::Worker, true);
    assert_error(
        send_tx(&mut f.svm, &[instruction], &[&f.boss]),
        OnreError::LiquidityVaultRequired,
    );
    let instruction = f.offer_ix(model::OfferFlow::Permissioned, false);
    send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    f.liquidity = Some(vault(model::VaultKind::Liquidity, 0));
    let instruction = f.offer_ix(model::OfferFlow::Worker, true);
    send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    send_tx(
        &mut f.svm,
        &[create_quoter(
            f.boss.pubkey(),
            model::QuoterKind::NavPermissionless,
            0,
        )],
        &[&f.boss],
    )
    .unwrap();
    f.selected_quoter = quoter(model::QuoterKind::NavPermissionless, 0);
    for reverse in [false, true] {
        let instruction = f.offer_ix(model::OfferFlow::Permissionless, reverse);
        send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    }
}

#[test]
fn prop_amm_requires_initialized_pair_and_cannot_be_rebound() {
    let mut f = Fixture::new();
    let boss = f.boss.pubkey();
    f.selected_quoter = quoter(model::QuoterKind::PropAmm, 0);
    send_tx(
        &mut f.svm,
        &[create_quoter(boss, model::QuoterKind::PropAmm, 0)],
        &[&f.boss],
    )
    .unwrap();
    let instruction = f.offer_ix(model::OfferFlow::Permissionless, false);
    assert_error(
        send_tx(&mut f.svm, &[instruction], &[&f.boss]),
        OnreError::IncompatibleQuoter,
    );
    let mut invalid = base_curve();
    invalid.epoch_duration_seconds = 0;
    let instruction = f.pair_ix(f.selected_quoter, f.asset, invalid);
    assert_error(
        send_tx(&mut f.svm, &[instruction], &[&f.boss]),
        OnreError::InvalidAmount,
    );
    let instruction = f.pair_ix(f.selected_quoter, f.asset, base_curve());
    let meta = send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    let events = creation_events(&meta);
    assert_eq!(events.len(), 1);
    assert_creation_event::<PropAmmStateCreatedEvent, model::PropAmmState>(
        &f.svm,
        &events[0],
        pda(&[b"prop_amm", f.selected_quoter.as_ref()]),
        boss,
    );
    f.pair = Some(pda(&[b"prop_amm", f.selected_quoter.as_ref()]));
    for reverse in [false, true] {
        let instruction = f.offer_ix(model::OfferFlow::Permissionless, reverse);
        send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    }
    let other = create_mint(&mut f.svm, &f.boss, 6, &boss);
    let instruction = f.pair_ix(f.selected_quoter, other, base_curve());
    assert!(send_tx(&mut f.svm, &[instruction], &[&f.boss]).is_err());
    f.asset = other;
    let instruction = f.offer_ix(model::OfferFlow::Permissionless, false);
    assert_error(
        send_tx(&mut f.svm, &[instruction], &[&f.boss]),
        OnreError::IncompatibleQuoter,
    );
}

#[test]
fn buffer_initialization_is_per_mint_and_preserves_legacy_state() {
    let mut f = Fixture::new();
    let boss = f.boss.pubkey();
    let legacy_key = Pubkey::find_program_address(&[b"buffer_state"], &PROGRAM_ID).0;
    let legacy = onreapp::instructions::BufferState {
        onyc_mint: f.mint,
        gross_apr: 150_000,
        previous_supply: 99,
        management_fee_basis_points: 10,
        performance_fee_basis_points: 20,
        performance_fee_high_watermark: 100,
        performance_fee_high_watermark_enabled: true,
        last_accrual_timestamp: 100,
        bump: 1,
        reserved: [0; 135],
    };
    let mut data = Vec::new();
    legacy.try_serialize(&mut data).unwrap();
    let legacy_account = Account {
        data,
        owner: PROGRAM_ID,
        lamports: INITIAL_LAMPORTS,
        executable: false,
        rent_epoch: 0,
    };
    f.svm
        .set_account(legacy_key, legacy_account.clone())
        .unwrap();
    let other = create_mint(&mut f.svm, &f.boss, 9, &boss);
    send_tx(&mut f.svm, &[register_managed(boss, other)], &[&f.boss]).unwrap();
    send_tx(&mut f.svm, &[create_pricer(boss, other, 0)], &[&f.boss]).unwrap();
    for (mint, supply) in [(f.mint, 75_000_000_000_u64), (other, 125_000_000_000)] {
        let mut account = f.svm.get_account(&mint).unwrap();
        account.data[36..44].copy_from_slice(&supply.to_le_bytes());
        f.svm.set_account(mint, account).unwrap();
        let meta = send_tx(&mut f.svm, &[initialize_buffer(boss, mint)], &[&f.boss]).unwrap();
        let events = creation_events(&meta);
        assert_eq!(events.len(), 4);
        for (index, seed) in [
            b"buffer_reserve".as_slice(),
            b"buffer_management",
            b"buffer_performance",
        ]
        .iter()
        .enumerate()
        {
            assert_creation_event::<VaultConfigCreatedEvent, model::VaultConfig>(
                &f.svm,
                &events[index],
                pda(&[seed, mint.as_ref()]),
                boss,
            );
        }
        assert_creation_event::<ManagedTokenBufferInitializedEvent, model::ManagedTokenBufferState>(
            &f.svm,
            &events[3],
            pda(&[b"buffer", mint.as_ref()]),
            boss,
        );
        let buffer: model::ManagedTokenBufferState = read(&f.svm, pda(&[b"buffer", mint.as_ref()]));
        assert_eq!(buffer.managed_mint, mint);
        assert_eq!(buffer.previous_supply, supply);
        assert_eq!(buffer.last_accrual_timestamp, get_clock_time(&f.svm) as i64);
        assert_eq!(buffer.gross_apr, 0);
        assert!(buffer.performance_fee_high_watermark_enabled);
        for (key, kind) in [
            (buffer.reserve_vault, model::VaultKind::BufferReserve),
            (buffer.management_fee_vault, model::VaultKind::Fee),
            (buffer.performance_fee_vault, model::VaultKind::Fee),
        ] {
            let config: model::VaultConfig = read(&f.svm, key);
            assert_eq!(config.kind, kind);
            assert_eq!(config.withdrawal_destination, boss);
            assert!(f
                .svm
                .get_account(&get_associated_token_address(&key, &mint))
                .is_none());
        }
        let before = f
            .svm
            .get_account(&pda(&[b"buffer", mint.as_ref()]))
            .unwrap();
        advance_clock_by(&mut f.svm, 10);
        assert!(send_tx(&mut f.svm, &[initialize_buffer(boss, mint)], &[&f.boss]).is_err());
        assert_eq!(
            f.svm
                .get_account(&pda(&[b"buffer", mint.as_ref()]))
                .unwrap(),
            before
        );
    }
    assert_eq!(f.svm.get_account(&legacy_key).unwrap(), legacy_account);
    assert_ne!(
        pda(&[b"buffer_reserve", f.mint.as_ref()]),
        pda(&[b"buffer_reserve", other.as_ref()])
    );
}

#[test]
fn offer_creation_validates_legacy_liquidity_identity() {
    use onreapp::state::{ConfigurableVault, ConfigurableVaultKind};
    let mut f = Fixture::new();
    let kind = ConfigurableVaultKind::PropAmmProceeds;
    let selected = pda(&[b"configurable_vault", kind.seed()]);
    let setup = build_set_configurable_vault_destination_ix(
        &f.boss.pubkey(),
        &selected,
        kind.as_u8(),
        &f.boss.pubkey(),
    );
    send_tx(&mut f.svm, &[setup], &[&f.boss]).unwrap();
    f.liquidity = Some(selected);
    let original = f.svm.get_account(&selected).unwrap();
    for (index, expected) in [
        OnreError::InvalidConfigurableVaultOwner,
        OnreError::InvalidConfigurableVaultData,
        OnreError::InvalidConfigurableVaultKind,
        OnreError::InvalidConfigurableVault,
        OnreError::InvalidConfigurableVault,
    ]
    .into_iter()
    .enumerate()
    {
        let mut account = original.clone();
        match index {
            0 => account.owner = SYSTEM_PROGRAM_ID,
            1 => account.data[0] ^= 255,
            2 => account.data[8] = 255,
            3 => {
                let mut vault =
                    ConfigurableVault::try_deserialize(&mut account.data.as_slice()).unwrap();
                vault.bump ^= 255;
                account.data.clear();
                vault.try_serialize(&mut account.data).unwrap();
            }
            4 => account.data[8] = ConfigurableVaultKind::OfferProceeds.as_u8(),
            _ => unreachable!(),
        }
        f.svm.set_account(selected, account).unwrap();
        f.svm.expire_blockhash();
        let instruction = f.offer_ix(model::OfferFlow::Permissioned, true);
        assert_error(send_tx(&mut f.svm, &[instruction], &[&f.boss]), expected);
    }
    f.svm.set_account(selected, original).unwrap();
    f.svm.expire_blockhash();
    let instruction = f.offer_ix(model::OfferFlow::Permissioned, true);
    send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    let offer: model::OfferConfig = read(&f.svm, f.offer_key(model::OfferFlow::Permissioned, true));
    assert_eq!(offer.liquidity_vault, selected);
}

#[test]
fn pressure_links_are_boss_configured_pair_bound_and_independent_of_nav() {
    let mut f = Fixture::new();
    let boss = f.boss.pubkey();
    let prop = quoter(model::QuoterKind::PropAmm, 0);
    let pair = pda(&[b"prop_amm", prop.as_ref()]);
    for instruction in [
        create_quoter(boss, model::QuoterKind::PropAmm, 0),
        f.pair_ix(prop, f.asset, base_curve()),
    ] {
        send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    }
    // NAV may bind at creation without selecting PropAmm pricing.
    f.pair = Some(pair);
    let instruction = f.offer_ix(model::OfferFlow::Permissioned, false);
    send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    let offer = f.offer_key(model::OfferFlow::Permissioned, false);
    assert_eq!(
        read::<model::OfferConfig>(&f.svm, offer).pressure_state,
        pair
    );
    let setter = |boss, tracker| {
        ix(
            accounts::SetOfferPressureState {
                offer_config: offer,
                quoter: f.selected_quoter,
                prop_amm_state: tracker,
                state: find_state_pda().0,
                boss,
            },
            instruction::SetOfferPressureState {},
        )
    };
    assert_error(
        send_tx(&mut f.svm, &[setter(boss, Some(pair))], &[&f.boss]),
        OnreError::NoChange,
    );
    let rogue = Keypair::new();
    f.svm.airdrop(&rogue.pubkey(), INITIAL_LAMPORTS).unwrap();
    assert_error(
        send_tx(&mut f.svm, &[setter(rogue.pubkey(), None)], &[&rogue]),
        OnreError::InvalidBoss,
    );
    let before = f.svm.get_account(&pair).unwrap().data;
    let original: model::PropAmmState = read(&f.svm, pair);
    for wrong_asset in [false, true] {
        let mut state: model::PropAmmState = read(&f.svm, pair);
        if wrong_asset {
            state.asset_mint = Pubkey::new_unique();
        } else {
            state.managed_mint = Pubkey::new_unique();
        }
        write(&mut f.svm, pair, &state);
        f.svm.expire_blockhash();
        assert_error(
            send_tx(&mut f.svm, &[setter(boss, Some(pair))], &[&f.boss]),
            OnreError::IncompatibleQuoter,
        );
    }
    write(&mut f.svm, pair, &original);
    for target in [None, Some(pair)] {
        f.svm.expire_blockhash();
        send_tx(&mut f.svm, &[setter(boss, target)], &[&f.boss]).unwrap();
        assert_eq!(
            read::<model::OfferConfig>(&f.svm, offer).pressure_state,
            target.unwrap_or_default()
        );
    }
    assert_eq!(f.svm.get_account(&pair).unwrap().data, before);
    // The quote-side PropAmm state cannot be detached, and both sides need custody.
    f.selected_quoter = prop;
    f.liquidity = None;
    for reverse in [false, true] {
        let instruction = f.offer_ix(model::OfferFlow::Permissionless, reverse);
        assert_error(
            send_tx(&mut f.svm, &[instruction], &[&f.boss]),
            OnreError::LiquidityVaultRequired,
        );
    }
    f.liquidity = Some(vault(model::VaultKind::Liquidity, 0));
    let instruction = f.offer_ix(model::OfferFlow::Permissionless, false);
    send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    let instruction = ix(
        accounts::SetOfferPressureState {
            offer_config: f.offer_key(model::OfferFlow::Permissionless, false),
            quoter: prop,
            prop_amm_state: None,
            state: find_state_pda().0,
            boss,
        },
        instruction::SetOfferPressureState {},
    );
    assert_error(
        send_tx(&mut f.svm, &[instruction], &[&f.boss]),
        OnreError::IncompatibleQuoter,
    );
}

#[test]
fn cross_asset_and_cross_managed_pressure_links_fail_in_both_directions() {
    for wrong_asset in [true, false] {
        let mut f = Fixture::new();
        let boss = f.boss.pubkey();
        let quoter = quoter(model::QuoterKind::PropAmm, 0);
        let tracker = pda(&[b"prop_amm", quoter.as_ref()]);
        let original_mint = f.mint;
        let original_asset = f.asset;
        if wrong_asset {
            f.asset = create_mint(&mut f.svm, &f.boss, 6, &boss);
        } else {
            f.mint = create_mint(&mut f.svm, &f.boss, 9, &boss);
            send_tx(&mut f.svm, &[register_managed(boss, f.mint)], &[&f.boss]).unwrap();
        }
        for instruction in [
            create_quoter(boss, model::QuoterKind::PropAmm, 0),
            f.pair_ix(quoter, f.asset, base_curve()),
        ] {
            send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
        }
        f.mint = original_mint;
        f.asset = original_asset;
        for reverse in [false, true] {
            f.pair = Some(tracker);
            let instruction = f.offer_ix(model::OfferFlow::Permissioned, reverse);
            assert_error(
                send_tx(&mut f.svm, &[instruction], &[&f.boss]),
                OnreError::IncompatibleQuoter,
            );
            f.pair = None;
            f.svm.expire_blockhash();
            let instruction = f.offer_ix(model::OfferFlow::Permissioned, reverse);
            send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
            let offer = f.offer_key(model::OfferFlow::Permissioned, reverse);
            let instruction = ix(
                accounts::SetOfferPressureState {
                    offer_config: offer,
                    quoter: f.selected_quoter,
                    prop_amm_state: Some(tracker),
                    state: find_state_pda().0,
                    boss,
                },
                instruction::SetOfferPressureState {},
            );
            assert_error(
                send_tx(&mut f.svm, &[instruction], &[&f.boss]),
                OnreError::IncompatibleQuoter,
            );
            assert_eq!(
                read::<model::OfferConfig>(&f.svm, offer).pressure_state,
                Pubkey::default()
            );
        }
    }
}
