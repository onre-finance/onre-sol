#![allow(dead_code)]

use anchor_lang::AccountDeserialize;
use litesvm::LiteSVM;
use onreapp::instructions::RedemptionRequest;
use solana_sdk::{
    account::Account,
    clock::Clock,
    instruction::{AccountMeta, Instruction},
    message::Message,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};
use std::convert::TryInto;

// ---------------------------------------------------------------------------
// Program IDs
// ---------------------------------------------------------------------------
pub const PROGRAM_ID: Pubkey = solana_sdk::pubkey!("onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe");
pub const TOKEN_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const TOKEN_2022_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
pub const ATA_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
pub const SYSTEM_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("11111111111111111111111111111111");
pub const BPF_UPGRADEABLE_LOADER_ID: Pubkey =
    solana_sdk::pubkey!("BPFLoaderUpgradeab1e11111111111111111111111");
pub const ED25519_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("Ed25519SigVerify111111111111111111111111111");
pub const SYSVAR_INSTRUCTIONS_ID: Pubkey =
    solana_sdk::pubkey!("Sysvar1nstructions1111111111111111111111111");

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
pub const INITIAL_LAMPORTS: u64 = 1_000_000_000;
pub const MAX_ADMINS: usize = 20;

// PDA seeds (must match constants.rs in the program)
pub const STATE_SEED: &[u8] = b"state";
pub const OFFER_SEED: &[u8] = b"offer";
pub const OFFER_VAULT_AUTHORITY_SEED: &[u8] = b"offer_vault_authority";
pub const REDEMPTION_OFFER_VAULT_AUTHORITY_SEED: &[u8] = b"redemption_offer_vault_authority";
pub const PERMISSIONLESS_AUTHORITY_SEED: &[u8] = b"permissionless-1";
pub const MINT_AUTHORITY_SEED: &[u8] = b"mint_authority";
pub const CACHE_STATE_SEED: &[u8] = b"cache_state";
pub const CACHE_VAULT_AUTHORITY_SEED: &[u8] = b"cache_vault_authority";

// ---------------------------------------------------------------------------
// ATA derivation
// ---------------------------------------------------------------------------
pub fn get_associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[wallet.as_ref(), TOKEN_PROGRAM_ID.as_ref(), mint.as_ref()],
        &ATA_PROGRAM_ID,
    )
    .0
}

pub fn get_associated_token_address_2022(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            wallet.as_ref(),
            TOKEN_2022_PROGRAM_ID.as_ref(),
            mint.as_ref(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0
}

pub fn derive_ata(owner: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), token_program.as_ref(), mint.as_ref()],
        &ATA_PROGRAM_ID,
    )
    .0
}

// ---------------------------------------------------------------------------
// Anchor discriminators
// ---------------------------------------------------------------------------
pub fn anchor_discriminator(namespace: &str, name: &str) -> [u8; 8] {
    let preimage = format!("{}:{}", namespace, name);
    let hash = solana_sdk::hash::hash(preimage.as_bytes());
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash.to_bytes()[..8]);
    disc
}

pub fn ix_discriminator(name: &str) -> [u8; 8] {
    anchor_discriminator("global", name)
}

// ---------------------------------------------------------------------------
// PDA derivation helpers
// ---------------------------------------------------------------------------
pub fn find_state_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[STATE_SEED], &PROGRAM_ID)
}

pub fn find_offer_pda(token_in_mint: &Pubkey, token_out_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[OFFER_SEED, token_in_mint.as_ref(), token_out_mint.as_ref()],
        &PROGRAM_ID,
    )
}

pub fn find_offer_vault_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[OFFER_VAULT_AUTHORITY_SEED], &PROGRAM_ID)
}

pub fn find_redemption_vault_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[REDEMPTION_OFFER_VAULT_AUTHORITY_SEED], &PROGRAM_ID)
}

pub fn find_permissionless_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PERMISSIONLESS_AUTHORITY_SEED], &PROGRAM_ID)
}

pub fn find_mint_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MINT_AUTHORITY_SEED], &PROGRAM_ID)
}

pub fn find_cache_state_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CACHE_STATE_SEED], &PROGRAM_ID)
}

pub fn find_cache_vault_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CACHE_VAULT_AUTHORITY_SEED], &PROGRAM_ID)
}

pub fn find_program_data_pda() -> Pubkey {
    Pubkey::find_program_address(&[PROGRAM_ID.as_ref()], &BPF_UPGRADEABLE_LOADER_ID).0
}

// ---------------------------------------------------------------------------
// Transaction helpers
// ---------------------------------------------------------------------------
pub fn send_tx(
    svm: &mut LiteSVM,
    ixs: &[Instruction],
    signers: &[&Keypair],
) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    let payer = signers[0].pubkey();
    let blockhash = svm.latest_blockhash();
    let msg = Message::new(ixs, Some(&payer));
    let tx = Transaction::new(signers, msg, blockhash);
    svm.send_transaction(tx)
}

pub fn get_token_balance(svm: &LiteSVM, token_account: &Pubkey) -> u64 {
    let account = svm.get_account(token_account).expect("account not found");
    u64::from_le_bytes(account.data[64..72].try_into().unwrap())
}

// ---------------------------------------------------------------------------
// Setup: load program as upgradeable, create payer as upgrade authority
// ---------------------------------------------------------------------------
pub fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();

    // Create payer FIRST so we can set them as upgrade authority
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 100 * INITIAL_LAMPORTS)
        .unwrap();

    let program_bytes = include_bytes!("../../../../target/deploy/onreapp.so");
    let program_data_pda = find_program_data_pda();

    // UpgradeableLoaderState::ProgramData (bincode serialization):
    //   [0..4]:   variant discriminator = 3
    //   [4..12]:  slot: u64
    //   [12]:     Option tag: 1 = Some
    //   [13..45]: upgrade_authority_address: Pubkey
    //   [45..]:   ELF bytes
    let mut program_data_account_data = vec![0u8; 45 + program_bytes.len()];
    program_data_account_data[0..4].copy_from_slice(&3u32.to_le_bytes());
    program_data_account_data[4..12].copy_from_slice(&0u64.to_le_bytes());
    program_data_account_data[12] = 1; // Some(upgrade_authority)
    program_data_account_data[13..45].copy_from_slice(payer.pubkey().as_ref());
    program_data_account_data[45..].copy_from_slice(program_bytes);

    svm.set_account(
        program_data_pda,
        Account {
            executable: false,
            data: program_data_account_data,
            lamports: 100 * INITIAL_LAMPORTS,
            owner: BPF_UPGRADEABLE_LOADER_ID,
            rent_epoch: 0,
        },
    )
    .unwrap();

    // UpgradeableLoaderState::Program (bincode serialization):
    //   [0..4]:   variant discriminator = 2
    //   [4..36]:  programdata_address: Pubkey
    let mut program_account_data = vec![0u8; 36];
    program_account_data[0..4].copy_from_slice(&2u32.to_le_bytes());
    program_account_data[4..36].copy_from_slice(program_data_pda.as_ref());

    svm.set_account(
        PROGRAM_ID,
        Account {
            executable: true,
            data: program_account_data,
            lamports: INITIAL_LAMPORTS,
            owner: BPF_UPGRADEABLE_LOADER_ID,
            rent_epoch: 0,
        },
    )
    .unwrap();

    // Set clock to Jan 1, 2024
    svm.set_sysvar(&Clock {
        slot: 0,
        epoch_start_timestamp: 0,
        epoch: 0,
        leader_schedule_epoch: 0,
        unix_timestamp: 1704067200i64,
    });

    (svm, payer)
}

/// Initialize the program state. Convenience wrapper used by most tests.
pub fn setup_initialized() -> (LiteSVM, Keypair, Pubkey) {
    let (mut svm, payer) = setup();
    let boss = payer.pubkey();
    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);
    let ix = build_initialize_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).expect("initialize failed");
    (svm, payer, onyc_mint)
}

/// Initialize program state and create a fresh offer pair (token_in, token_out).
/// Returns initialized test context and mints ready for offer-vector tests.
pub fn setup_offer_with_mints() -> (LiteSVM, Keypair, Pubkey, Pubkey) {
    let (mut svm, payer, _) = setup_initialized();
    let boss = payer.pubkey();

    let token_in = create_mint(&mut svm, &payer, 9, &boss);
    let token_out = create_mint(&mut svm, &payer, 9, &boss);

    let ix = build_make_offer_ix(&boss, &token_in, &token_out, 0, false, false);
    send_tx(&mut svm, &[ix], &[&payer]).expect("make_offer failed");

    (svm, payer, token_in, token_out)
}

