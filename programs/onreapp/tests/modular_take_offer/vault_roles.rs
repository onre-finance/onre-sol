use super::*;

fn liquidity_vault(f: &mut Fixture, id: u64, target: u16) -> Pubkey {
    let key = pda(&[b"vault", &[VaultKind::Liquidity as u8], &id.to_le_bytes()]);
    let instruction = ix(
        accounts::CreateVaultConfig {
            vault: key,
            state: find_state_pda().0,
            boss: f.boss.pubkey(),
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::CreateVaultConfig {
            kind: VaultKind::Liquidity,
            instance_id: id,
            withdrawal_destination: f.boss.pubkey(),
            refill_target_bps: target,
        },
    );
    send_tx(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    key
}
fn execute(
    f: &mut Fixture,
    gross: u64,
    minimum: u64,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let instruction = f.take(gross, minimum);
    if read::<OfferConfig>(&f.svm, f.offer).flow == OfferFlow::Permissioned {
        f.send(instruction)
    } else {
        f.send_without_approval(instruction)
    }
}
fn kind(flow: OfferFlow) -> QuoterKind {
    if flow == OfferFlow::Permissioned {
        QuoterKind::Nav
    } else {
        QuoterKind::NavPermissionless
    }
}

#[test]
fn nav_buys_independently_select_inventory_and_destination_refill() {
    for onyc in [false, true] {
        for token_2022 in [false, true] {
            for flow in [OfferFlow::Permissioned, OfferFlow::Permissionless] {
                for inventory in [false, true] {
                    for destination_target in [None, Some(0), Some(1000)] {
                        let mut f = Fixture::new(onyc, token_2022);
                        modular_prop_amm::init_buffer(&mut f);
                        let source = inventory.then(|| liquidity_vault(&mut f, 1, 0));
                        let destination =
                            destination_target.map(|target| liquidity_vault(&mut f, 2, target));
                        // Identical pre-trade supply whether output is minted or transferred.
                        f.seed_tokens(
                            f.mint,
                            f.boss.pubkey(),
                            if inventory {
                                8_000_000_000
                            } else {
                                10_000_000_000
                            },
                        );
                        if let Some(source) = source {
                            f.seed_tokens(f.mint, source, 2_000_000_000);
                        }
                        f.create_route_with_vaults(
                            OfferDirection::AssetToManaged,
                            flow,
                            kind(flow),
                            source,
                            destination,
                        );
                        let before = get_mint_supply(&f.svm, &f.mint);
                        if inventory {
                            edit::<ManagedTokenConfig>(&mut f.svm, managed(f.mint), |m| {
                                m.max_supply = before;
                                m.max_mint_amount = 1;
                            });
                        }
                        let event =
                            executed_event(&execute(&mut f, 2_000_000, 990_000_000).unwrap());
                        let deposited = if destination_target == Some(1000) {
                            // ONyc's canonical main NAV is 1, other managed mints NAV is 2.
                            if onyc {
                                1_000_000
                            } else {
                                1_980_000
                            }
                        } else {
                            0
                        };
                        assert_eq!(
                            (
                                event.amount_out,
                                event.source_amount,
                                event.destination_amount
                            ),
                            (
                                990_000_000,
                                if inventory { 990_000_000 } else { 0 },
                                deposited
                            )
                        );
                        assert_eq!(event.proceeds_amount, 1_980_000 - deposited);
                        assert_eq!(
                            get_mint_supply(&f.svm, &f.mint),
                            before + if inventory { 0 } else { 990_000_000 }
                        );
                        if let Some(source) = source {
                            assert_eq!(f.balance(f.mint, source, f.token_program), 1_010_000_000);
                        }
                        if let Some(destination) = destination {
                            if deposited > 0 {
                                assert_eq!(
                                    f.balance(f.asset, destination, f.asset_program),
                                    deposited
                                );
                            } else {
                                assert!(f
                                    .svm
                                    .get_account(&derive_ata(
                                        &destination,
                                        &f.asset,
                                        &f.asset_program
                                    ))
                                    .is_none());
                            }
                        }
                        assert_eq!(
                            f.balance(f.mint, f.user.pubkey(), f.token_program),
                            990_000_000
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn sell_destination_receives_all_net_managed_input_instead_of_burning() {
    for onyc in [false, true] {
        for token_2022 in [false, true] {
            for flow in [OfferFlow::Permissioned, OfferFlow::Permissionless] {
                for retain in [false, true] {
                    let mut f = Fixture::new(onyc, token_2022);
                    modular_prop_amm::init_buffer(&mut f);
                    let source = liquidity_vault(&mut f, 1, 0);
                    let destination = retain.then(|| liquidity_vault(&mut f, 2, 0));
                    f.seed_tokens(f.mint, f.user.pubkey(), 2_000_000_000);
                    f.seed_tokens(f.asset, source, 10_000_000);
                    f.create_route_with_vaults(
                        OfferDirection::ManagedToAsset,
                        flow,
                        kind(flow),
                        Some(source),
                        destination,
                    );
                    let before = get_mint_supply(&f.svm, &f.mint);
                    let event = executed_event(&execute(&mut f, 1_000_000_000, 1_980_000).unwrap());
                    assert_eq!(
                        (event.source_amount, event.destination_amount),
                        (1_980_000, if retain { 990_000_000 } else { 0 })
                    );
                    assert_eq!(
                        get_mint_supply(&f.svm, &f.mint),
                        before - if retain { 0 } else { 990_000_000 }
                    );
                    if let Some(destination) = destination {
                        assert_eq!(f.balance(f.mint, destination, f.token_program), 990_000_000);
                    }
                    assert_eq!(f.balance(f.asset, source, f.asset_program), 8_020_000);
                    assert_eq!(
                        f.balance(f.mint, vault(VaultKind::Fee), f.token_program),
                        10_000_000
                    );
                }
            }
        }
    }
}

#[test]
fn managed_inventory_signs_for_modular_legacy_and_redemption_custody() {
    for token_2022 in [false, true] {
        for custody in 0..3 {
            let mut f = Fixture::new(false, token_2022);
            let source = match custody {
                0 => liquidity_vault(&mut f, 1, 0),
                1 => {
                    let key = find_prop_amm_proceeds_vault_pda().0;
                    let setup = build_set_configurable_vault_destination_ix(
                        &f.boss.pubkey(),
                        &key,
                        5,
                        &f.boss.pubkey(),
                    );
                    send_tx(&mut f.svm, &[setup], &[&f.boss]).unwrap();
                    key
                }
                _ => find_redemption_vault_authority_pda().0,
            };
            let asset_custody = liquidity_vault(&mut f, 2, 10000);
            f.seed_tokens(f.mint, source, 2_000_000_000);
            f.create_route_with_vaults(
                OfferDirection::AssetToManaged,
                OfferFlow::Permissionless,
                QuoterKind::NavPermissionless,
                Some(source),
                Some(asset_custody),
            );
            let before = get_mint_supply(&f.svm, &f.mint);
            execute(&mut f, 2_000_000, 990_000_000).unwrap();
            f.create_route_with_vaults(
                OfferDirection::ManagedToAsset,
                OfferFlow::Permissionless,
                QuoterKind::NavPermissionless,
                Some(asset_custody),
                Some(source),
            );
            execute(&mut f, 500_000_000, 990_000).unwrap();
            assert_eq!(get_mint_supply(&f.svm, &f.mint), before);
            assert_eq!(f.balance(f.mint, source, f.token_program), 1_505_000_000);
            assert_eq!(f.balance(f.asset, asset_custody, f.asset_program), 990_000);
        }
    }
}

#[test]
fn configured_inventory_cannot_fall_back_to_minting_and_bad_roles_roll_back() {
    let mut f = Fixture::new(false, false);
    let source = liquidity_vault(&mut f, 1, 0);
    let destination = liquidity_vault(&mut f, 2, 1000);
    f.create_route_with_vaults(
        OfferDirection::AssetToManaged,
        OfferFlow::Permissionless,
        QuoterKind::NavPermissionless,
        Some(source),
        Some(destination),
    );
    let original = f.svm.clone();
    let error = execute(&mut f, 2_000_000, 1).unwrap_err();
    assert_eq!(
        error.err,
        TransactionError::InstructionError(
            1,
            InstructionError::Custom(6000 + OnreError::InsufficientBalance as u32)
        )
    );
    assert_eq!(get_mint_supply(&f.svm, &f.mint), 0);
    assert_eq!(
        f.balance(f.asset, f.user.pubkey(), f.asset_program),
        10_000_000
    );
    assert!(f
        .svm
        .get_account(&derive_ata(&destination, &f.asset, &f.asset_program))
        .is_none());
    f.svm = original;
    f.seed_tokens(f.mint, source, 2_000_000_000);
    // Both authorities are legitimate Liquidity vaults; their configured roles
    // and the ATA's mint still cannot be swapped by a caller.
    for omit in [source, destination] {
        let mut instruction = f.take(2_000_000, 1);
        omit_account(&mut instruction, omit);
        assert!(f.send_without_approval(instruction).is_err());
    }
    let mut instruction = f.take(2_000_000, 1);
    for account in &mut instruction.accounts {
        if account.pubkey == source {
            account.pubkey = destination;
        } else if account.pubkey == destination {
            account.pubkey = source;
        }
    }
    assert!(f.send_without_approval(instruction).is_err());
    let mut instruction = f.take(2_000_000, 1);
    let expected = derive_ata(&source, &f.mint, &f.token_program);
    for account in &mut instruction.accounts {
        if account.pubkey == expected {
            account.pubkey = derive_ata(&source, &f.asset, &f.asset_program);
        }
    }
    assert!(f.send_without_approval(instruction).is_err());
    assert_eq!(f.balance(f.mint, source, f.token_program), 2_000_000_000);
}

#[test]
fn prop_curve_uses_asset_side_vault_and_inventory_only_controls_settlement() {
    for token_2022 in [false, true] {
        let mut f = Fixture::new(false, token_2022);
        modular_prop_amm::init_buffer(&mut f);
        let inventory = liquidity_vault(&mut f, 1, 0);
        let assets = liquidity_vault(&mut f, 2, 0);
        f.seed_tokens(f.mint, inventory, 2_000_000_000);
        f.seed_tokens(f.asset, assets, 20_000_000);
        f.create_route_with_vaults(
            OfferDirection::AssetToManaged,
            OfferFlow::Permissionless,
            QuoterKind::PropAmm,
            Some(inventory),
            Some(assets),
        );
        let buy = f.offer;
        let before = get_mint_supply(&f.svm, &f.mint);
        let original = f.svm.clone();
        let inventory_event = executed_event(&execute(&mut f, 2_000_000, 1).unwrap());
        assert!(inventory_event.amount_out < 990_000_000);
        assert_eq!(get_mint_supply(&f.svm, &f.mint), before);
        f.svm = original;
        edit::<OfferConfig>(&mut f.svm, buy, |o| o.source_vault = Pubkey::default());
        let mint_event = executed_event(&execute(&mut f, 2_000_000, 1).unwrap());
        assert_eq!(mint_event.amount_out, inventory_event.amount_out);
        assert_eq!(mint_event.source_amount, 0);
        assert_eq!(
            get_mint_supply(&f.svm, &f.mint),
            before + mint_event.amount_out
        );
        f.create_route_with_vaults(
            OfferDirection::ManagedToAsset,
            OfferFlow::Permissionless,
            QuoterKind::PropAmm,
            Some(assets),
            Some(inventory),
        );
        let sell = f.offer;
        let original = f.svm.clone();
        let supply = get_mint_supply(&f.svm, &f.mint);
        let retained = executed_event(&execute(&mut f, 500_000_000, 1).unwrap());
        assert_eq!(retained.destination_amount, 495_000_000);
        assert_eq!(get_mint_supply(&f.svm, &f.mint), supply);
        f.svm = original;
        edit::<OfferConfig>(&mut f.svm, sell, |o| {
            o.destination_vault = Pubkey::default()
        });
        let burned = executed_event(&execute(&mut f, 500_000_000, 1).unwrap());
        assert_eq!(burned.amount_out, retained.amount_out);
        assert_eq!(burned.destination_amount, 0);
        assert_eq!(get_mint_supply(&f.svm, &f.mint), supply - 495_000_000);
    }
}

#[test]
fn failed_source_payout_rolls_back_retained_input_fees_and_destination_creation() {
    let mut f = Fixture::new(false, true);
    let source = liquidity_vault(&mut f, 1, 0);
    let destination = liquidity_vault(&mut f, 2, 0);
    f.seed_tokens(f.mint, f.user.pubkey(), 2_000_000_000);
    f.seed_tokens(f.asset, source, 1_000_000);
    f.create_route_with_vaults(
        OfferDirection::ManagedToAsset,
        OfferFlow::Permissionless,
        QuoterKind::NavPermissionless,
        Some(source),
        Some(destination),
    );
    let supply = get_mint_supply(&f.svm, &f.mint);
    let failed = execute(&mut f, 1_000_000_000, 1).unwrap_err();
    assert_eq!(
        failed.err,
        TransactionError::InstructionError(
            1,
            InstructionError::Custom(6000 + OnreError::InsufficientBalance as u32)
        )
    );
    assert_eq!(get_mint_supply(&f.svm, &f.mint), supply);
    assert_eq!(
        f.balance(f.mint, f.user.pubkey(), f.token_program),
        2_000_000_000
    );
    assert_eq!(f.balance(f.asset, source, f.asset_program), 1_000_000);
    assert!(f
        .svm
        .get_account(&derive_ata(&destination, &f.mint, &f.token_program))
        .is_none());
    assert!(f
        .svm
        .get_account(&derive_ata(
            &vault(VaultKind::Fee),
            &f.mint,
            &f.token_program
        ))
        .is_none());
}

#[test]
fn buy_inventory_includes_buffer_accrual_into_the_selected_legacy_fee_vault() {
    let mut f = Fixture::new(true, false);
    f.seed_tokens(f.mint, f.boss.pubkey(), 10_000_000_000);
    modular_prop_amm::init_buffer(&mut f);
    let buffer = f.buffer();
    edit::<onreapp::instructions::BufferState>(&mut f.svm, buffer, |b| {
        b.gross_apr = 100_000;
        b.management_fee_basis_points = 100;
        b.previous_supply = 10_000_000_000;
    });
    let source = find_management_fee_vault_pda().0;
    f.create_route_with_vaults(
        OfferDirection::AssetToManaged,
        OfferFlow::Permissionless,
        QuoterKind::NavPermissionless,
        Some(source),
        None,
    );
    advance_clock_by(&mut f.svm, 31_536_000);
    let event = executed_event(&execute(&mut f, 100_000, 49_500_000).unwrap());
    assert_eq!(event.source_amount, 49_500_000);
    assert_eq!(f.balance(f.mint, source, f.token_program), 50_500_000);
    // Only BUFFER issues supply; transferring inventory to the user issues none.
    assert_eq!(get_mint_supply(&f.svm, &f.mint), 11_000_000_000);
    assert_eq!(
        read::<onreapp::instructions::BufferState>(&f.svm, buffer).previous_supply,
        11_000_000_000
    );
}
