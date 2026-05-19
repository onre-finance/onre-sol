use super::*;

pub fn build_set_configurable_vault_destination_ix(
    boss: &Pubkey,
    configurable_vault: &Pubkey,
    kind: u8,
    withdrawal_destination: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let mut data = ix_discriminator("set_configurable_vault_destination").to_vec();
    data.push(kind);
    data.extend_from_slice(withdrawal_destination.as_ref());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new(*configurable_vault, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_withdraw_configurable_vault_ix(
    caller: &Pubkey,
    configurable_vault: &Pubkey,
    destination: &Pubkey,
    mint: &Pubkey,
    kind: u8,
    amount: u64,
    token_program: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let vault_token_account = derive_ata(configurable_vault, mint, token_program);
    let destination_token_account = derive_ata(destination, mint, token_program);
    let mut data = ix_discriminator("withdraw_configurable_vault").to_vec();
    data.push(kind);
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(*caller, true),
            AccountMeta::new_readonly(*configurable_vault, false),
            AccountMeta::new(vault_token_account, false),
            AccountMeta::new_readonly(*destination, false),
            AccountMeta::new(destination_token_account, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(*token_program, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}
