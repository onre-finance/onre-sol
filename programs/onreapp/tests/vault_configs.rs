mod common;

use anchor_lang::{
    AccountDeserialize, AccountSerialize, AnchorDeserialize, Event, InstructionData, ToAccountMetas,
};
use common::*;
use litesvm::{
    types::{FailedTransactionMetadata, TransactionMetadata},
    LiteSVM,
};
use onreapp::{
    accounts, instruction,
    instructions::vault_operations::{VaultConfigDepositedEvent, VaultConfigWithdrawnEvent},
    model::*,
    OnreError,
};
use solana_sdk::{
    instruction::{Instruction, InstructionError},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::TransactionError,
};

fn pda(seeds: &[&[u8]]) -> Pubkey {
    Pubkey::find_program_address(seeds, &PROGRAM_ID).0
}
fn ix(accounts: impl ToAccountMetas, data: impl InstructionData) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}
fn event<E: Event>(meta: &TransactionMetadata) -> E {
    use anchor_lang::__private::base64::{engine::general_purpose::STANDARD, Engine};
    meta.logs
        .iter()
        .filter_map(|s| s.strip_prefix("Program data: "))
        .filter_map(|s| STANDARD.decode(s).ok())
        .find_map(|bytes| {
            bytes
                .strip_prefix(E::DISCRIMINATOR)
                .map(|data| E::deserialize(&mut &data[..]).unwrap())
        })
        .unwrap()
}
fn assert_error(
    result: Result<TransactionMetadata, FailedTransactionMetadata>,
    expected: OnreError,
) {
    let e = result.unwrap_err();
    assert_eq!(
        e.err,
        TransactionError::InstructionError(1, InstructionError::Custom(6000 + expected as u32)),
        "{:?}",
        e.meta.logs
    );
}

