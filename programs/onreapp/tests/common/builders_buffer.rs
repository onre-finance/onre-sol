use super::*;

pub fn build_initialize_buffer_ix(
    boss: &Pubkey,
    offer: &Pubkey,
    onyc_mint: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (buffer_state_pda, _) = find_buffer_state_pda();
    let (reserve_vault_authority_pda, _) = find_reserve_vault_authority_pda();
    let (management_fee_vault_authority_pda, _) = find_management_fee_vault_authority_pda();
    let (performance_fee_vault_authority_pda, _) = find_performance_fee_vault_authority_pda();
    let buffer_vault_onyc_ata =
        derive_ata(&reserve_vault_authority_pda, onyc_mint, &TOKEN_PROGRAM_ID);
    let management_fee_vault_onyc_ata = derive_ata(
        &management_fee_vault_authority_pda,
        onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let performance_fee_vault_onyc_ata = derive_ata(
        &performance_fee_vault_authority_pda,
        onyc_mint,
        &TOKEN_PROGRAM_ID,
    );

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new(buffer_state_pda, false),
            AccountMeta::new(reserve_vault_authority_pda, false),
            AccountMeta::new(management_fee_vault_authority_pda, false),
            AccountMeta::new(performance_fee_vault_authority_pda, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new(*onyc_mint, false),
            AccountMeta::new_readonly(*offer, false),
            AccountMeta::new(buffer_vault_onyc_ata, false),
            AccountMeta::new(management_fee_vault_onyc_ata, false),
            AccountMeta::new(performance_fee_vault_onyc_ata, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data: ix_discriminator("initialize_buffer").to_vec(),
    }
}

pub fn build_set_main_offer_ix(boss: &Pubkey, offer: &Pubkey) -> Instruction {
    let (state_pda, _) = find_state_pda();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new_readonly(*boss, true),
            AccountMeta::new_readonly(*offer, false),
        ],
        data: ix_discriminator("set_main_offer").to_vec(),
    }
}