pub fn advance_slot(svm: &mut LiteSVM) {
    let clock: Clock = svm.get_sysvar();
    svm.warp_to_slot(clock.slot + 1);
    svm.expire_blockhash();
}

pub fn get_clock_time(svm: &LiteSVM) -> u64 {
    let clock: Clock = svm.get_sysvar();
    clock.unix_timestamp as u64
}

pub fn advance_clock_by(svm: &mut LiteSVM, seconds: u64) {
    let clock: Clock = svm.get_sysvar();
    svm.set_sysvar(&Clock {
        slot: clock.slot + 1,
        epoch_start_timestamp: clock.epoch_start_timestamp,
        epoch: clock.epoch,
        leader_schedule_epoch: clock.leader_schedule_epoch,
        unix_timestamp: clock.unix_timestamp + seconds as i64,
    });
    svm.expire_blockhash();
}

// ---------------------------------------------------------------------------
// Account creation helpers
// ---------------------------------------------------------------------------
pub fn create_mint(
    svm: &mut LiteSVM,
    _payer: &Keypair,
    decimals: u8,
    mint_authority: &Pubkey,
) -> Pubkey {
    let mint = Keypair::new();

    // SPL Token Mint layout (82 bytes)
    // COption<Pubkey> uses a 4-byte LE tag (0=None, 1=Some) + 32-byte Pubkey
    let mut mint_data = vec![0u8; 82];
    // [0..4]:   mint_authority COption tag
    // Some
    mint_data[0..4].copy_from_slice(&1u32.to_le_bytes());
    // [4..36]:  mint_authority Pubkey
    mint_data[4..36].copy_from_slice(mint_authority.as_ref());
    // [36..44]: supply = 0 (already zero)
    // [44]:     decimals
    mint_data[44] = decimals;
    // [45]:     is_initialized
    mint_data[45] = 1;
    // [46..50]: freeze_authority COption tag
    // Some
    mint_data[46..50].copy_from_slice(&1u32.to_le_bytes());
    // [50..82]: freeze_authority Pubkey
    mint_data[50..82].copy_from_slice(mint_authority.as_ref());

    svm.set_account(
        mint.pubkey(),
        Account {
            executable: false,
            data: mint_data,
            lamports: INITIAL_LAMPORTS,
            owner: TOKEN_PROGRAM_ID,
            rent_epoch: 0,
        },
    )
    .unwrap();

    mint.pubkey()
}

pub fn create_token_account(
    svm: &mut LiteSVM,
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
) -> Pubkey {
    let ata = get_associated_token_address(owner, mint);

    // SPL Token Account layout (165 bytes)
    let mut token_data = vec![0u8; 165];
    token_data[0..32].copy_from_slice(mint.as_ref()); // mint
    token_data[32..64].copy_from_slice(owner.as_ref()); // owner
    token_data[64..72].copy_from_slice(&amount.to_le_bytes()); // amount
    token_data[108] = 1; // state = Initialized

    svm.set_account(
        ata,
        Account {
            executable: false,
            data: token_data,
            lamports: INITIAL_LAMPORTS,
            owner: TOKEN_PROGRAM_ID,
            rent_epoch: 0,
        },
    )
    .unwrap();

    ata
}

// ---------------------------------------------------------------------------
// Token-2022 account creation helpers
// ---------------------------------------------------------------------------
pub fn create_mint_2022(
    svm: &mut LiteSVM,
    _payer: &Keypair,
    decimals: u8,
    mint_authority: &Pubkey,
) -> Pubkey {
    let mint = Keypair::new();

    // Token-2022 Mint without extensions: same 82-byte layout as SPL Token,
    // just owned by TOKEN_2022_PROGRAM_ID. StateWithExtensionsOwned::unpack
    // handles exactly 82 bytes as a base type without extensions.
    let mut mint_data = vec![0u8; 82];
    mint_data[0..4].copy_from_slice(&1u32.to_le_bytes()); // mint_authority COption = Some
    mint_data[4..36].copy_from_slice(mint_authority.as_ref());
    mint_data[44] = decimals;
    mint_data[45] = 1; // is_initialized
    mint_data[46..50].copy_from_slice(&1u32.to_le_bytes()); // freeze_authority = Some
    mint_data[50..82].copy_from_slice(mint_authority.as_ref());

    svm.set_account(
        mint.pubkey(),
        Account {
            executable: false,
            data: mint_data,
            lamports: INITIAL_LAMPORTS,
            owner: TOKEN_2022_PROGRAM_ID,
            rent_epoch: 0,
        },
    )
    .unwrap();

    mint.pubkey()
}

pub fn create_token_account_2022(
    svm: &mut LiteSVM,
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
) -> Pubkey {
    let ata = get_associated_token_address_2022(owner, mint);

    // Token-2022 Account without extensions: same 165-byte layout as SPL Token,
    // just owned by TOKEN_2022_PROGRAM_ID.
    let mut token_data = vec![0u8; 165];
    token_data[0..32].copy_from_slice(mint.as_ref()); // mint
    token_data[32..64].copy_from_slice(owner.as_ref()); // owner
    token_data[64..72].copy_from_slice(&amount.to_le_bytes()); // amount
    token_data[108] = 1; // state = Initialized

    svm.set_account(
        ata,
        Account {
            executable: false,
            data: token_data,
            lamports: INITIAL_LAMPORTS,
            owner: TOKEN_2022_PROGRAM_ID,
            rent_epoch: 0,
        },
    )
    .unwrap();

    ata
}

pub fn create_mint_2022_with_transfer_fee(
    svm: &mut LiteSVM,
    _payer: &Keypair,
    decimals: u8,
    mint_authority: &Pubkey,
    fee_basis_points: u16,
    max_fee: u64,
) -> Pubkey {
    let mint = Keypair::new();

    // Token-2022 Mint with TransferFeeConfig extension (278 bytes)
    // Layout: [0..82] Base Mint | [82..165] padding | [165] AccountType |
    //         [166..170] ExtHeader | [170..278] TransferFeeConfig
    let mut mint_data = vec![0u8; 278];
    // Base Mint State [0..82]
    // mint_authority COption = Some
    mint_data[0..4].copy_from_slice(&1u32.to_le_bytes());
    mint_data[4..36].copy_from_slice(mint_authority.as_ref());
    // decimals
    mint_data[44] = decimals;
    // is_initialized
    mint_data[45] = 1;
    // freeze_authority = Some
    mint_data[46..50].copy_from_slice(&1u32.to_le_bytes());
    mint_data[50..82].copy_from_slice(mint_authority.as_ref());
    // [82..165] = zero padding (already zero)
    // AccountType at BASE_ACCOUNT_LENGTH (165)
    // AccountType::Mint
    mint_data[165] = 1;
    // Extension header [166..170]
    // ExtensionType = TransferFeeConfig
    mint_data[166..168].copy_from_slice(&1u16.to_le_bytes());
    // Length = 108
    mint_data[168..170].copy_from_slice(&108u16.to_le_bytes());
    // TransferFeeConfig body [170..278]
    // transfer_fee_config_authority
    mint_data[170..202].copy_from_slice(mint_authority.as_ref());
    // withdraw_withheld_authority
    mint_data[202..234].copy_from_slice(mint_authority.as_ref());
    // withheld_amount [234..242] = 0
    // older_transfer_fee epoch [242..250] = 0
    // maximum_fee
    mint_data[250..258].copy_from_slice(&max_fee.to_le_bytes());
    // transfer_fee_basis_points
    mint_data[258..260].copy_from_slice(&fee_basis_points.to_le_bytes());
    // newer_transfer_fee epoch [260..268] = 0
    // maximum_fee
    mint_data[268..276].copy_from_slice(&max_fee.to_le_bytes());
    // transfer_fee_basis_points
    mint_data[276..278].copy_from_slice(&fee_basis_points.to_le_bytes());

    svm.set_account(
        mint.pubkey(),
        Account {
            executable: false,
            data: mint_data,
            lamports: INITIAL_LAMPORTS,
            owner: TOKEN_2022_PROGRAM_ID,
            rent_epoch: 0,
        },
    )
    .unwrap();

    mint.pubkey()
}

// ---------------------------------------------------------------------------
// Instruction builders
// ---------------------------------------------------------------------------
pub fn build_initialize_ix(boss: &Pubkey, onyc_mint: &Pubkey) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();
    let (offer_vault_authority_pda, _) = find_offer_vault_authority_pda();
    let program_data_pda = find_program_data_pda();

    let data = ix_discriminator("initialize").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new(mint_authority_pda, false),
            AccountMeta::new(offer_vault_authority_pda, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new_readonly(PROGRAM_ID, false),
            AccountMeta::new(program_data_pda, false),
            AccountMeta::new_readonly(*onyc_mint, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_add_admin_ix(boss: &Pubkey, new_admin: &Pubkey) -> Instruction {
    let (state_pda, _) = find_state_pda();

    let mut data = ix_discriminator("add_admin").to_vec();
    data.extend_from_slice(new_admin.as_ref());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new_readonly(*boss, true),
        ],
        data,
    }
}

