use super::*;

pub fn build_make_redemption_offer_ix(
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

pub fn build_create_redemption_request_ix(
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
        data: ix_discriminator("cancel_redemption_request").to_vec(),
    }
}

pub fn build_fulfill_redemption_request_ix(
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
    let (redemption_fee_vault_authority_pda, _) = find_redemption_fee_vault_authority_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();
    let (offer_vault_authority_pda, _) = find_offer_vault_authority_pda();
    let (market_stats_pda, _) = find_market_stats_pda();
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
    let offer_vault_onyc_ata =
        derive_ata(&offer_vault_authority_pda, token_in_mint, token_in_program);
    let user_token_out_ata = derive_ata(redeemer, token_out_mint, token_out_program);
    let boss_token_in_ata = derive_ata(boss, token_in_mint, token_in_program);
    let fee_destination = redemption_fee_vault_authority_pda;
    let fee_destination_token_in_ata =
        derive_ata(&fee_destination, token_in_mint, token_in_program);
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
            AccountMeta::new(redemption_fee_vault_authority_pda, false),
            AccountMeta::new_readonly(fee_destination, false),
            AccountMeta::new(fee_destination_token_in_ata, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new_readonly(*redeemer, false),
            AccountMeta::new(*redemption_admin, true),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(offer_vault_authority_pda, false),
            AccountMeta::new_readonly(offer_vault_onyc_ata, false),
            AccountMeta::new(market_stats_pda, false),
        ],
        data,
    }
}

pub fn build_fulfill_redemption_request_v2_ix(
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
    let mut ix = build_fulfill_redemption_request_ix(
        redemption_admin,
        boss,
        redeemer,
        token_in_mint,
        token_out_mint,
        request_id,
        token_in_program,
        token_out_program,
        amount,
    );
    let (buffer_state_pda, _) = find_buffer_state_pda();
    let (reserve_vault_authority_pda, _) = find_reserve_vault_authority_pda();
    let (management_fee_vault_authority_pda, _) = find_management_fee_vault_authority_pda();
    let (performance_fee_vault_authority_pda, _) = find_performance_fee_vault_authority_pda();
    let buffer_vault_onyc_ata = derive_ata(
        &reserve_vault_authority_pda,
        token_in_mint,
        token_in_program,
    );
    let management_fee_vault_onyc_ata = derive_ata(
        &management_fee_vault_authority_pda,
        token_in_mint,
        token_in_program,
    );
    let performance_fee_vault_onyc_ata = derive_ata(
        &performance_fee_vault_authority_pda,
        token_in_mint,
        token_in_program,
    );

    ix.data = ix_discriminator("fulfill_redemption_request_v2").to_vec();
    ix.data.extend_from_slice(&amount.to_le_bytes());
    ix.accounts
        .insert(20, AccountMeta::new(buffer_state_pda, false));
    ix.accounts
        .insert(21, AccountMeta::new(buffer_vault_onyc_ata, false));
    ix.accounts
        .insert(22, AccountMeta::new(management_fee_vault_onyc_ata, false));
    ix.accounts
        .insert(23, AccountMeta::new(performance_fee_vault_onyc_ata, false));

    ix
}

pub fn build_set_redemption_fee_destination_ix(
    boss: &Pubkey,
    fee_destination: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (redemption_fee_vault_authority_pda, _) = find_redemption_fee_vault_authority_pda();
    let mut data = ix_discriminator("set_redemption_fee_destination").to_vec();
    data.extend_from_slice(fee_destination.as_ref());
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new(redemption_fee_vault_authority_pda, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_fulfill_redemption_request_with_fee_dest_ix(
    redemption_admin: &Pubkey,
    boss: &Pubkey,
    redeemer: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    request_id: u64,
    token_in_program: &Pubkey,
    token_out_program: &Pubkey,
    amount: u64,
    fee_destination: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (offer_pda, _) = find_offer_pda(token_out_mint, token_in_mint);
    let (redemption_offer_pda, _) = find_redemption_offer_pda(token_in_mint, token_out_mint);
    let (redemption_request_pda, _) =
        find_redemption_request_pda(&redemption_offer_pda, request_id);
    let (redemption_vault_authority_pda, _) = find_redemption_vault_authority_pda();
    let (redemption_fee_vault_authority_pda, _) = find_redemption_fee_vault_authority_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();
    let (offer_vault_authority_pda, _) = find_offer_vault_authority_pda();
    let (market_stats_pda, _) = find_market_stats_pda();
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
    let offer_vault_onyc_ata =
        derive_ata(&offer_vault_authority_pda, token_in_mint, token_in_program);
    let user_token_out_ata = derive_ata(redeemer, token_out_mint, token_out_program);
    let boss_token_in_ata = derive_ata(boss, token_in_mint, token_in_program);
    let fee_destination_token_in_ata = derive_ata(fee_destination, token_in_mint, token_in_program);
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
            AccountMeta::new(redemption_fee_vault_authority_pda, false),
            AccountMeta::new_readonly(*fee_destination, false),
            AccountMeta::new(fee_destination_token_in_ata, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new_readonly(*redeemer, false),
            AccountMeta::new(*redemption_admin, true),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(offer_vault_authority_pda, false),
            AccountMeta::new_readonly(offer_vault_onyc_ata, false),
            AccountMeta::new(market_stats_pda, false),
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
