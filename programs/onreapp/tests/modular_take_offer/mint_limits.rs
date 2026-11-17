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
