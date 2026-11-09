mod common;
#[path = "modular_take_offer/prop_amm.rs"]
mod modular_prop_amm;
#[path = "modular_take_offer/permissionless.rs"]
mod permissionless;

use anchor_lang::{
    AccountDeserialize, AccountSerialize, AnchorDeserialize, InstructionData, ToAccountMetas,
};
use common::*;
use litesvm::{
    types::{FailedTransactionMetadata, TransactionMetadata},
    LiteSVM,
};
use onreapp::{
    accounts, instruction, instructions::execution::OfferExecutedEvent, model::*,
    state::State as GlobalState, utils::ApprovalMessage, OnreError,
};
use solana_sdk::{
    account::Account,
    instruction::{Instruction, InstructionError},
    message::{v0, AddressLookupTableAccount, VersionedMessage},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::{TransactionError, VersionedTransaction},
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
fn managed(mint: Pubkey) -> Pubkey {
    pda(&[b"managed_token", mint.as_ref()])
}
fn price(mint: Pubkey, id: u64) -> Pubkey {
    pda(&[b"pricer", mint.as_ref(), &[0], &id.to_le_bytes()])
}
fn vault(kind: VaultKind) -> Pubkey {
    pda(&[b"vault", &[kind as u8], &0_u64.to_le_bytes()])
}
fn read<T: AccountDeserialize>(svm: &LiteSVM, key: Pubkey) -> T {
    T::try_deserialize(&mut svm.get_account(&key).unwrap().data.as_slice()).unwrap()
}
fn edit<T: AccountDeserialize + AccountSerialize>(
    svm: &mut LiteSVM,
    key: Pubkey,
    mutate: impl FnOnce(&mut T),
) {
    let mut value: T = read(svm, key);
    mutate(&mut value);
    let mut account = svm.get_account(&key).unwrap();
    account.data.clear();
    value.try_serialize(&mut account.data).unwrap();
    svm.set_account(key, account).unwrap();
}
fn assert_error(
    result: Result<TransactionMetadata, FailedTransactionMetadata>,
    expected: OnreError,
) {
    let error = result.unwrap_err();
    assert_eq!(
        error.err,
        TransactionError::InstructionError(2, InstructionError::Custom(6000 + expected as u32)),
        "{:?}",
        error.meta.logs
    );
}

struct Fixture {
    svm: LiteSVM,
    boss: Keypair,
    user: Keypair,
    approver: Keypair,
    mint: Pubkey,
    asset: Pubkey,
    offer: Pubkey,
    quoter: Pubkey,
    fee: Pubkey,
    onyc: bool,
    token_program: Pubkey,
    asset_program: Pubkey,
}

impl Fixture {
    fn new(onyc: bool, token_2022: bool) -> Self {
        let (mut svm, boss, legacy_mint) = setup_initialized();
        let key = boss.pubkey();
        let authority = find_mint_authority_pda().0;
        let mint = if onyc {
            let mut a = svm.get_account(&legacy_mint).unwrap();
            a.data[4..36].copy_from_slice(authority.as_ref());
            svm.set_account(legacy_mint, a).unwrap();
            legacy_mint
        } else if token_2022 {
            create_mint_2022(&mut svm, &boss, 9, &authority)
        } else {
            create_mint(&mut svm, &boss, 9, &authority)
        };
        let token_program = svm.get_account(&mint).unwrap().owner;
        let asset = if token_2022 {
            create_mint_2022(&mut svm, &boss, 6, &key)
        } else {
            create_mint(&mut svm, &boss, 6, &key)
        };
        let asset_program = svm.get_account(&asset).unwrap().owner;
        let quoter = pda(&[b"quoter", &[QuoterKind::Nav as u8], &0_u64.to_le_bytes()]);
        let fee = pda(&[b"fee_config", &0_u64.to_le_bytes()]);
        let offer = pda(&[
            b"offer_config",
            asset.as_ref(),
            mint.as_ref(),
            &[OfferFlow::Permissioned as u8],
        ]);
        let now = get_clock_time(&svm);
        let vector = PricingVector {
            start_time: now,
            base_time: now,
            base_price: 2_000_000_000,
            apr: 0,
            price_fix_duration: 86400,
        };
        let mut instructions = vec![
            ix(
                accounts::RegisterManagedToken {
                    managed_mint: mint,
                    managed_token: managed(mint),
                    state: find_state_pda().0,
                    boss: key,
                    system_program: SYSTEM_PROGRAM_ID,
                },
                instruction::RegisterManagedToken {},
            ),
            ix(
                accounts::CreatePricer {
                    managed_token: managed(mint),
                    pricer: price(mint, 0),
                    state: find_state_pda().0,
                    boss: key,
                    system_program: SYSTEM_PROGRAM_ID,
                },
                instruction::CreatePricer {
                    instance_id: 0,
                    denomination: PricingDenomination::Usd,
                },
            ),
            ix(
                accounts::AddPricerVector {
                    pricer: price(mint, 0),
                    state: find_state_pda().0,
                    boss: key,
                },
                instruction::AddPricerVector { vector },
            ),
            ix(
                accounts::CreateQuoter {
                    quoter,
                    state: find_state_pda().0,
                    boss: key,
                    system_program: SYSTEM_PROGRAM_ID,
                },
                instruction::CreateQuoter {
                    kind: QuoterKind::Nav,
                    instance_id: 0,
                },
            ),
        ];
        for kind in [VaultKind::Fee, VaultKind::Proceeds] {
            instructions.push(ix(
                accounts::CreateVaultConfig {
                    vault: vault(kind),
                    state: find_state_pda().0,
                    boss: key,
                    system_program: SYSTEM_PROGRAM_ID,
                },
                instruction::CreateVaultConfig {
                    kind,
                    instance_id: 0,
                    withdrawal_destination: key,
                    refill_target_bps: 0,
                },
            ));
        }
        instructions.push(ix(
            accounts::CreateFeeConfig {
                fee_vault: vault(VaultKind::Fee),
                fee_config: fee,
                state: find_state_pda().0,
                boss: key,
                system_program: SYSTEM_PROGRAM_ID,
            },
            instruction::CreateFeeConfig {
                instance_id: 0,
                basis_points: 100,
                minimum_fee_amount: 5,
            },
        ));
        instructions.push(ix(
            accounts::CreateOfferConfig {
                token_in_mint: asset,
                token_out_mint: mint,
                managed_token: managed(mint),
                asset_managed_registration: managed(asset),
                pricer: price(mint, 0),
                quoter,
                prop_amm_state: None,
                fee_config: fee,
                proceeds_vault: vault(VaultKind::Proceeds),
                liquidity_vault: None,
                offer_config: offer,
                state: find_state_pda().0,
                boss: key,
                system_program: SYSTEM_PROGRAM_ID,
            },
            instruction::CreateOfferConfig {
                flow: OfferFlow::Permissioned,
            },
        ));
        for instruction in instructions {
            send_tx(&mut svm, &[instruction], &[&boss]).unwrap();
        }
        let approver = Keypair::new();
        send_tx(
            &mut svm,
            &[build_add_approver_ix(&key, &approver.pubkey())],
            &[&boss],
        )
        .unwrap();
        let user = Keypair::new();
        svm.airdrop(&user.pubkey(), 10 * INITIAL_LAMPORTS).unwrap();
        if token_2022 {
            create_token_account_2022(&mut svm, &asset, &user.pubkey(), 10_000_000);
        } else {
            create_token_account(&mut svm, &asset, &user.pubkey(), 10_000_000);
        }
        Self {
            svm,
            boss,
            user,
            approver,
            mint,
            asset,
            offer,
            quoter,
            fee,
            onyc,
            token_program,
            asset_program,
        }
    }

    fn seed_tokens(&mut self, mint: Pubkey, owner: Pubkey, amount: u64) {
        if self.svm.get_account(&mint).unwrap().owner == TOKEN_2022_PROGRAM_ID {
            create_token_account_2022(&mut self.svm, &mint, &owner, amount);
        } else {
            create_token_account(&mut self.svm, &mint, &owner, amount);
        }
    }

    fn create_route(
        &mut self,
        direction: OfferDirection,
        flow: OfferFlow,
        kind: QuoterKind,
        with_liquidity: bool,
    ) {
        let liquidity = with_liquidity.then(|| vault(VaultKind::Liquidity));
        if let Some(vault) = liquidity.filter(|vault| self.svm.get_account(vault).is_none()) {
            let instruction = ix(
                accounts::CreateVaultConfig {
                    vault,
                    state: find_state_pda().0,
                    boss: self.boss.pubkey(),
                    system_program: SYSTEM_PROGRAM_ID,
                },
                instruction::CreateVaultConfig {
                    kind: VaultKind::Liquidity,
                    instance_id: 0,
                    withdrawal_destination: self.boss.pubkey(),
                    refill_target_bps: 0,
                },
            );
            send_tx(&mut self.svm, &[instruction], &[&self.boss]).unwrap();
        }
        self.create_route_with_vault(direction, flow, kind, liquidity);
    }

    fn create_route_with_vault(
        &mut self,
        direction: OfferDirection,
        flow: OfferFlow,
        kind: QuoterKind,
        liquidity: Option<Pubkey>,
    ) {
        let quoter = pda(&[b"quoter", &[kind as u8], &0_u64.to_le_bytes()]);
        if self.svm.get_account(&quoter).is_none() {
            let instruction = ix(
                accounts::CreateQuoter {
                    quoter,
                    state: find_state_pda().0,
                    boss: self.boss.pubkey(),
                    system_program: SYSTEM_PROGRAM_ID,
                },
                instruction::CreateQuoter {
                    kind,
                    instance_id: 0,
                },
            );
            send_tx(&mut self.svm, &[instruction], &[&self.boss]).unwrap();
        }
        let prop_amm_state =
            (kind == QuoterKind::PropAmm).then(|| pda(&[b"prop_amm", quoter.as_ref()]));
        if let Some(pair) = prop_amm_state.filter(|pair| self.svm.get_account(pair).is_none()) {
            let instruction = ix(
                accounts::CreatePropAmmState {
                    quoter,
                    managed_token: managed(self.mint),
                    asset_mint: self.asset,
                    prop_amm_state: pair,
                    state: find_state_pda().0,
                    boss: self.boss.pubkey(),
                    system_program: SYSTEM_PROGRAM_ID,
                },
                instruction::CreatePropAmmState {
                    config: PropAmmConfig {
                        epoch_duration_seconds: 86400,
                        curve_exponent_scaled: 25000,
                        cadence_threshold: 20,
                        cadence_wave_scaled: 10000,
                        wall_sensitivity_scaled: 20000,
                        curve_peg_haircut_bps: 700,
                    },
                },
            );
            send_tx(&mut self.svm, &[instruction], &[&self.boss]).unwrap();
        }
        let (token_in_mint, token_out_mint) = match direction {
            OfferDirection::AssetToManaged => (self.asset, self.mint),
            OfferDirection::ManagedToAsset => (self.mint, self.asset),
        };
        let offer = pda(&[
            b"offer_config",
            token_in_mint.as_ref(),
            token_out_mint.as_ref(),
            &[flow as u8],
        ]);
        // The initial permissioned buy already exists. Its creation used the same
        // handler; changing a component here only prepares an unsupported-path test.
        if offer == self.offer {
            edit::<OfferConfig>(&mut self.svm, offer, |c| {
                c.liquidity_vault = liquidity.unwrap_or_default();
            });
        } else {
            let instruction = ix(
                accounts::CreateOfferConfig {
                    token_in_mint,
                    token_out_mint,
                    managed_token: managed(self.mint),
                    asset_managed_registration: managed(self.asset),
                    pricer: price(self.mint, 0),
                    quoter,
                    prop_amm_state,
                    fee_config: self.fee,
                    proceeds_vault: vault(VaultKind::Proceeds),
                    liquidity_vault: liquidity,
                    offer_config: offer,
                    state: find_state_pda().0,
                    boss: self.boss.pubkey(),
                    system_program: SYSTEM_PROGRAM_ID,
                },
                instruction::CreateOfferConfig { flow },
            );
            send_tx(&mut self.svm, &[instruction], &[&self.boss]).unwrap();
        }
        self.offer = offer;
    }

    fn initialize_legacy_main_offer(&mut self) {
        let now = get_clock_time(&self.svm);
        for instruction in [
            build_make_offer_ix(
                &self.boss.pubkey(),
                &self.asset,
                &self.mint,
                0,
                false,
                false,
                &self.token_program,
            ),
            build_add_offer_vector_ix(
                &self.boss.pubkey(),
                &self.asset,
                &self.mint,
                Some(now),
                now,
                1_000_000_000,
                0,
                86400,
            ),
            build_set_main_offer_ix(
                &self.boss.pubkey(),
                &find_offer_pda(&self.asset, &self.mint).0,
            ),
        ] {
            send_tx(&mut self.svm, &[instruction], &[&self.boss]).unwrap();
        }
    }

    fn buffer(&self) -> Pubkey {
        if self.onyc {
            find_buffer_state_pda().0
        } else {
            pda(&[b"buffer", self.mint.as_ref()])
        }
    }
    fn buffer_vaults(&self) -> [Pubkey; 3] {
        if self.onyc {
            [
                find_reserve_vault_authority_pda().0,
                find_management_fee_vault_pda().0,
                find_performance_fee_vault_pda().0,
            ]
        } else {
            [
                pda(&[b"buffer_reserve", self.mint.as_ref()]),
                pda(&[b"buffer_management", self.mint.as_ref()]),
                pda(&[b"buffer_performance", self.mint.as_ref()]),
            ]
        }
    }
    fn approval(&self) -> ApprovalMessage {
        ApprovalMessage {
            program_id: PROGRAM_ID,
            user_pubkey: self.user.pubkey(),
            expiry_unix: get_clock_time(&self.svm) + 1000,
        }
    }
    fn take(&self, gross: u64, minimum: u64) -> Instruction {
        let config: OfferConfig = read(&self.svm, self.offer);
        let direction = config.direction;
        let (token_in, token_out, in_program, out_program) = match direction {
            OfferDirection::AssetToManaged => (
                self.asset,
                self.mint,
                self.asset_program,
                self.token_program,
            ),
            OfferDirection::ManagedToAsset => (
                self.mint,
                self.asset,
                self.token_program,
                self.asset_program,
            ),
        };
        let has_liquidity = config.liquidity_vault != Pubkey::default();
        let [reserve, management, performance] = self
            .buffer_vaults()
            .map(|v| derive_ata(&v, &self.mint, &self.token_program));
        ix(
            accounts::TakeOffer {
                offer_config: self.offer,
                managed_token: managed(self.mint),
                pricer: price(self.mint, 0),
                quoter: config.quoter,
                fee_config: self.fee,
                proceeds_vault: vault(VaultKind::Proceeds),
                fee_vault: vault(VaultKind::Fee),
                token_in_mint: token_in,
                token_out_mint: token_out,
                user_token_in: derive_ata(&self.user.pubkey(), &token_in, &in_program),
                user_token_out: derive_ata(&self.user.pubkey(), &token_out, &out_program),
                proceeds_token_in: (direction == OfferDirection::AssetToManaged).then(|| {
                    derive_ata(
                        &vault(VaultKind::Proceeds),
                        &self.asset,
                        &self.asset_program,
                    )
                }),
                fee_token_in: derive_ata(&vault(VaultKind::Fee), &token_in, &in_program),
                mint_authority: find_mint_authority_pda().0,
                state: find_state_pda().0,
                buffer_state: self.buffer(),
                buffer_pricer: if self.onyc {
                    None
                } else {
                    Some(price(self.mint, 0))
                },
                main_offer: if self.onyc {
                    Some(read::<GlobalState>(&self.svm, find_state_pda().0).main_offer)
                } else {
                    None
                },
                market_stats: find_market_stats_pda().0,
                circulating_supply_excluded_balance: find_circulating_supply_excluded_balance_pda()
                    .0,
                buffer_reserve_token: reserve,
                buffer_management_token: management,
                buffer_performance_token: performance,
                instructions_sysvar: SYSVAR_INSTRUCTIONS_ID,
                user: self.user.pubkey(),
                token_in_program: in_program,
                token_out_program: out_program,
                associated_token_program: ATA_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
                prop_amm_state: (config.pressure_state != Pubkey::default())
                    .then_some(config.pressure_state),
                liquidity_vault: has_liquidity.then_some(config.liquidity_vault),
                liquidity_token: has_liquidity
                    .then(|| derive_ata(&config.liquidity_vault, &self.asset, &self.asset_program)),
                permissionless_authority: (config.flow == OfferFlow::Permissionless)
                    .then(|| find_permissionless_authority_pda().0),
                permissionless_token_in: (config.flow == OfferFlow::Permissionless).then(|| {
                    derive_ata(
                        &find_permissionless_authority_pda().0,
                        &token_in,
                        &in_program,
                    )
                }),
                permissionless_token_out: (config.flow == OfferFlow::Permissionless).then(|| {
                    derive_ata(
                        &find_permissionless_authority_pda().0,
                        &token_out,
                        &out_program,
                    )
                }),
            },
            instruction::TakeOffer {
                gross_input_amount: gross,
                minimum_amount_out: minimum,
                deadline: get_clock_time(&self.svm) + 100,
                approval_message: (config.flow == OfferFlow::Permissioned).then(|| self.approval()),
            },
        )
    }
    fn send(
        &mut self,
        instruction: Instruction,
    ) -> Result<TransactionMetadata, FailedTransactionMetadata> {
        let data = serialize_approval_message(
            &PROGRAM_ID,
            &self.user.pubkey(),
            self.approval().expiry_unix,
        );
        let verify = build_ed25519_verify_ix(&self.approver, &data);
        self.send_v0(instruction, Some(verify))
    }

    fn send_without_approval(
        &mut self,
        instruction: Instruction,
    ) -> Result<TransactionMetadata, FailedTransactionMetadata> {
        self.send_v0(instruction, None)
    }

    fn send_v0(
        &mut self,
        instruction: Instruction,
        verify: Option<Instruction>,
    ) -> Result<TransactionMetadata, FailedTransactionMetadata> {
        // v0 with lookup tables is the supported integration baseline. Execute
        // that wire format and enforce its limits without depending on v1.
        let addresses: Vec<Pubkey> = instruction
            .accounts
            .iter()
            .filter(|a| !a.is_signer)
            .map(|a| a.pubkey)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let lookup = AddressLookupTableAccount {
            key: Pubkey::new_unique(),
            addresses,
        };
        // ALT ProgramState::LookupTable metadata: active, immutable, extended in
        // slot zero. Addresses follow the canonical 56-byte metadata prefix.
        let mut table_data = vec![0_u8; 56];
        table_data[..4].copy_from_slice(&1_u32.to_le_bytes());
        table_data[4..12].copy_from_slice(&u64::MAX.to_le_bytes());
        for address in &lookup.addresses {
            table_data.extend_from_slice(address.as_ref());
        }
        self.svm
            .set_account(
                lookup.key,
                Account {
                    lamports: INITIAL_LAMPORTS,
                    data: table_data,
                    owner: solana_sdk::pubkey!("AddressLookupTab1e1111111111111111111111111"),
                    executable: false,
                    rent_epoch: 0,
                },
            )
            .unwrap();
        advance_slot(&mut self.svm);
        let budget =
            solana_compute_budget_interface::ComputeBudgetInstruction::set_compute_unit_limit(
                1_400_000,
            );
        let mut instructions = vec![budget];
        instructions.extend(verify);
        instructions.push(instruction);
        let message = VersionedMessage::V0(
            v0::Message::try_compile(
                &self.user.pubkey(),
                &instructions,
                &[lookup],
                self.svm.latest_blockhash(),
            )
            .unwrap(),
        );
        // One compact signature count byte, one signature, and serialized message.
        assert!(1 + 64 + message.serialize().len() <= 1232);
        if let VersionedMessage::V0(compiled) = &message {
            let lookup_accounts: usize = compiled
                .address_table_lookups
                .iter()
                .map(|table| table.writable_indexes.len() + table.readonly_indexes.len())
                .sum();
            assert!(compiled.account_keys.len() + lookup_accounts <= 64);
        }
        let transaction = VersionedTransaction::try_new(message, &[&self.user]).unwrap();
        self.svm.send_transaction(transaction)
    }
    fn balance(&self, mint: Pubkey, owner: Pubkey, program: Pubkey) -> u64 {
        get_token_balance(&self.svm, &derive_ata(&owner, &mint, &program))
    }
    fn initialize_managed_buffer(&mut self) {
        let [reserve, management, performance] = self.buffer_vaults();
        let instruction = ix(
            accounts::InitializeManagedTokenBuffer {
                managed_token: managed(self.mint),
                managed_mint: self.mint,
                pricer: price(self.mint, 0),
                buffer_state: self.buffer(),
                reserve_vault: reserve,
                management_fee_vault: management,
                performance_fee_vault: performance,
                state: find_state_pda().0,
                boss: self.boss.pubkey(),
                system_program: SYSTEM_PROGRAM_ID,
            },
            instruction::InitializeManagedTokenBuffer {},
        );
        send_tx(&mut self.svm, &[instruction], &[&self.boss]).unwrap();
        for owner in [reserve, management, performance] {
            self.seed_tokens(self.mint, owner, 0);
        }
    }
}

#[test]
fn executes_created_configuration_and_emits_accounting_for_spl_and_token_2022() {
    for token_2022 in [false, true] {
        let mut f = Fixture::new(false, token_2022);
        let instruction = f.take(2_000_000, 990_000_000);
        let meta = f.send(instruction).unwrap();
        assert_eq!(
            f.balance(f.asset, f.user.pubkey(), f.asset_program),
            8_000_000
        );
        assert_eq!(
            f.balance(f.asset, vault(VaultKind::Proceeds), f.asset_program),
            1_980_000
        );
        assert_eq!(
            f.balance(f.asset, vault(VaultKind::Fee), f.asset_program),
            20_000
        );
        assert_eq!(
            f.balance(f.mint, f.user.pubkey(), f.token_program),
            990_000_000
        );
        use anchor_lang::{
            __private::base64::{engine::general_purpose::STANDARD, Engine},
            Discriminator,
        };
        let data = meta
            .logs
            .iter()
            .find_map(|log| log.strip_prefix("Program data: "))
            .unwrap();
        let data = STANDARD.decode(data).unwrap();
        assert_eq!(&data[..8], OfferExecutedEvent::DISCRIMINATOR);
        let event = OfferExecutedEvent::deserialize(&mut &data[8..]).unwrap();
        assert_eq!(event.offer_config, f.offer);
        assert_eq!(event.user, f.user.pubkey());
        assert_eq!(event.flow, OfferFlow::Permissioned);
        assert_eq!(event.gross_input_amount, 2_000_000);
        assert_eq!(event.net_input_amount, 1_980_000);
        assert_eq!(event.fee_amount, 20_000);
        assert_eq!(event.amount_out, 990_000_000);
        assert_eq!(event.price, 2_000_000_000);
        assert_eq!(event.proceeds_amount, event.net_input_amount);
        assert_eq!(event.liquidity_refill_amount, 0);
        assert_eq!(event.timestamp, get_clock_time(&f.svm) as i64);
    }
}

#[test]
fn fee_floor_applies_and_slippage_failure_is_atomic() {
    let mut f = Fixture::new(false, false);
    let before = f.svm.get_account(&f.mint).unwrap();
    let instruction = f.take(100, 47_501);
    assert_error(f.send(instruction), OnreError::MinimumOutNotMet);
    assert_eq!(f.svm.get_account(&f.mint).unwrap().data, before.data);
    assert_eq!(
        f.balance(f.asset, f.user.pubkey(), f.asset_program),
        10_000_000
    );
    assert!(f
        .svm
        .get_account(&derive_ata(&f.user.pubkey(), &f.mint, &f.token_program))
        .is_none());
    let instruction = f.take(100, 47_500);
    f.send(instruction).unwrap();
    assert_eq!(f.balance(f.mint, f.user.pubkey(), f.token_program), 47_500);
    assert_eq!(
        f.balance(f.asset, vault(VaultKind::Fee), f.asset_program),
        5
    );
}

#[test]
fn enforces_authorization_deadline_caps_and_enabled_controls() {
    let mut f = Fixture::new(false, false);
    let mut instruction = f.take(100, 0);
    instruction.data = instruction::TakeOffer {
        gross_input_amount: 100,
        minimum_amount_out: 0,
        deadline: get_clock_time(&f.svm) + 1,
        approval_message: None,
    }
    .data();
    assert_error(f.send(instruction), OnreError::ApprovalRequired);
    let mut instruction = f.take(100, 0);
    instruction.data = instruction::TakeOffer {
        gross_input_amount: 100,
        minimum_amount_out: 0,
        deadline: get_clock_time(&f.svm) - 1,
        approval_message: Some(f.approval()),
    }
    .data();
    assert_error(f.send(instruction), OnreError::Expired);
    edit::<ManagedTokenConfig>(&mut f.svm, managed(f.mint), |c| c.max_mint_amount = 1);
    let instruction = f.take(100, 0);
    assert_error(f.send(instruction), OnreError::MaxMintAmountExceeded);
    edit::<ManagedTokenConfig>(&mut f.svm, managed(f.mint), |c| {
        c.max_mint_amount = 0;
        c.max_supply = 1;
    });
    let instruction = f.take(101, 0);
    assert_error(f.send(instruction), OnreError::MaxSupplyExceeded);
    edit::<ManagedTokenConfig>(&mut f.svm, managed(f.mint), |c| {
        c.max_supply = 0;
        c.enabled = false;
    });
    let instruction = f.take(102, 0);
    assert_error(f.send(instruction), OnreError::ConfigurationDisabled);
}

#[test]
fn cannot_substitute_configurations_or_bypass_buffer() {
    let mut f = Fixture::new(false, false);
    let mut instruction = f.take(100, 0);
    instruction.accounts[15].pubkey = Pubkey::new_unique(); // canonical BUFFER is mandatory even if absent
    assert_error(f.send(instruction), OnreError::InvalidBufferStateAccount);
    edit::<OfferConfig>(&mut f.svm, f.offer, |c| c.pricer = Pubkey::new_unique());
    let instruction = f.take(101, 0);
    assert_error(f.send(instruction), OnreError::IncompatiblePricer);
}

#[test]
fn unsupported_routes_and_global_kill_fail_closed() {
    let mut f = Fixture::new(false, false);
    f.create_route(
        OfferDirection::AssetToManaged,
        OfferFlow::Permissioned,
        QuoterKind::Nav,
        true,
    );
    let instruction = f.take(100, 0);
    let event = executed_event(&f.send(instruction).unwrap());
    assert_eq!(event.liquidity_refill_amount, 0);
    assert_eq!(event.proceeds_amount, 95);
    f.seed_tokens(f.mint, f.user.pubkey(), 1_000_000_000);
    f.create_route(
        OfferDirection::ManagedToAsset,
        OfferFlow::Worker,
        QuoterKind::Nav,
        true,
    );
    let instruction = f.take(101, 0);
    assert_error(f.send(instruction), OnreError::UnsupportedOfferExecution);
    send_tx(
        &mut f.svm,
        &[build_set_kill_switch_ix(&f.boss.pubkey(), true)],
        &[&f.boss],
    )
    .unwrap();
    let instruction = f.take(102, 0);
    assert_error(f.send(instruction), OnreError::KillSwitchActivated);
}

#[test]
fn per_mint_buffer_accrues_before_execution_and_tracks_actual_supply() {
    let mut f = Fixture::new(false, false);
    create_token_account(&mut f.svm, &f.mint, &f.boss.pubkey(), 1_000_000_000);
    f.initialize_managed_buffer();
    let buffer = f.buffer();
    edit::<ManagedTokenBufferState>(&mut f.svm, buffer, |b| b.gross_apr = 100_000);
    advance_clock_by(&mut f.svm, 31_536_000);
    let instruction = f.take(2_000_000, 990_000_000);
    f.send(instruction).unwrap();
    let b: ManagedTokenBufferState = read(&f.svm, f.buffer());
    assert_eq!(b.previous_supply, 2_090_000_000);
    assert_eq!(b.last_accrual_timestamp, get_clock_time(&f.svm) as i64);
    assert_eq!(
        f.balance(f.mint, f.buffer_vaults()[0], f.token_program),
        100_000_000
    );
    advance_clock_by(&mut f.svm, 1);
    let instruction = f.take(2_000_000, 990_000_000);
    f.send(instruction).unwrap();
    let b: ManagedTokenBufferState = read(&f.svm, f.buffer());
    assert_eq!(b.previous_supply, 3_080_000_006);
}

#[test]
fn onyc_coexists_with_v2_using_one_legacy_buffer_baseline() {
    let mut f = Fixture::new(true, false);
    let now = get_clock_time(&f.svm);
    send_tx(
        &mut f.svm,
        &[build_make_offer_ix(
            &f.boss.pubkey(),
            &f.asset,
            &f.mint,
            0,
            false,
            false,
            &TOKEN_PROGRAM_ID,
        )],
        &[&f.boss],
    )
    .unwrap();
    send_tx(
        &mut f.svm,
        &[build_add_offer_vector_ix(
            &f.boss.pubkey(),
            &f.asset,
            &f.mint,
            Some(now),
            now,
            1_000_000_000,
            0,
            86400,
        )],
        &[&f.boss],
    )
    .unwrap();
    let legacy_offer = find_offer_pda(&f.asset, &f.mint).0;
    send_tx(
        &mut f.svm,
        &[build_set_main_offer_ix(&f.boss.pubkey(), &legacy_offer)],
        &[&f.boss],
    )
    .unwrap();
    send_tx(
        &mut f.svm,
        &[build_initialize_buffer_ix(
            &f.boss.pubkey(),
            &legacy_offer,
            &f.mint,
        )],
        &[&f.boss],
    )
    .unwrap();
    // Initialize through an actual modular issuance, then accrue on the shared baseline.
    let instruction = f.take(2_000_000, 990_000_000);
    f.send(instruction).unwrap();
    let mut b: onreapp::instructions::BufferState = read(&f.svm, f.buffer());
    assert_eq!(b.previous_supply, 990_000_000);
    let buffer = f.buffer();
    edit::<onreapp::instructions::BufferState>(&mut f.svm, buffer, |b| b.gross_apr = 100_000);
    advance_clock_by(&mut f.svm, 31_536_000);
    let instruction = f.take(2_000_000, 990_000_000);
    f.send(instruction).unwrap();
    b = read(&f.svm, f.buffer());
    assert_eq!(b.previous_supply, 2_079_000_000);
    // The old adapter observes the same clock and supply; no second year of accrual.
    let authority = find_offer_vault_authority_pda().0;
    create_token_account(&mut f.svm, &f.asset, &authority, 0);
    create_token_account(&mut f.svm, &f.mint, &authority, 0);
    let instruction = build_take_offer_v2_ix(
        &f.user.pubkey(),
        &f.boss.pubkey(),
        &f.asset,
        &f.mint,
        2_000_000,
        None,
        &TOKEN_PROGRAM_ID,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(&mut f.svm, &[instruction], &[&f.user]).unwrap();
    b = read(&f.svm, f.buffer());
    assert_eq!(b.previous_supply, 4_079_000_000);
    assert_eq!(
        f.balance(f.mint, f.buffer_vaults()[0], f.token_program),
        99_000_000
    );
}

#[test]
fn vector_updates_require_boss_and_valid_scheduling() {
    let mut f = Fixture::new(false, false);
    let now = get_clock_time(&f.svm);
    let vector = PricingVector {
        start_time: now + 10,
        base_time: now,
        base_price: 3_000_000_000,
        apr: 0,
        price_fix_duration: 86400,
    };
    let instruction = ix(
        accounts::AddPricerVector {
            pricer: price(f.mint, 0),
            state: find_state_pda().0,
            boss: f.user.pubkey(),
        },
        instruction::AddPricerVector { vector },
    );
    let result = send_tx(&mut f.svm, &[instruction], &[&f.user]).unwrap_err();
    assert_eq!(
        result.err,
        TransactionError::InstructionError(
            1,
            InstructionError::Custom(6000 + OnreError::InvalidBoss as u32)
        )
    );
    let instruction = ix(
        accounts::AddPricerVector {
            pricer: price(f.mint, 0),
            state: find_state_pda().0,
            boss: f.boss.pubkey(),
        },
        instruction::AddPricerVector { vector },
    );
    send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    advance_clock_by(&mut f.svm, 10);
    let instruction = f.take(3_000_000, 990_000_000);
    f.send(instruction).unwrap();
    assert_eq!(
        f.balance(f.mint, f.user.pubkey(), f.token_program),
        990_000_000
    );
}

#[test]
fn explicit_offer_pricer_is_independent_of_canonical_buffer_pricer() {
    let mut f = Fixture::new(false, false);
    create_token_account(&mut f.svm, &f.mint, &f.boss.pubkey(), 1_000_000_000);
    f.initialize_managed_buffer();
    let buffer = f.buffer();
    edit::<ManagedTokenBufferState>(&mut f.svm, buffer, |b| b.gross_apr = 100_000);
    let selected = price(f.mint, 1);
    let instruction = ix(
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
    );
    send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    let now = get_clock_time(&f.svm);
    let instruction = ix(
        accounts::AddPricerVector {
            pricer: selected,
            state: find_state_pda().0,
            boss: f.boss.pubkey(),
        },
        instruction::AddPricerVector {
            vector: PricingVector {
                start_time: now,
                base_time: now,
                base_price: 3_000_000_000,
                apr: 0,
                price_fix_duration: 86400,
            },
        },
    );
    send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    edit::<OfferConfig>(&mut f.svm, f.offer, |c| c.pricer = selected);
    advance_clock_by(&mut f.svm, 31_536_000);
    let mut instruction = f.take(3_000_000, 990_000_000);
    instruction.accounts[2].pubkey = selected;
    f.send(instruction).unwrap();
    assert_eq!(
        f.balance(f.mint, f.user.pubkey(), f.token_program),
        990_000_000
    );
    let b: ManagedTokenBufferState = read(&f.svm, buffer);
    assert_eq!(b.previous_supply, 2_090_000_000);
    assert_eq!(b.performance_fee_high_watermark, 2_000_000_000);
}

#[test]
fn component_disable_and_transfer_fee_controls_are_enforced() {
    let mut f = Fixture::new(false, false);
    edit::<Pricer>(&mut f.svm, price(f.mint, 0), |c| c.enabled = false);
    let instruction = f.take(100, 0);
    assert_error(f.send(instruction), OnreError::ConfigurationDisabled);
    edit::<Pricer>(&mut f.svm, price(f.mint, 0), |c| c.enabled = true);
    edit::<Quoter>(&mut f.svm, f.quoter, |c| c.enabled = false);
    let instruction = f.take(101, 0);
    assert_error(f.send(instruction), OnreError::ConfigurationDisabled);
    edit::<Quoter>(&mut f.svm, f.quoter, |c| c.enabled = true);
    edit::<FeeConfig>(&mut f.svm, f.fee, |c| c.enabled = false);
    let instruction = f.take(102, 0);
    assert_error(f.send(instruction), OnreError::ConfigurationDisabled);
    edit::<FeeConfig>(&mut f.svm, f.fee, |c| c.enabled = true);
    edit::<OfferConfig>(&mut f.svm, f.offer, |c| c.enabled = false);
    let instruction = f.take(103, 0);
    assert_error(f.send(instruction), OnreError::ConfigurationDisabled);

    let mut f = Fixture::new(false, true);
    let fee_mint =
        create_mint_2022_with_transfer_fee(&mut f.svm, &f.boss, 6, &f.boss.pubkey(), 100, 1000);
    let mut asset = f.svm.get_account(&fee_mint).unwrap();
    asset.data[36..44].copy_from_slice(&10_000_000_u64.to_le_bytes());
    f.svm.set_account(f.asset, asset).unwrap();
    let instruction = f.take(100, 0);
    assert_error(f.send(instruction), OnreError::TransferFeeNotSupported);
}

#[test]
fn execution_failure_rolls_back_buffer_and_fee_splits() {
    let mut f = Fixture::new(false, false);
    create_token_account(&mut f.svm, &f.mint, &f.boss.pubkey(), 1_000_000_000);
    f.initialize_managed_buffer();
    let buffer = f.buffer();
    edit::<ManagedTokenBufferState>(&mut f.svm, buffer, |b| {
        b.gross_apr = 100_000;
        b.management_fee_basis_points = 100;
        b.performance_fee_basis_points = 2000;
    });
    advance_clock_by(&mut f.svm, 31_536_000);
    let before = f.svm.get_account(&buffer).unwrap();
    // Accrual fits, but the subsequent user mint exceeds the total cap.
    edit::<ManagedTokenConfig>(&mut f.svm, managed(f.mint), |c| {
        c.max_supply = 1_500_000_000
    });
    let instruction = f.take(2_000_000, 990_000_000);
    assert_error(f.send(instruction), OnreError::MaxSupplyExceeded);
    assert_eq!(f.svm.get_account(&buffer).unwrap().data, before.data);
    for owner in f.buffer_vaults() {
        assert_eq!(f.balance(f.mint, owner, f.token_program), 0);
    }
    edit::<ManagedTokenConfig>(&mut f.svm, managed(f.mint), |c| c.max_supply = 0);
    let instruction = f.take(2_000_001, 0);
    f.send(instruction).unwrap();
    let [reserve, management, performance] = f.buffer_vaults();
    assert_eq!(f.balance(f.mint, reserve, f.token_program), 90_000_000);
    assert_eq!(f.balance(f.mint, management, f.token_program), 10_000_000);
    assert_eq!(f.balance(f.mint, performance, f.token_program), 0);
}

fn executed_event(meta: &TransactionMetadata) -> OfferExecutedEvent {
    use anchor_lang::{
        __private::base64::{engine::general_purpose::STANDARD, Engine},
        Discriminator,
    };
    meta.logs
        .iter()
        .filter_map(|log| log.strip_prefix("Program data: "))
        .filter_map(|data| STANDARD.decode(data).ok())
        .find_map(|data| {
            data.strip_prefix(OfferExecutedEvent::DISCRIMINATOR)
                .map(|data| OfferExecutedEvent::deserialize(&mut &data[..]).unwrap())
        })
        .unwrap()
}

fn omit_account(instruction: &mut Instruction, key: Pubkey) {
    let account = instruction
        .accounts
        .iter_mut()
        .find(|a| a.pubkey == key)
        .unwrap();
    account.pubkey = PROGRAM_ID;
    account.is_writable = false;
}

#[test]
fn one_entrypoint_executes_both_directions_and_buffer_tracks_the_managed_input() {
    for token_2022 in [false, true] {
        let mut f = Fixture::new(false, token_2022);
        let instruction = f.take(2_000_000, 990_000_000);
        f.send(instruction).unwrap();
        f.initialize_managed_buffer();
        let buffer = f.buffer();
        edit::<ManagedTokenBufferState>(&mut f.svm, buffer, |b| b.gross_apr = 100_000);
        f.create_route(
            OfferDirection::ManagedToAsset,
            OfferFlow::Permissioned,
            QuoterKind::Nav,
            true,
        );
        f.seed_tokens(f.asset, vault(VaultKind::Liquidity), 2_000_000);
        advance_clock_by(&mut f.svm, 31_536_000);
        let instruction = f.take(990_000_000, 1_960_200);
        let result = f.send(instruction).unwrap();
        assert_eq!(f.balance(f.mint, f.user.pubkey(), f.token_program), 0);
        assert_eq!(
            f.balance(f.asset, f.user.pubkey(), f.asset_program),
            9_960_200
        );
        assert_eq!(
            f.balance(f.mint, vault(VaultKind::Fee), f.token_program),
            9_900_000
        );
        assert_eq!(
            f.balance(f.asset, vault(VaultKind::Liquidity), f.asset_program),
            39_800
        );
        assert_eq!(get_mint_supply(&f.svm, &f.mint), 108_900_000);
        let b: ManagedTokenBufferState = read(&f.svm, buffer);
        assert_eq!(b.previous_supply, 108_900_000);
        assert_eq!(b.last_accrual_timestamp, get_clock_time(&f.svm) as i64);
        assert_eq!(
            f.balance(f.mint, f.buffer_vaults()[0], f.token_program),
            99_000_000
        );
        let event = executed_event(&result);
        assert_eq!(event.direction, OfferDirection::ManagedToAsset);
        assert_eq!(event.proceeds_amount, 0);
        assert_eq!(event.liquidity_amount, 1_960_200);
        assert!(f
            .svm
            .get_account(&derive_ata(
                &vault(VaultKind::Proceeds),
                &f.mint,
                &f.token_program
            ))
            .is_none());
    }
}

#[test]
fn onyc_uses_only_selected_liquidity_and_keeps_the_legacy_buffer() {
    for (legacy, current, succeeds) in [
        (3_000_000, 3_000_000, true),
        (3_000_000, 1_000_000, false),
        (3_000_000, 0, false),
        (0, 3_000_000, true),
    ] {
        let mut f = Fixture::new(true, false);
        f.initialize_legacy_main_offer();
        f.seed_tokens(f.mint, f.user.pubkey(), 1_000_000_000);
        send_tx(
            &mut f.svm,
            &[build_initialize_buffer_ix(
                &f.boss.pubkey(),
                &find_offer_pda(&f.asset, &f.mint).0,
                &f.mint,
            )],
            &[&f.boss],
        )
        .unwrap();
        f.create_route(
            OfferDirection::ManagedToAsset,
            OfferFlow::Permissioned,
            QuoterKind::Nav,
            true,
        );
        if legacy > 0 {
            f.seed_tokens(f.asset, find_redemption_vault_authority_pda().0, legacy);
        }
        if current > 0 {
            f.seed_tokens(f.asset, vault(VaultKind::Liquidity), current);
        }
        let buffer_before = f.svm.get_account(&f.buffer()).unwrap().data;
        let instruction = f.take(1_000_000_000, 1_980_000);
        let paid = if succeeds {
            let result = f.send(instruction).unwrap();
            assert_eq!(executed_event(&result).liquidity_amount, 1_980_000);
            1_980_000
        } else {
            assert_error(f.send(instruction), OnreError::InsufficientBalance);
            0
        };
        assert_eq!(
            f.balance(f.asset, f.user.pubkey(), f.asset_program),
            10_000_000 + paid
        );
        if legacy > 0 {
            assert_eq!(
                f.balance(
                    f.asset,
                    find_redemption_vault_authority_pda().0,
                    f.asset_program
                ),
                legacy
            );
        }
        if current > 0 {
            assert_eq!(
                f.balance(f.asset, vault(VaultKind::Liquidity), f.asset_program),
                current - paid
            );
        }
        let supply = if succeeds { 10_000_000 } else { 1_000_000_000 };
        assert_eq!(get_mint_supply(&f.svm, &f.mint), supply);
        if succeeds {
            let buffer: onreapp::instructions::BufferState = read(&f.svm, f.buffer());
            assert_eq!(buffer.previous_supply, supply);
        } else {
            assert_eq!(f.svm.get_account(&f.buffer()).unwrap().data, buffer_before);
        }
    }
}

#[test]
fn insufficient_selected_liquidity_rolls_back_accrual_fees_and_burn() {
    let mut f = Fixture::new(false, false);
    f.seed_tokens(f.mint, f.user.pubkey(), 1_000_000_000);
    f.initialize_managed_buffer();
    let buffer = f.buffer();
    edit::<ManagedTokenBufferState>(&mut f.svm, buffer, |b| b.gross_apr = 100_000);
    f.create_route(
        OfferDirection::ManagedToAsset,
        OfferFlow::Permissioned,
        QuoterKind::Nav,
        true,
    );
    f.seed_tokens(f.asset, vault(VaultKind::Liquidity), 1_000_000);
    advance_clock_by(&mut f.svm, 31_536_000);
    let before = f.svm.get_account(&buffer).unwrap().data;
    let instruction = f.take(1_000_000_000, 0);
    assert_error(f.send(instruction), OnreError::InsufficientBalance);
    assert_eq!(f.svm.get_account(&buffer).unwrap().data, before);
    assert_eq!(get_mint_supply(&f.svm, &f.mint), 1_000_000_000);
    assert_eq!(
        f.balance(f.mint, f.user.pubkey(), f.token_program),
        1_000_000_000
    );
    assert_eq!(f.balance(f.mint, f.buffer_vaults()[0], f.token_program), 0);
    assert_eq!(
        f.balance(f.asset, vault(VaultKind::Liquidity), f.asset_program),
        1_000_000
    );
    assert!(f
        .svm
        .get_account(&derive_ata(
            &vault(VaultKind::Fee),
            &f.mint,
            &f.token_program
        ))
        .is_none());
}

#[test]
fn cannot_omit_or_substitute_selected_liquidity() {
    let mut f = Fixture::new(true, false);
    f.initialize_legacy_main_offer();
    f.seed_tokens(f.mint, f.user.pubkey(), 1_000_000_000);
    f.create_route(
        OfferDirection::ManagedToAsset,
        OfferFlow::Permissioned,
        QuoterKind::Nav,
        true,
    );
    let legacy = find_redemption_vault_authority_pda().0;
    f.seed_tokens(f.asset, legacy, 4_000_000);
    let old_ata = derive_ata(&legacy, &f.asset, &f.asset_program);
    let new_ata = derive_ata(&vault(VaultKind::Liquidity), &f.asset, &f.asset_program);
    for (account, expected) in [
        (
            vault(VaultKind::Liquidity),
            OnreError::LiquidityVaultRequired,
        ),
        (new_ata, OnreError::InvalidConfigurableVaultTokenAccount),
    ] {
        let mut instruction = f.take(1_000_000_000, 0);
        omit_account(&mut instruction, account);
        assert_error(f.send(instruction), expected);
    }
    let mut instruction = f.take(1_000_000_001, 0);
    instruction
        .accounts
        .iter_mut()
        .find(|a| a.pubkey == new_ata)
        .unwrap()
        .pubkey = old_ata;
    assert_error(
        f.send(instruction),
        OnreError::InvalidConfigurableVaultTokenAccount,
    );
    assert_eq!(f.balance(f.asset, legacy, f.asset_program), 4_000_000);
    assert_eq!(
        f.balance(f.mint, f.user.pubkey(), f.token_program),
        1_000_000_000
    );
}

#[test]
fn prop_amm_requires_only_its_selected_pair_before_dispatch() {
    for onyc in [false, true] {
        let mut f = Fixture::new(onyc, false);
        f.seed_tokens(f.mint, f.user.pubkey(), 1_000_000_000);
        f.create_route(
            OfferDirection::ManagedToAsset,
            OfferFlow::Permissionless,
            QuoterKind::PropAmm,
            true,
        );
        let config: OfferConfig = read(&f.svm, f.offer);
        let pair = pda(&[b"prop_amm", config.quoter.as_ref()]);
        let before = f.svm.get_account(&pair).unwrap().data;
        let mut instruction = f.take(1_000_000_000, 0);
        omit_account(&mut instruction, pair);
        assert_error(f.send(instruction), OnreError::IncompatibleQuoter);
        let instruction = f.take(1_000_000_002, 0);
        assert_error(f.send(instruction), OnreError::InsufficientBalance);
        assert_eq!(f.svm.get_account(&pair).unwrap().data, before);
        assert_eq!(
            f.balance(f.mint, f.user.pubkey(), f.token_program),
            1_000_000_000
        );
    }
}

#[test]
fn offers_can_share_a_preexisting_modular_or_legacy_liquidity_vault() {
    use onreapp::state::ConfigurableVaultKind;

    for token_2022 in [false, true] {
        for legacy_kind in [
            None,
            Some(ConfigurableVaultKind::PropAmmProceeds),
            Some(ConfigurableVaultKind::OfferProceeds),
        ] {
            let mut f = Fixture::new(false, token_2022);
            let selected = if let Some(kind) = legacy_kind {
                let selected = pda(&[b"configurable_vault", kind.seed()]);
                let setup = build_set_configurable_vault_destination_ix(
                    &f.boss.pubkey(),
                    &selected,
                    kind.as_u8(),
                    &f.boss.pubkey(),
                );
                send_tx(&mut f.svm, &[setup], &[&f.boss]).unwrap();
                selected
            } else {
                // A preexisting instance with a nonzero identity can be reused.
                let selected = pda(&[
                    b"vault",
                    &[VaultKind::Liquidity as u8],
                    &17_u64.to_le_bytes(),
                ]);
                let setup = ix(
                    accounts::CreateVaultConfig {
                        vault: selected,
                        state: find_state_pda().0,
                        boss: f.boss.pubkey(),
                        system_program: SYSTEM_PROGRAM_ID,
                    },
                    instruction::CreateVaultConfig {
                        kind: VaultKind::Liquidity,
                        instance_id: 17,
                        withdrawal_destination: f.boss.pubkey(),
                        refill_target_bps: 0,
                    },
                );
                send_tx(&mut f.svm, &[setup], &[&f.boss]).unwrap();
                selected
            };
            let configuration_before = f.svm.get_account(&selected).unwrap().data;
            f.seed_tokens(f.mint, f.user.pubkey(), 1_000_000_000);
            f.seed_tokens(f.asset, selected, 3_000_000);
            // These are two actual boss-authorized offer creations referencing
            // the same vault. No rebinding or migration is needed.
            for (flow, quoter) in [
                (OfferFlow::Permissioned, QuoterKind::Nav),
                (OfferFlow::Permissionless, QuoterKind::NavPermissionless),
            ] {
                f.create_route_with_vault(
                    OfferDirection::ManagedToAsset,
                    flow,
                    quoter,
                    Some(selected),
                );
                let instruction = f.take(500_000_000, 990_000);
                let result = if flow == OfferFlow::Permissioned {
                    f.send(instruction)
                } else {
                    f.send_without_approval(instruction)
                }
                .unwrap();
                assert_eq!(executed_event(&result).liquidity_amount, 990_000);
            }
            assert_eq!(f.balance(f.asset, selected, f.asset_program), 1_020_000);
            assert_eq!(
                f.balance(f.asset, f.user.pubkey(), f.asset_program),
                11_980_000
            );
            assert_eq!(get_mint_supply(&f.svm, &f.mint), 10_000_000);
            assert_eq!(
                f.svm.get_account(&selected).unwrap().data,
                configuration_before
            );
        }
    }
}

#[test]
fn modular_offers_can_explicitly_share_the_old_redemption_vault() {
    for token_2022 in [false, true] {
        let mut f = Fixture::new(false, token_2022);
        if token_2022 {
            f.asset = create_mint_2022(&mut f.svm, &f.boss, 6, &f.boss.pubkey());
            f.asset_program = TOKEN_2022_PROGRAM_ID;
            f.seed_tokens(f.asset, f.user.pubkey(), 10_000_000);
        }
        let selected = find_redemption_vault_authority_pda().0;
        f.seed_tokens(f.mint, f.user.pubkey(), 1_000_000_000);
        f.seed_tokens(f.asset, selected, 3_000_000);
        // The redemption authority itself is a stateless PDA. Its ATA is custody.
        for (flow, quoter) in [
            (OfferFlow::Permissioned, QuoterKind::Nav),
            (OfferFlow::Permissionless, QuoterKind::NavPermissionless),
        ] {
            f.create_route_with_vault(OfferDirection::ManagedToAsset, flow, quoter, Some(selected));
            let instruction = f.take(500_000_000, 990_000);
            let result = if flow == OfferFlow::Permissioned {
                f.send(instruction)
            } else {
                f.send_without_approval(instruction)
            }
            .unwrap();
            assert_eq!(executed_event(&result).liquidity_amount, 990_000);
        }
        assert_eq!(f.balance(f.asset, selected, f.asset_program), 1_020_000);
        assert_eq!(
            f.balance(f.asset, f.user.pubkey(), f.asset_program),
            11_980_000
        );
        assert_eq!(get_mint_supply(&f.svm, &f.mint), 10_000_000);
    }
}