struct Fixture {
    svm: LiteSVM,
    boss: Keypair,
    user: Keypair,
    destination: Pubkey,
    mint: Pubkey,
    program: Pubkey,
}
impl Fixture {
    fn new(token_2022: bool) -> Self {
        let (mut svm, boss, _) = setup_initialized();
        let user = Keypair::new();
        let destination = Pubkey::new_unique();
        svm.airdrop(&user.pubkey(), INITIAL_LAMPORTS).unwrap();
        let program = if token_2022 {
            TOKEN_2022_PROGRAM_ID
        } else {
            TOKEN_PROGRAM_ID
        };
        let mint = if token_2022 {
            create_mint_2022(&mut svm, &boss, 6, &boss.pubkey())
        } else {
            create_mint(&mut svm, &boss, 6, &boss.pubkey())
        };
        if token_2022 {
            create_token_account_2022(&mut svm, &mint, &user.pubkey(), 1000);
        } else {
            create_token_account(&mut svm, &mint, &user.pubkey(), 1000);
        }
        Self {
            svm,
            boss,
            user,
            destination,
            mint,
            program,
        }
    }
    fn create(&mut self, kind: VaultKind) -> Pubkey {
        let vault = pda(&[b"vault", &[kind as u8], &7_u64.to_le_bytes()]);
        let instruction = ix(
            accounts::CreateVaultConfig {
                vault,
                state: find_state_pda().0,
                boss: self.boss.pubkey(),
                system_program: SYSTEM_PROGRAM_ID,
            },
            instruction::CreateVaultConfig {
                kind,
                instance_id: 7,
                withdrawal_destination: self.destination,
                refill_target_bps: 0,
            },
        );
        send_tx(&mut self.svm, &[instruction], &[&self.boss]).unwrap();
        vault
    }
    fn buffer_vaults(&mut self) -> [Pubkey; 3] {
        let managed = pda(&[b"managed_token", self.mint.as_ref()]);
        let pricer = pda(&[b"pricer", self.mint.as_ref(), &[0], &0_u64.to_le_bytes()]);
        let vaults = [
            b"buffer_reserve".as_slice(),
            b"buffer_management",
            b"buffer_performance",
        ]
        .map(|role| pda(&[role, self.mint.as_ref()]));
        for instruction in [
            ix(
                accounts::RegisterManagedToken {
                    managed_mint: self.mint,
                    managed_token: managed,
                    state: find_state_pda().0,
                    boss: self.boss.pubkey(),
                    system_program: SYSTEM_PROGRAM_ID,
                },
                instruction::RegisterManagedToken {},
            ),
            ix(
                accounts::CreatePricer {
                    managed_token: managed,
                    pricer,
                    state: find_state_pda().0,
                    boss: self.boss.pubkey(),
                    system_program: SYSTEM_PROGRAM_ID,
                },
                instruction::CreatePricer {
                    instance_id: 0,
                    denomination: PricingDenomination::Usd,
                },
            ),
            ix(
                accounts::InitializeManagedTokenBuffer {
                    managed_token: managed,
                    managed_mint: self.mint,
                    pricer,
                    buffer_state: pda(&[b"buffer", self.mint.as_ref()]),
                    reserve_vault: vaults[0],
                    management_fee_vault: vaults[1],
                    performance_fee_vault: vaults[2],
                    state: find_state_pda().0,
                    boss: self.boss.pubkey(),
                    system_program: SYSTEM_PROGRAM_ID,
                },
                instruction::InitializeManagedTokenBuffer {},
            ),
        ] {
            send_tx(&mut self.svm, &[instruction], &[&self.boss]).unwrap();
        }
        vaults
    }
    fn deposit(&self, vault: Pubkey, amount: u64) -> Instruction {
        ix(
            accounts::DepositVaultConfig {
                vault,
                mint: self.mint,
                depositor_token_account: derive_ata(&self.user.pubkey(), &self.mint, &self.program),
                vault_token_account: derive_ata(&vault, &self.mint, &self.program),
                depositor: self.user.pubkey(),
                state: find_state_pda().0,
                token_program: self.program,
                associated_token_program: ATA_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            },
            instruction::DepositVaultConfig { amount },
        )
    }
    fn withdraw(&self, vault: Pubkey, boss: bool, destination: Pubkey, amount: u64) -> Instruction {
        ix(
            accounts::WithdrawVaultConfig {
                vault,
                mint: self.mint,
                vault_token_account: derive_ata(&vault, &self.mint, &self.program),
                destination,
                destination_token_account: derive_ata(&destination, &self.mint, &self.program),
                caller: if boss {
                    self.boss.pubkey()
                } else {
                    self.user.pubkey()
                },
                state: find_state_pda().0,
                token_program: self.program,
                associated_token_program: ATA_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            },
            instruction::WithdrawVaultConfig { amount },
        )
    }
    fn send(
        &mut self,
        instruction: Instruction,
        boss: bool,
    ) -> Result<TransactionMetadata, FailedTransactionMetadata> {
        self.svm.expire_blockhash();
        send_tx(
            &mut self.svm,
            &[instruction],
            &[if boss { &self.boss } else { &self.user }],
        )
    }
    fn balance(&self, owner: Pubkey) -> u64 {
        let account = self
            .svm
            .get_account(&derive_ata(&owner, &self.mint, &self.program))
            .unwrap();
        u64::from_le_bytes(account.data[64..72].try_into().unwrap())
    }
}

#[test]
fn all_generic_vault_kinds_support_deposit_partial_withdraw_and_withdraw_all() {
    for token_2022 in [false, true] {
        for kind in [
            VaultKind::Fee,
            VaultKind::Proceeds,
            VaultKind::Liquidity,
            VaultKind::BufferReserve,
        ] {
            let mut f = Fixture::new(token_2022);
            let vault = f.create(kind);
            let before = f.svm.get_account(&vault).unwrap().data;
            let instruction = f.deposit(vault, 600);
            let deposited: VaultConfigDepositedEvent = event(&f.send(instruction, false).unwrap());
            assert_eq!(
                (
                    deposited.vault,
                    deposited.mint,
                    deposited.depositor,
                    deposited.amount
                ),
                (vault, f.mint, f.user.pubkey(), 600)
            );
            let restricted = matches!(kind, VaultKind::Liquidity | VaultKind::BufferReserve);
            if restricted {
                let instruction = f.withdraw(vault, false, f.destination, 100);
                assert_error(f.send(instruction, false), OnreError::InvalidBoss);
                assert_eq!(f.balance(vault), 600);
                assert!(f
                    .svm
                    .get_account(&derive_ata(&f.destination, &f.mint, &f.program))
                    .is_none());
            }
            let instruction = f.withdraw(vault, restricted, f.destination, 100);
            let withdrawn: VaultConfigWithdrawnEvent =
                event(&f.send(instruction, restricted).unwrap());
            assert_eq!(
                (
                    withdrawn.vault,
                    withdrawn.mint,
                    withdrawn.destination,
                    withdrawn.amount
                ),
                (vault, f.mint, f.destination, 100)
            );
            assert_eq!(
                withdrawn.caller,
                if restricted {
                    f.boss.pubkey()
                } else {
                    f.user.pubkey()
                }
            );
            let instruction = f.withdraw(vault, restricted, f.destination, 0);
            f.send(instruction, restricted).unwrap();
            assert_eq!(f.balance(vault), 0);
            assert_eq!(f.balance(f.destination), 600);
            assert_eq!(f.balance(f.user.pubkey()), 400);
            assert_eq!(f.svm.get_account(&vault).unwrap().data, before);
            let instruction = f.withdraw(vault, restricted, f.destination, 0);
            assert_error(f.send(instruction, restricted), OnreError::ZeroBalance);
        }
    }
}