pub fn build_remove_admin_ix(boss: &Pubkey, admin_to_remove: &Pubkey) -> Instruction {
    let (state_pda, _) = find_state_pda();

    let mut data = ix_discriminator("remove_admin").to_vec();
    data.extend_from_slice(admin_to_remove.as_ref());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new_readonly(*boss, true),
        ],
        data,
    }
}

pub fn build_clear_admins_ix(boss: &Pubkey) -> Instruction {
    let (state_pda, _) = find_state_pda();

    let data = ix_discriminator("clear_admins").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new_readonly(*boss, true),
        ],
        data,
    }
}

pub fn build_propose_boss_ix(boss: &Pubkey, new_boss: &Pubkey) -> Instruction {
    let (state_pda, _) = find_state_pda();

    let mut data = ix_discriminator("propose_boss").to_vec();
    data.extend_from_slice(new_boss.as_ref());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new_readonly(*boss, true),
        ],
        data,
    }
}

pub fn build_accept_boss_ix(new_boss: &Pubkey) -> Instruction {
    let (state_pda, _) = find_state_pda();

    let data = ix_discriminator("accept_boss").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new_readonly(*new_boss, true),
        ],
        data,
    }
}

pub fn build_set_kill_switch_ix(signer: &Pubkey, enable: bool) -> Instruction {
    let (state_pda, _) = find_state_pda();

    let mut data = ix_discriminator("set_kill_switch").to_vec();
    data.push(if enable { 1 } else { 0 });

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new_readonly(*signer, true),
        ],
        data,
    }
}

pub fn build_add_approver_ix(boss: &Pubkey, approver: &Pubkey) -> Instruction {
    let (state_pda, _) = find_state_pda();

    let mut data = ix_discriminator("add_approver").to_vec();
    data.extend_from_slice(approver.as_ref());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new_readonly(*boss, true),
        ],
        data,
    }
}

pub fn build_remove_approver_ix(boss: &Pubkey, approver: &Pubkey) -> Instruction {
    let (state_pda, _) = find_state_pda();

    let mut data = ix_discriminator("remove_approver").to_vec();
    data.extend_from_slice(approver.as_ref());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new_readonly(*boss, true),
        ],
        data,
    }
}

pub fn build_configure_max_supply_ix(boss: &Pubkey, max_supply: u64) -> Instruction {
    let (state_pda, _) = find_state_pda();

    let mut data = ix_discriminator("configure_max_supply").to_vec();
    data.extend_from_slice(&max_supply.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new_readonly(*boss, true),
        ],
        data,
    }
}

pub fn build_set_redemption_admin_ix(boss: &Pubkey, redemption_admin: &Pubkey) -> Instruction {
    let (state_pda, _) = find_state_pda();

    let mut data = ix_discriminator("set_redemption_admin").to_vec();
    data.extend_from_slice(redemption_admin.as_ref());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new_readonly(*boss, true),
        ],
        data,
    }
}

pub fn build_close_state_ix(boss: &Pubkey) -> Instruction {
    let (state_pda, _) = find_state_pda();

    let data = ix_discriminator("close_state").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_set_onyc_mint_ix(boss: &Pubkey, onyc_mint: &Pubkey) -> Instruction {
    let (state_pda, _) = find_state_pda();

    let data = ix_discriminator("set_onyc_mint").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new_readonly(*boss, true),
            AccountMeta::new_readonly(*onyc_mint, false),
        ],
        data,
    }
}

// ---------------------------------------------------------------------------
// Cache instruction builders
// ---------------------------------------------------------------------------
pub fn build_initialize_cache_ix(
    boss: &Pubkey,
    onyc_mint: &Pubkey,
    cache_admin: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (cache_state_pda, _) = find_cache_state_pda();
    let (cache_vault_authority_pda, _) = find_cache_vault_authority_pda();
    let cache_vault_onyc_ata = derive_ata(&cache_vault_authority_pda, onyc_mint, &TOKEN_PROGRAM_ID);

    let mut data = ix_discriminator("initialize_cache").to_vec();
    data.extend_from_slice(cache_admin.as_ref());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new(cache_state_pda, false),
            AccountMeta::new(cache_vault_authority_pda, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new(*onyc_mint, false),
            AccountMeta::new(cache_vault_onyc_ata, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_set_cache_admin_ix(boss: &Pubkey, new_cache_admin: &Pubkey) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (cache_state_pda, _) = find_cache_state_pda();

    let mut data = ix_discriminator("set_cache_admin").to_vec();
    data.extend_from_slice(new_cache_admin.as_ref());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(cache_state_pda, false),
            AccountMeta::new_readonly(*boss, true),
        ],
        data,
    }
}

pub fn build_set_cache_yields_ix(
    boss: &Pubkey,
    gross_yield: u64,
    current_yield: u64,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (cache_state_pda, _) = find_cache_state_pda();

    let mut data = ix_discriminator("set_cache_yields").to_vec();
    data.extend_from_slice(&gross_yield.to_le_bytes());
    data.extend_from_slice(&current_yield.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(cache_state_pda, false),
            AccountMeta::new_readonly(*boss, true),
        ],
        data,
    }
}

pub fn build_update_lowest_supply_ix(onyc_mint: &Pubkey) -> Instruction {
    let (cache_state_pda, _) = find_cache_state_pda();

    let data = ix_discriminator("update_lowest_supply").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(cache_state_pda, false),
            AccountMeta::new_readonly(*onyc_mint, false),
        ],
        data,
    }
}

pub fn build_accrue_cache_ix(cache_admin: &Pubkey, onyc_mint: &Pubkey) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (cache_state_pda, _) = find_cache_state_pda();
    let (cache_vault_authority_pda, _) = find_cache_vault_authority_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();
    let cache_vault_onyc_ata = derive_ata(&cache_vault_authority_pda, onyc_mint, &TOKEN_PROGRAM_ID);

    let data = ix_discriminator("accrue_cache").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(cache_state_pda, false),
            AccountMeta::new_readonly(*cache_admin, true),
            AccountMeta::new(*onyc_mint, false),
            AccountMeta::new_readonly(cache_vault_authority_pda, false),
            AccountMeta::new(cache_vault_onyc_ata, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_burn_for_nav_increase_ix(
    boss: &Pubkey,
    token_in_mint: &Pubkey,
    onyc_mint: &Pubkey,
    asset_adjustment_amount: u64,
    target_nav: u64,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (offer_pda, _) = find_offer_pda(token_in_mint, onyc_mint);
    let (cache_state_pda, _) = find_cache_state_pda();
    let (offer_vault_authority_pda, _) = find_offer_vault_authority_pda();
    let (cache_vault_authority_pda, _) = find_cache_vault_authority_pda();
    let vault_token_out_ata = derive_ata(&offer_vault_authority_pda, onyc_mint, &TOKEN_PROGRAM_ID);
    let cache_vault_onyc_ata = derive_ata(&cache_vault_authority_pda, onyc_mint, &TOKEN_PROGRAM_ID);

    let mut data = ix_discriminator("burn_for_nav_increase").to_vec();
    data.extend_from_slice(&asset_adjustment_amount.to_le_bytes());
    data.extend_from_slice(&target_nav.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(cache_state_pda, false),
            AccountMeta::new_readonly(*boss, true),
            AccountMeta::new_readonly(offer_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new(*onyc_mint, false),
            AccountMeta::new_readonly(offer_vault_authority_pda, false),
            AccountMeta::new_readonly(cache_vault_authority_pda, false),
            AccountMeta::new_readonly(vault_token_out_ata, false),
            AccountMeta::new(cache_vault_onyc_ata, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ],
        data,
    }
}

// ---------------------------------------------------------------------------
// Offer instruction builders
// ---------------------------------------------------------------------------
pub fn build_make_offer_ix(
    boss: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    fee_basis_points: u16,
    needs_approval: bool,
    allow_permissionless: bool,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);
    let (vault_authority_pda, _) = find_offer_vault_authority_pda();
    let vault_token_in_ata = get_associated_token_address(&vault_authority_pda, token_in_mint);

    let mut data = ix_discriminator("make_offer").to_vec();
    data.extend_from_slice(&fee_basis_points.to_le_bytes());
    data.push(if needs_approval { 1 } else { 0 });
    data.push(if allow_permissionless { 1 } else { 0 });

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false), // token_in_program
            AccountMeta::new(vault_token_in_ata, false),
            AccountMeta::new_readonly(*token_out_mint, false),
            AccountMeta::new(offer_pda, false),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_add_offer_vector_ix(
    boss: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    start_time: Option<u64>,
    base_time: u64,
    base_price: u64,
    apr: u64,
    price_fix_duration: u64,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);

    let mut data = ix_discriminator("add_offer_vector").to_vec();
    // Borsh Option<u64>: 0 = None, 1 + 8 bytes = Some(value)
    match start_time {
        Some(t) => {
            data.push(1);
            data.extend_from_slice(&t.to_le_bytes());
        }
        None => {
            data.push(0);
        }
    }
    data.extend_from_slice(&base_time.to_le_bytes());
    data.extend_from_slice(&base_price.to_le_bytes());
    data.extend_from_slice(&apr.to_le_bytes());
    data.extend_from_slice(&price_fix_duration.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(offer_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new_readonly(*token_out_mint, false),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*boss, true),
        ],
        data,
    }
}