pub fn build_set_buffer_gross_yield_ix(
    boss: &Pubkey,
    main_offer: &Pubkey,
    onyc_mint: &Pubkey,
    gross_yield: u64,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (buffer_state_pda, _) = find_buffer_state_pda();
    let (offer_vault_authority_pda, _) = find_offer_vault_authority_pda();
    let (reserve_vault_authority_pda, _) = find_reserve_vault_authority_pda();
    let (management_fee_vault_authority_pda, _) = find_management_fee_vault_authority_pda();
    let (performance_fee_vault_authority_pda, _) = find_performance_fee_vault_authority_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();
    let (market_stats_pda, _) = find_market_stats_pda();
    let vault_token_out_ata = derive_ata(&offer_vault_authority_pda, onyc_mint, &TOKEN_PROGRAM_ID);
    let reserve_vault_onyc_ata =
        derive_ata(&reserve_vault_authority_pda, onyc_mint, &TOKEN_PROGRAM_ID);
    let management_fee_vault_onyc_ata = derive_ata(
        &management_fee_vault_authority_pda,
        onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let performance_fee_vault_onyc_ata = derive_ata(
        &performance_fee_vault_authority_pda,
        onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let mut data = ix_discriminator("set_buffer_gross_apr").to_vec();
    data.extend_from_slice(&gross_yield.to_le_bytes());
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*boss, true),
            AccountMeta::new_readonly(*main_offer, false),
            AccountMeta::new(*onyc_mint, false),
            AccountMeta::new_readonly(offer_vault_authority_pda, false),
            AccountMeta::new_readonly(vault_token_out_ata, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new(buffer_state_pda, false),
            AccountMeta::new(reserve_vault_onyc_ata, false),
            AccountMeta::new(management_fee_vault_onyc_ata, false),
            AccountMeta::new(performance_fee_vault_onyc_ata, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new(market_stats_pda, false),
        ],
        data,
    }
}

pub fn build_set_buffer_fee_config_ix(
    boss: &Pubkey,
    main_offer: &Pubkey,
    onyc_mint: &Pubkey,
    management_fee_basis_points: u16,
    performance_fee_basis_points: u16,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (buffer_state_pda, _) = find_buffer_state_pda();
    let (offer_vault_authority_pda, _) = find_offer_vault_authority_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();
    let (market_stats_pda, _) = find_market_stats_pda();
    let vault_token_out_ata = derive_ata(&offer_vault_authority_pda, onyc_mint, &TOKEN_PROGRAM_ID);
    let reserve_vault_onyc_ata = derive_ata(
        &find_reserve_vault_authority_pda().0,
        onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let management_fee_vault_onyc_ata = derive_ata(
        &find_management_fee_vault_authority_pda().0,
        onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let performance_fee_vault_onyc_ata = derive_ata(
        &find_performance_fee_vault_authority_pda().0,
        onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let mut data = ix_discriminator("set_buffer_fee_config").to_vec();
    data.extend_from_slice(&management_fee_basis_points.to_le_bytes());
    data.extend_from_slice(boss.as_ref());
    data.extend_from_slice(&performance_fee_basis_points.to_le_bytes());
    data.extend_from_slice(boss.as_ref());
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*boss, true),
            AccountMeta::new_readonly(*main_offer, false),
            AccountMeta::new(*onyc_mint, false),
            AccountMeta::new_readonly(offer_vault_authority_pda, false),
            AccountMeta::new_readonly(vault_token_out_ata, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new(buffer_state_pda, false),
            AccountMeta::new(reserve_vault_onyc_ata, false),
            AccountMeta::new(management_fee_vault_onyc_ata, false),
            AccountMeta::new(performance_fee_vault_onyc_ata, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new(market_stats_pda, false),
        ],
        data,
    }
}

pub fn build_deposit_reserve_vault_ix(
    depositor: &Pubkey,
    onyc_mint: &Pubkey,
    amount: u64,
) -> Instruction {
    let (buffer_state_pda, _) = find_buffer_state_pda();
    let (reserve_vault_authority_pda, _) = find_reserve_vault_authority_pda();
    let depositor_onyc_ata = derive_ata(depositor, onyc_mint, &TOKEN_PROGRAM_ID);
    let reserve_vault_onyc_ata =
        derive_ata(&reserve_vault_authority_pda, onyc_mint, &TOKEN_PROGRAM_ID);
    let mut data = ix_discriminator("deposit_reserve_vault").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(buffer_state_pda, false),
            AccountMeta::new_readonly(reserve_vault_authority_pda, false),
            AccountMeta::new_readonly(*onyc_mint, false),
            AccountMeta::new(depositor_onyc_ata, false),
            AccountMeta::new(reserve_vault_onyc_ata, false),
            AccountMeta::new(*depositor, true),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_withdraw_reserve_vault_ix(
    boss: &Pubkey,
    onyc_mint: &Pubkey,
    amount: u64,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (buffer_state_pda, _) = find_buffer_state_pda();
    let (reserve_vault_authority_pda, _) = find_reserve_vault_authority_pda();
    let boss_onyc_ata = derive_ata(boss, onyc_mint, &TOKEN_PROGRAM_ID);
    let reserve_vault_onyc_ata =
        derive_ata(&reserve_vault_authority_pda, onyc_mint, &TOKEN_PROGRAM_ID);
    let mut data = ix_discriminator("withdraw_reserve_vault").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(buffer_state_pda, false),
            AccountMeta::new_readonly(reserve_vault_authority_pda, false),
            AccountMeta::new_readonly(*onyc_mint, false),
            AccountMeta::new(boss_onyc_ata, false),
            AccountMeta::new(reserve_vault_onyc_ata, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_withdraw_management_fees_ix(
    boss: &Pubkey,
    onyc_mint: &Pubkey,
    amount: u64,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (buffer_state_pda, _) = find_buffer_state_pda();
    let (management_fee_vault_authority_pda, _) = find_management_fee_vault_authority_pda();
    let boss_onyc_ata = derive_ata(boss, onyc_mint, &TOKEN_PROGRAM_ID);
    let management_fee_vault_onyc_ata = derive_ata(
        &management_fee_vault_authority_pda,
        onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let mut data = ix_discriminator("withdraw_management_fees").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(buffer_state_pda, false),
            AccountMeta::new_readonly(management_fee_vault_authority_pda, false),
            AccountMeta::new_readonly(*boss, false),
            AccountMeta::new(*onyc_mint, false),
            AccountMeta::new(boss_onyc_ata, false),
            AccountMeta::new(management_fee_vault_onyc_ata, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_withdraw_performance_fees_ix(
    boss: &Pubkey,
    onyc_mint: &Pubkey,
    amount: u64,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (buffer_state_pda, _) = find_buffer_state_pda();
    let (performance_fee_vault_authority_pda, _) = find_performance_fee_vault_authority_pda();
    let boss_onyc_ata = derive_ata(boss, onyc_mint, &TOKEN_PROGRAM_ID);
    let performance_fee_vault_onyc_ata = derive_ata(
        &performance_fee_vault_authority_pda,
        onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let mut data = ix_discriminator("withdraw_performance_fees").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(buffer_state_pda, false),
            AccountMeta::new_readonly(performance_fee_vault_authority_pda, false),
            AccountMeta::new_readonly(*boss, false),
            AccountMeta::new(*onyc_mint, false),
            AccountMeta::new(boss_onyc_ata, false),
            AccountMeta::new(performance_fee_vault_onyc_ata, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn build_burn_for_nav_increase_ix(
    boss: &Pubkey,
    main_offer: &Pubkey,
    onyc_mint: &Pubkey,
    asset_adjustment_amount: u64,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (buffer_state_pda, _) = find_buffer_state_pda();
    let (offer_vault_authority_pda, _) = find_offer_vault_authority_pda();
    let (reserve_vault_authority_pda, _) = find_reserve_vault_authority_pda();
    let (management_fee_vault_authority_pda, _) = find_management_fee_vault_authority_pda();
    let (performance_fee_vault_authority_pda, _) = find_performance_fee_vault_authority_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();
    let (market_stats_pda, _) = find_market_stats_pda();
    let vault_token_out_ata = derive_ata(&offer_vault_authority_pda, onyc_mint, &TOKEN_PROGRAM_ID);
    let buffer_vault_onyc_ata =
        derive_ata(&reserve_vault_authority_pda, onyc_mint, &TOKEN_PROGRAM_ID);
    let management_fee_vault_onyc_ata = derive_ata(
        &management_fee_vault_authority_pda,
        onyc_mint,
        &TOKEN_PROGRAM_ID,
    );
    let performance_fee_vault_onyc_ata = derive_ata(
        &performance_fee_vault_authority_pda,
        onyc_mint,
        &TOKEN_PROGRAM_ID,
    );

    let mut data = ix_discriminator("burn_for_nav_increase").to_vec();
    data.extend_from_slice(&asset_adjustment_amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(buffer_state_pda, false),
            AccountMeta::new_readonly(*boss, true),
            AccountMeta::new_readonly(*main_offer, false),
            AccountMeta::new(*onyc_mint, false),
            AccountMeta::new_readonly(offer_vault_authority_pda, false),
            AccountMeta::new_readonly(reserve_vault_authority_pda, false),
            AccountMeta::new_readonly(vault_token_out_ata, false),
            AccountMeta::new(buffer_vault_onyc_ata, false),
            AccountMeta::new_readonly(management_fee_vault_authority_pda, false),
            AccountMeta::new(management_fee_vault_onyc_ata, false),
            AccountMeta::new_readonly(performance_fee_vault_authority_pda, false),
            AccountMeta::new(performance_fee_vault_onyc_ata, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new(market_stats_pda, false),
        ],
        data,
    }
}
