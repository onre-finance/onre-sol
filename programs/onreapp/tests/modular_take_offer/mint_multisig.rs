//! Modular minting through a 1-of-N SPL multisig shared with a second minter (e.g. a
//! CCIP token pool), and the boss-only switch between direct and multisig minting.
use super::*;
use crate::common::send_tx;
use solana_program::program_pack::Pack;
use spl_token_2022::{instruction::AuthorityType, state::Multisig};

pub(super) fn send(
    svm: &mut LiteSVM,
    instructions: &[Instruction],
    signers: &[&Keypair],
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    advance_slot(svm);
    send_tx(svm, instructions, signers)
}

/// Asserts a program error regardless of the failing instruction's index.
pub(super) fn expect_error(
    result: Result<TransactionMetadata, FailedTransactionMetadata>,
    expected: OnreError,
) {
    let failed = result.unwrap_err();
    assert!(
        matches!(
            failed.err,
            TransactionError::InstructionError(_, InstructionError::Custom(code))
                if code == 6000 + expected as u32
        ),
        "expected {expected:?}, got {:?}: {:?}",
        failed.err,
        failed.meta.logs
    );
}

/// Creates an initialized `m`-of-N SPL multisig owned by `program`.
pub(super) fn create_multisig(
    program: Pubkey,
    f: &mut Fixture,
    m: u8,
    signers: &[Pubkey],
) -> Pubkey {
    let key = Pubkey::new_unique();
    let lamports = f.svm.minimum_balance_for_rent_exemption(Multisig::LEN);
    f.svm
        .set_account(
            key,
            Account {
                lamports,
                data: vec![0; Multisig::LEN],
                owner: program,
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();
    let signers: Vec<&Pubkey> = signers.iter().collect();
    let instruction =
        spl_token_2022::instruction::initialize_multisig2(&program, &key, &signers, m).unwrap();
    send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
    key
}

pub(super) fn toggle_ix(f: &Fixture, boss: Pubkey, multisig: Pubkey, enable: bool) -> Instruction {
    let accounts = accounts::SetManagedTokenMintMultisig {
        managed_token: managed(f.mint),
        managed_mint: f.mint,
        mint_authority: find_mint_authority_pda().0,
        mint_multisig: multisig,
        state: find_state_pda().0,
        boss,
        token_program: f.token_program,
    };
    if enable {
        ix(accounts, instruction::EnableManagedTokenMintMultisig {})
    } else {
        ix(accounts, instruction::DisableManagedTokenMintMultisig {})
    }
}

/// The boss flow in one transaction: withdraw mint authority to the boss, hand it to
/// the multisig with SPL `set_authority`, and enable multisig minting on the program.
pub(super) fn switch_to_multisig(
    f: &mut Fixture,
    multisig: Pubkey,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let boss = f.boss.pubkey();
    let instructions = [
        build_transfer_mint_authority_to_boss_ix(&boss, &f.mint, &f.token_program),
        spl_token_2022::instruction::set_authority(
            &f.token_program,
            &f.mint,
            Some(&multisig),
            AuthorityType::MintTokens,
            &boss,
            &[],
        )
        .unwrap(),
        toggle_ix(f, boss, multisig, true),
    ];
    send(&mut f.svm, &instructions, &[&f.boss])
}

fn configured(f: &Fixture) -> Pubkey {
    read::<ManagedTokenConfig>(&f.svm, managed(f.mint)).mint_multisig
}

/// The second minter signs as its own multisig member, independently of the program.
fn second_minter_mint(
    f: &mut Fixture,
    minter: &Keypair,
    multisig: Pubkey,
    amount: u64,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let destination = derive_ata(&f.user.pubkey(), &f.mint, &f.token_program);
    let instruction = spl_token_2022::instruction::mint_to_checked(
        &f.token_program,
        &f.mint,
        &destination,
        &multisig,
        &[&minter.pubkey()],
        amount,
        9,
    )
    .unwrap();
    send(&mut f.svm, &[instruction], &[minter])
}

#[test]
fn program_and_second_minter_share_the_multisig_and_boss_can_revert() {
    for token_2022 in [false, true] {
        let mut f = Fixture::new(false, token_2022);
        let pda = find_mint_authority_pda().0;
        let ccip = Keypair::new();
        f.svm.airdrop(&ccip.pubkey(), INITIAL_LAMPORTS).unwrap();
        let multisig = create_multisig(f.token_program, &mut f, 1, &[pda, ccip.pubkey()]);

        // Direct mode rejects a supplied multisig rather than ignoring it.
        let instruction = f.take_with_mint_multisig(2_000_000, 0, Some(multisig));
        assert_error(f.send(instruction), OnreError::UnexpectedMintMultisig);

        switch_to_multisig(&mut f, multisig).unwrap();
        assert_eq!(configured(&f), multisig);
        assert_eq!(get_mint_authority_pubkey(&f.svm, &f.mint), Some(multisig));

        // The program mints through the multisig.
        let instruction = f.take(2_000_000, 990_000_000);
        f.send(instruction).unwrap();
        assert_eq!(
            f.balance(f.mint, f.user.pubkey(), f.token_program),
            990_000_000
        );

        // The configured multisig cannot be omitted or substituted.
        let instruction = f.take_with_mint_multisig(2_000_000, 0, None);
        assert_error(f.send(instruction), OnreError::MintMultisigRequired);
        let other = create_multisig(f.token_program, &mut f, 1, &[pda]);
        let instruction = f.take_with_mint_multisig(2_000_000, 0, Some(other));
        assert_error(f.send(instruction), OnreError::InvalidMintMultisig);

        // The second minter mints with only its own signature.
        second_minter_mint(&mut f, &ccip, multisig, 5).unwrap();
        assert_eq!(
            f.balance(f.mint, f.user.pubkey(), f.token_program),
            990_000_005
        );

        // Disabling reclaims mint authority to the program and restores direct minting.
        let instruction = toggle_ix(&f, f.boss.pubkey(), multisig, false);
        send(&mut f.svm, &[instruction], &[&f.boss]).unwrap();
        assert_eq!(configured(&f), Pubkey::default());
        assert_eq!(get_mint_authority_pubkey(&f.svm, &f.mint), Some(pda));
        let instruction = f.take(2_000_000, 990_000_000);
        f.send(instruction).unwrap();
        assert_eq!(
            f.balance(f.mint, f.user.pubkey(), f.token_program),
            1_980_000_005
        );
        assert!(second_minter_mint(&mut f, &ccip, multisig, 5).is_err());
    }
}

#[test]
fn enabling_requires_a_valid_one_of_n_multisig_holding_mint_authority() {
    let mut f = Fixture::new(false, false);
    let pda = find_mint_authority_pda().0;
    let boss = f.boss.pubkey();
    let other = Pubkey::new_unique();
    let enable = |f: &mut Fixture, multisig: Pubkey| {
        let instruction = toggle_ix(f, f.boss.pubkey(), multisig, true);
        let boss = f.boss.insecure_clone();
        send(&mut f.svm, &[instruction], &[&boss])
    };

    let valid = create_multisig(f.token_program, &mut f, 1, &[other, pda]);
    // Mint authority is still the program PDA.
    expect_error(enable(&mut f, valid), OnreError::ProgramNotMintAuthority);
    // Disabling while direct minting is already active is a no-op.
    let instruction = toggle_ix(&f, boss, valid, false);
    expect_error(
        send(&mut f.svm, &[instruction], &[&f.boss]),
        OnreError::NoChange,
    );

    for invalid in [
        create_multisig(f.token_program, &mut f, 2, &[other, pda]),
        create_multisig(f.token_program, &mut f, 1, &[other]),
        create_multisig(TOKEN_2022_PROGRAM_ID, &mut f, 1, &[pda]),
        Pubkey::new_unique(),
    ] {
        expect_error(enable(&mut f, invalid), OnreError::InvalidMintMultisig);
    }

    // Only the boss can switch.
    let rogue = Keypair::new();
    f.svm.airdrop(&rogue.pubkey(), INITIAL_LAMPORTS).unwrap();
    let instruction = toggle_ix(&f, rogue.pubkey(), valid, true);
    expect_error(
        send(&mut f.svm, &[instruction], &[&rogue]),
        OnreError::InvalidBoss,
    );

    switch_to_multisig(&mut f, valid).unwrap();
    expect_error(enable(&mut f, valid), OnreError::NoChange);
    // Disabling must name the configured multisig.
    let decoy = create_multisig(f.token_program, &mut f, 1, &[pda]);
    let instruction = toggle_ix(&f, boss, decoy, false);
    expect_error(
        send(&mut f.svm, &[instruction], &[&f.boss]),
        OnreError::InvalidMintMultisig,
    );
    assert_eq!(configured(&f), valid);
}