pub fn build_take_offer_permissionless_ix(
    user: &Pubkey,
    boss: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    token_in_amount: u64,
    approval_message: Option<&[u8]>, // pre-serialized ApprovalMessage bytes
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);
    let (vault_authority_pda, _) = find_offer_vault_authority_pda();
    let (permissionless_authority_pda, _) = find_permissionless_authority_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();

    let vault_token_in_ata = get_associated_token_address(&vault_authority_pda, token_in_mint);
    let vault_token_out_ata = get_associated_token_address(&vault_authority_pda, token_out_mint);
    let permissionless_token_in_ata =
        get_associated_token_address(&permissionless_authority_pda, token_in_mint);
    let permissionless_token_out_ata =
        get_associated_token_address(&permissionless_authority_pda, token_out_mint);
    let user_token_in_ata = get_associated_token_address(user, token_in_mint);
    let user_token_out_ata = get_associated_token_address(user, token_out_mint);
    let boss_token_in_ata = get_associated_token_address(boss, token_in_mint);

    let mut data = ix_discriminator("take_offer_permissionless").to_vec();
    data.extend_from_slice(&token_in_amount.to_le_bytes());
    // Borsh Option<ApprovalMessage>
    match approval_message {
        Some(msg_bytes) => {
            data.push(1); // Some
            data.extend_from_slice(msg_bytes);
        }
        None => {
            data.push(0); // None
        }
    }

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(offer_pda, false),                    // offer
            AccountMeta::new_readonly(state_pda, false),           // state
            AccountMeta::new_readonly(*boss, false),               // boss
            AccountMeta::new_readonly(vault_authority_pda, false), // vault_authority
            AccountMeta::new(vault_token_in_ata, false),           // vault_token_in_account
            AccountMeta::new(vault_token_out_ata, false),          // vault_token_out_account
            AccountMeta::new_readonly(permissionless_authority_pda, false), // permissionless_authority
            AccountMeta::new(permissionless_token_in_ata, false), // permissionless_token_in_account
            AccountMeta::new(permissionless_token_out_ata, false), // permissionless_token_out_account
            AccountMeta::new(*token_in_mint, false),               // token_in_mint
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),    // token_in_program
            AccountMeta::new(*token_out_mint, false),              // token_out_mint
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),    // token_out_program
            AccountMeta::new(user_token_in_ata, false),            // user_token_in_account
            AccountMeta::new(user_token_out_ata, false),           // user_token_out_account
            AccountMeta::new(boss_token_in_ata, false),            // boss_token_in_account
            AccountMeta::new_readonly(mint_authority_pda, false),  // mint_authority
            AccountMeta::new_readonly(SYSVAR_INSTRUCTIONS_ID, false), // instructions_sysvar
            AccountMeta::new(*user, true),                         // user (signer)
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),      // associated_token_program
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),   // system_program
        ],
        data,
    }
}

// ---------------------------------------------------------------------------
// Ed25519 helpers
// ---------------------------------------------------------------------------

/// Serialize an ApprovalMessage (Borsh-compatible: program_id || user_pubkey || expiry_unix)
pub fn serialize_approval_message(
    program_id: &Pubkey,
    user_pubkey: &Pubkey,
    expiry_unix: u64,
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(72);
    buf.extend_from_slice(program_id.as_ref());
    buf.extend_from_slice(user_pubkey.as_ref());
    buf.extend_from_slice(&expiry_unix.to_le_bytes());
    buf
}

/// Build an Ed25519 verify instruction for the Solana Ed25519 native program.
/// The approver signs the message, and the instruction verifies the signature on-chain.
pub fn build_ed25519_verify_ix(approver: &Keypair, message: &[u8]) -> Instruction {
    let signature = approver.sign_message(message);
    let sig_bytes = <[u8; 64]>::from(signature);
    let pubkey_bytes = approver.pubkey().to_bytes();

    // Ed25519 instruction data layout:
    // [0]:     num_signatures (u8) = 1
    // [1]:     padding (u8) = 0
    // [2..16]: Ed25519SignatureOffsets (14 bytes, 7 x u16 LE)
    // [16..48]: pubkey (32 bytes)
    // [48..112]: signature (64 bytes)
    // [112..]: message
    let public_key_offset: u16 = 16;
    let signature_offset: u16 = 48;
    let message_data_offset: u16 = 112;
    let message_data_size: u16 = message.len() as u16;

    let mut data = Vec::with_capacity(112 + message.len());
    // num_signatures + padding
    data.push(1u8);
    data.push(0u8);
    // Ed25519SignatureOffsets
    data.extend_from_slice(&signature_offset.to_le_bytes());
    // signature_instruction_index
    data.extend_from_slice(&u16::MAX.to_le_bytes());
    data.extend_from_slice(&public_key_offset.to_le_bytes());
    // public_key_instruction_index
    data.extend_from_slice(&u16::MAX.to_le_bytes());
    data.extend_from_slice(&message_data_offset.to_le_bytes());
    data.extend_from_slice(&message_data_size.to_le_bytes());
    // message_instruction_index
    data.extend_from_slice(&u16::MAX.to_le_bytes());
    // Pubkey
    data.extend_from_slice(&pubkey_bytes);
    // Signature
    data.extend_from_slice(&sig_bytes);
    // Message
    data.extend_from_slice(message);

    Instruction {
        program_id: ED25519_PROGRAM_ID,
        accounts: vec![],
        data,
    }
}

// ---------------------------------------------------------------------------
// More offer instruction builders
// ---------------------------------------------------------------------------
pub fn build_update_offer_fee_ix(
    boss: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    new_fee_basis_points: u16,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);

    let mut data = ix_discriminator("update_offer_fee").to_vec();
    data.extend_from_slice(&new_fee_basis_points.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(offer_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new_readonly(*token_out_mint, false),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*boss, true),
        ],
        data,
    }
}

pub fn build_delete_offer_vector_ix(
    boss: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    vector_start_time: u64,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);

    let mut data = ix_discriminator("delete_offer_vector").to_vec();
    data.extend_from_slice(&vector_start_time.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(offer_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new_readonly(*token_out_mint, false),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*boss, true),
        ],
        data,
    }
}

pub fn build_delete_all_offer_vectors_ix(
    boss: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);

    let data = ix_discriminator("delete_all_offer_vectors").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(offer_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new_readonly(*token_out_mint, false),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*boss, true),
        ],
        data,
    }
}

