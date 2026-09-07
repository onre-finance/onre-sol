use super::*;

#[allow(clippy::result_large_err)]
pub fn send_tx(
    svm: &mut LiteSVM,
    ixs: &[Instruction],
    signers: &[&Keypair],
) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    let payer = signers[0].pubkey();
    let blockhash = svm.latest_blockhash();
    let mut all_ixs = if ixs
        .iter()
        .any(|ix| solana_compute_budget_interface::check_id(&ix.program_id))
    {
        Vec::new()
    } else {
        vec![ComputeBudgetInstruction::set_compute_unit_limit(1_400_000)]
    };
    all_ixs.extend_from_slice(ixs);
    let msg = Message::new(&all_ixs, Some(&payer));
    let tx = Transaction::new(signers, msg, blockhash);
    let result = svm.send_transaction(tx);
    if std::env::var_os("ONRE_CU_PROFILE").is_some() {
        log_compute_profile(&all_ixs, &result);
    }
    result
}

pub fn coverage_compute_unit_limit(normal_limit: u32) -> u32 {
    if std::env::var_os("SBF_TRACE_DIR").is_some() {
        1_400_000
    } else {
        normal_limit
    }
}

fn log_compute_profile(
    ixs: &[Instruction],
    result: &Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata>,
) {
    let names: Vec<_> = ixs
        .iter()
        .filter(|ix| ix.program_id == PROGRAM_ID)
        .map(profile_instruction_name)
        .collect();
    if names.is_empty() {
        return;
    }

    let name = names.join("+");
    match result {
        Ok(metadata) => {
            println!("CU_PROFILE\t{}\t{}", metadata.compute_units_consumed, name);
        }
        Err(metadata) => {
            println!(
                "CU_PROFILE_ERR\t{}\t{}\t{:?}",
                metadata.meta.compute_units_consumed, name, metadata.err
            );
        }
    }
}

fn profile_instruction_name(ix: &Instruction) -> &'static str {
    let Some(discriminator) = ix.data.get(..8) else {
        return "unknown";
    };
    PROFILE_INSTRUCTION_NAMES
        .iter()
        .copied()
        .find(|name| ix_discriminator(name).as_ref() == discriminator)
        .unwrap_or("unknown")
}

pub fn get_token_balance(svm: &LiteSVM, token_account: &Pubkey) -> u64 {
    let account = svm.get_account(token_account).expect("account not found");
    u64::from_le_bytes(account.data[64..72].try_into().unwrap())
}

const PROFILE_INSTRUCTION_NAMES: &[&str] = &[
    "accept_boss",
    "add_admin",
    "add_approver",
    "add_offer_vector",
    "burn_for_nav_increase",
    "cancel_redemption_request",
    "clear_admins",
    "close_state",
    "configure_max_mint_amount",
    "configure_max_supply",
    "configure_prop_amm",
    "create_redemption_request",
    "delete_all_offer_vectors",
    "delete_offer_vector",
    "deposit_reserve_vault",
    "fulfill_redemption_request",
    "get_apy",
    "get_circulating_supply",
    "get_circulating_supply_v2",
    "get_nav",
    "get_nav_adjustment",
    "get_tvl",
    "get_tvl_v2",
    "initialize",
    "initialize_buffer",
    "initialize_permissionless_authority",
    "make_offer",
    "make_redemption_offer",
    "mint_to",
    "offer_vault_deposit",
    "offer_vault_withdraw",
    "open_swap_buy",
    "open_swap_sell",
    "propose_boss",
    "quote_swap_buy",
    "quote_swap_sell",
    "redemption_vault_deposit",
    "redemption_vault_withdraw",
    "refresh_market_stats",
    "remove_admin",
    "remove_approver",
    "set_buffer_fee_config",
    "set_buffer_gross_apr",
    "set_circulating_supply_excluded_accounts",
    "set_configurable_vault_destination",
    "set_kill_switch",
    "set_main_offer",
    "set_onyc_mint",
    "set_offer_disabled",
    "set_worker",
    "settle_buffer",
    "set_redemption_offer_disabled",
    "take_offer",
    "take_offer_permissionless",
    "take_offer_permissionless_v2",
    "take_offer_v2",
    "transfer_mint_authority_to_boss",
    "transfer_mint_authority_to_program",
    "update_circulating_supply_excluded_balance",
    "update_offer_fee",
    "update_offer_permissionless_fee",
    "update_redemption_offer_fee",
    "update_redemption_offer_prop_amm_sell_fee",
    "update_redemption_offer_vault_target",
    "withdraw_configurable_vault",
    "withdraw_reserve_vault",
];

pub fn read_market_stats(svm: &LiteSVM) -> MarketStats {
    let (market_stats_pda, _) = find_market_stats_pda();
    let account = svm
        .get_account(&market_stats_pda)
        .expect("market stats account not found");
    let mut data: &[u8] = &account.data;
    MarketStats::try_deserialize(&mut data).expect("Failed to deserialize MarketStats")
}

pub fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new().with_precompiles();
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 100 * INITIAL_LAMPORTS)
        .unwrap();

    let program_bytes = include_bytes!("../../../../target/deploy/onreapp.so");
    let program_data_pda = find_program_data_pda();

    let mut program_data_account_data = vec![0u8; 45 + program_bytes.len()];
    program_data_account_data[0..4].copy_from_slice(&3u32.to_le_bytes());
    program_data_account_data[4..12].copy_from_slice(&0u64.to_le_bytes());
    program_data_account_data[12] = 1;
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

    svm.set_sysvar(&Clock {
        slot: 0,
        epoch_start_timestamp: 0,
        epoch: 0,
        leader_schedule_epoch: 0,
        unix_timestamp: 1704067200i64,
    });

    (svm, payer)
}

pub fn setup_initialized() -> (LiteSVM, Keypair, Pubkey) {
    let (mut svm, payer) = setup();
    let boss = payer.pubkey();
    let onyc_mint = create_mint(&mut svm, &payer, 9, &boss);
    let ix = build_initialize_ix(&boss, &onyc_mint);
    send_tx(&mut svm, &[ix], &[&payer]).expect("initialize failed");
    (svm, payer, onyc_mint)
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

pub fn set_and_refresh_circulating_supply_exclusions(
    svm: &mut LiteSVM,
    boss: &Keypair,
    onyc_mint: &Pubkey,
    excluded_owners: &[Pubkey],
) -> Vec<Pubkey> {
    assert!(
        excluded_owners.len() <= 20,
        "excluded owners list cannot exceed 20"
    );

    let ix = build_set_circulating_supply_excluded_accounts_ix(&boss.pubkey(), excluded_owners);
    send_tx(svm, &[ix], &[boss]).unwrap();
    advance_slot(svm);

    refresh_circulating_supply_excluded_balance(svm, boss, onyc_mint, excluded_owners)
}

pub fn refresh_circulating_supply_excluded_balance(
    svm: &mut LiteSVM,
    signer: &Keypair,
    onyc_mint: &Pubkey,
    excluded_owners: &[Pubkey],
) -> Vec<Pubkey> {
    let token_accounts: Vec<Pubkey> = excluded_owners
        .iter()
        .map(|owner| derive_ata(owner, onyc_mint, &TOKEN_PROGRAM_ID))
        .collect();
    let ix = build_update_circulating_supply_excluded_balance_ix(
        &signer.pubkey(),
        onyc_mint,
        &token_accounts,
        &TOKEN_PROGRAM_ID,
    );
    send_tx(svm, &[ix], &[signer]).unwrap();
    advance_slot(svm);
    token_accounts
}
