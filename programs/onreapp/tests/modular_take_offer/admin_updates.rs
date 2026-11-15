//! Boss-only enable flags and post-creation updates of modular components.
use super::mint_multisig::{expect_error, send};
use super::*;

/// The five components with an `enabled` flag checked by `take_offer`.
const COMPONENTS: [&str; 5] = [
    "managed_token",
    "pricer",
    "quoter",
    "fee_config",
    "offer_config",
];

fn toggle_ix(f: &Fixture, component: &str, boss: Pubkey, enabled: bool) -> Instruction {
    let state = find_state_pda().0;
    match component {
        "managed_token" => ix(
            accounts::SetManagedTokenEnabled {
                managed_token: managed(f.mint),
                state,
                boss,
            },
            instruction::SetManagedTokenEnabled { enabled },
        ),
        "pricer" => ix(
            accounts::SetPricerEnabled {
                pricer: price(f.mint, 0),
                state,
                boss,
            },
            instruction::SetPricerEnabled { enabled },
        ),
        "quoter" => ix(
            accounts::SetQuoterEnabled {
                quoter: f.quoter,
                state,
                boss,
            },
            instruction::SetQuoterEnabled { enabled },
        ),
        "fee_config" => ix(
            accounts::SetFeeConfigEnabled {
                fee_config: f.fee,
                state,
                boss,
            },
            instruction::SetFeeConfigEnabled { enabled },
        ),
        _ => ix(
            accounts::SetOfferConfigEnabled {
                offer_config: f.offer,
                state,
                boss,
            },
            instruction::SetOfferConfigEnabled { enabled },
        ),
    }
}

pub(super) fn boss_send(
    f: &mut Fixture,
    instruction: Instruction,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let boss = f.boss.insecure_clone();
    send(&mut f.svm, &[instruction], &[&boss])
}

#[test]
fn boss_disables_and_reenables_each_component() {
    let mut f = Fixture::new(false, false);
    let rogue = Keypair::new();
    f.svm.airdrop(&rogue.pubkey(), INITIAL_LAMPORTS).unwrap();
    for component in COMPONENTS {
        let instruction = toggle_ix(&f, component, rogue.pubkey(), false);
        expect_error(
            send(&mut f.svm, &[instruction], &[&rogue]),
            OnreError::InvalidBoss,
        );
        let instruction = toggle_ix(&f, component, f.boss.pubkey(), true);
        expect_error(boss_send(&mut f, instruction), OnreError::NoChange);

        // Disabled: execution fails closed.
        let instruction = toggle_ix(&f, component, f.boss.pubkey(), false);
        boss_send(&mut f, instruction).unwrap();
        let instruction = f.take(1_000_000, 0);
        assert_error(f.send(instruction), OnreError::ConfigurationDisabled);

        // Re-enabled: execution works again.
        let instruction = toggle_ix(&f, component, f.boss.pubkey(), true);
        boss_send(&mut f, instruction).unwrap();
        let instruction = f.take(1_000_000, 495_000_000);
        f.send(instruction).unwrap();
    }
}