pub fn build_take_offer_ix(
    user: &Pubkey,
    boss: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    token_in_amount: u64,
    approval_message: Option<&[u8]>,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);
    let (vault_authority_pda, _) = find_offer_vault_authority_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();

    let vault_token_in_ata = get_associated_token_address(&vault_authority_pda, token_in_mint);
    let vault_token_out_ata = get_associated_token_address(&vault_authority_pda, token_out_mint);
    let user_token_in_ata = get_associated_token_address(user, token_in_mint);
    let user_token_out_ata = get_associated_token_address(user, token_out_mint);
    let boss_token_in_ata = get_associated_token_address(boss, token_in_mint);

    let mut data = ix_discriminator("take_offer").to_vec();
    data.extend_from_slice(&token_in_amount.to_le_bytes());
    match approval_message {
        Some(msg_bytes) => {
            data.push(1);
            data.extend_from_slice(msg_bytes);
        }
        None => {
            data.push(0);
        }
    }

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(offer_pda, false),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*boss, false),
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new(vault_token_in_ata, false),
            AccountMeta::new(vault_token_out_ata, false),
            AccountMeta::new(*token_in_mint, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false), // token_in_program
            AccountMeta::new(*token_out_mint, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false), // token_out_program
            AccountMeta::new(user_token_in_ata, false),
            AccountMeta::new(user_token_out_ata, false),
            AccountMeta::new(boss_token_in_ata, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new_readonly(SYSVAR_INSTRUCTIONS_ID, false),
            AccountMeta::new(*user, true),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_offer_vault_deposit_ix(
    boss: &Pubkey,
    token_mint: &Pubkey,
    amount: u64,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (vault_authority_pda, _) = find_offer_vault_authority_pda();
    let boss_token_ata = get_associated_token_address(boss, token_mint);
    let vault_token_ata = get_associated_token_address(&vault_authority_pda, token_mint);

    let mut data = ix_discriminator("offer_vault_deposit").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new_readonly(*token_mint, false),
            AccountMeta::new(boss_token_ata, false),
            AccountMeta::new(vault_token_ata, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

// ---------------------------------------------------------------------------
// Offer deserialization
// ---------------------------------------------------------------------------
pub const MAX_VECTORS: usize = 10;

#[derive(Debug, Clone, Copy)]
pub struct OfferVectorData {
    pub start_time: u64,
    pub base_time: u64,
    pub base_price: u64,
    pub apr: u64,
    pub price_fix_duration: u64,
}

impl OfferVectorData {
    pub fn is_active(&self) -> bool {
        self.start_time != 0
    }
}

#[derive(Debug)]
pub struct OfferData {
    pub token_in_mint: Pubkey,
    pub token_out_mint: Pubkey,
    pub vectors: [OfferVectorData; MAX_VECTORS],
    pub fee_basis_points: u16,
    pub bump: u8,
    pub needs_approval: u8,
    pub allow_permissionless: u8,
}

impl OfferData {
    pub fn active_vectors(&self) -> Vec<&OfferVectorData> {
        self.vectors.iter().filter(|v| v.is_active()).collect()
    }
}

pub fn read_offer(svm: &LiteSVM, token_in_mint: &Pubkey, token_out_mint: &Pubkey) -> OfferData {
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);
    let account = svm
        .get_account(&offer_pda)
        .expect("offer account not found");
    let data = &account.data;

    let mut offset = 8; // skip Anchor discriminator

    let tin = Pubkey::try_from(&data[offset..offset + 32]).unwrap();
    offset += 32;

    let tout = Pubkey::try_from(&data[offset..offset + 32]).unwrap();
    offset += 32;

    // vectors: [OfferVector; 10], each is 5 x u64 = 40 bytes (repr(C), 8-byte aligned)
    let mut vectors = [OfferVectorData {
        start_time: 0,
        base_time: 0,
        base_price: 0,
        apr: 0,
        price_fix_duration: 0,
    }; MAX_VECTORS];
    for i in 0..MAX_VECTORS {
        let st = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let bt = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let bp = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let ap = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let pfd = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;
        vectors[i] = OfferVectorData {
            start_time: st,
            base_time: bt,
            base_price: bp,
            apr: ap,
            price_fix_duration: pfd,
        };
    }

    let fee_basis_points = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap());
    offset += 2;

    let bump = data[offset];
    offset += 1;

    let needs_approval = data[offset];
    offset += 1;

    let allow_permissionless = data[offset];

    OfferData {
        token_in_mint: tin,
        token_out_mint: tout,
        vectors,
        fee_basis_points,
        bump,
        needs_approval,
        allow_permissionless,
    }
}

pub fn get_mint_supply(svm: &LiteSVM, mint: &Pubkey) -> u64 {
    let account = svm.get_account(mint).expect("mint account not found");
    // SPL Token Mint layout: supply is at offset 36..44
    u64::from_le_bytes(account.data[36..44].try_into().unwrap())
}

// ---------------------------------------------------------------------------
// State deserialization
// ---------------------------------------------------------------------------
pub struct StateData {
    pub boss: Pubkey,
    pub proposed_boss: Pubkey,
    pub is_killed: bool,
    pub onyc_mint: Pubkey,
    pub admins: [Pubkey; MAX_ADMINS],
    pub approver1: Pubkey,
    pub approver2: Pubkey,
    pub bump: u8,
    pub max_supply: u64,
    pub redemption_admin: Pubkey,
}

impl StateData {
    pub fn active_admins(&self) -> Vec<Pubkey> {
        let default_pubkey = Pubkey::default();
        self.admins
            .iter()
            .filter(|a| **a != default_pubkey)
            .copied()
            .collect()
    }
}

pub fn read_state(svm: &LiteSVM) -> StateData {
    let (state_pda, _) = find_state_pda();
    let account = svm
        .get_account(&state_pda)
        .expect("state account not found");
    let data = &account.data;

    let mut offset = 8; // skip Anchor discriminator

    let boss = Pubkey::try_from(&data[offset..offset + 32]).unwrap();
    offset += 32;

    let proposed_boss = Pubkey::try_from(&data[offset..offset + 32]).unwrap();
    offset += 32;

    let is_killed = data[offset] != 0;
    offset += 1;

    let onyc_mint = Pubkey::try_from(&data[offset..offset + 32]).unwrap();
    offset += 32;

    let mut admins = [Pubkey::default(); MAX_ADMINS];
    for i in 0..MAX_ADMINS {
        admins[i] = Pubkey::try_from(&data[offset..offset + 32]).unwrap();
        offset += 32;
    }

    let approver1 = Pubkey::try_from(&data[offset..offset + 32]).unwrap();
    offset += 32;

    let approver2 = Pubkey::try_from(&data[offset..offset + 32]).unwrap();
    offset += 32;

    let bump = data[offset];
    offset += 1;

    let max_supply = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    offset += 8;

    let redemption_admin = Pubkey::try_from(&data[offset..offset + 32]).unwrap();

    StateData {
        boss,
        proposed_boss,
        is_killed,
        onyc_mint,
        admins,
        approver1,
        approver2,
        bump,
        max_supply,
        redemption_admin,
    }
}

pub struct CacheStateData {
    pub onyc_mint: Pubkey,
    pub cache_admin: Pubkey,
    pub gross_yield: u64,
    pub current_yield: u64,
    pub lowest_supply: u64,
    pub last_accrual_timestamp: i64,
    pub bump: u8,
}

pub fn read_cache_state(svm: &LiteSVM) -> CacheStateData {
    let (cache_state_pda, _) = find_cache_state_pda();
    let account = svm
        .get_account(&cache_state_pda)
        .expect("cache state account not found");
    let mut data_slice = account.data.as_slice();
    let cache_state = onreapp::instructions::CacheState::try_deserialize(&mut data_slice)
        .expect("failed to deserialize CacheState account");

    CacheStateData {
        onyc_mint: cache_state.onyc_mint,
        cache_admin: cache_state.cache_admin,
        gross_yield: cache_state.gross_yield,
        current_yield: cache_state.current_yield,
        lowest_supply: cache_state.lowest_supply,
        last_accrual_timestamp: cache_state.last_accrual_timestamp,
        bump: cache_state.bump,
    }
}

// ---------------------------------------------------------------------------
// Additional PDA seeds
// ---------------------------------------------------------------------------
pub const REDEMPTION_OFFER_SEED: &[u8] = b"redemption_offer";
pub const REDEMPTION_REQUEST_SEED: &[u8] = b"redemption_request";

pub fn find_redemption_offer_pda(token_in_mint: &Pubkey, token_out_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            REDEMPTION_OFFER_SEED,
            token_in_mint.as_ref(),
            token_out_mint.as_ref(),
        ],
        &PROGRAM_ID,
    )
}

pub fn find_redemption_request_pda(redemption_offer: &Pubkey, counter: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            REDEMPTION_REQUEST_SEED,
            redemption_offer.as_ref(),
            &counter.to_le_bytes(),
        ],
        &PROGRAM_ID,
    )
}

