use super::*;

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

pub fn read_market_stats(svm: &LiteSVM) -> MarketStats {
    let (market_stats_pda, _) = find_market_stats_pda();
    let account = svm
        .get_account(&market_stats_pda)
        .expect("market stats account not found");
    let mut data: &[u8] = &account.data;
    MarketStats::try_deserialize(&mut data).expect("Failed to deserialize MarketStats")
}

pub fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 100 * INITIAL_LAMPORTS).unwrap();

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