#[test]
fn per_mint_buffer_vaults_use_their_role_seeds_without_changing_accrual_state() {
    for token_2022 in [false, true] {
        let mut f = Fixture::new(token_2022);
        let vaults = f.buffer_vaults();
        let buffer = pda(&[b"buffer", f.mint.as_ref()]);
        let before = f.svm.get_account(&buffer).unwrap().data;
        for (index, vault) in vaults.into_iter().enumerate() {
            let instruction = f.deposit(vault, 100);
            f.send(instruction, false).unwrap();
            let instruction = f.withdraw(vault, index == 0, f.boss.pubkey(), 0);
            f.send(instruction, index == 0).unwrap();
            assert_eq!(f.balance(vault), 0);
        }
        assert_eq!(f.balance(f.boss.pubkey()), 300);
        assert_eq!(f.svm.get_account(&buffer).unwrap().data, before);
    }
}

#[test]
fn withdrawal_cannot_redirect_and_overdraw_and_zero_deposit_are_atomic() {
    let mut f = Fixture::new(false);
    let vault = f.create(VaultKind::Fee);
    let instruction = f.deposit(vault, 0);
    assert_error(f.send(instruction, false), OnreError::ZeroValue);
    assert!(f
        .svm
        .get_account(&derive_ata(&vault, &f.mint, &f.program))
        .is_none());
    let instruction = f.deposit(vault, 1001);
    assert!(f.send(instruction, false).is_err());
    assert_eq!(f.balance(f.user.pubkey()), 1000);
    assert!(f
        .svm
        .get_account(&derive_ata(&vault, &f.mint, &f.program))
        .is_none());
    let instruction = f.deposit(vault, 500);
    f.send(instruction, false).unwrap();
    for (destination, amount, error) in [
        (f.user.pubkey(), 100, OnreError::InvalidFeeRecipient),
        (f.destination, 501, OnreError::InsufficientBalance),
    ] {
        let instruction = f.withdraw(vault, false, destination, amount);
        assert_error(f.send(instruction, false), error);
        assert_eq!(f.balance(vault), 500);
    }
    assert!(f
        .svm
        .get_account(&derive_ata(&f.destination, &f.mint, &f.program))
        .is_none());
}

#[test]
fn vault_movements_obey_the_global_kill_switch() {
    let mut f = Fixture::new(false);
    let vault = f.create(VaultKind::Liquidity);
    let instruction = f.deposit(vault, 500);
    f.send(instruction, false).unwrap();
    let instruction = build_set_kill_switch_ix(&f.boss.pubkey(), true);
    f.send(instruction, true).unwrap();
    for (instruction, boss) in [
        (f.deposit(vault, 1), false),
        (f.withdraw(vault, true, f.destination, 1), true),
    ] {
        assert_error(f.send(instruction, boss), OnreError::KillSwitchActivated);
        assert_eq!(f.balance(vault), 500);
    }
}

