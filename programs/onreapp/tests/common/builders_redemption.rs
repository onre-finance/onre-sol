#![allow(clippy::too_many_arguments)]

use super::*;

pub fn build_make_redemption_offer_ix(
    signer: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    fee_basis_points: u16,
    token_in_program: &Pubkey,
    token_out_program: &Pubkey,
) -> Instruction {
    build_make_redemption_offer_ix_with_prop_amm_sell_fee(
        signer,
        token_in_mint,
        token_out_mint,
        fee_basis_points,
        0,
        token_in_program,
        token_out_program,
    )
}

pub fn build_make_redemption_offer_ix_with_prop_amm_sell_fee(
    signer: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    fee_basis_points: u16,
    fee_basis_points_prop_amm_sell: u16,
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
    data.extend_from_slice(&fee_basis_points_prop_amm_sell.to_le_bytes());
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
    request_id: impl ToString,
    token_program: &Pubkey,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (redemption_offer_pda, _) = find_redemption_offer_pda(token_in_mint, token_out_mint);
    let (offer_pda, _) = find_offer_pda(token_out_mint, token_in_mint);
    let request_id = test_redemption_request_id(request_id);
    let (redemption_request_pda, _) =
        find_redemption_request_pda(&redemption_offer_pda, redeemer, &request_id);
    let (redemption_vault_authority_pda, _) = find_redemption_vault_authority_pda();
    let redeemer_token_ata = derive_ata(redeemer, token_in_mint, token_program);
    let vault_token_ata = derive_ata(
        &redemption_vault_authority_pda,
        token_in_mint,
        token_program,
    );
    let mut data = ix_discriminator("create_redemption_request").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&(request_id.len() as u32).to_le_bytes());
    data.extend_from_slice(request_id.as_bytes());
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(redemption_offer_pda, false),
            AccountMeta::new_readonly(offer_pda, false),
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
    worker: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    request_id: impl ToString,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (redemption_offer_pda, _) = find_redemption_offer_pda(token_in_mint, token_out_mint);
    let (redemption_request_pda, _) =
        find_redemption_request_pda(&redemption_offer_pda, redeemer, request_id);
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
            AccountMeta::new(*worker, false),
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
    worker: &Pubkey,
    _boss: &Pubkey,
    main_offer: &Pubkey,
    redeemer: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    request_id: impl ToString,
    token_in_program: &Pubkey,
    token_out_program: &Pubkey,
    amount: u64,
) -> Instruction {
    build_fulfill_redemption_request_ix_with_main_offer(
        worker,
        _boss,
        main_offer,
        main_offer,
        redeemer,
        token_in_mint,
        token_out_mint,
        request_id,
        token_in_program,
        token_out_program,
        amount,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_fulfill_redemption_request_ix_with_main_offer(
    worker: &Pubkey,
    _boss: &Pubkey,
    offer: &Pubkey,
    main_offer: &Pubkey,
    redeemer: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    request_id: impl ToString,
    token_in_program: &Pubkey,
    token_out_program: &Pubkey,
    amount: u64,
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (redemption_offer_pda, _) = find_redemption_offer_pda(token_in_mint, token_out_mint);
    let (redemption_request_pda, _) =
        find_redemption_request_pda(&redemption_offer_pda, redeemer, request_id);
    let (redemption_vault_authority_pda, _) = find_redemption_vault_authority_pda();
    let (redemption_fee_vault_pda, _) = find_redemption_fee_vault_pda();
    let (offer_proceeds_vault_pda, _) = find_offer_proceeds_vault_pda();
    let (mint_authority_pda, _) = find_mint_authority_pda();
    let (offer_vault_authority_pda, _) = find_offer_vault_authority_pda();
    let (market_stats_pda, _) = find_market_stats_pda();
    let (excluded_balance_pda, _) = find_circulating_supply_excluded_balance_pda();
    let (buffer_state_pda, _) = find_buffer_state_pda();
    let (reserve_vault_authority_pda, _) = find_reserve_vault_authority_pda();
    let (management_fee_vault_pda, _) = find_management_fee_vault_pda();
    let (performance_fee_vault_pda, _) = find_performance_fee_vault_pda();
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
    let offer_proceeds_token_in_ata =
        derive_ata(&offer_proceeds_vault_pda, token_in_mint, token_in_program);
    let redemption_fee_token_in_ata =
        derive_ata(&redemption_fee_vault_pda, token_in_mint, token_in_program);
    let buffer_vault_onyc_ata = derive_ata(
        &reserve_vault_authority_pda,
        token_in_mint,
        token_in_program,
    );
    let management_fee_vault_onyc_ata =
        derive_ata(&management_fee_vault_pda, token_in_mint, token_in_program);
    let performance_fee_vault_onyc_ata =
        derive_ata(&performance_fee_vault_pda, token_in_mint, token_in_program);
    let mut data = ix_discriminator("fulfill_redemption_request").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*offer, false),
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
            AccountMeta::new(offer_proceeds_vault_pda, false),
            AccountMeta::new(offer_proceeds_token_in_ata, false),
            AccountMeta::new(redemption_fee_vault_pda, false),
            AccountMeta::new(redemption_fee_token_in_ata, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new_readonly(*redeemer, false),
            AccountMeta::new(*worker, true),
            AccountMeta::new(buffer_state_pda, false),
            AccountMeta::new(buffer_vault_onyc_ata, false),
            AccountMeta::new(management_fee_vault_onyc_ata, false),
            AccountMeta::new(performance_fee_vault_onyc_ata, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(offer_vault_authority_pda, false),
            AccountMeta::new_readonly(offer_vault_onyc_ata, false),
            AccountMeta::new(market_stats_pda, false),
            AccountMeta::new_readonly(excluded_balance_pda, false),
            AccountMeta::new_readonly(*main_offer, false),
        ],
        data,
    }
}

fn build_redemption_offer_admin_ix(
    discriminator: &str,
    signer: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    data_suffix: &[u8],
) -> Instruction {
    let (state_pda, _) = find_state_pda();
    let (redemption_offer_pda, _) = find_redemption_offer_pda(token_in_mint, token_out_mint);
    let mut data = ix_discriminator(discriminator).to_vec();
    data.extend_from_slice(data_suffix);
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(redemption_offer_pda, false),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new_readonly(*signer, true),
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
    build_redemption_offer_admin_ix(
        "update_redemption_offer_fee",
        boss,
        token_in_mint,
        token_out_mint,
        &new_fee_basis_points.to_le_bytes(),
    )
}

pub fn build_update_redemption_offer_prop_amm_sell_fee_ix(
    boss: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    new_fee_basis_points_prop_amm_sell: u16,
) -> Instruction {
    build_redemption_offer_admin_ix(
        "update_redemption_offer_prop_amm_sell_fee",
        boss,
        token_in_mint,
        token_out_mint,
        &new_fee_basis_points_prop_amm_sell.to_le_bytes(),
    )
}

pub fn build_set_redemption_offer_disabled_ix(
    signer: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    disabled: bool,
) -> Instruction {
    build_redemption_offer_admin_ix(
        "set_redemption_offer_disabled",
        signer,
        token_in_mint,
        token_out_mint,
        &[if disabled { 1 } else { 0 }],
    )
}

pub fn build_update_redemption_offer_vault_target_ix(
    boss: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    new_vault_target_bps: u16,
) -> Instruction {
    build_redemption_offer_admin_ix(
        "update_redemption_offer_vault_target",
        boss,
        token_in_mint,
        token_out_mint,
        &new_vault_target_bps.to_le_bytes(),
    )
}
