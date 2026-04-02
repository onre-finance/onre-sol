use super::*;

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
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new_readonly(*boss, true),
        ],
        data: ix_discriminator("clear_admins").to_vec(),
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
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new_readonly(*new_boss, true),
        ],
        data: ix_discriminator("accept_boss").to_vec(),
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
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new(*boss, true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data: ix_discriminator("close_state").to_vec(),
    }
}

pub fn build_set_onyc_mint_ix(boss: &Pubkey, onyc_mint: &Pubkey) -> Instruction {
    let (state_pda, _) = find_state_pda();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new_readonly(*boss, true),
            AccountMeta::new_readonly(*onyc_mint, false),
        ],
        data: ix_discriminator("set_onyc_mint").to_vec(),
    }
}

pub fn build_offer_vault_withdraw_ix(
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

pub fn build_redemption_vault_deposit_ix(
    depositor: &Pubkey,
    token_mint: &Pubkey,
    amount: u64,
    token_program: &Pubkey,
) -> Instruction {
    let (vault_authority_pda, _) = find_redemption_vault_authority_pda();
    let depositor_token_ata = derive_ata(depositor, token_mint, token_program);
    let vault_token_ata = derive_ata(&vault_authority_pda, token_mint, token_program);
    let mut data = ix_discriminator("redemption_vault_deposit").to_vec();
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

pub fn build_redemption_vault_withdraw_ix(
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

pub fn build_transfer_mint_authority_to_program_ix(
    boss: &Pubkey,
    mint: &Pubkey,
    token_program: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*boss, true),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(*mint, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new_readonly(*token_program, false),
        ],
        data: ix_discriminator("transfer_mint_authority_to_program").to_vec(),
    }
}

pub fn build_transfer_mint_authority_to_boss_ix(
    boss: &Pubkey,
    mint: &Pubkey,
    token_program: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*boss, true),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(*mint, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new_readonly(*token_program, false),
        ],
        data: ix_discriminator("transfer_mint_authority_to_boss").to_vec(),
    }
}

pub fn build_mint_to_ix(
    boss: &Pubkey,
    onyc_mint: &Pubkey,
    amount: u64,
    token_program: &Pubkey,
) -> Instruction {
    build_mint_to_ix_for_offer(boss, onyc_mint, amount, token_program, &Pubkey::default())
}

pub fn build_mint_to_ix_for_offer(
    boss: &Pubkey,
    onyc_mint: &Pubkey,
    amount: u64,
    token_program: &Pubkey,
    main_offer: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();
    let (offer_vault_authority_pda, _) = find_offer_vault_authority_pda();
    let (market_stats_pda, _) = find_market_stats_pda();
    let buffer_state_pda = find_buffer_state_pda().0;
    let boss_onyc_ata = derive_ata(boss, onyc_mint, token_program);
    let offer_vault_onyc = derive_ata(&offer_vault_authority_pda, onyc_mint, token_program);
    let buffer_vault_onyc = derive_ata(
        &find_reserve_vault_authority_pda().0,
        onyc_mint,
        token_program,
    );
    let management_fee_vault_onyc = derive_ata(
        &find_management_fee_vault_authority_pda().0,
        onyc_mint,
        token_program,
    );
    let performance_fee_vault_onyc = derive_ata(
        &find_performance_fee_vault_authority_pda().0,
        onyc_mint,
        token_program,
    );
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
            AccountMeta::new_readonly(*main_offer, false),
            AccountMeta::new(buffer_state_pda, false),
            AccountMeta::new(buffer_vault_onyc, false),
            AccountMeta::new(management_fee_vault_onyc, false),
            AccountMeta::new(performance_fee_vault_onyc, false),
            AccountMeta::new_readonly(offer_vault_authority_pda, false),
            AccountMeta::new(offer_vault_onyc, false),
            AccountMeta::new(market_stats_pda, false),
        ],
        data,
    }
}

pub fn build_get_nav_ix(token_in_mint: &Pubkey, token_out_mint: &Pubkey) -> Instruction {
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(offer_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new_readonly(*token_out_mint, false),
        ],
        data: ix_discriminator("get_nav").to_vec(),
    }
}

pub fn build_get_apy_ix(token_in_mint: &Pubkey, token_out_mint: &Pubkey) -> Instruction {
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(offer_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new_readonly(*token_out_mint, false),
        ],
        data: ix_discriminator("get_apy").to_vec(),
    }
}

pub fn build_get_nav_adjustment_ix(token_in_mint: &Pubkey, token_out_mint: &Pubkey) -> Instruction {
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(offer_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new_readonly(*token_out_mint, false),
        ],
        data: ix_discriminator("get_nav_adjustment").to_vec(),
    }
}

pub fn build_get_tvl_ix(token_in_mint: &Pubkey, token_out_mint: &Pubkey) -> Instruction {
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);
    let (vault_authority_pda, _) = find_offer_vault_authority_pda();
    let vault_token_out_ata = get_associated_token_address(&vault_authority_pda, token_out_mint);
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
        data: ix_discriminator("get_tvl").to_vec(),
    }
}

pub fn build_get_circulating_supply_ix(onyc_mint: &Pubkey) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (vault_authority_pda, _) = find_offer_vault_authority_pda();
    let onyc_vault_ata = get_associated_token_address(&vault_authority_pda, onyc_mint);
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*onyc_mint, false),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new_readonly(onyc_vault_ata, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ],
        data: ix_discriminator("get_circulating_supply").to_vec(),
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
        data: ix_discriminator("get_tvl").to_vec(),
    }
}

pub fn build_get_circulating_supply_ix_with_token_program(
    onyc_mint: &Pubkey,
    token_program: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (vault_authority_pda, _) = find_offer_vault_authority_pda();
    let onyc_vault_ata = derive_ata(&vault_authority_pda, onyc_mint, token_program);
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*onyc_mint, false),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new_readonly(onyc_vault_ata, false),
            AccountMeta::new_readonly(*token_program, false),
        ],
        data: ix_discriminator("get_circulating_supply").to_vec(),
    }
}

pub fn build_refresh_market_stats_ix(
    signer: &Pubkey,
    token_in_mint: &Pubkey,
    onyc_mint: &Pubkey,
) -> Instruction {
    let (main_offer_pda, _) = find_offer_pda(token_in_mint, onyc_mint);
    let (state_pda, _) = find_state_pda();
    let (vault_authority_pda, _) = find_offer_vault_authority_pda();
    let (market_stats_pda, _) = find_market_stats_pda();
    let onyc_vault_ata = get_associated_token_address(&vault_authority_pda, onyc_mint);
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(main_offer_pda, false),
            AccountMeta::new_readonly(*token_in_mint, false),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*onyc_mint, false),
            AccountMeta::new_readonly(vault_authority_pda, false),
            AccountMeta::new_readonly(onyc_vault_ata, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new(market_stats_pda, false),
            AccountMeta::new(*signer, true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data: ix_discriminator("refresh_market_stats").to_vec(),
    }
}