// ---------------------------------------------------------------------------
// Vault instruction builders
// ---------------------------------------------------------------------------
pub fn build_offer_vault_withdraw_ix(
    boss: &Pubkey,
    token_mint: &Pubkey,
    amount: u64,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (vault_authority_pda, _) = find_offer_vault_authority_pda();
    let boss_token_ata = get_associated_token_address(boss, token_mint);
    let vault_token_ata = get_associated_token_address(&vault_authority_pda, token_mint);

    let mut data = ix_discriminator("offer_vault_withdraw").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new_readonly(*token_mint, false),
            AccountMeta::new(boss_token_ata, false),
            AccountMeta::new(vault_token_ata, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_redemption_vault_deposit_ix(
    boss: &Pubkey,
    token_mint: &Pubkey,
    amount: u64,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (vault_authority_pda, _) = find_redemption_vault_authority_pda();
    let boss_token_ata = get_associated_token_address(boss, token_mint);
    let vault_token_ata = get_associated_token_address(&vault_authority_pda, token_mint);

    let mut data = ix_discriminator("redemption_vault_deposit").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new_readonly(*token_mint, false),
            AccountMeta::new(boss_token_ata, false),
            AccountMeta::new(vault_token_ata, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_redemption_vault_withdraw_ix(
    boss: &Pubkey,
    token_mint: &Pubkey,
    amount: u64,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (vault_authority_pda, _) = find_redemption_vault_authority_pda();
    let boss_token_ata = get_associated_token_address(boss, token_mint);
    let vault_token_ata = get_associated_token_address(&vault_authority_pda, token_mint);

    let mut data = ix_discriminator("redemption_vault_withdraw").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new_readonly(*token_mint, false),
            AccountMeta::new(boss_token_ata, false),
            AccountMeta::new(vault_token_ata, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

// ---------------------------------------------------------------------------
// Mint authority instruction builders
// ---------------------------------------------------------------------------
pub fn build_transfer_mint_authority_to_program_ix(boss: &Pubkey, mint: &Pubkey) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();

    let data = ix_discriminator("transfer_mint_authority_to_program").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*boss, true),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(*mint, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_transfer_mint_authority_to_boss_ix(boss: &Pubkey, mint: &Pubkey) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();

    let data = ix_discriminator("transfer_mint_authority_to_boss").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*boss, true),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(*mint, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_mint_to_ix(boss: &Pubkey, onyc_mint: &Pubkey, amount: u64) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();
    let boss_onyc_ata = get_associated_token_address(boss, onyc_mint);

    let mut data = ix_discriminator("mint_to").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new(*onyc_mint, false),
            AccountMeta::new(boss_onyc_ata, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

// ---------------------------------------------------------------------------
// Market info instruction builders
// ---------------------------------------------------------------------------
pub fn build_get_nav_ix(token_in_mint: &Pubkey, token_out_mint: &Pubkey) -> Instruction {
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);

    let data = ix_discriminator("get_nav").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(offer_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new_readonly(*token_out_mint, false),
        ],
        data,
    }
}

pub fn build_get_apy_ix(token_in_mint: &Pubkey, token_out_mint: &Pubkey) -> Instruction {
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);

    let data = ix_discriminator("get_apy").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(offer_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new_readonly(*token_out_mint, false),
        ],
        data,
    }
}

pub fn build_get_nav_adjustment_ix(token_in_mint: &Pubkey, token_out_mint: &Pubkey) -> Instruction {
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);

    let data = ix_discriminator("get_nav_adjustment").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(offer_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new_readonly(*token_out_mint, false),
        ],
        data,
    }
}

pub fn build_get_tvl_ix(token_in_mint: &Pubkey, token_out_mint: &Pubkey) -> Instruction {
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);
    let (vault_authority_pda, _) = find_offer_vault_authority_pda();
    let vault_token_out_ata = get_associated_token_address(&vault_authority_pda, token_out_mint);

    let data = ix_discriminator("get_tvl").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(offer_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new_readonly(*token_out_mint, false),
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new_readonly(vault_token_out_ata, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_get_circulating_supply_ix(onyc_mint: &Pubkey) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (vault_authority_pda, _) = find_offer_vault_authority_pda();
    let onyc_vault_ata = get_associated_token_address(&vault_authority_pda, onyc_mint);

    let data = ix_discriminator("get_circulating_supply").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*onyc_mint, false),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new_readonly(onyc_vault_ata, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_get_tvl_ix_with_token_program(
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    token_out_program: &Pubkey,
) -> Instruction {
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);
    let (vault_authority_pda, _) = find_offer_vault_authority_pda();
    let vault_token_out_ata = derive_ata(&vault_authority_pda, token_out_mint, token_out_program);

    let data = ix_discriminator("get_tvl").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(offer_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new_readonly(*token_out_mint, false),
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new_readonly(vault_token_out_ata, false),
            AccountMeta::new_readonly(*token_out_program, false),
        ],
        data,
    }
}

pub fn build_get_circulating_supply_ix_with_token_program(
    onyc_mint: &Pubkey,
    token_program: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (vault_authority_pda, _) = find_offer_vault_authority_pda();
    let onyc_vault_ata = derive_ata(&vault_authority_pda, onyc_mint, token_program);

    let data = ix_discriminator("get_circulating_supply").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*onyc_mint, false),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new_readonly(onyc_vault_ata, false),
            AccountMeta::new_readonly(*token_program, false),
        ],
        data,
    }
}

