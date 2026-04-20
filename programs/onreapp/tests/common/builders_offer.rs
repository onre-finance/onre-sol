use super::*;

pub fn build_make_offer_ix(
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
    match start_time {
        Some(t) => {
            data.push(1);
            data.extend_from_slice(&t.to_le_bytes());
        }
        None => data.push(0),
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
        None => data.push(0),
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

pub fn build_take_offer_permissionless_v2_ix(
    user: &Pubkey,
    boss: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    token_in_amount: u64,
    approval_message: Option<&[u8]>,
    token_in_program: &Pubkey,
    token_out_program: &Pubkey,
) -> Instruction {
    let mut ix = build_take_offer_permissionless_ix(
        user,
        boss,
        token_in_mint,
        token_out_mint,
        token_in_amount,
        approval_message,
        token_in_program,
        token_out_program,
    );
    let (main_offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);
    let (buffer_state_pda, _) = find_buffer_state_pda();
    let (reserve_vault_authority_pda, _) = find_reserve_vault_authority_pda();
    let (management_fee_vault_authority_pda, _) = find_management_fee_vault_authority_pda();
    let (performance_fee_vault_authority_pda, _) = find_performance_fee_vault_authority_pda();
    let (market_stats_pda, _) = find_market_stats_pda();
    let buffer_vault_onyc_ata = derive_ata(
        &reserve_vault_authority_pda,
        token_out_mint,
        token_out_program,
    );
    let management_fee_vault_onyc_ata = derive_ata(
        &management_fee_vault_authority_pda,
        token_out_mint,
        token_out_program,
    );
    let performance_fee_vault_onyc_ata = derive_ata(
        &performance_fee_vault_authority_pda,
        token_out_mint,
        token_out_program,
    );
    ix.data = ix_discriminator("take_offer_permissionless_v2").to_vec();
    ix.data.extend_from_slice(&token_in_amount.to_le_bytes());
    match approval_message {
        Some(msg_bytes) => {
            ix.data.push(1);
            ix.data.extend_from_slice(msg_bytes);
        }
        None => ix.data.push(0),
    }
    ix.accounts
        .insert(17, AccountMeta::new(buffer_state_pda, false));
    ix.accounts
        .insert(18, AccountMeta::new(buffer_vault_onyc_ata, false));
    ix.accounts
        .insert(19, AccountMeta::new(management_fee_vault_onyc_ata, false));
    ix.accounts
        .insert(20, AccountMeta::new(performance_fee_vault_onyc_ata, false));
    ix.accounts
        .insert(21, AccountMeta::new(market_stats_pda, false));
    ix.accounts
        .push(AccountMeta::new_readonly(main_offer_pda, false));
    ix
}

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
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(offer_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new_readonly(*token_out_mint, false),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*boss, true),
        ],
        data: ix_discriminator("delete_all_offer_vectors").to_vec(),
    }
}

pub fn build_take_offer_ix(
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
        None => data.push(0),
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

pub fn build_take_offer_v2_ix(
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
    let (buffer_state_pda, _) = find_buffer_state_pda();
    let (reserve_vault_authority_pda, _) = find_reserve_vault_authority_pda();
    let (management_fee_vault_authority_pda, _) = find_management_fee_vault_authority_pda();
    let (performance_fee_vault_authority_pda, _) = find_performance_fee_vault_authority_pda();
    let (market_stats_pda, _) = find_market_stats_pda();
    let (main_offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);
    let vault_token_in_ata = derive_ata(&vault_authority_pda, token_in_mint, token_in_program);
    let vault_token_out_ata = derive_ata(&vault_authority_pda, token_out_mint, token_out_program);
    let user_token_in_ata = derive_ata(user, token_in_mint, token_in_program);
    let user_token_out_ata = derive_ata(user, token_out_mint, token_out_program);
    let boss_token_in_ata = derive_ata(boss, token_in_mint, token_in_program);
    let buffer_vault_onyc_ata = derive_ata(
        &reserve_vault_authority_pda,
        token_out_mint,
        token_out_program,
    );
    let management_fee_vault_onyc_ata = derive_ata(
        &management_fee_vault_authority_pda,
        token_out_mint,
        token_out_program,
    );
    let performance_fee_vault_onyc_ata = derive_ata(
        &performance_fee_vault_authority_pda,
        token_out_mint,
        token_out_program,
    );
    let mut data = ix_discriminator("take_offer_v2").to_vec();
    data.extend_from_slice(&token_in_amount.to_le_bytes());
    match approval_message {
        Some(msg_bytes) => {
            data.push(1);
            data.extend_from_slice(msg_bytes);
        }
        None => data.push(0),
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
            AccountMeta::new(buffer_state_pda, false),
            AccountMeta::new(buffer_vault_onyc_ata, false),
            AccountMeta::new(management_fee_vault_onyc_ata, false),
            AccountMeta::new(performance_fee_vault_onyc_ata, false),
            AccountMeta::new(market_stats_pda, false),
            AccountMeta::new_readonly(SYSVAR_INSTRUCTIONS_ID, false),
            AccountMeta::new(*user, true),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(main_offer_pda, false),
        ],
        data,
    }
}

pub fn build_quote_swap_ix(
    onyc_mint: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    token_in_amount: u64,
    quote_expiry: i64,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let canonical_token_in = if token_in_mint == onyc_mint {
        token_out_mint
    } else {
        token_in_mint
    };
    let canonical_token_out = if token_out_mint == onyc_mint {
        token_out_mint
    } else {
        token_in_mint
    };
    let (offer_pda, _) = find_offer_pda(canonical_token_in, canonical_token_out);
    let mut data = ix_discriminator("quote_swap").to_vec();
    data.extend_from_slice(&token_in_amount.to_le_bytes());
    data.extend_from_slice(&quote_expiry.to_le_bytes());
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(offer_pda, false),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new_readonly(*token_out_mint, false),
        ],
        data,
    }
}

