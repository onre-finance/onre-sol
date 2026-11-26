//! Permissionless refresh of a managed token's MarketStats between trades.
use super::admin_updates::boss_send;
use super::mint_multisig::{expect_error, send};
use super::*;

pub(super) fn refresh_ix(
    f: &Fixture,
    payer: Pubkey,
    pricer: Pubkey,
    market_stats: Pubkey,
    legacy_market_stats: Option<Pubkey>,
) -> Instruction {
    ix(
        accounts::RefreshManagedTokenMarketStats {
            state: find_state_pda().0,
            managed_token: managed(f.mint),
            managed_mint: f.mint,
            pricer,
            market_stats,
            legacy_market_stats,
            supply_exclusions: exclusions(f.mint),
            payer,
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::RefreshManagedTokenMarketStats {},
    )
}

pub(super) fn per_mint_stats(f: &Fixture) -> Pubkey {
    pda(&[b"market_stats", f.mint.as_ref()])
}

fn caller(f: &mut Fixture) -> Keypair {
    let caller = Keypair::new();
    f.svm.airdrop(&caller.pubkey(), INITIAL_LAMPORTS).unwrap();
    caller
}

#[test]
fn anyone_refreshes_selected_market_stats_without_a_trade() {
    let mut f = Fixture::new(false, false);
    let anyone = caller(&mut f);
    let stats = per_mint_stats(&f);
    // Nothing selected: nothing to refresh.
    let instruction = refresh_ix(&f, anyone.pubkey(), price(f.mint, 0), stats, None);
    expect_error(
        send(&mut f.svm, &[instruction], &[&anyone]),
        OnreError::MarketStatsNotSelected,
    );

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
    let instruction = f.take(2_000_000, 990_000_000);
    f.send(instruction).unwrap();

    // A new NAV takes effect without any trade; the refresh picks it up.
    let now = get_clock_time(&f.svm);
    let instruction = ix(
        accounts::AddPricerVector {
            pricer: price(f.mint, 0),
            state: find_state_pda().0,
            boss: f.boss.pubkey(),
        },
        instruction::AddPricerVector {
            vector: PricingVector {
                start_time: now + 100,
                base_time: now + 100,
                base_price: 3_000_000_000,
                apr: 0,
                price_fix_duration: 86_400,
            },
        },
    );
    boss_send(&mut f, instruction).unwrap();
    advance_clock_by(&mut f.svm, 100);
    let instruction = refresh_ix(&f, anyone.pubkey(), price(f.mint, 0), stats, None);
    send(&mut f.svm, &[instruction], &[&anyone]).unwrap();
    let report: onreapp::state::MarketStats = read(&f.svm, stats);
    assert_eq!(report.nav, 3_000_000_000);
    assert_eq!(report.circulating_supply, 990_000_000);
    assert_eq!(report.tvl, 2_970_000_000);
    assert_eq!(report.last_updated_at, get_clock_time(&f.svm) as i64);

    // Only the selected account and the canonical Pricer are accepted.
    let instruction = refresh_ix(
        &f,
        anyone.pubkey(),
        price(f.mint, 0),
        find_market_stats_pda().0,
        None,
    );
    expect_error(
        send(&mut f.svm, &[instruction], &[&anyone]),
        OnreError::InvalidMarketStatsPda,
    );
    let other = price(f.mint, 1);
    let instruction = ix(
        accounts::CreatePricer {
            managed_token: managed(f.mint),
            pricer: other,
            state: find_state_pda().0,
            boss: f.boss.pubkey(),
            system_program: SYSTEM_PROGRAM_ID,
        },
        instruction::CreatePricer {
            instance_id: 1,
            denomination: PricingDenomination::Usd,
        },
    );
    boss_send(&mut f, instruction).unwrap();
    let instruction = refresh_ix(&f, anyone.pubkey(), other, stats, None);
    expect_error(
        send(&mut f.svm, &[instruction], &[&anyone]),
        OnreError::IncompatiblePricer,
    );

    // A disabled managed token is not reported.
    let instruction = ix(
        accounts::SetManagedTokenEnabled {
            managed_token: managed(f.mint),
            state: find_state_pda().0,
            boss: f.boss.pubkey(),
        },
        instruction::SetManagedTokenEnabled { enabled: false },
    );
    boss_send(&mut f, instruction).unwrap();
    let instruction = refresh_ix(&f, anyone.pubkey(), price(f.mint, 0), stats, None);
    expect_error(
        send(&mut f.svm, &[instruction], &[&anyone]),
        OnreError::ConfigurationDisabled,
    );
}

#[test]
fn onyc_refresh_creates_and_refreshes_the_legacy_singleton() {
    let mut f = Fixture::new(true, false);
    let anyone = caller(&mut f);
    let legacy = find_market_stats_pda().0;
    let instruction = refresh_ix(&f, anyone.pubkey(), price(f.mint, 0), legacy, None);
    expect_error(
        send(&mut f.svm, &[instruction], &[&anyone]),
        OnreError::LegacyMarketStatsRequired,
    );
    assert!(f.svm.get_account(&legacy).is_none());

    let instruction = refresh_ix(&f, anyone.pubkey(), price(f.mint, 0), legacy, Some(legacy));
    send(&mut f.svm, &[instruction], &[&anyone]).unwrap();
    let report = read_market_stats(&f.svm);
    assert_eq!(report.nav, 2_000_000_000);
    assert_eq!(report.circulating_supply, get_mint_supply(&f.svm, &f.mint));
    assert_eq!(report.last_updated_at, get_clock_time(&f.svm) as i64);
}