// ---------------------------------------------------------------------------
// Redemption instruction builders
// ---------------------------------------------------------------------------
pub fn build_make_redemption_offer_ix(
    signer: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    fee_basis_points: u16,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    // The original offer has reversed mints: offer(token_out, token_in)
    let (offer_pda, _) = find_offer_pda(token_out_mint, token_in_mint);
    let (redemption_vault_authority_pda, _) = find_redemption_vault_authority_pda();
    let (redemption_offer_pda, _) = find_redemption_offer_pda(token_in_mint, token_out_mint);
    let vault_token_in_ata =
        get_associated_token_address(&redemption_vault_authority_pda, token_in_mint);
    let vault_token_out_ata =
        get_associated_token_address(&redemption_vault_authority_pda, token_out_mint);

    let mut data = ix_discriminator("make_redemption_offer").to_vec();
    data.extend_from_slice(&fee_basis_points.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(offer_pda, false),
            AccountMeta::new_readonly(redemption_vault_authority_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false), // token_in_program
            AccountMeta::new(vault_token_in_ata, false),
            AccountMeta::new_readonly(*token_out_mint, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false), // token_out_program
            AccountMeta::new(vault_token_out_ata, false),
            AccountMeta::new(redemption_offer_pda, false),
            AccountMeta::new(*signer, true),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_create_redemption_request_ix(
    redeemer: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    amount: u64,
    counter: u64,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (redemption_offer_pda, _) = find_redemption_offer_pda(token_in_mint, token_out_mint);
    let (redemption_request_pda, _) = find_redemption_request_pda(&redemption_offer_pda, counter);
    let (redemption_vault_authority_pda, _) = find_redemption_vault_authority_pda();
    let redeemer_token_ata = get_associated_token_address(redeemer, token_in_mint);
    let vault_token_ata =
        get_associated_token_address(&redemption_vault_authority_pda, token_in_mint);

    let mut data = ix_discriminator("create_redemption_request").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(redemption_offer_pda, false),
            AccountMeta::new(redemption_request_pda, false),
            AccountMeta::new(*redeemer, true),
            AccountMeta::new_readonly(redemption_vault_authority_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new(redeemer_token_ata, false),
            AccountMeta::new(vault_token_ata, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_cancel_redemption_request_ix(
    signer: &Pubkey,
    redeemer: &Pubkey,
    redemption_admin: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    request_id: u64,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (redemption_offer_pda, _) = find_redemption_offer_pda(token_in_mint, token_out_mint);
    let (redemption_request_pda, _) =
        find_redemption_request_pda(&redemption_offer_pda, request_id);
    let (redemption_vault_authority_pda, _) = find_redemption_vault_authority_pda();
    let redeemer_token_ata = get_associated_token_address(redeemer, token_in_mint);
    let vault_token_ata =
        get_associated_token_address(&redemption_vault_authority_pda, token_in_mint);

    let data = ix_discriminator("cancel_redemption_request").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(redemption_offer_pda, false),
            AccountMeta::new(redemption_request_pda, false),
            AccountMeta::new(*signer, true),
            AccountMeta::new_readonly(*redeemer, false),
            AccountMeta::new(*redemption_admin, false),
            AccountMeta::new_readonly(redemption_vault_authority_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new(vault_token_ata, false),
            AccountMeta::new(redeemer_token_ata, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_fulfill_redemption_request_ix(
    redemption_admin: &Pubkey,
    boss: &Pubkey,
    redeemer: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    request_id: u64,
    amount: u64,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    // Original offer has reversed mints
    let (offer_pda, _) = find_offer_pda(token_out_mint, token_in_mint);
    let (redemption_offer_pda, _) = find_redemption_offer_pda(token_in_mint, token_out_mint);
    let (redemption_request_pda, _) =
        find_redemption_request_pda(&redemption_offer_pda, request_id);
    let (redemption_vault_authority_pda, _) = find_redemption_vault_authority_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();
    let vault_token_in_ata =
        get_associated_token_address(&redemption_vault_authority_pda, token_in_mint);
    let vault_token_out_ata =
        get_associated_token_address(&redemption_vault_authority_pda, token_out_mint);
    let user_token_out_ata = get_associated_token_address(redeemer, token_out_mint);
    let boss_token_in_ata = get_associated_token_address(boss, token_in_mint);

    let mut data = ix_discriminator("fulfill_redemption_request").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*boss, false),
            AccountMeta::new_readonly(offer_pda, false),
            AccountMeta::new(redemption_offer_pda, false),
            AccountMeta::new(redemption_request_pda, false),
            AccountMeta::new_readonly(redemption_vault_authority_pda, false),
            AccountMeta::new(vault_token_in_ata, false),
            AccountMeta::new(vault_token_out_ata, false),
            AccountMeta::new(*token_in_mint, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false), // token_in_program
            AccountMeta::new(*token_out_mint, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false), // token_out_program
            AccountMeta::new(user_token_out_ata, false),
            AccountMeta::new(boss_token_in_ata, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new_readonly(*redeemer, false),
            AccountMeta::new(*redemption_admin, true),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_update_redemption_offer_fee_ix(
    boss: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    new_fee_basis_points: u16,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (redemption_offer_pda, _) = find_redemption_offer_pda(token_in_mint, token_out_mint);

    let mut data = ix_discriminator("update_redemption_offer_fee").to_vec();
    data.extend_from_slice(&new_fee_basis_points.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(redemption_offer_pda, false),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*boss, true),
        ],
        data,
    }
}

// ---------------------------------------------------------------------------
// Mint account helpers
// ---------------------------------------------------------------------------
pub fn get_mint_authority_pubkey(svm: &LiteSVM, mint: &Pubkey) -> Option<Pubkey> {
    let account = svm.get_account(mint)?;
    // COption<Pubkey>: [0..4] tag, [4..36] pubkey
    let tag = u32::from_le_bytes(account.data[0..4].try_into().unwrap());
    if tag == 1 {
        Some(Pubkey::try_from(&account.data[4..36]).unwrap())
    } else {
        None
    }
}

pub fn set_mint_authority(svm: &mut LiteSVM, mint: &Pubkey, new_authority: &Pubkey) {
    let mut account = svm.get_account(mint).expect("mint not found");
    account.data[0..4].copy_from_slice(&1u32.to_le_bytes());
    account.data[4..36].copy_from_slice(new_authority.as_ref());
    svm.set_account(*mint, account).unwrap();
}

pub fn get_return_u64(metadata: &litesvm::types::TransactionMetadata) -> u64 {
    u64::from_le_bytes(metadata.return_data.data[..8].try_into().unwrap())
}

pub fn get_return_i64(metadata: &litesvm::types::TransactionMetadata) -> i64 {
    i64::from_le_bytes(metadata.return_data.data[..8].try_into().unwrap())
}

// ---------------------------------------------------------------------------
// Redemption offer deserialization
// ---------------------------------------------------------------------------
pub struct RedemptionOfferData {
    pub offer: Pubkey,
    pub token_in_mint: Pubkey,
    pub token_out_mint: Pubkey,
    pub executed_redemptions: u128,
    pub requested_redemptions: u128,
    pub fee_basis_points: u16,
    pub request_counter: u64,
    pub bump: u8,
}

pub fn read_redemption_offer(
    svm: &LiteSVM,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
) -> RedemptionOfferData {
    let (pda, _) = find_redemption_offer_pda(token_in_mint, token_out_mint);
    let account = svm.get_account(&pda).expect("redemption offer not found");
    let data = &account.data;

    let mut offset = 8; // skip discriminator

    let offer = Pubkey::try_from(&data[offset..offset + 32]).unwrap();
    offset += 32;
    let tin = Pubkey::try_from(&data[offset..offset + 32]).unwrap();
    offset += 32;
    let tout = Pubkey::try_from(&data[offset..offset + 32]).unwrap();
    offset += 32;
    let executed_redemptions = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
    offset += 16;
    let requested_redemptions = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
    offset += 16;
    let fee_basis_points = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap());
    offset += 2;
    let request_counter = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    offset += 8;
    let bump = data[offset];

    RedemptionOfferData {
        offer,
        token_in_mint: tin,
        token_out_mint: tout,
        executed_redemptions,
        requested_redemptions,
        fee_basis_points,
        request_counter,
        bump,
    }
}

/// Decoded fields from a `RedemptionRequest` on-chain account
pub struct RedemptionRequestData {
    pub offer: Pubkey,
    pub request_id: u64,
    pub redeemer: Pubkey,
    pub amount: u64,
    pub fulfilled_amount: u64,
    pub bump: u8,
}

/// Read and decode a `RedemptionRequest` account from the SVM
pub fn read_redemption_request(
    svm: &LiteSVM,
    redemption_offer: &Pubkey,
    request_id: u64,
) -> RedemptionRequestData {
    let (pda, _) = find_redemption_request_pda(redemption_offer, request_id);
    let account = svm
        .get_account(&pda)
        .expect("redemption request account not found");
    let mut data: &[u8] = &account.data;
    let request = RedemptionRequest::try_deserialize(&mut data)
        .expect("Failed to deserialize RedemptionRequest");

    RedemptionRequestData {
        offer: request.offer,
        request_id: request.request_id,
        redeemer: request.redeemer,
        amount: request.amount,
        fulfilled_amount: request.fulfilled_amount,
        bump: request.bump,
    }
}

// ---------------------------------------------------------------------------
// Token-2022 instruction builder variants
// ---------------------------------------------------------------------------

pub fn build_make_offer_ix_with_programs(
    boss: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    fee_basis_points: u16,
    needs_approval: bool,
    allow_permissionless: bool,
    token_in_program: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);
    let (vault_authority_pda, _) = find_offer_vault_authority_pda();
    let vault_token_in_ata = derive_ata(&vault_authority_pda, token_in_mint, token_in_program);

    let mut data = ix_discriminator("make_offer").to_vec();
    data.extend_from_slice(&fee_basis_points.to_le_bytes());
    data.push(if needs_approval { 1 } else { 0 });
    data.push(if allow_permissionless { 1 } else { 0 });

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new_readonly(*token_in_program, false),
            AccountMeta::new(vault_token_in_ata, false),
            AccountMeta::new_readonly(*token_out_mint, false),
            AccountMeta::new(offer_pda, false),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_take_offer_ix_with_programs(
    user: &Pubkey,
    boss: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    token_in_amount: u64,
    approval_message: Option<&[u8]>,
    token_in_program: &Pubkey,
    token_out_program: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);
    let (vault_authority_pda, _) = find_offer_vault_authority_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();

    let vault_token_in_ata = derive_ata(&vault_authority_pda, token_in_mint, token_in_program);
    let vault_token_out_ata = derive_ata(&vault_authority_pda, token_out_mint, token_out_program);
    let user_token_in_ata = derive_ata(user, token_in_mint, token_in_program);
    let user_token_out_ata = derive_ata(user, token_out_mint, token_out_program);
    let boss_token_in_ata = derive_ata(boss, token_in_mint, token_in_program);

    let mut data = ix_discriminator("take_offer").to_vec();
    data.extend_from_slice(&token_in_amount.to_le_bytes());
    match approval_message {
        Some(msg_bytes) => {
            data.push(1);
            data.extend_from_slice(msg_bytes);
        }
        None => {
            data.push(0);
        }
    }

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(offer_pda, false),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*boss, false),
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new(vault_token_in_ata, false),
            AccountMeta::new(vault_token_out_ata, false),
            AccountMeta::new(*token_in_mint, false),
            AccountMeta::new_readonly(*token_in_program, false),
            AccountMeta::new(*token_out_mint, false),
            AccountMeta::new_readonly(*token_out_program, false),
            AccountMeta::new(user_token_in_ata, false),
            AccountMeta::new(user_token_out_ata, false),
            AccountMeta::new(boss_token_in_ata, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new_readonly(SYSVAR_INSTRUCTIONS_ID, false),
            AccountMeta::new(*user, true),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_take_offer_permissionless_ix_with_programs(
    user: &Pubkey,
    boss: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    token_in_amount: u64,
    approval_message: Option<&[u8]>,
    token_in_program: &Pubkey,
    token_out_program: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);
    let (vault_authority_pda, _) = find_offer_vault_authority_pda();
    let (permissionless_authority_pda, _) = find_permissionless_authority_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();

    let vault_token_in_ata = derive_ata(&vault_authority_pda, token_in_mint, token_in_program);
    let vault_token_out_ata = derive_ata(&vault_authority_pda, token_out_mint, token_out_program);
    let permissionless_token_in_ata = derive_ata(
        &permissionless_authority_pda,
        token_in_mint,
        token_in_program,
    );
    let permissionless_token_out_ata = derive_ata(
        &permissionless_authority_pda,
        token_out_mint,
        token_out_program,
    );
    let user_token_in_ata = derive_ata(user, token_in_mint, token_in_program);
    let user_token_out_ata = derive_ata(user, token_out_mint, token_out_program);
    let boss_token_in_ata = derive_ata(boss, token_in_mint, token_in_program);

    let mut data = ix_discriminator("take_offer_permissionless").to_vec();
    data.extend_from_slice(&token_in_amount.to_le_bytes());
    match approval_message {
        Some(msg_bytes) => {
            data.push(1);
            data.extend_from_slice(msg_bytes);
        }
        None => {
            data.push(0);
        }
    }

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(offer_pda, false),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*boss, false),
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new(vault_token_in_ata, false),
            AccountMeta::new(vault_token_out_ata, false),
            AccountMeta::new_readonly(permissionless_authority_pda, false),
            AccountMeta::new(permissionless_token_in_ata, false),
            AccountMeta::new(permissionless_token_out_ata, false),
            AccountMeta::new(*token_in_mint, false),
            AccountMeta::new_readonly(*token_in_program, false),
            AccountMeta::new(*token_out_mint, false),
            AccountMeta::new_readonly(*token_out_program, false),
            AccountMeta::new(user_token_in_ata, false),
            AccountMeta::new(user_token_out_ata, false),
            AccountMeta::new(boss_token_in_ata, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new_readonly(SYSVAR_INSTRUCTIONS_ID, false),
            AccountMeta::new(*user, true),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_offer_vault_deposit_ix_with_token_program(
    boss: &Pubkey,
    token_mint: &Pubkey,
    amount: u64,
    token_program: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (vault_authority_pda, _) = find_offer_vault_authority_pda();
    let boss_token_ata = derive_ata(boss, token_mint, token_program);
    let vault_token_ata = derive_ata(&vault_authority_pda, token_mint, token_program);

    let mut data = ix_discriminator("offer_vault_deposit").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new_readonly(*token_mint, false),
            AccountMeta::new(boss_token_ata, false),
            AccountMeta::new(vault_token_ata, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*token_program, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_offer_vault_withdraw_ix_with_token_program(
    boss: &Pubkey,
    token_mint: &Pubkey,
    amount: u64,
    token_program: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (vault_authority_pda, _) = find_offer_vault_authority_pda();
    let boss_token_ata = derive_ata(boss, token_mint, token_program);
    let vault_token_ata = derive_ata(&vault_authority_pda, token_mint, token_program);

    let mut data = ix_discriminator("offer_vault_withdraw").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new_readonly(*token_mint, false),
            AccountMeta::new(boss_token_ata, false),
            AccountMeta::new(vault_token_ata, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*token_program, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_redemption_vault_deposit_ix_with_token_program(
    boss: &Pubkey,
    token_mint: &Pubkey,
    amount: u64,
    token_program: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (vault_authority_pda, _) = find_redemption_vault_authority_pda();
    let boss_token_ata = derive_ata(boss, token_mint, token_program);
    let vault_token_ata = derive_ata(&vault_authority_pda, token_mint, token_program);

    let mut data = ix_discriminator("redemption_vault_deposit").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new_readonly(*token_mint, false),
            AccountMeta::new(boss_token_ata, false),
            AccountMeta::new(vault_token_ata, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*token_program, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_redemption_vault_withdraw_ix_with_token_program(
    boss: &Pubkey,
    token_mint: &Pubkey,
    amount: u64,
    token_program: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (vault_authority_pda, _) = find_redemption_vault_authority_pda();
    let boss_token_ata = derive_ata(boss, token_mint, token_program);
    let vault_token_ata = derive_ata(&vault_authority_pda, token_mint, token_program);

    let mut data = ix_discriminator("redemption_vault_withdraw").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new_readonly(*token_mint, false),
            AccountMeta::new(boss_token_ata, false),
            AccountMeta::new(vault_token_ata, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*token_program, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_transfer_mint_authority_to_program_ix_with_token_program(
    boss: &Pubkey,
    mint: &Pubkey,
    token_program: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();

    let data = ix_discriminator("transfer_mint_authority_to_program").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*boss, true),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(*mint, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new_readonly(*token_program, false),
        ],
        data,
    }
}

pub fn build_transfer_mint_authority_to_boss_ix_with_token_program(
    boss: &Pubkey,
    mint: &Pubkey,
    token_program: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();

    let data = ix_discriminator("transfer_mint_authority_to_boss").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*boss, true),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(*mint, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new_readonly(*token_program, false),
        ],
        data,
    }
}

pub fn build_mint_to_ix_with_token_program(
    boss: &Pubkey,
    onyc_mint: &Pubkey,
    amount: u64,
    token_program: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();
    let boss_onyc_ata = derive_ata(boss, onyc_mint, token_program);

    let mut data = ix_discriminator("mint_to").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new(*onyc_mint, false),
            AccountMeta::new(boss_onyc_ata, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new_readonly(*token_program, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_make_redemption_offer_ix_with_programs(
    signer: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    fee_basis_points: u16,
    token_in_program: &Pubkey,
    token_out_program: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (offer_pda, _) = find_offer_pda(token_out_mint, token_in_mint);
    let (redemption_vault_authority_pda, _) = find_redemption_vault_authority_pda();
    let (redemption_offer_pda, _) = find_redemption_offer_pda(token_in_mint, token_out_mint);
    let vault_token_in_ata = derive_ata(
        &redemption_vault_authority_pda,
        token_in_mint,
        token_in_program,
    );
    let vault_token_out_ata = derive_ata(
        &redemption_vault_authority_pda,
        token_out_mint,
        token_out_program,
    );

    let mut data = ix_discriminator("make_redemption_offer").to_vec();
    data.extend_from_slice(&fee_basis_points.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(offer_pda, false),
            AccountMeta::new_readonly(redemption_vault_authority_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new_readonly(*token_in_program, false),
            AccountMeta::new(vault_token_in_ata, false),
            AccountMeta::new_readonly(*token_out_mint, false),
            AccountMeta::new_readonly(*token_out_program, false),
            AccountMeta::new(vault_token_out_ata, false),
            AccountMeta::new(redemption_offer_pda, false),
            AccountMeta::new(*signer, true),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_create_redemption_request_ix_with_token_program(
    redeemer: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    amount: u64,
    counter: u64,
    token_program: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (redemption_offer_pda, _) = find_redemption_offer_pda(token_in_mint, token_out_mint);
    let (redemption_request_pda, _) = find_redemption_request_pda(&redemption_offer_pda, counter);
    let (redemption_vault_authority_pda, _) = find_redemption_vault_authority_pda();
    let redeemer_token_ata = derive_ata(redeemer, token_in_mint, token_program);
    let vault_token_ata = derive_ata(
        &redemption_vault_authority_pda,
        token_in_mint,
        token_program,
    );

    let mut data = ix_discriminator("create_redemption_request").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(redemption_offer_pda, false),
            AccountMeta::new(redemption_request_pda, false),
            AccountMeta::new(*redeemer, true),
            AccountMeta::new_readonly(redemption_vault_authority_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new(redeemer_token_ata, false),
            AccountMeta::new(vault_token_ata, false),
            AccountMeta::new_readonly(*token_program, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_fulfill_redemption_request_ix_with_programs(
    redemption_admin: &Pubkey,
    boss: &Pubkey,
    redeemer: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    request_id: u64,
    token_in_program: &Pubkey,
    token_out_program: &Pubkey,
    amount: u64,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (offer_pda, _) = find_offer_pda(token_out_mint, token_in_mint);
    let (redemption_offer_pda, _) = find_redemption_offer_pda(token_in_mint, token_out_mint);
    let (redemption_request_pda, _) =
        find_redemption_request_pda(&redemption_offer_pda, request_id);
    let (redemption_vault_authority_pda, _) = find_redemption_vault_authority_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();
    let vault_token_in_ata = derive_ata(
        &redemption_vault_authority_pda,
        token_in_mint,
        token_in_program,
    );
    let vault_token_out_ata = derive_ata(
        &redemption_vault_authority_pda,
        token_out_mint,
        token_out_program,
    );
    let user_token_out_ata = derive_ata(redeemer, token_out_mint, token_out_program);
    let boss_token_in_ata = derive_ata(boss, token_in_mint, token_in_program);

    let mut data = ix_discriminator("fulfill_redemption_request").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*boss, false),
            AccountMeta::new_readonly(offer_pda, false),
            AccountMeta::new(redemption_offer_pda, false),
            AccountMeta::new(redemption_request_pda, false),
            AccountMeta::new_readonly(redemption_vault_authority_pda, false),
            AccountMeta::new(vault_token_in_ata, false),
            AccountMeta::new(vault_token_out_ata, false),
            AccountMeta::new(*token_in_mint, false),
            AccountMeta::new_readonly(*token_in_program, false),
            AccountMeta::new(*token_out_mint, false),
            AccountMeta::new_readonly(*token_out_program, false),
            AccountMeta::new(user_token_out_ata, false),
            AccountMeta::new(boss_token_in_ata, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new_readonly(*redeemer, false),
            AccountMeta::new(*redemption_admin, true),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}
