use super::*;

fn permission(offer: Pubkey, user: Pubkey) -> Pubkey {
    pda(&[b"offer_permission", offer.as_ref(), user.as_ref()])
}

fn grant(
    f: &mut Fixture,
    offer: Pubkey,
    user: Pubkey,
    signer: &Keypair,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let instruction = ix(
        accounts::GrantOfferPermission {
            offer_config: offer,
            permission: permission(offer, user),
            state: find_state_pda().0,
            boss: signer.pubkey(),
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::GrantOfferPermission { user },
    );
    send_tx(&mut f.svm, &[instruction], &[signer])
}

/// `take_offer` on the selected offer, with `permission` as the remaining account.
fn take(f: &Fixture, permission: Option<Pubkey>) -> Instruction {
    let mut instruction = f.take(GROSS, 0);
    instruction
        .accounts
        .extend(permission.map(|key| AccountMeta::new_readonly(key, false)));
    instruction
}

/// Fails `PermissionRequired` at `take_offer`, which follows the compute budget.
fn assert_denied(result: Result<TransactionMetadata, FailedTransactionMetadata>) {
    let failure = result.unwrap_err();
    assert_eq!(
        failure.err,
        TransactionError::InstructionError(
            1,
            InstructionError::Custom(6000 + OnreError::PermissionRequired as u32)
        ),
        "{:?}",
        failure.meta.logs
    );
}

const GROSS: u64 = 100_000_000;

#[test]
fn allowlisted_offer_requires_the_users_permission_pda_and_charges_only_the_base_fee() {
    let mut f = Fixture::new(false, false);
    let instruction = f.take(2_000_000, 990_000_000);
    f.send(instruction).unwrap();
    f.create_route(
        OfferDirection::ManagedToAsset,
        OfferMode::Permissioned,
        QuoterKind::Nav,
        true,
    );
    f.seed_tokens(f.asset, vault(VaultKind::Liquidity), 2_000_000);
    let permissioned = f.offer;

    // Reference: the same NAV sell through today's approval-gated offer.
    let instruction = f.take(GROSS, 0);
    let reference = f.send(instruction).unwrap();
    let reference_event = executed_event(&reference);

    // An allowlisted twin of that offer: same Pricer, quoter, fee and custody.
    let config: OfferConfig = read(&f.svm, permissioned);
    let offer = offer_pda(f.mint, f.asset, 9);
    let instruction = ix(
        accounts::CreateOfferConfig {
            token_in_mint: f.mint,
            token_out_mint: f.asset,
            managed_token: managed(f.mint),
            asset_managed_registration: managed(f.asset),
            pricer: config.pricer,
            quoter: config.quoter,
            prop_amm_state: None,
            fee_config: config.fee_config,
            proceeds_vault: config.proceeds_vault,
            source_vault: Some(config.source_vault),
            destination_vault: None,
            haircut_vault: None,
            offer_config: offer,
            state: find_state_pda().0,
            boss: f.boss.pubkey(),
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::CreateOfferConfig {
            instance_id: 9,
            access: OfferAccess::Allowlisted,
            execution: OfferExecution::Instant,
        },
    );
    let boss = f.boss.insecure_clone();
    send_tx(&mut f.svm, &[instruction], &[&boss]).unwrap();
    f.offer = offer;
    let user = f.user.pubkey();
    let own = permission(offer, user);

    // Without the permission account.
    let instruction = take(&f, None);
    assert_denied(f.send_without_approval(instruction));

    // A valid KYC approval does not open an allowlisted offer.
    let mut instruction = take(&f, None);
    instruction.data = instruction::TakeOffer {
        gross_input_amount: GROSS,
        minimum_amount_out: 0,
        deadline: get_clock_time(&f.svm) + 100,
        approval_message: Some(f.approval()),
    }
    .data();
    assert_error(f.send(instruction), OnreError::PermissionRequired);

    // The canonical address before any grant.
    let instruction = take(&f, Some(own));
    assert_denied(f.send_without_approval(instruction));

    // Only the boss grants.
    let stranger = f.user.insecure_clone();
    assert!(grant(&mut f, offer, user, &stranger).is_err());
    assert!(f.svm.get_account(&own).is_none());

    // Another user's permission, and this user's permission for another offer.
    grant(&mut f, offer, boss.pubkey(), &boss).unwrap();
    grant(&mut f, permissioned, user, &boss).unwrap();
    for other in [
        permission(offer, boss.pubkey()),
        permission(permissioned, user),
    ] {
        let instruction = take(&f, Some(other));
        assert_denied(f.send_without_approval(instruction));
    }

    // Granted: executes with no approval and no web2 call, at NAV less the base fee.
    grant(&mut f, offer, user, &boss).unwrap();
    let instruction = take(&f, Some(own));
    let result = f.send_without_approval(instruction).unwrap();
    let event = executed_event(&result);
    assert_eq!(event.access, OfferAccess::Allowlisted);
    assert_eq!(event.fee_amount, GROSS / 100);
    assert_eq!(event.fee_amount, reference_event.fee_amount);
    assert_eq!(event.amount_out, reference_event.amount_out);
    assert_eq!(event.haircut_amount, 0);
    // Revoked by closing: the same instruction fails again.
    let instruction = ix(
        accounts::RevokeOfferPermission {
            permission: own,
            state: find_state_pda().0,
            boss: boss.pubkey(),
        },
        instruction::RevokeOfferPermission {},
    );
    send_tx(&mut f.svm, &[instruction], &[&boss]).unwrap();
    let instruction = take(&f, Some(own));
    assert_denied(f.send_without_approval(instruction));

    // Existing integrations are untouched: the unchanged account list still
    // executes the approval-gated offer.
    f.offer = permissioned;
    let instruction = take(&f, None);
    let approved = f.send(instruction).unwrap();
    // Both ran with every token account already created.
    println!(
        "compute units: approval {} / permission {}",
        approved.compute_units_consumed, result.compute_units_consumed
    );
}
