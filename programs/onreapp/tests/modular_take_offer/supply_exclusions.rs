//! Per-mint circulating-supply exclusions: a boss-set token-account list whose
//! summed balance is cached on set and on permissionless refresh, and read by
//! modular market stats, liquidity targets and burn for NAV.
use super::admin_updates::boss_send;
use super::market_stats_refresh::{per_mint_stats, refresh_ix};
use super::mint_multisig::{expect_error, send};
use super::*;
use onreapp::state::CirculatingSupplyExcludedBalance;

fn refresh_exclusions_ix(f: &Fixture, payer: Pubkey, accounts: &[Pubkey]) -> Instruction {
    let onyc = read_state(&f.svm).onyc_mint == f.mint;
    let mut instruction = ix(
        accounts::RefreshManagedTokenSupplyExclusions {
            state: find_state_pda().0,
            managed_mint: f.mint,
            supply_exclusions: exclusions(f.mint),
            legacy_excluded_balance: onyc.then(|| find_circulating_supply_excluded_balance_pda().0),
            payer,
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::RefreshManagedTokenSupplyExclusions {},
    );
    instruction.accounts.extend(
        accounts
            .iter()
            .map(|key| AccountMeta::new_readonly(*key, false)),
    );
    instruction
}

fn set(
    f: &mut Fixture,
    accounts: &[Pubkey],
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let instruction = set_exclusions_ix(&f.svm, f.boss.pubkey(), f.mint, accounts);
    boss_send(f, instruction)
}

fn holder(f: &mut Fixture, amount: u64) -> (Pubkey, Pubkey) {
    let owner = Pubkey::new_unique();
    f.seed_tokens(f.mint, owner, amount);
    (owner, derive_ata(&owner, &f.mint, &f.token_program))
}

fn cached(f: &Fixture) -> ManagedTokenSupplyExclusions {
    read(&f.svm, exclusions(f.mint))
}

fn anyone(f: &mut Fixture) -> Keypair {
    let caller = Keypair::new();
    f.svm.airdrop(&caller.pubkey(), INITIAL_LAMPORTS).unwrap();
    caller
}

#[test]
fn boss_sets_validated_token_accounts_and_the_amount_is_cached_immediately() {
    for token_2022 in [false, true] {
        let mut f = Fixture::new(false, token_2022);
        let (_, a) = holder(&mut f, 3_000_000_000);
        let (_, b) = holder(&mut f, 2_000_000_000);
        // A mint without exclusions excludes nothing and has no account.
        assert!(f.svm.get_account(&exclusions(f.mint)).is_none());

        let rogue = anyone(&mut f);
        let instruction = set_exclusions_ix(&f.svm, rogue.pubkey(), f.mint, &[a]);
        expect_error(
            send(&mut f.svm, &[instruction], &[&rogue]),
            OnreError::InvalidBoss,
        );
        expect_error(
            set(&mut f, &[a, b, a]),
            OnreError::DuplicateExcludedTokenAccount,
        );
        let many: Vec<Pubkey> = (0..21).map(|_| Pubkey::new_unique()).collect();
        expect_error(set(&mut f, &many), OnreError::TooManyExcludedTokenAccounts);
        // A wallet, a missing account, or another mint's token account is rejected.
        let wallet = f.boss.pubkey();
        expect_error(
            set(&mut f, &[a, wallet]),
            OnreError::InvalidExcludedTokenAccount,
        );
        let missing = Pubkey::new_unique();
        expect_error(
            set(&mut f, &[missing]),
            OnreError::InvalidExcludedTokenAccount,
        );
        let asset_owner = Pubkey::new_unique();
        f.seed_tokens(f.asset, asset_owner, 1);
        let asset_account = derive_ata(&asset_owner, &f.asset, &f.asset_program);
        expect_error(
            set(&mut f, &[asset_account]),
            OnreError::InvalidExcludedTokenAccount,
        );

        set(&mut f, &[a, b]).unwrap();
        let stored = cached(&f);
        assert_eq!(stored.managed_mint, f.mint);
        assert_eq!(stored.active(), &[a, b]);
        assert_eq!(stored.excluded_amount, 5_000_000_000);
        assert_eq!(stored.last_updated_at, get_clock_time(&f.svm) as i64);
        // Only ONyc mirrors into the legacy cache.
        assert!(f
            .svm
            .get_account(&find_circulating_supply_excluded_balance_pda().0)
            .is_none());

        // Replacing shortens the list and clears stale entries; empty clears all.
        set(&mut f, &[b]).unwrap();
        let stored = cached(&f);
        assert_eq!(stored.active(), &[b]);
        assert_eq!(stored.token_accounts[1], Pubkey::default());
        assert_eq!(stored.excluded_amount, 2_000_000_000);
        set(&mut f, &[]).unwrap();
        assert_eq!(cached(&f).count, 0);
        assert_eq!(cached(&f).excluded_amount, 0);
    }
}

#[test]
fn anyone_refreshes_with_exactly_the_stored_accounts_in_order() {
    let mut f = Fixture::new(false, false);
    let (a_owner, a) = holder(&mut f, 3_000_000_000);
    let (_, b) = holder(&mut f, 2_000_000_000);
    set(&mut f, &[a, b]).unwrap();
    let caller = anyone(&mut f);

    // Balances move; the cache keeps the set-time amount until a refresh.
    f.seed_tokens(f.mint, a_owner, 7_000_000_000);
    assert_eq!(cached(&f).excluded_amount, 5_000_000_000);

    for (accounts, error) in [
        (vec![a], OnreError::MissingExcludedTokenAccount),
        (
            vec![a, b, Pubkey::new_unique()],
            OnreError::TooManyExcludedTokenAccounts,
        ),
        (vec![b, a], OnreError::InvalidExcludedTokenAccount),
    ] {
        let instruction = refresh_exclusions_ix(&f, caller.pubkey(), &accounts);
        expect_error(send(&mut f.svm, &[instruction], &[&caller]), error);
    }
    let set_slot = cached(&f).last_updated_slot;
    let instruction = refresh_exclusions_ix(&f, caller.pubkey(), &[a, b]);
    send(&mut f.svm, &[instruction], &[&caller]).unwrap();
    assert_eq!(cached(&f).excluded_amount, 9_000_000_000);
    assert!(cached(&f).last_updated_slot > set_slot);

    // A closed excluded account counts as zero instead of blocking refreshes.
    f.svm.set_account(b, Account::default()).unwrap();
    let instruction = refresh_exclusions_ix(&f, caller.pubkey(), &[a, b]);
    send(&mut f.svm, &[instruction], &[&caller]).unwrap();
    assert_eq!(cached(&f).excluded_amount, 7_000_000_000);
}

#[test]
fn modular_market_stats_subtract_the_cached_amount() {
    let mut f = Fixture::new(false, false);
    let caller = anyone(&mut f);
    let stats = per_mint_stats(&f);
    let instruction = ix(
        accounts::InitializeManagedTokenMarketStats {
            managed_token: managed(f.mint),
            market_stats: stats,
            state: find_state_pda().0,
            boss: f.boss.pubkey(),
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::InitializeManagedTokenMarketStats {},
    );
    boss_send(&mut f, instruction).unwrap();
    let (owner, account) = holder(&mut f, 4_000_000_000);
    let instruction = refresh_ix(&f, caller.pubkey(), price(f.mint, 0), stats, None);
    send(&mut f.svm, &[instruction], &[&caller]).unwrap();
    let report: onreapp::state::MarketStats = read(&f.svm, stats);
    assert_eq!(report.circulating_supply, 4_000_000_000);

    set(&mut f, &[account]).unwrap();
    // Seeding replaces the holder's balance with 1 and raises supply to 5.
    f.seed_tokens(f.mint, owner, 1_000_000_000);
    assert_eq!(get_mint_supply(&f.svm, &f.mint), 5_000_000_000);
    // Stats subtract the cached 4 until the exclusions are refreshed.
    let instruction = refresh_ix(&f, caller.pubkey(), price(f.mint, 0), stats, None);
    send(&mut f.svm, &[instruction], &[&caller]).unwrap();
    let report: onreapp::state::MarketStats = read(&f.svm, stats);
    assert_eq!(report.circulating_supply, 1_000_000_000);
    // Refreshing both in one transaction reports current balances.
    let instructions = [
        refresh_exclusions_ix(&f, caller.pubkey(), &[account]),
        refresh_ix(&f, caller.pubkey(), price(f.mint, 0), stats, None),
    ];
    send(&mut f.svm, &instructions, &[&caller]).unwrap();
    let report: onreapp::state::MarketStats = read(&f.svm, stats);
    assert_eq!(report.circulating_supply, 4_000_000_000);
    assert_eq!(report.tvl, 8_000_000_000);

    // A stale cache above current supply (excluded tokens burned since the last
    // refresh) floors circulating supply at zero instead of failing.
    edit::<ManagedTokenSupplyExclusions>(&mut f.svm, exclusions(f.mint), |e| {
        e.excluded_amount = u64::MAX
    });
    let instruction = refresh_ix(&f, caller.pubkey(), price(f.mint, 0), stats, None);
    send(&mut f.svm, &[instruction], &[&caller]).unwrap();
    let report: onreapp::state::MarketStats = read(&f.svm, stats);
    assert_eq!(report.circulating_supply, 0);
    assert_eq!(report.tvl, 0);
}

#[test]
fn onyc_requires_its_exclusions_and_mirrors_them_into_the_legacy_cache() {
    let mut f = Fixture::new(true, false);
    let caller = anyone(&mut f);
    let legacy_stats = find_market_stats_pda().0;
    let legacy_cache = find_circulating_supply_excluded_balance_pda().0;
    let (owner, account) = holder(&mut f, 2_000_000_000);

    // ONyc's set and refresh must also write the legacy cache.
    let mut instruction = set_exclusions_ix(&f.svm, f.boss.pubkey(), f.mint, &[account]);
    instruction.accounts[3] = AccountMeta::new_readonly(PROGRAM_ID, false);
    expect_error(
        boss_send(&mut f, instruction),
        OnreError::InvalidCirculatingSupplyExcludedBalance,
    );
    set(&mut f, &[account]).unwrap();
    let mirrored: CirculatingSupplyExcludedBalance = read(&f.svm, legacy_cache);
    assert_eq!(mirrored.amount, 2_000_000_000);
    assert_eq!(cached(&f).excluded_amount, 2_000_000_000);

    f.seed_tokens(f.mint, owner, 500_000_000);
    let instruction = refresh_exclusions_ix(&f, caller.pubkey(), &[account]);
    send(&mut f.svm, &[instruction], &[&caller]).unwrap();
    let mirrored: CirculatingSupplyExcludedBalance = read(&f.svm, legacy_cache);
    assert_eq!(mirrored.amount, 500_000_000);
    assert_eq!(mirrored.last_updated_at, cached(&f).last_updated_at);

    // The legacy singleton written by the modular refresh uses the same amount.
    let instruction = refresh_ix(
        &f,
        caller.pubkey(),
        price(f.mint, 0),
        legacy_stats,
        Some(legacy_stats),
    );
    send(&mut f.svm, &[instruction], &[&caller]).unwrap();
    assert_eq!(
        read_market_stats(&f.svm).circulating_supply,
        get_mint_supply(&f.svm, &f.mint) - 500_000_000
    );

    // Without its exclusions account ONyc's modular reporting fails instead of
    // writing unexcluded supply into the legacy singleton.
    f.svm
        .set_account(exclusions(f.mint), Account::default())
        .unwrap();
    let instruction = refresh_ix(
        &f,
        caller.pubkey(),
        price(f.mint, 0),
        legacy_stats,
        Some(legacy_stats),
    );
    expect_error(
        send(&mut f.svm, &[instruction], &[&caller]),
        OnreError::SupplyExclusionsRequired,
    );
}