pub fn build_open_swap_ix(
    onyc_mint: &Pubkey,
    user: &Pubkey,
    boss: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    token_in_amount: u64,
    minimum_out: u64,
    quote_expiry: i64,
    approval_message: Option<&[u8]>,
    token_in_program: &Pubkey,
    token_out_program: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let canonical_token_in = if token_in_mint == onyc_mint {
        token_out_mint
    } else {
        token_in_mint
    };
    let canonical_token_out = if token_out_mint == onyc_mint {
        token_out_mint
    } else {
        token_in_mint
    };
    let (offer_pda, _) = find_offer_pda(canonical_token_in, canonical_token_out);
    let (offer_vault_authority_pda, _) = find_offer_vault_authority_pda();
    let (redemption_vault_authority_pda, _) = find_redemption_vault_authority_pda();
    let (permissionless_authority_pda, _) = find_permissionless_authority_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();
    let (buffer_state_pda, _) = find_buffer_state_pda();
    let (reserve_vault_authority_pda, _) = find_reserve_vault_authority_pda();
    let (management_fee_vault_authority_pda, _) = find_management_fee_vault_authority_pda();
    let (performance_fee_vault_authority_pda, _) = find_performance_fee_vault_authority_pda();
    let (market_stats_pda, _) = find_market_stats_pda();
    let (main_offer_pda, _) = find_offer_pda(canonical_token_in, canonical_token_out);
    let offer_vault_token_in_ata =
        derive_ata(&offer_vault_authority_pda, token_in_mint, token_in_program);
    let offer_vault_token_out_ata = derive_ata(
        &offer_vault_authority_pda,
        token_out_mint,
        token_out_program,
    );
    let redemption_vault_token_in_ata = derive_ata(
        &redemption_vault_authority_pda,
        token_in_mint,
        token_in_program,
    );
    let redemption_vault_token_out_ata = derive_ata(
        &redemption_vault_authority_pda,
        token_out_mint,
        token_out_program,
    );
    let user_token_in_ata = derive_ata(user, token_in_mint, token_in_program);
    let user_token_out_ata = derive_ata(user, token_out_mint, token_out_program);
    let boss_token_in_ata = derive_ata(boss, token_in_mint, token_in_program);
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
    let buffer_vault_onyc_ata = derive_ata(
        &reserve_vault_authority_pda,
        token_out_mint,
        token_out_program,
    );
    let management_fee_vault_onyc_ata = derive_ata(
        &management_fee_vault_authority_pda,
        token_out_mint,
        token_out_program,
    );
    let performance_fee_vault_onyc_ata = derive_ata(
        &performance_fee_vault_authority_pda,
        canonical_token_out,
        token_out_program,
    );
    let offer_vault_onyc_ata = derive_ata(
        &offer_vault_authority_pda,
        canonical_token_out,
        token_out_program,
    );
    let mut data = ix_discriminator("open_swap").to_vec();
    data.extend_from_slice(&token_in_amount.to_le_bytes());
    data.extend_from_slice(&minimum_out.to_le_bytes());
    data.extend_from_slice(&quote_expiry.to_le_bytes());
    match approval_message {
        Some(msg_bytes) => {
            data.push(1);
            data.extend_from_slice(msg_bytes);
        }
        None => data.push(0),
    }
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(offer_pda, false),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*boss, false),
            AccountMeta::new_readonly(offer_vault_authority_pda, false),
            AccountMeta::new(offer_vault_token_in_ata, false),
            AccountMeta::new(offer_vault_token_out_ata, false),
            AccountMeta::new_readonly(redemption_vault_authority_pda, false),
            AccountMeta::new(redemption_vault_token_in_ata, false),
            AccountMeta::new(redemption_vault_token_out_ata, false),
            AccountMeta::new(*token_in_mint, false),
            AccountMeta::new_readonly(*token_in_program, false),
            AccountMeta::new(*token_out_mint, false),
            AccountMeta::new_readonly(*token_out_program, false),
            AccountMeta::new(user_token_in_ata, false),
            AccountMeta::new(user_token_out_ata, false),
            AccountMeta::new(boss_token_in_ata, false),
            AccountMeta::new_readonly(permissionless_authority_pda, false),
            AccountMeta::new(permissionless_token_in_ata, false),
            AccountMeta::new(permissionless_token_out_ata, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new(buffer_state_pda, false),
            AccountMeta::new(buffer_vault_onyc_ata, false),
            AccountMeta::new(management_fee_vault_onyc_ata, false),
            AccountMeta::new(performance_fee_vault_onyc_ata, false),
            AccountMeta::new(market_stats_pda, false),
            AccountMeta::new_readonly(SYSVAR_INSTRUCTIONS_ID, false),
            AccountMeta::new(*user, true),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(main_offer_pda, false),
            AccountMeta::new_readonly(offer_vault_onyc_ata, false),
        ],
        data,
    }
}

pub fn build_offer_vault_deposit_ix(
    depositor: &Pubkey,
    token_mint: &Pubkey,
    amount: u64,
    token_program: &Pubkey,
) -> Instruction {
    let (vault_authority_pda, _) = find_offer_vault_authority_pda();
    let depositor_token_ata = derive_ata(depositor, token_mint, token_program);
    let vault_token_ata = derive_ata(&vault_authority_pda, token_mint, token_program);
    let mut data = ix_discriminator("offer_vault_deposit").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new_readonly(*token_mint, false),
            AccountMeta::new(depositor_token_ata, false),
            AccountMeta::new(vault_token_ata, false),
            AccountMeta::new(*depositor, true),
            AccountMeta::new_readonly(*token_program, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}