#[test]
fn forged_vault_identity_and_corrupt_bump_cannot_move_funds() {
    let mut f = Fixture::new(false);
    let vault = f.create(VaultKind::Liquidity);
    let original = f.svm.get_account(&vault).unwrap();
    let forged = Pubkey::new_unique();
    f.svm.set_account(forged, original.clone()).unwrap();
    let instruction = f.deposit(forged, 100);
    assert_error(
        f.send(instruction, false),
        OnreError::InvalidConfigurableVault,
    );
    let mut value = VaultConfig::try_deserialize(&mut original.data.as_slice()).unwrap();
    value.bump ^= 255;
    let mut corrupt = original.clone();
    corrupt.data.clear();
    value.try_serialize(&mut corrupt.data).unwrap();
    f.svm.set_account(vault, corrupt).unwrap();
    let instruction = f.deposit(vault, 100);
    assert_error(
        f.send(instruction, false),
        OnreError::InvalidConfigurableVault,
    );
    assert_eq!(f.balance(f.user.pubkey()), 1000);
}

#[test]
fn existing_ata_with_changed_authority_is_rejected() {
    let mut f = Fixture::new(false);
    let vault = f.create(VaultKind::Fee);
    let instruction = f.deposit(vault, 500);
    f.send(instruction, false).unwrap();
    let ata = derive_ata(&vault, &f.mint, &f.program);
    let original = f.svm.get_account(&ata).unwrap();
    let mut account = original.clone();
    account.data[32..64].copy_from_slice(f.user.pubkey().as_ref());
    f.svm.set_account(ata, account).unwrap();
    let instruction = f.deposit(vault, 100);
    assert!(f.send(instruction, false).is_err());
    assert_eq!(f.balance(vault), 500);
    assert_eq!(f.balance(f.user.pubkey()), 500);
    f.svm.set_account(ata, original).unwrap();
    let destination_ata = create_token_account(&mut f.svm, &f.mint, &f.destination, 0);
    let mut redirected = f.svm.get_account(&destination_ata).unwrap();
    redirected.data[32..64].copy_from_slice(f.user.pubkey().as_ref());
    f.svm.set_account(destination_ata, redirected).unwrap();
    let instruction = f.withdraw(vault, false, f.destination, 100);
    assert!(f.send(instruction, false).is_err());
    assert_eq!(f.balance(vault), 500);
    assert_eq!(f.balance(f.destination), 0);
}

#[test]
fn transfer_fee_mints_are_rejected_for_deposit_and_withdrawal() {
    let mut f = Fixture::new(true);
    let vault = f.create(VaultKind::Fee);
    f.mint =
        create_mint_2022_with_transfer_fee(&mut f.svm, &f.boss, 6, &f.boss.pubkey(), 100, 1000);
    create_token_account_2022(&mut f.svm, &f.mint, &f.user.pubkey(), 1000);
    create_token_account_2022(&mut f.svm, &f.mint, &vault, 500);
    let instruction = f.deposit(vault, 100);
    assert_error(
        f.send(instruction, false),
        OnreError::TransferFeeNotSupported,
    );
    let instruction = f.withdraw(vault, false, f.destination, 100);
    assert_error(
        f.send(instruction, false),
        OnreError::TransferFeeNotSupported,
    );
    assert_eq!(f.balance(vault), 500);
    assert_eq!(f.balance(f.user.pubkey()), 1000);
}

#[test]
fn buffer_custody_cannot_be_rebound_to_another_mint() {
    let mut f = Fixture::new(false);
    let vaults = f.buffer_vaults();
    f.mint = create_mint(&mut f.svm, &f.boss, 6, &f.boss.pubkey());
    create_token_account(&mut f.svm, &f.mint, &f.user.pubkey(), 1000);
    for vault in vaults {
        let instruction = f.deposit(vault, 100);
        assert_error(
            f.send(instruction, false),
            OnreError::InvalidConfigurableVault,
        );
        assert!(f
            .svm
            .get_account(&derive_ata(&vault, &f.mint, &f.program))
            .is_none());
        // Even a funded ATA for the wrong mint must not authorize a withdrawal.
        create_token_account(&mut f.svm, &f.mint, &vault, 50);
        let instruction = f.withdraw(vault, true, f.boss.pubkey(), 50);
        assert_error(
            f.send(instruction, true),
            OnreError::InvalidConfigurableVault,
        );
        assert_eq!(f.balance(vault), 50);
    }
}
