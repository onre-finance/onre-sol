//! Boss-only per-mint issuance limits and their enforcement on modular mints.
use super::mint_multisig::{expect_error, send};
use super::*;

fn limits_ix(f: &Fixture, boss: Pubkey, max_supply: u64, max_mint_amount: u64) -> Instruction {
    ix(
        accounts::SetManagedTokenMintLimits {
            managed_token: managed(f.mint),
            managed_mint: f.mint,
            state: find_state_pda().0,
            boss,
        },
        instruction::SetManagedTokenMintLimits {
            max_supply,
            max_mint_amount,
        },
    )
}

fn set_limits(
    f: &mut Fixture,
    max_supply: u64,
    max_mint_amount: u64,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let instruction = limits_ix(f, f.boss.pubkey(), max_supply, max_mint_amount);
    let boss = f.boss.insecure_clone();
    send(&mut f.svm, &[instruction], &[&boss])
}

#[test]
fn boss_sets_per_mint_limits_that_bind_offer_mints() {
    let mut f = Fixture::new(false, false);
    let instruction = f.take(2_000_000, 990_000_000);
    f.send(instruction).unwrap();
    let supply = get_mint_supply(&f.svm, &f.mint);
    assert_eq!(supply, 990_000_000);

    // Only the boss can set limits, and identical limits are rejected.
    let rogue = Keypair::new();
    f.svm.airdrop(&rogue.pubkey(), INITIAL_LAMPORTS).unwrap();
    let instruction = limits_ix(&f, rogue.pubkey(), 0, 1);
    expect_error(
        send(&mut f.svm, &[instruction], &[&rogue]),
        OnreError::InvalidBoss,
    );
    expect_error(set_limits(&mut f, 0, 0), OnreError::NoChange);

    // A nonzero max supply cannot undercut the current supply but may equal it.
    expect_error(
        set_limits(&mut f, supply - 1, 0),
        OnreError::MaxSupplyBelowCurrentSupply,
    );
    set_limits(&mut f, supply, 0).unwrap();
    let config: ManagedTokenConfig = read(&f.svm, managed(f.mint));
    assert_eq!((config.max_supply, config.max_mint_amount), (supply, 0));
    let instruction = f.take(2_000_000, 0);
    assert_error(f.send(instruction), OnreError::MaxSupplyExceeded);

    // The per-mint amount cap binds each mint independently of the supply cap.
    set_limits(&mut f, 0, 989_999_999).unwrap();
    let instruction = f.take(2_000_000, 0);
    assert_error(f.send(instruction), OnreError::MaxMintAmountExceeded);
    set_limits(&mut f, 0, 990_000_000).unwrap();
    let instruction = f.take(2_000_000, 990_000_000);
    f.send(instruction).unwrap();

    // Zero disables both limits.
    set_limits(&mut f, 0, 0).unwrap();
    let instruction = f.take(2_000_000, 990_000_000);
    f.send(instruction).unwrap();
    assert_eq!(get_mint_supply(&f.svm, &f.mint), 3 * 990_000_000);
}

#[test]
fn onyc_modular_mints_use_per_mint_limits_and_legacy_mints_global_limits() {
    let mut f = Fixture::new(true, false);
    f.initialize_legacy_main_offer(); // legacy asset -> ONyc offer at NAV 1.0, no fee
    let instruction = f.take(2_000_000, 0);
    f.send(instruction).unwrap();
    let supply = get_mint_supply(&f.svm, &f.mint);
    assert_eq!(supply, 990_000_000);
    let boss = f.boss.insecure_clone();
    let authority = find_offer_vault_authority_pda().0;
    create_token_account(&mut f.svm, &f.asset, &authority, 0);
    create_token_account(&mut f.svm, &f.mint, &authority, 0);
    let legacy_take = |f: &Fixture, amount: u64| {
        build_take_offer_v2_ix(
            &f.user.pubkey(),
            &f.boss.pubkey(),
            &f.asset,
            &f.mint,
            amount,
            None,
            &TOKEN_PROGRAM_ID,
            &TOKEN_PROGRAM_ID,
        )
    };

    // The global State cap binds only the legacy ONyc instructions.
    let instruction = build_configure_max_supply_ix(&boss.pubkey(), supply);
    send(&mut f.svm, &[instruction], &[&boss]).unwrap();
    let instruction = legacy_take(&f, 2_000_000);
    expect_error(
        send(&mut f.svm, &[instruction], &[&f.user]),
        OnreError::MaxSupplyExceeded,
    );
    let instruction = f.take(2_000_000, 990_000_000);
    f.send(instruction).unwrap();
    let supply = get_mint_supply(&f.svm, &f.mint);
    assert_eq!(supply, 1_980_000_000);

    // The per-mint cap binds only the modular instructions.
    let instruction = build_configure_max_supply_ix(&boss.pubkey(), 0);
    send(&mut f.svm, &[instruction], &[&boss]).unwrap();
    set_limits(&mut f, supply, 0).unwrap();
    let instruction = f.take(2_000_001, 0);
    assert_error(f.send(instruction), OnreError::MaxSupplyExceeded);
    let instruction = legacy_take(&f, 2_000_001);
    send(&mut f.svm, &[instruction], &[&f.user]).unwrap();
    assert_eq!(get_mint_supply(&f.svm, &f.mint), supply + 2_000_001_000);
}

#[test]
fn zero_buffer_accrual_does_not_block_trades_above_the_cap() {
    let mut f = Fixture::new(false, false);
    let instruction = f.take(2_000_000, 990_000_000);
    f.send(instruction).unwrap();
    f.initialize_managed_buffer(); // gross APR 0: every accrual mints 0
    f.create_route(
        OfferDirection::ManagedToAsset,
        OfferMode::Permissioned,
        QuoterKind::Nav,
        true,
    );
    f.seed_tokens(f.asset, vault(VaultKind::Liquidity), 5_000_000);
    let cap = get_mint_supply(&f.svm, &f.mint);
    set_limits(&mut f, cap, 0).unwrap();

    // Supply rises above the cap through a mint authority the cap cannot bind.
    f.seed_tokens(f.mint, Pubkey::new_unique(), 1);
    assert_eq!(get_mint_supply(&f.svm, &f.mint), cap + 1);

    // A sell still works: the BUFFER accrual mints nothing, so no cap applies.
    let instruction = f.take(100_000_000, 0);
    f.send(instruction).unwrap();
    assert_eq!(get_mint_supply(&f.svm, &f.mint), cap + 1 - 99_000_000);
}
