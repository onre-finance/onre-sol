// SCOUT:TESTS:BEGIN
#[cfg(test)]
mod scout_reachability {
    use super::*;

    /// setup() must build a world in which the whole value flow is live. If this regresses,
    /// every coverage number below it is measuring a broken harness rather than the program.
    #[test]
    fn t_core_value_flow_is_reachable() {
        let mut f = OnreappFixture::setup();
        assert!(f.action_take_offer(10_000_000), "take_offer");
        assert!(f.action_take_offer_permissionless(5_000_000), "take_offer_permissionless");
        assert!(f.action_create_redemption_request(1_000_000), "create_redemption_request");
        assert!(f.action_fulfill_redemption_request(), "fulfill_redemption_request");
        assert!(f.action_create_redemption_request(2_000_001), "create_redemption_request #2");
        assert!(f.action_cancel_redemption_request(), "cancel_redemption_request");
    }

    /// P-0003's regression, preserved deterministically now that its block is retired.
    ///
    /// `RedemptionOffer.requested_redemptions` must equal the sum of `amount` over the requests
    /// that still exist, across every path that retires one. P-0003 survived 3.4M+ fuzz
    /// executions; this pins the same statement so retiring its block does not silently drop it.
    ///
    /// Uses the plain-SPL forward offer, where deposit == credit, so any drift here is the
    /// program's own accounting rather than a transfer fee (that is P-0008's subject).
    #[test]
    fn t_p0003_requested_redemptions_tracks_open_requests() {
        let mut f = OnreappFixture::setup();

        fn check(f: &OnreappFixture, label: &str) -> u128 {
            let recorded = f
                .onchain_requested_redemptions()
                .expect("requested_redemptions readable");
            let summed: u128 = f
                .open_requests_of(&f.redemption_offer_pda)
                .expect("requests enumerable")
                .iter()
                .map(|(_, _, a)| *a as u128)
                .sum();
            assert_eq!(recorded, summed, "P-0003 drift after {}", label);
            recorded
        }

        assert_eq!(check(&f, "setup"), 0);

        assert!(f.action_create_redemption_request(1_000_000), "create #1");
        assert_eq!(check(&f, "create #1"), 1_000_000);

        assert!(f.action_create_redemption_request(2_500_000), "create #2");
        assert_eq!(check(&f, "create #2"), 3_500_000);

        // programV5 fulfils PARTIALLY: the harness's first bite is half the remainder, which
        // decrements the counter without retiring the request. The statement under test is
        // unchanged -- the counter must equal the sum of what is still owed - but it now has to
        // hold mid-request, which is the case the partial path added.
        assert!(f.action_fulfill_redemption_request(), "fulfil oldest (partial)");
        assert_eq!(check(&f, "partial fulfil"), 3_000_000);

        // The next bite settles #1 outright and closes it.
        assert!(f.action_fulfill_redemption_request(), "fulfil oldest (settle)");
        assert_eq!(check(&f, "full fulfil"), 2_500_000);

        // Cancellation is the other retirement path.
        assert!(f.action_cancel_redemption_request(), "cancel remaining");
        assert_eq!(check(&f, "cancel"), 0);
    }

    /// The transfer-fee fixture must still be fee-bearing, and the OFFER path must still refuse it.
    ///
    /// `mint_fee` is the harness's only Token-2022 mint with a live `TransferFeeConfig`. If it ever
    /// stops being fee-bearing, every fee-related result below goes quietly green for a reason that
    /// has nothing to do with the program — so the fee is asserted directly, by transferring
    /// through it and measuring the shortfall.
    #[test]
    fn t_fee_mint_fixture_is_live_and_the_offer_path_refuses_it() {
        let mut f = OnreappFixture::setup();

        // The offer usdc -> fee exists and `take_offer` must refuse it, because token_out is
        // fee-bearing (`token_utils.rs:374,378`). This is the control that proves the fee is live.
        assert!(
            !f.action_scout_take_fee_offer(1_000_000),
            "take_offer MUST refuse a fee-bearing leg (token_utils.rs:374,378)"
        );

        // And the fee is really configured: the extension is present on the mint account.
        let data = f.ctx.account_data(&f.mint_fee).expect("mint_fee exists");
        assert!(
            data.len() > 165,
            "mint_fee must carry Token-2022 extension data (len {})",
            data.len()
        );
    }

    /// programV5 closed P-0008 by construction, and this pins the closure.
    ///
    /// P-0008 was: a redemption offer whose token_in charges a transfer fee credits the vault NET
    /// while `create_redemption_request` records GROSS, so the pooled vault goes insolvent through
    /// honest use. programV5 makes such an offer unconstructible — `make_redemption_offer.rs:66-72`
    /// requires `token_in_mint == state.onyc_mint` AND `token_in_mint.owner == spl_token::ID`, so
    /// neither a fee-bearing mint nor any Token-2022 mint can be a redemption token_in. Both halves
    /// are asserted: if either constraint is relaxed the shortfall path is reachable again.
    #[test]
    fn t_p0008_fee_bearing_redemption_offer_is_unconstructible() {
        let mut f = OnreappFixture::setup();
        let boss = f.boss.insecure_clone();
        let rva = f.redemption_vault_authority;
        let (mint_fee, mint_usdc) = (f.mint_fee, f.mint_usdc);
        let pid = f.program_id;
        let state_pda = f.state_pda;
        let offer_fee_pda = f.offer_fee_pda;
        let ro = scout_pda(
            &[SEED_REDEMPTION_OFFER, mint_fee.as_ref(), mint_usdc.as_ref()],
            &pid,
        );
        let refused = f
            .ctx
            .program(pid)
            .call(instruction::MakeRedemptionOffer {
                fee_basis_points: 50,
                fee_basis_points_prop_amm_sell: 50,
            })
            .accounts(accounts::MakeRedemptionOffer {
                state: state_pda,
                offer: offer_fee_pda,
                redemption_vault_authority: rva,
                token_in_mint: mint_fee,
                token_in_program: SPL_TOKEN_2022_ID,
                vault_token_in_account: scout_ata(&rva, &mint_fee, &SPL_TOKEN_2022_ID),
                token_out_mint: mint_usdc,
                token_out_program: SPL_TOKEN_ID,
                vault_token_out_account: scout_ata(&rva, &mint_usdc, &SPL_TOKEN_ID),
                redemption_offer: ro,
                boss: boss.pubkey(),
            })
            .signers(&[&boss])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        assert!(!refused, "a fee-bearing Token-2022 token_in redemption offer must be refused");
        assert!(
            f.ctx.get_account(&ro).is_err(),
            "and no redemption offer account may have been created"
        );
    }

    /// Every hand-computed borsh offset this harness reads, checked against the live accounts.
    ///
    /// The properties read `BufferState`, `MarketStats` and the excluded-balance cache by byte
    /// offset, because the invariant predicate grammar cannot call a deserializer. An offset that
    /// silently drifts (a field inserted upstream) would make those properties compare the wrong
    /// bytes and go quiet — the worst failure mode there is. This pins them against values the
    /// program itself wrote.
    #[test]
    fn t_v5_state_layout_offsets() {
        let mut f = OnreappFixture::setup();

        assert!(f.action_settle_buffer(), "settle_buffer");
        let buf = f.ctx.account_data(&f.buffer_state_pda()).expect("buffer state");
        assert_eq!(
            Pubkey::new_from_array(buf[8..40].try_into().unwrap()),
            f.mint_onyc,
            "BufferState.onyc_mint at 8..40"
        );
        assert_eq!(buf[SCOUT_BUF_HWM_ENABLED], 1, "high-watermark gate defaults to enabled");
        assert_eq!(
            f.buffer_previous_supply(),
            f.onyc_supply(),
            "after a settle the baseline IS the live supply"
        );
        assert_eq!(
            f.buffer_last_accrual(),
            Some(scout_now(&f.ctx) as i64),
            "last_accrual_timestamp is stamped from the clock"
        );

        assert!(f.action_refresh_market_stats(), "refresh_market_stats");
        let circulating = f.market_stats_circulating().expect("circulating");
        let nav = f.market_stats_nav().expect("nav");
        let tvl = f.market_stats_tvl().expect("tvl");
        assert_eq!(
            circulating,
            f.onyc_supply().unwrap() - f.excluded_balance_cached().unwrap_or(0),
            "MarketStats.circulating_supply = supply - cached excluded"
        );
        assert!(nav > 0, "nav must be priced");
        assert_eq!(
            tvl,
            ((circulating as u128) * (nav as u128) / 1_000_000_000u128) as u64,
            "MarketStats.tvl = circulating * nav / 1e9"
        );

        assert!(f.action_update_circulating_supply_excluded_balance(), "update excluded balance");
        assert_eq!(
            f.excluded_balance_cached(),
            Some(f.excluded_balance_live()),
            "the cache equals the live excluded holdings right after an update"
        );
    }

    /// P-0009, measured end to end: a permissionless burn leaves the BUFFER charging yield on
    /// tokens that no longer exist.
    ///
    /// `docs/BUFFER_ACCRUAL.md` promises that after any supply-changing operation the next baseline
    /// IS the post-change supply, then exempts the legacy takes from doing it. This runs the
    /// exemption: settle so the baseline is exact, burn ONyc through the legacy `take_offer` (any
    /// user, no privilege, no admin), advance a day, and settle again.
    ///
    /// The accrual is LINEAR in the baseline —
    /// `mint = previous_supply * apr_delta * seconds / denominator`
    /// (`accrual_utils.rs:14-40`) — so the correct mint for the same interval is exactly
    /// `minted * live_supply / stale_baseline`, and the excess is
    /// `minted * (baseline - live_supply) / baseline`. That excess is ONyc minted to the reserve
    /// and fee vaults against supply that was already burned: unbacked, and a dilution of every
    /// holder. It scales with the burn and with the time until the next accrual, so a large burn
    /// left unsettled for a month mints a month of yield on nothing.
    #[test]
    fn t_p0009_legacy_burn_leaves_the_buffer_minting_on_burned_supply() {
        let mut f = OnreappFixture::setup();
        assert!(f.action_offer_vault_deposit(50_000_000), "fund the offer vault");
        assert!(f.action_set_buffer_gross_apr(200_000), "20% gross APR");
        assert!(f.action_settle_buffer(), "settle to set the baseline");
        assert_eq!(
            f.buffer_previous_supply(),
            f.onyc_supply(),
            "the baseline starts exact, as the docs promise"
        );

        // A permissionless burn through the exempted path.
        assert!(f.action_scout_take_offer_rev(1_000_000_000, 0), "legacy take burns ONyc");
        let supply = f.onyc_supply().unwrap();
        let baseline = f.buffer_previous_supply().unwrap();
        assert!(
            baseline > supply,
            "the legacy burn must leave the baseline above the live supply (baseline {baseline}, \
             supply {supply})"
        );
        let stranded = baseline - supply;

        assert!(f.action_scout_advance_time(86_400), "one day");
        let before = f.onyc_supply().unwrap();
        assert!(f.action_settle_buffer(), "settle the stale day");
        let minted = f.onyc_supply().unwrap() - before;
        assert!(minted > 0, "the BUFFER must actually accrue, or this test proves nothing");

        // Linearity: what the same interval mints from a correct baseline, and the excess.
        let correct = ((minted as u128) * (supply as u128) / (baseline as u128)) as u64;
        let excess = minted - correct;
        println!(
            "P-0009: baseline exceeded supply by {stranded}; one day minted {minted} where a \
             correct baseline mints {correct} — {excess} ONyc backed by nothing"
        );
        assert!(
            excess > 0,
            "the stale baseline must over-mint (minted {minted}, correct {correct})"
        );
    }

    /// How much usdc a user holds — the only thing these Prop AMM tests actually compare.
    ///
    /// Every amount below is EVEN on purpose: the swap actions pick their signer with
    /// `pick_user(token_in_amount)`, whose low bit selects user_a or user_b, so an odd amount would
    /// silently move the trade to the other actor and compare two different people's balances.
    fn usdc_of(f: &OnreappFixture, who: &Pubkey) -> u64 {
        f.ctx.token_balance(&scout_ata(who, &f.mint_usdc, &SPL_TOKEN_ID))
    }

    /// Splitting a sell must not beat selling in one order.
    ///
    /// `docs/PROP_AMM_PRICING_MODEL.md` says outright that the endpoint formula "is not
    /// mathematically split-proof", and claims the compensating mechanism is that "every sell adds
    /// pair-local pressure and shrinks the wall for subsequent sells". That claim is empirical.
    /// This measures it: two identical fixtures, identical clock, identical total size — one order
    /// versus four.
    ///
    /// The hard wall exists to protect redemption liquidity from large or rapid sells. If splitting
    /// pays materially MORE, the protection is nominal: a seller simply chops the order up.
    #[test]
    fn t_v5_prop_amm_split_sell_does_not_beat_one_order() {
        // 80 ONyc total. Each quarter must clear `minimum_sell_haircut_onyc` (5 ONyc) with room
        // to spare, or the split leg is rejected outright rather than priced — the fee floor is
        // `max(pct_fee, minimum)` and a sell of exactly the floor nets zero.
        const TOTAL: u64 = 80_000_000_000;

        // NOTE: each `setup()` mints FRESH keypairs, so the actor must be read from the fixture it
        // belongs to. Measuring one fixture's balance at the other's pubkey silently reads an
        // account that does not exist and reports 0 — which made an earlier version of this test
        // pass vacuously.
        let mut single = OnreappFixture::setup();
        let single_user = single.pick_user(0).pubkey();
        let before_single = usdc_of(&single, &single_user);
        assert!(single.scout_swap_sell(0, TOTAL, 0), "single sell");
        let single_out = usdc_of(&single, &single_user) - before_single;

        let mut split = OnreappFixture::setup();
        let split_user = split.pick_user(0).pubkey();
        let before_split = usdc_of(&split, &split_user);
        for i in 0..4 {
            assert!(split.scout_swap_sell(0, TOTAL / 4, 0), "split sell {i}");
        }
        let split_out = usdc_of(&split, &split_user) - before_split;

        println!("split-order: one order paid {single_out}, four orders paid {split_out}");
        assert!(
            split_out <= single_out,
            "splitting one {TOTAL} sell into four paid {split_out} against {single_out} for the \
             same size — the hard wall is avoidable by chopping the order up"
        );
    }

    /// Interleaving buys must not erase the sell pressure that makes splitting expensive.
    ///
    /// `preview_effective_sell_volume` (`pricing.rs:283-285`) computes pressure as
    /// `curr_sell_value.saturating_sub(curr_buy_value)`, and a Prop AMM buy ALSO refills the
    /// redemption vault, which is the base of the dynamic wall. One action therefore undoes both
    /// halves of the defence against chopping an order up.
    ///
    /// The experiment isolates ORDER, not spend: both fixtures perform exactly the same four sells
    /// and four buys of exactly the same sizes, so both end holding the same ONyc and having spent
    /// the same usdc on buys. The only difference is the sequence — all buys first, versus buys
    /// interleaved between the sells. If interleaving pays more, the pressure signal is washable.
    #[test]
    fn t_v5_prop_amm_buys_do_not_wash_out_sell_pressure() {
        const TOTAL: u64 = 80_000_000_000; // 80 ONyc; each quarter clears the 5 ONyc fee floor
        const BUY: u64 = 5_000_000; // 5 usdc per buy

        // Buys first, then the sells: the buy relief lands in the epoch before any sell pressure.
        let mut upfront = OnreappFixture::setup();
        let upfront_user = upfront.pick_user(0).pubkey();
        let before_upfront = usdc_of(&upfront, &upfront_user);
        for i in 0..4 {
            assert!(upfront.scout_swap_buy(0, BUY, 0), "upfront buy {i}");
        }
        for i in 0..4 {
            assert!(upfront.scout_swap_sell(0, TOTAL / 4, 0), "upfront sell {i}");
        }
        let upfront_net = usdc_of(&upfront, &upfront_user) as i128 - before_upfront as i128;

        // The same eight operations, interleaved.
        let mut washed = OnreappFixture::setup();
        let washed_user = washed.pick_user(0).pubkey();
        let before_washed = usdc_of(&washed, &washed_user);
        for i in 0..4 {
            assert!(washed.scout_swap_sell(0, TOTAL / 4, 0), "washed sell {i}");
            assert!(washed.scout_swap_buy(0, BUY, 0), "washed buy {i}");
        }
        let washed_net = usdc_of(&washed, &washed_user) as i128 - before_washed as i128;

        println!("buy-relief: buys-first netted {upfront_net}, interleaved netted {washed_net}");
        assert!(
            washed_net <= upfront_net,
            "interleaving the same buys between the sells netted {washed_net} against \
             {upfront_net} for buys-first — the buys are erasing the pressure signal they are \
             meant to be independent of"
        );
    }

    /// A round trip at an unchanged clock must cost the user, never pay them.
    ///
    /// Buy ONyc with usdc and sell it straight back in the same block. There is no time for yield,
    /// so the only movements are fees and haircuts, and every one of them is against the user. A
    /// profitable round trip would be a money printer that needs no privilege and no price move.
    #[test]
    fn t_v5_round_trip_at_one_clock_cannot_profit() {
        let mut f = OnreappFixture::setup();
        let user = f.pick_user(0).pubkey();
        let onyc_ata = scout_ata(&user, &f.mint_onyc, &SPL_TOKEN_ID);

        let usdc_before = usdc_of(&f, &user);
        let onyc_before = f.ctx.token_balance(&onyc_ata);

        assert!(f.scout_swap_buy(0, 10_000_000, 0), "buy ONyc with 10 usdc");
        let bought = f.ctx.token_balance(&onyc_ata) - onyc_before;
        assert!(bought > 0, "the buy must deliver ONyc");

        // Sell exactly what the buy delivered, at the same clock.
        assert!(f.scout_swap_sell(0, bought, 0), "sell it straight back");

        let usdc_after = usdc_of(&f, &user);
        let onyc_after = f.ctx.token_balance(&onyc_ata);
        println!(
            "round trip: usdc {usdc_before} -> {usdc_after}, onyc {onyc_before} -> {onyc_after}"
        );
        assert!(
            usdc_after <= usdc_before,
            "a same-clock buy/sell round trip paid the user {} usdc",
            usdc_after - usdc_before
        );
        assert!(
            onyc_after <= onyc_before,
            "and it must not leave them holding more ONyc either"
        );
    }

    /// The Ed25519 precompile must be registered in the SVM, otherwise every approval branch is
    /// silently unreachable and reads as "the program rejects approvals" instead of "the harness
    /// cannot present one". Enabled by depending on litesvm with `features = ["precompiles"]`.
    #[test]
    fn t_ed25519_precompile_is_registered() {
        let f = OnreappFixture::setup();
        let acc = f.ctx.get_account(&ED25519_PROGRAM_ID)
            .expect("Ed25519SigVerify program account must exist");
        assert!(acc.executable, "Ed25519SigVerify must be executable");
    }

    /// The approval-gated offer accepts a correctly signed, in-date, correctly-bound message and
    /// rejects the two ways it can be wrong. Covers verify_approval_message_generic's Expired and
    /// WrongUser branches, which no corpus can reach through the single-instruction actions.
    #[test]
    fn t_approval_branches() {
        let mut f = OnreappFixture::setup();
        assert!(f.action_take_offer_with_approval(1_000_000, 100_000, 0), "valid approval");
        assert!(!f.action_take_offer_with_approval(1_000_001, -100_000, 0), "expired approval");
        assert!(!f.action_take_offer_with_approval(1_000_002, 100_000, 2), "wrong-user approval");
    }

    /// The kill switch really halts the value flow, and really releases it again.
    #[test]
    fn t_kill_switch_gates_take_offer() {
        let mut f = OnreappFixture::setup();
        assert!(f.action_set_kill_switch(true), "enable kill switch");
        assert!(!f.action_take_offer(10_000_000), "take_offer must fail while killed");
        assert!(f.action_set_kill_switch(false), "disable kill switch");
        assert!(f.action_take_offer(10_000_000), "take_offer must work once released");
    }

    /// close_state really does deallocate `State`, and `action_scout_rebuild_state` is the way back.
    ///
    /// Recovery is an ACTION rather than close_state's hook because a hook region may contain only
    /// pure assignments — no calls, no conditionals. This pins both halves: that the instruction is
    /// genuinely destructive, and that the fuzzer has a route out of the state it leaves behind.
    #[test]
    fn t_close_state_is_destructive_and_recoverable() {
        let mut f = OnreappFixture::setup();
        assert!(f.state_exists(), "State must exist after setup");

        assert!(f.action_close_state(), "close_state");
        assert!(!f.state_exists(), "close_state must really deallocate State");
        assert!(!f.action_take_offer(10_000_000), "no world means no take_offer");

        assert!(f.action_scout_rebuild_state(), "rebuild must succeed while State is missing");
        assert!(f.state_exists(), "State must be back");
        assert!(f.action_take_offer(10_000_000), "world must be usable again");

        // Idempotent: it must never silently reset a live world mid-chain.
        assert!(!f.action_scout_rebuild_state(), "rebuild must be a no-op when State exists");
    }

    /// P-0002's counterexample, as a direct reproduction rather than a fuzzer artefact.
    ///
    /// `redemption_vault_withdraw` moves an arbitrary amount out of the same token account that
    /// `create_redemption_request` locks user deposits into, consulting neither
    /// `requested_redemptions` nor any `RedemptionRequest`. Once drained, the request can be
    /// neither cancelled nor fulfilled -- both paths transfer out of that account -- so the
    /// redeemer's tokens are gone AND the request account cannot be closed.
    #[test]
    fn t_p0002_vault_drain_strands_open_requests() {
        let mut f = OnreappFixture::setup();
        let vault_onyc = f.redemption_vault_ata(&f.mint_onyc);
        let amount: u64 = 5_000_000_001; // odd -> the vault-op actions select the ONyc mint

        assert!(f.action_create_redemption_request(amount), "create_redemption_request");
        let locked = f.ctx.token_balance(&vault_onyc);
        assert_eq!(locked, amount, "the request must have locked its tokens in the vault");
        assert_eq!(f.onchain_requested_redemptions(), Some(amount as u128));

        assert!(f.action_redemption_vault_withdraw(locked | 1), "boss drains the redemption vault");
        assert_eq!(f.ctx.token_balance(&vault_onyc), 0, "vault must now be empty");

        // The claim is still recorded, but nothing backs it.
        assert_eq!(f.onchain_requested_redemptions(), Some(amount as u128));
        assert_eq!(f.open_request_total(), Some(amount as u128));

        // Both exits are now closed.
        assert!(!f.action_cancel_redemption_request(), "cancel must fail once the vault is drained");
        assert!(!f.action_fulfill_redemption_request(), "fulfil must fail once the vault is drained");
        assert_eq!(f.open_requests().map(|v| v.len()), Some(1), "the request is stranded, not retired");
    }

    /// P-0004's counterexample as a controlled differential, not a fuzzer artefact.
    ///
    /// With the cap pinned to exactly the current supply, mint_to and take_offer are BOTH refused
    /// (they hand `state.max_supply` to `mint_tokens`), while fulfilling a redemption whose payout
    /// leg is the program-controlled mint succeeds and mints straight past it — because
    /// `fulfill_redemption_request.rs:274` hands `mint_tokens` a hard-coded 0 instead.
    #[test]
    fn t_v5_redemption_token_in_must_be_onyc() {
        let mut f = OnreappFixture::setup();
        // Controls first: the two callers that pass `state.max_supply` still refuse to mint past
        // the cap, so this test fails loudly if the cap machinery itself regresses.
        assert!(f.action_scout_configure_max_supply(0), "configure_max_supply with zero headroom");
        let before = f.onyc_supply().expect("onyc supply");
        assert!(!f.action_mint_to(1), "mint_to must respect the cap");
        assert_eq!(f.onyc_supply(), Some(before), "no control path may move the supply");

        // The old P-0004 route to minting past the cap was a redemption offer whose PAYOUT leg was
        // ONyc — reachable only while `make_redemption_offer` accepted an arbitrary token_in.
        // programV5 pins token_in to `state.onyc_mint` (make_redemption_offer.rs:66-70), so the
        // reverse offer cannot be created and the payout leg can never be program-controlled.
        let boss = f.boss.insecure_clone();
        let rva = f.redemption_vault_authority;
        let (mint_usdc, mint_onyc) = (f.mint_usdc, f.mint_onyc);
        let refused = f
            .ctx
            .program(f.program_id)
            .call(instruction::MakeRedemptionOffer {
                fee_basis_points: 50,
                fee_basis_points_prop_amm_sell: 50,
            })
            .accounts(accounts::MakeRedemptionOffer {
                state: f.state_pda,
                offer: f.offer_rev_pda,
                redemption_vault_authority: rva,
                token_in_mint: mint_usdc,
                token_in_program: SPL_TOKEN_ID,
                vault_token_in_account: scout_ata(&rva, &mint_usdc, &SPL_TOKEN_ID),
                token_out_mint: mint_onyc,
                token_out_program: SPL_TOKEN_ID,
                vault_token_out_account: scout_ata(&rva, &mint_onyc, &SPL_TOKEN_ID),
                redemption_offer: f.redemption_offer_rev_pda,
                boss: boss.pubkey(),
            })
            .signers(&[&boss])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        assert!(!refused, "a non-ONyc token_in redemption offer must be refused");
    }

    /// P-0005 under programV5: the ROOT CAUSE is still there, the EXPLOIT is not.
    ///
    /// `make_offer` still relates nothing between its two mint arguments (make_offer.rs:129-162),
    /// so a self-referential offer is still creatable and still priceable. What changed is the
    /// framework: programV5 moved to anchor-lang 1.1.2, whose duplicate-mutable-account check
    /// rejects `take_offer` before the handler runs, because a same-mint offer necessarily passes
    /// ONE token account in two `mut` slots (`vault_token_in_account` == `vault_token_out_account`,
    /// both the vault authority's ATA for that mint).
    ///
    /// So this is a mitigation, not a fix, and it is the FRAMEWORK's, not the program's. This test
    /// pins both halves: if the account layout ever separates those slots, or the framework check
    /// weakens, the assertion that the take is refused fails and the money printer is back.
    #[test]
    fn t_p0005_same_mint_offer_is_creatable_but_no_longer_takeable() {
        let mut f = OnreappFixture::setup();
        // fee_basis_points = 3 selects make_offer_pair variant 3 = (onyc, onyc). The program
        // accepts it: nothing relates the two mint arguments.
        assert!(f.action_make_offer(3, false, false), "make_offer still accepts token_in == token_out");
        assert!(f.action_scout_price_same_mint_offer(500_000_000), "and it is still priceable at 0.5");

        let user = f.pick_user(0).pubkey();
        let uata = scout_ata(&user, &f.mint_onyc, &SPL_TOKEN_ID);
        let held_before = f.ctx.token_balance(&uata);
        let supply_before = f.onyc_supply().expect("supply");

        assert!(
            !f.action_scout_take_same_mint_offer(1_000_000_000, 4),
            "the take must be refused (ConstraintDuplicateMutableAccount, anchor-lang 1.1.2)"
        );

        assert_eq!(f.ctx.token_balance(&uata), held_before, "no taker gain");
        assert_eq!(f.onyc_supply(), Some(supply_before), "no supply inflation");
    }

    /// LEAD (low): an Anchor CONSTRAINT calls `mint.mint_authority.unwrap()`, so a mint with no
    /// authority makes the program PANIC rather than return its declared error code.
    #[test]
    fn t_lead_unwrap_panics_on_authorityless_mint() {
        let mut f = OnreappFixture::setup();
        let boss = f.boss.insecure_clone();
        let dead = f.ctx.create_mint().pubkey(Keypair::new().pubkey())
            .decimals(6).supply(0).create().unwrap();  // mint_authority = COption::None
        let out = f.ctx.program(f.program_id)
            .call(instruction::TransferMintAuthorityToProgram {})
            .accounts(accounts::TransferMintAuthorityToProgram {
                boss: boss.pubkey(), state: f.state_pda, mint: dead,
                mint_authority: f.mint_authority_pda, token_program: SPL_TOKEN_ID })
            .signers(&[&boss]).send().expect("tx submitted");
        assert!(!out.is_success(), "must not succeed");
        let logs = out.logs().join("\n");
        // The runtime spells this "SBF program Panicked in <file> at <line>" under the programV5
        // toolchain and "SBF program panicked" under the previous one; match either.
        assert!(logs.to_lowercase().contains("sbf program panicked"),
            "expected a panic, not a clean error. logs:\n{logs}");
        assert!(logs.contains("COption::unwrap()"), "logs:\n{logs}");
    }

    /// LEAD (low): `create_redemption_request` has `payer = redeemer` but
    /// `cancel_redemption_request` has `close = redemption_admin`, so the request account's rent
    /// moves from the user to the admin on every create/cancel cycle. `cancel` may be signed by
    /// the redemption_admin or the boss, so they can harvest it from any pending request.
    #[test]
    fn t_lead_cancel_rent_goes_to_admin_not_payer() {
        let mut f = OnreappFixture::setup();
        let user = f.pick_user(0).pubkey();
        let admin = f.redemption_admin.pubkey();
        let lam = |f: &OnreappFixture, k: &Pubkey| f.ctx.get_account(k).map(|a| a.lamports).unwrap_or(0);

        let (u0, a0) = (lam(&f, &user), lam(&f, &admin));
        assert!(f.action_create_redemption_request(1_000_000), "create");
        assert!(f.action_cancel_redemption_request(), "cancel");
        let (u1, a1) = (lam(&f, &user), lam(&f, &admin));

        let user_delta = u0 - u1;
        let admin_delta = a1 - a0;
        assert!(user_delta > 0, "the redeemer paid the rent");
        assert_eq!(user_delta, admin_delta,
            "and the admin received exactly it back on cancel ({user_delta} lamports)");
    }

    /// P-0006 is FIXED in programV5, and this pins the fix.
    ///
    /// The reported defect was `update_offer_fee` bounding against `MAX_BASIS_POINTS` (10000) where
    /// its three sibling fee writers used `MAX_ALLOWED_FEE_BPS` (1000). programV5's
    /// `update_offer_fee.rs:98` now uses `MAX_ALLOWED_FEE_BPS`, and the new
    /// `update_offer_permissionless_fee.rs:71` was written with the same bound. All four writers
    /// agree; this test fails the moment one of them drifts again.
    #[test]
    fn t_p0006_fee_ceiling_is_uniform_across_every_writer() {
        let mut f = OnreappFixture::setup();
        let offer = f.offer_pda;
        let fee_of = |f: &OnreappFixture, o: &Pubkey| -> u16 {
            let d = f.ctx.account_data(o).unwrap();
            u16::from_le_bytes(d[SCOUT_OFFER_FEE_OFFSET..SCOUT_OFFER_FEE_END].try_into().unwrap())
        };
        assert_eq!(fee_of(&f, &offer), 100, "setup created it at 1%");

        for (label, over, at_limit) in [
            ("make_offer", f.action_make_offer(1001, false, false), f.action_make_offer(1000, false, false)),
        ] {
            assert!(!over, "{label} must refuse 1001 bp");
            assert!(at_limit, "{label} must accept exactly 1000 bp");
        }
        assert!(!f.action_update_redemption_offer_fee(1001), "redemption fee must refuse 1001 bp");
        assert!(f.action_update_redemption_offer_fee(1000), "...but accepts exactly 1000 bp");
        assert!(!f.action_update_offer_permissionless_fee(1001), "permissionless fee must refuse 1001 bp");
        assert!(f.action_update_offer_permissionless_fee(1000), "...but accepts exactly 1000 bp");

        // The formerly-broken writer.
        assert!(!f.action_update_offer_fee(9999), "update_offer_fee must now refuse 9999 bp");
        assert!(!f.action_update_offer_fee(1001), "and must refuse 1001 bp");
        assert!(f.action_update_offer_fee(1000), "while still accepting exactly 1000 bp");
        assert_eq!(fee_of(&f, &offer), 1000, "the offer caps out at 10%");
    }

    // ---- escalation-chaining pass 2: helpers ---------------------------------------------------
    fn vault_onyc(f: &OnreappFixture) -> u64 { f.ctx.token_balance(&f.redemption_vault_onyc) }
    fn is_killed(f: &OnreappFixture) -> bool {
        f.ctx.account_data(&f.state_pda).map(|d| d[72] != 0).unwrap_or(false)
    }

    /// CHAIN C1 = P-0002 ∧ amplifier #6 (gate removal).
    /// The kill switch closes the user exits but not the boss's drain.
    #[test]
    fn c1_killswitch_closes_exits_but_not_the_drain() {
        let mut f = OnreappFixture::setup();
        let amount: u64 = 5_000_000_001; // odd -> vault ops select the ONyc mint
        assert!(f.action_create_redemption_request(amount), "user locks collateral");
        let locked = vault_onyc(&f);
        assert_eq!(locked, amount);

        assert!(f.action_set_kill_switch(true), "boss engages the emergency stop");
        assert!(is_killed(&f));

        // Every user exit is now shut.
        println!("C1 killed: cancel  -> {}", f.action_cancel_redemption_request());
        println!("C1 killed: fulfil  -> {}", f.action_fulfill_redemption_request());
        println!("C1 killed: create  -> {}", f.action_create_redemption_request(1_000_000_001));
        // The drain is not.
        let drained = f.action_redemption_vault_withdraw(locked | 1);
        println!("C1 killed: vault_withdraw -> {drained}");
        println!("C1 vault {locked} -> {}", vault_onyc(&f));
    }

    /// CHAIN C2 = P-0006 ∧ amplifier #4/#10 (snapshot staleness at the config extreme).
    /// take_offer has no minimum-output parameter, so a fee raised to 100% between quote and
    /// execution confiscates the taker's entire input for zero output.
    #[test]
    fn c2_hundred_percent_fee_confiscates_the_taker() {
        let mut f = OnreappFixture::setup();
        let user = f.pick_user(0).pubkey();
        let u_in = scout_ata(&user, &f.mint_usdc, &SPL_TOKEN_ID);
        let u_out = scout_ata(&user, &f.mint_onyc, &SPL_TOKEN_ID);
        let boss_in = scout_ata(&f.boss.pubkey(), &f.mint_usdc, &SPL_TOKEN_ID);

        println!("C2 update_offer_fee(10000) -> {}", f.action_update_offer_fee(10_000));
        let (a0, b0, c0) = (f.ctx.token_balance(&u_in), f.ctx.token_balance(&u_out), f.ctx.token_balance(&boss_in));
        let ok = f.action_take_offer(1_000_000_000);
        let (a1, b1, c1) = (f.ctx.token_balance(&u_in), f.ctx.token_balance(&u_out), f.ctx.token_balance(&boss_in));
        println!("C2 take_offer(1e9) -> {ok}");
        println!("C2 user token_in  {a0} -> {a1}   (paid {})", a0 as i128 - a1 as i128);
        println!("C2 user token_out {b0} -> {b1}   (received {})", b1 as i128 - b0 as i128);
        println!("C2 boss token_in  {c0} -> {c1}   (gained {})", c1 as i128 - c0 as i128);
    }

    /// CHAIN C3 = P-0005 ∧ amplifier #10 (config extreme).
    /// The MINIMUM legal price is 1 (add_offer_vector only requires base_price > 0), so a
    /// self-referential offer multiplies the taker's balance by 1e9 per call, not 2x.
    #[test]
    fn c3_same_mint_at_minimum_legal_price() {
        let mut f = OnreappFixture::setup();
        assert!(f.action_make_offer(3, false, false), "same-mint offer (variant 3)");
        assert!(f.action_scout_price_same_mint_offer(1), "base_price = 1, the minimum legal value");
        let user = f.pick_user(4).pubkey();
        let uata = scout_ata(&user, &f.mint_onyc, &SPL_TOKEN_ID);
        let (before, sup0) = (f.ctx.token_balance(&uata), f.onyc_supply().unwrap());
        let ok = f.action_scout_take_same_mint_offer(1_000, 4);
        let (after, sup1) = (f.ctx.token_balance(&uata), f.onyc_supply().unwrap());
        println!("C3 take(1000) -> {ok}");
        println!("C3 user onyc {before} -> {after}  (x{})", if before>0 {after/before} else {0});
        println!("C3 supply    {sup0} -> {sup1}  (minted {})", sup1 as i128 - sup0 as i128);
    }

    /// CHAIN C7 = amplifier #4 (snapshot staleness) ALONE — no other finding required.
    ///
    /// `add_offer_vector` accepts `start_time = max(now, base_time)`, so a new vector priced by the
    /// boss becomes active IMMEDIATELY. Combined with take_offer having no minimum-output
    /// parameter, the boss can move the price under an in-flight take and confiscate the input.
    /// This does NOT depend on P-0006 and survives fixing it.
    #[test]
    fn c7_price_can_be_moved_under_an_inflight_take() {
        let mut f = OnreappFixture::setup();
        let user = f.pick_user(0).pubkey();
        let u_in = scout_ata(&user, &f.mint_usdc, &SPL_TOKEN_ID);
        let u_out = scout_ata(&user, &f.mint_onyc, &SPL_TOKEN_ID);

        // Baseline: an honest take at the price the user would have quoted.
        let (a0, b0) = (f.ctx.token_balance(&u_in), f.ctx.token_balance(&u_out));
        assert!(f.action_take_offer(1_000_000_000), "honest take");
        let (a1, b1) = (f.ctx.token_balance(&u_in), f.ctx.token_balance(&u_out));
        println!("C7 honest: paid {} received {}", a0 - a1, b1 - b0);

        // Boss adds a vector that activates immediately, at an absurd but LEGAL price.
        assert!(f.action_scout_advance_time(3_600), "move past the existing vector");
        let now = scout_now(&f.ctx);
        let ok = f.action_add_offer_vector(now, u64::MAX / 4, 0, 3_600);
        println!("C7 add_offer_vector(base_price = u64::MAX/4) -> {ok}");

        // Same call, same arguments, from the user's point of view.
        let (a2, b2) = (f.ctx.token_balance(&u_in), f.ctx.token_balance(&u_out));
        let took = f.action_take_offer(1_000_000_000);
        let (a3, b3) = (f.ctx.token_balance(&u_in), f.ctx.token_balance(&u_out));
        println!("C7 after repricing: take -> {took}");
        println!("C7   paid {}  received {}", a2 as i128 - a3 as i128, b3 as i128 - b2 as i128);
    }

    /// CHAIN C4 = P-0004 ∧ amplifier #1 (constant collision).
    /// max_supply == 0 is overloaded to mean "no cap", so a boss setting 0 to HALT issuance
    /// actually removes the cap entirely.
    #[test]
    fn c4_max_supply_zero_means_unlimited_not_frozen() {
        let mut f = OnreappFixture::setup();
        // First set a real cap so minting is refused...
        assert!(f.action_scout_configure_max_supply(0), "cap at current supply");
        println!("C4 with cap == supply: mint_to(1) -> {}", f.action_mint_to(1));
        // ...then "freeze" it by setting the cap to zero.
        let boss = f.boss.insecure_clone();
        let ok = f.ctx.program(f.program_id)
            .call(instruction::ConfigureMaxSupply { max_supply: 0 })
            .accounts(accounts::ConfigureMaxSupply { state: f.state_pda, boss: boss.pubkey() })
            .signers(&[&boss]).send().map(|o| o.is_success()).unwrap_or(false);
        println!("C4 configure_max_supply(0) -> {ok}");
        let s0 = f.onyc_supply().unwrap();
        println!("C4 with cap == 0:      mint_to(1e9) -> {}", f.action_mint_to(1_000_000_000));
        println!("C4 supply {s0} -> {}", f.onyc_supply().unwrap());
    }

    /// Prove the P-0007 subject is actually constructible: TWO redemption offers whose token_in is
    /// ONyc, both funding the SAME vault token account. Without this the property degenerates into
    /// P-0002 and is silently dead.
    #[test]
    fn p7_two_offers_share_one_vault() {
        let mut f = OnreappFixture::setup();
        let vault = f.redemption_vault_onyc;
        assert!(f.action_make_offer(0, false, false), "offer play->onyc (pair variant 0)");
        assert!(f.action_make_redemption_offer(25), "redemption offer onyc->play");

        let v0 = f.ctx.token_balance(&vault);
        assert!(f.action_create_redemption_request(1_000_000), "request on setup's onyc->usdc offer");
        let v1 = f.ctx.token_balance(&vault);
        assert!(f.action_scout_create_request_play(2_000_000, 0), "request on the onyc->play offer");
        let v2 = f.ctx.token_balance(&vault);

        println!("P7 vault {v0} -> {v1} -> {v2}");
        println!("P7 registry entries: {}", f.scout_p7_next);
        assert!(v1 > v0 && v2 > v1, "both offers must fund the SAME vault account");
        assert_eq!(f.scout_p7_next, 2, "both requests registered");
    }

    /// C5 decisive experiment: does one offer's lifecycle consume ANOTHER offer's collateral?
    ///
    /// Two redemption offers share one ONyc vault account. Drive create/fulfil/cancel across both,
    /// interleaved, WITHOUT ever calling redemption_vault_withdraw (the confirmed P-0002 drain), and
    /// check the pooled bound after every step. If the pool only breaks via the drain, the
    /// cross-offer contamination hypothesis is refuted and P-0007's firings are P-0002 restated.
    #[test]
    fn c5_pool_stays_solvent_without_the_drain() {
        let mut f = OnreappFixture::setup();
        assert!(f.action_make_offer(0, false, false), "offer play->onyc");
        assert!(f.action_make_redemption_offer(25), "second onyc-denominated redemption offer");

        let vault = f.redemption_vault_onyc;
        let claimed = |f: &OnreappFixture| -> u64 {
            let mut t = 0u64;
            for pda in f.scout_p7_reqs {
                if pda == Pubkey::default() { continue; }
                if let Ok(d) = f.ctx.account_data(&pda) {
                    if d.len() >= SCOUT_REQ_FULFILLED_END {
                        // remaining = amount - fulfilled_amount (partial fulfilment, programV5)
                        let amount = u64::from_le_bytes(
                            d[SCOUT_REQ_AMOUNT_OFFSET..SCOUT_REQ_MIN_LEN].try_into().unwrap());
                        let fulfilled = u64::from_le_bytes(
                            d[SCOUT_REQ_FULFILLED_OFFSET..SCOUT_REQ_FULFILLED_END].try_into().unwrap());
                        t = t.saturating_add(amount.saturating_sub(fulfilled));
                    }
                }
            }
            t
        };
        let mut step = 0;
        let mut check = |f: &OnreappFixture, what: &str, step: &mut i32| {
            let held = f.ctx.token_balance(&vault);
            let c = claimed(f);
            *step += 1;
            println!("C5 {:>2}. {:<34} vault={:<14} claimed={:<14} {}",
                     step, what, held, c, if held >= c { "ok" } else { "SHORTFALL" });
            assert!(held >= c, "pool became insolvent at step {step} after {what}");
        };

        check(&f, "baseline", &mut step);
        assert!(f.action_create_redemption_request(3_000_000), "A1 on offer A");
        check(&f, "create A1", &mut step);
        assert!(f.action_scout_create_request_play(5_000_000, 0), "B1 on offer B");
        check(&f, "create B1", &mut step);
        assert!(f.action_create_redemption_request(7_000_001), "A2 on offer A");
        check(&f, "create A2", &mut step);
        assert!(f.action_scout_create_request_play(11_000_000, 1), "B2 on offer B");
        check(&f, "create B2", &mut step);

        // Retire them out of creation order, alternating offers.
        assert!(f.action_fulfill_redemption_request(), "fulfil A1");
        check(&f, "fulfil A1", &mut step);
        assert!(f.action_cancel_redemption_request(), "cancel A2");
        check(&f, "cancel A2", &mut step);
        // Both remaining are offer B's; drive more A traffic on top.
        assert!(f.action_create_redemption_request(2_500_000), "A3");
        check(&f, "create A3", &mut step);
        assert!(f.action_fulfill_redemption_request(), "fulfil A3");
        check(&f, "fulfil A3", &mut step);
        println!("C5 offer B's collateral still in the pool: {}", claimed(&f));
        assert!(claimed(&f) > 0, "offer B's requests must still be outstanding and covered");
    }

    /// Time must actually move, and a later vector must become addable and active once it has.
    #[test]
    fn t_clock_advances_and_new_vector_activates() {
        let mut f = OnreappFixture::setup();
        let t0 = scout_now(&f.ctx);
        assert!(f.action_scout_advance_time(7_200));
        let t1 = scout_now(&f.ctx);
        assert!(t1 > t0, "clock must advance: {t0} -> {t1}");
        assert!(f.action_add_offer_vector(t1, 2_000_000_000, 1_000_000, 600), "add later vector");
        assert!(f.action_get_nav(), "nav must price against the new vector");
    }
}
// SCOUT:TESTS:END
use crucible_test_context::*;
use crucible_fuzzer::anchor_lang::system_program;
use crucible_fuzzer::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::rc::Rc;

// SCOUT:CHECK-CONTRACT:BEGIN sha256=c4b20795d13638b9cbca54acc8669b4394eb8494fe1116eb26b75f0b968aaf9e
// Semantic invariant checks have two modes:
//   default / SCOUT_CHECK_MODE=enforce: record a real Crucible fuzz violation;
//   SCOUT_CHECK_MODE=observe: emit nonce-bound reachability markers, never a violation.
// This exact alias is part of the trusted contract.  Generated setup and the
// macros below use `crate::`/`$crate` paths so a mutable prelude cannot replace
// Crucible's TestContext or violation/session functions with local lookalikes.
#[doc(hidden)]
extern crate crucible_test_context as __scout_crucible_test_context;

fn __scout_check_observe_mode() -> bool {
    std::env::var("SCOUT_CHECK_MODE").as_deref() == Ok("observe")
}

// Mute a property whose finding is already investigated and written up. Such a property keeps
// firing on the SAME known defect and floods the objective, hiding every other property's first
// finding behind thousands of duplicates -- observed at ~160 crashes per 25s on one target.
//
// Muting is ALWAYS announced on stderr, once per process. A silently disabled check is the exact
// false-negative trap this pipeline exists to avoid: a muted property is indistinguishable from a
// passing one unless the run says so out loud. `SCOUT_CHECK_MUTE` is also stripped from ordinary
// fuzz subprocesses alongside the other audit switches, so a stray shell variable can never
// quietly disable a check -- a caller must pass it explicitly.
fn __scout_check_announce_mutes(list: &str) {
    static MUTE_ONCE: std::sync::Once = std::sync::Once::new();
    MUTE_ONCE.call_once(|| {
        eprintln!("[SCOUT_CHECK_MUTED] {}", list);
    });
}

fn __scout_check_muted(property: &str) -> bool {
    match std::env::var("SCOUT_CHECK_MUTE") {
        Ok(list) => {
            let muted = list.split(',').any(|entry| entry.trim() == property);
            if muted {
                __scout_check_announce_mutes(&list);
            }
            muted
        }
        Err(_) => false,
    }
}

fn __scout_check_selected(property: &str) -> bool {
    if __scout_check_muted(property) {
        return false;
    }
    match std::env::var("SCOUT_CHECK_ONLY") {
        Ok(selected) => selected == property,
        Err(_) => true,
    }
}

fn __scout_check_nonce() -> Result<String, &'static str> {
    let nonce = std::env::var("SCOUT_CHECK_RUN")
        .map_err(|_| "missing or non-Unicode SCOUT_CHECK_RUN")?;
    if nonce.is_empty() {
        return Err("empty SCOUT_CHECK_RUN");
    }
    if !nonce.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b':' | b'-')
    }) {
        return Err("SCOUT_CHECK_RUN contains unsafe characters");
    }
    Ok(nonce)
}

fn __scout_check_emit_error(reason: &str) {
    static ERROR_ONCE: std::sync::Once = std::sync::Once::new();
    ERROR_ONCE.call_once(|| {
        // Never echo an invalid value: whitespace/newlines would forge protocol fields.
        eprintln!("[SCOUT_CHECK_ERROR] INVALID {}", reason);
    });
}

macro_rules! scout_check_session {
    () => {{
        if $crate::__scout_check_observe_mode() {
            // Coverage-only replay runs before Crucible's stateful initializer.  Set
            // this per-thread flag here so failed actions terminate accumulated chains
            // exactly as they did in the stateful campaign that produced the corpus.
            $crate::__scout_crucible_test_context::set_stateful_chain_mode(true);
            static SESSION_ONCE: std::sync::Once = std::sync::Once::new();
            SESSION_ONCE.call_once(|| {
                match $crate::__scout_check_nonce() {
                    Ok(nonce) => eprintln!("[SCOUT_CHECK_SESSION] {}", nonce),
                    Err(reason) => $crate::__scout_check_emit_error(reason),
                }
            });
        }
    }};
}

// Gate the *entire* property computation, not only its final predicate.  This
// prevents another property's fallible reads, eligibility logic, or shadow-hook
// arithmetic from panicking/starving an isolated SCOUT_CHECK_ONLY replay.
macro_rules! scout_run_property {
    ($property:literal, $expression:expr $(,)?) => {{
        if $crate::__scout_check_selected($property) {
            let _ = $expression;
        }
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __scout_check_impl {
    ($property:literal, $site:literal, $predicate:expr, $message:expr) => {{
        let __scout_observe = $crate::__scout_check_observe_mode();
        if !$crate::__scout_check_selected($property) {
            true
        } else {
            let __scout_nonce = if __scout_observe {
                Some($crate::__scout_check_nonce())
            } else {
                None
            };
            if let Some(Err(ref __scout_error)) = __scout_nonce {
                // An invalid session can never produce an EVALUATED marker.  The
                // mechanical verifier therefore cannot mistake it for sound evidence.
                $crate::__scout_check_emit_error(__scout_error);
                false
            } else {
                // Keep the predicate in one lexical/runtime position.  Expressions
                // with reads or counters are evaluated exactly once per selected check.
                let __scout_check_result: bool = $predicate;
                if let Some(Ok(ref __scout_run)) = __scout_nonce {
                    eprintln!(
                        "[SCOUT_CHECK_EVALUATED] {} {} {} {}:{}",
                        __scout_run, $property, $site, file!(), line!()
                    );
                    if !__scout_check_result {
                        eprintln!(
                            "[SCOUT_CHECK_WOULD_VIOLATE] {} {} {} {}:{}",
                            __scout_run, $property, $site, file!(), line!()
                        );
                    }
                } else if !__scout_check_result {
                    $crate::__scout_crucible_test_context::record_violation($message);
                }
                __scout_check_result
            }
        }
    }};
}

macro_rules! scout_check {
    ($property:literal, $site:literal, $predicate:expr $(,)?) => {{
        $crate::__scout_check_impl!(
            $property,
            $site,
            $predicate,
            format!(
                "Invariant {} check {} failed at {}:{}",
                $property, $site, file!(), line!()
            )
        )
    }};
    ($property:literal, $site:literal, $predicate:expr, $($arg:tt)+) => {{
        $crate::__scout_check_impl!($property, $site, $predicate, format!($($arg)+))
    }};
}
// SCOUT:CHECK-CONTRACT:END

const SCOUT_TARGET_PROGRAM_ARTIFACT: &str = "programs/onreapp.so";




// SCOUT:BINDINGS:BEGIN
// ---- program ids ----------------------------------------------------------------------------
// token_program = SPL_TOKEN_ID
// token_in_program = SPL_TOKEN_ID
// token_out_program = SPL_TOKEN_ID
// associated_token_program = ATA_PROGRAM_ID
// instructions_sysvar = INSTRUCTIONS_SYSVAR_ID
//
// ---- global PDAs ----------------------------------------------------------------------------
// state = self.state_pda
// mint_authority = self.mint_authority_pda
// vault_authority = self.offer_vault_authority
// offer_vault_authority = self.offer_vault_authority
// redemption_vault_authority = self.redemption_vault_authority
// permissionless_authority = self.permissionless_authority
// offer = self.offer_pda
// redemption_offer = self.redemption_offer_pda
//
// ---- the `initialize` triple ----------------------------------------------------------------
// Initialize.program = self.program_id
// programV5's IDL marks `program_data` optional, so the generated field is `Option<Address>` and
// the bare-PDA binding no longer type-checks -- `--features admin_actions` fails to build with
// `expected Option<Address>, found Address` at action_initialize.
// Initialize.program_data = Some(scout_pda(&[self.program_id.as_ref()], &BPF_LOADER_UPGRADEABLE_ID))
// Initialize.onyc_mint = self.mint_onyc
//
// ---- mints ----------------------------------------------------------------------------------
// The main offer is usdc -> onyc; the redemption offer is its inverse, so every redemption
// instruction has token_in/token_out the other way round.
// token_in_mint = self.mint_usdc
// token_out_mint = self.mint_onyc
// onyc_mint = self.mint_onyc
// CreateRedemptionRequest.token_in_mint = self.mint_onyc
// CancelRedemptionRequest.token_in_mint = self.mint_onyc
// FulfillRedemptionRequest.token_in_mint = self.mint_onyc
// FulfillRedemptionRequest.token_out_mint = self.mint_usdc
// MakeRedemptionOffer.token_in_mint = self.mint_onyc
// MakeRedemptionOffer.token_out_mint = self.mint_usdc
// UpdateRedemptionOfferFee.token_in_mint = self.mint_onyc
// UpdateRedemptionOfferFee.token_out_mint = self.mint_usdc
//
// `make_offer` is the one instruction whose offer must NOT already exist (`init`, not
// `init_if_needed`), so it gets the spare mint as its input leg and mints a genuinely new offer.
// The pair is chosen from `fee_basis_points`, so the fuzzer selects it. One of the four variants
// puts the same mint on both legs — legal as far as make_offer is concerned, and what P-0005 asserts
// must not be.
// MakeOffer.token_in_mint = self.make_offer_pair(fee_basis_points).0
// MakeOffer.token_out_mint = self.make_offer_pair(fee_basis_points).1
// MakeOffer.offer = self.make_offer_pda_for(fee_basis_points)
// MakeOffer.vault_token_in_account = scout_ata(&self.offer_vault_authority, &self.make_offer_pair(fee_basis_points).0, &SPL_TOKEN_ID)
//
// Likewise the mint-authority instructions operate on the spare mint, so they stay live instead
// of permanently failing against ONyc's already-transferred authority.
// TransferMintAuthorityToProgram.mint = self.mint_play
// TransferMintAuthorityToBoss.mint = self.mint_play
//
// ---- offer-side token accounts --------------------------------------------------------------
// vault_token_in_account = scout_ata(&self.offer_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID)
// vault_token_out_account = scout_ata(&self.offer_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID)
// boss_token_in_account = scout_ata(&self.boss.pubkey(), &self.mint_usdc, &SPL_TOKEN_ID)
// boss_onyc_account = scout_ata(&self.boss.pubkey(), &self.mint_onyc, &SPL_TOKEN_ID)
// permissionless_token_in_account = scout_ata(&self.permissionless_authority, &self.mint_usdc, &SPL_TOKEN_ID)
// permissionless_token_out_account = scout_ata(&self.permissionless_authority, &self.mint_onyc, &SPL_TOKEN_ID)
//
// `token_in_amount`'s low bit picks the acting user, so both actors really do trade against the
// same pools. A fixed single user makes every value-conservation property vacuous.
// TakeOffer.user = signer:self.pick_user(token_in_amount)
// TakeOffer.user_token_in_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_usdc, &SPL_TOKEN_ID)
// TakeOffer.user_token_out_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_onyc, &SPL_TOKEN_ID)
// TakeOfferPermissionless.user = signer:self.pick_user(token_in_amount)
// TakeOfferPermissionless.user_token_in_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_usdc, &SPL_TOKEN_ID)
// TakeOfferPermissionless.user_token_out_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_onyc, &SPL_TOKEN_ID)
// boss = self.boss.pubkey()
//
// ---- redemption-side token accounts ---------------------------------------------------------
// CreateRedemptionRequest.redeemer = signer:self.pick_user(amount)
// CreateRedemptionRequest.redeemer_token_account = scout_ata(&self.pick_user_pk(amount), &self.mint_onyc, &SPL_TOKEN_ID)
// CreateRedemptionRequest.vault_token_account = scout_ata(&self.redemption_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID)
// CreateRedemptionRequest.redemption_request = self.next_request_pda()
//
// Fulfil/cancel act on the OLDEST live request, tracked harness-side, because the request PDA is
// seeded by a counter the harness cannot guess from the action's arguments.
// FulfillRedemptionRequest.redemption_request = self.oldest_request_pda()
// FulfillRedemptionRequest.redeemer = self.oldest_request_redeemer()
// FulfillRedemptionRequest.redemption_admin = signer:self.redemption_admin.insecure_clone()
// FulfillRedemptionRequest.offer = self.offer_pda
// FulfillRedemptionRequest.vault_token_in_account = scout_ata(&self.redemption_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID)
// FulfillRedemptionRequest.vault_token_out_account = scout_ata(&self.redemption_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID)
// FulfillRedemptionRequest.user_token_out_account = scout_ata(&self.oldest_request_redeemer(), &self.mint_usdc, &SPL_TOKEN_ID)
// FulfillRedemptionRequest.boss_token_in_account = scout_ata(&self.boss.pubkey(), &self.mint_onyc, &SPL_TOKEN_ID)
//
// CancelRedemptionRequest.redemption_request = self.oldest_request_pda()
// CancelRedemptionRequest.redeemer = self.oldest_request_redeemer()
// CancelRedemptionRequest.redemption_admin = self.redemption_admin.pubkey()
// CancelRedemptionRequest.signer = signer:self.redemption_admin.insecure_clone()
// CancelRedemptionRequest.vault_token_account = scout_ata(&self.redemption_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID)
// CancelRedemptionRequest.redeemer_token_account = scout_ata(&self.oldest_request_redeemer(), &self.mint_onyc, &SPL_TOKEN_ID)
//
// `make_redemption_offer` is `init`, so it cannot target the redemption offer setup already
// built. It is pointed at onyc -> play instead, whose underlying offer (seeds are the mints
// SWAPPED: [OFFER, token_out, token_in] = [OFFER, play, onyc]) is exactly what `action_make_offer`
// creates. That makes this a genuine two-action sequence the fuzzer has to discover, rather than
// an action that can never succeed.
// MakeRedemptionOffer.signer = signer:self.redemption_admin.insecure_clone()
// MakeRedemptionOffer.redemption_offer = scout_pda(&[SEED_REDEMPTION_OFFER, self.mint_onyc.as_ref(), self.mint_play.as_ref()], &self.program_id)
// MakeRedemptionOffer.offer = scout_pda(&[SEED_OFFER, self.mint_play.as_ref(), self.mint_onyc.as_ref()], &self.program_id)
// MakeRedemptionOffer.token_in_mint = self.mint_onyc
// MakeRedemptionOffer.token_out_mint = self.mint_play
// MakeRedemptionOffer.vault_token_in_account = scout_ata(&self.redemption_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID)
// MakeRedemptionOffer.vault_token_out_account = scout_ata(&self.redemption_vault_authority, &self.mint_play, &SPL_TOKEN_ID)
//
// ---- market-info reads ----------------------------------------------------------------------
// onyc_vault_account = scout_ata(&self.offer_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID)
//
// ---- approval message ------------------------------------------------------------------------
// `None` for the plain actions: a valid ApprovalMessage is only accepted alongside an Ed25519
// precompile instruction in the SAME transaction, which a single-instruction `.send()` cannot
// carry. The approval-required path is driven by a compound action in SCOUT:EXTRA-ACTIONS.
// approval_message = None
// SetRedemptionAdmin.new_redemption_admin = self.redemption_admin.pubkey()
//
// ---- vault operations -----------------------------------------------------------------------
// OfferVaultDeposit.token_mint = self.mint_usdc
// programV5 opened both vault deposits to ANY signer (`depositor`, was `boss`). Binding them to a
// fuzzer-chosen user is what makes that new permissionless surface reachable at all.
// OfferVaultDeposit.depositor = signer:self.pick_user(amount)
// OfferVaultDeposit.depositor_token_account = scout_ata(&self.pick_user_pk(amount), &self.mint_usdc, &SPL_TOKEN_ID)
// OfferVaultDeposit.vault_token_account = scout_ata(&self.offer_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID)
// OfferVaultWithdraw.token_mint = self.mint_usdc
// OfferVaultWithdraw.boss_token_account = scout_ata(&self.boss.pubkey(), &self.mint_usdc, &SPL_TOKEN_ID)
// OfferVaultWithdraw.vault_token_account = scout_ata(&self.offer_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID)
// The redemption vault ops pick their mint from the fuzzer's `amount`, so they can reach the ONyc
// vault — the account that actually custodies user deposits from `create_redemption_request`.
// Pinning them to usdc would make P-0002's subject unreachable and the property permanently quiet.
// RedemptionVaultDeposit.token_mint = self.pick_vault_mint(amount)
// RedemptionVaultDeposit.depositor = signer:self.pick_user(amount)
// RedemptionVaultDeposit.depositor_token_account = scout_ata(&self.pick_user_pk(amount), &self.pick_vault_mint(amount), &SPL_TOKEN_ID)
// RedemptionVaultDeposit.vault_token_account = scout_ata(&self.redemption_vault_authority, &self.pick_vault_mint(amount), &SPL_TOKEN_ID)
// RedemptionVaultWithdraw.token_mint = self.pick_vault_mint(amount)
// RedemptionVaultWithdraw.boss_token_account = scout_ata(&self.boss.pubkey(), &self.pick_vault_mint(amount), &SPL_TOKEN_ID)
// RedemptionVaultWithdraw.vault_token_account = scout_ata(&self.redemption_vault_authority, &self.pick_vault_mint(amount), &SPL_TOKEN_ID)
//
// ---- governance args ------------------------------------------------------------------------
// A fresh random pubkey per call would make `remove_admin`, `remove_approver` and `accept_boss`
// structurally unreachable — they can only ever match a key some earlier action installed. These
// bind to fixed, already-known keys so each add/remove pair actually closes.
//
// `propose_boss` nominates the CURRENT boss on purpose. Nominating anyone else lets `accept_boss`
// hand authority to a key the harness does not sign with, after which every `has_one = boss`
// instruction fails for the rest of the iteration — a self-inflicted coverage collapse, not a
// finding. Proposing the incumbent still exercises both instructions end to end.
// ProposeBoss.new_boss = self.boss.pubkey()
// AddAdmin.new_admin = self.user_a.pubkey()
// RemoveAdmin.admin_to_remove = self.user_a.pubkey()
// AddApprover.approver = self.approver.pubkey()
// RemoveApprover.approver = self.approver.pubkey()
// InitializePermissionlessAuthority.name = String::from("permissionless-1")
// `add_offer_vector` with `start_time = None` resolves to max(now, base_time), i.e. always a
// vector that is active immediately. `delete_offer_vector` then has nothing it is allowed to touch:
// it refuses any vector whose start time is not strictly in the FUTURE
// (`delete_offer_vector.rs:94`). Alternating between an immediate vector and a scheduled one keeps
// both of `add_offer_vector`'s branches live and gives `delete_offer_vector` something to delete.
// AddOfferVector.start_time = self.next_vector_start_time()
//
// ---- programV5 ------------------------------------------------------------------------------
// The redemption path is cross-checked (`create_redemption_request.rs:168`:
// `redemption_offer.offer == offer.key()`), and the redemption offer setup() builds is the
// inverse of the main usdc -> onyc offer.
// CreateRedemptionRequest.offer = self.offer_pda
// programV5 made fulfilment PARTIAL: `amount` must be in `1..=remaining`, so a raw fuzzer u64
// fails on essentially every draw. Binding it also removes it from the action signature, which is
// why the selector is an internal tick rather than a fuzz parameter — the fuzzer still controls
// how many fulfils precede this one, so both the full-settlement and the partial-bite branch of
// `is_fully_fulfilled` stay reachable.
// FulfillRedemptionRequest.amount = self.fulfill_amount_next()
// CancelRedemptionRequest.worker = self.state_worker()

// MakeRedemptionOffer.boss = signer:self.boss.insecure_clone()
// MakeRedemptionOffer.fee_basis_points_prop_amm_sell = 50
//
// The configurable accounting vaults are one PDA per `ConfigurableVaultKind`. The generated
// `kind.seed()` does not exist on the IDL-derived enum, so both the kind and the PDA are bound.
// `withdraw_configurable_vault` takes its kind from the fuzzer's amount so one action reaches all
// nine vaults.
// WithdrawConfigurableVault.kind = self.pick_withdrawable_kind(amount)
// WithdrawConfigurableVault.configurable_vault = self.cv_pda(kind)
// SetConfigurableVaultDestination.kind = self.pick_vault_kind(0)
// SetConfigurableVaultDestination.withdrawal_destination = self.boss.pubkey()
// SetConfigurableVaultDestination.configurable_vault = self.cv_pda(kind)
// SetCirculatingSupplyExcludedAccounts.owners = self.excluded_owner_list()
//
// The BUFFER / market-stats account block is IDENTICAL in all thirteen instructions that carry it
// (buffer_state, the three ONyc vault ATAs, market_stats, the excluded-balance cache and the main
// offer), so these are global bindings rather than thirteen qualified copies.
// buffer_state = self.buffer_state_pda()
// reserve_vault_onyc_account = self.reserve_vault_onyc()
// management_fee_vault_onyc_account = self.cv_ata(CvKind::ManagementFee, &self.mint_onyc)
// performance_fee_vault_onyc_account = self.cv_ata(CvKind::PerformanceFee, &self.mint_onyc)
// market_stats = self.market_stats_pda()
// circulating_supply_excluded_balance = self.excluded_balance_pda()
// excluded_accounts = self.excluded_accounts_pda()
// main_offer = self.state_main_offer()
// offer_vault_onyc_account = scout_ata(&self.offer_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID)
// reserve_vault_authority = self.reserve_vault_authority()
//
// `worker` signs `settle_buffer` and `fulfill_redemption_request`; in `cancel_redemption_request`
// it is only the rent recipient, which the qualified binding above covers.
// worker = signer:self.worker_kp().insecure_clone()
// SetWorker.new_worker = self.other_worker()
// Rotating rather than always-valid: `set_main_offer` has two rejection branches and neither is
// reachable from a proposal that is always a different, ONyc-out offer — `NoChange` needs the
// incumbent (`set_main_offer.rs:38`) and `InvalidTokenOutMint` needs an offer whose token_out is
// not ONyc (`:31-35`). The rotation covers all three outcomes and still succeeds half the time.
// SetMainOffer.offer = self.next_main_offer_candidate()
// FulfillRedemptionRequest.worker = signer:self.worker_kp().insecure_clone()
// SettleBuffer.worker = signer:self.worker_kp().insecure_clone()
//
// Fee/proceeds vaults are per-KIND PDAs and their token account is denominated in the token_in of
// whichever instruction carries them — usdc on the offer path, ONyc on the redemption path.
// FulfillRedemptionRequest.offer_proceeds_vault = self.cv_pda(CvKind::OfferProceeds)
// FulfillRedemptionRequest.offer_proceeds_token_in_account = self.cv_ata(CvKind::OfferProceeds, &self.mint_onyc)
// FulfillRedemptionRequest.redemption_fee_vault = self.cv_pda(CvKind::RedemptionFee)
// FulfillRedemptionRequest.redemption_fee_token_in_account = self.cv_ata(CvKind::RedemptionFee, &self.mint_onyc)
//
// BUFFER reserve deposits are open to any signer as well; withdrawals are boss-only.
// DepositReserveVault.depositor = signer:self.pick_user(amount)
// DepositReserveVault.depositor_onyc_account = scout_ata(&self.pick_user_pk(amount), &self.mint_onyc, &SPL_TOKEN_ID)
// WithdrawReserveVault.boss_onyc_account = scout_ata(&self.boss.pubkey(), &self.mint_onyc, &SPL_TOKEN_ID)
// BurnForNavIncrease.boss_onyc_account = scout_ata(&self.boss.pubkey(), &self.mint_onyc, &SPL_TOKEN_ID)
//
// The configurable vaults hold whichever mint the fee/proceeds route paid in; usdc is the one the
// offer path fills, so a withdrawal targets it and pays out to the boss.
// Each vault kind is denominated in the leg that funds it: the offer/permissionless/prop-amm-buy
// routes pay in usdc, the buffer fee and redemption/prop-amm-sell routes pay in ONyc. Pinning the
// mint to usdc would make five of the nine kinds permanently unwithdrawable and would never touch
// the ONyc side of the vault code at all.
// WithdrawConfigurableVault.mint = self.cv_mint(kind)
// WithdrawConfigurableVault.vault_token_account = self.cv_ata(kind, &self.cv_mint(kind))
// The destination is whatever `set_configurable_vault_destination` last recorded on that vault;
// binding it to a fixed key would make every withdrawal fail the moment the fuzzer pointed a vault
// somewhere else, and would hide a mis-routed payout rather than expose it.
// WithdrawConfigurableVault.destination = self.cv_destination(kind)
// WithdrawConfigurableVault.destination_token_account = scout_ata(&self.cv_destination(kind), &self.cv_mint(kind), &SPL_TOKEN_ID)
//
// ---- the v2 take path -----------------------------------------------------------------------
// `take_offer_v2` / `take_offer_permissionless_v2` route token_in to per-KIND ConfigurableVault
// PDAs instead of the boss ATA. Those vaults do NOT have to pre-exist: the handler calls
// `get_or_create_configurable_vault_token_account_pair` (configurable_vault/accounts.rs:33),
// which creates BOTH the vault PDA and its ATA on first use with the user as payer. So only the
// ADDRESSES have to be right -- no setup glue is needed. token_in on this path is usdc.
// TakeOfferV2.user = signer:self.pick_user(token_in_amount)
// TakeOfferV2.user_token_in_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_usdc, &SPL_TOKEN_ID)
// TakeOfferV2.user_token_out_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_onyc, &SPL_TOKEN_ID)
// TakeOfferV2.redemption_vault_token_in_account = scout_ata(&self.redemption_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID)
// TakeOfferV2.offer_proceeds_vault = self.cv_pda(CvKind::OfferProceeds)
// TakeOfferV2.offer_proceeds_token_in_account = self.cv_ata(CvKind::OfferProceeds, &self.mint_usdc)
// TakeOfferV2.offer_fee_vault = self.cv_pda(CvKind::OfferFee)
// TakeOfferV2.offer_fee_token_in_account = self.cv_ata(CvKind::OfferFee, &self.mint_usdc)
// TakeOfferPermissionlessV2.user = signer:self.pick_user(token_in_amount)
// TakeOfferPermissionlessV2.user_token_in_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_usdc, &SPL_TOKEN_ID)
// TakeOfferPermissionlessV2.user_token_out_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_onyc, &SPL_TOKEN_ID)
// TakeOfferPermissionlessV2.redemption_vault_token_in_account = scout_ata(&self.redemption_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID)
// TakeOfferPermissionlessV2.offer_proceeds_vault = self.cv_pda(CvKind::OfferProceeds)
// TakeOfferPermissionlessV2.offer_proceeds_token_in_account = self.cv_ata(CvKind::OfferProceeds, &self.mint_usdc)
// TakeOfferPermissionlessV2.permissionless_offer_fee_vault = self.cv_pda(CvKind::PermissionlessOfferFee)
// TakeOfferPermissionlessV2.permissionless_offer_fee_token_in_account = self.cv_ata(CvKind::PermissionlessOfferFee, &self.mint_usdc)
//
// management/performance ConfigurableVault PDAs, carried by initialize_buffer and
// burn_for_nav_increase (the seeds are module constants, so the generator cannot derive them).
// management_fee_vault = self.cv_pda(CvKind::ManagementFee)
// performance_fee_vault = self.cv_pda(CvKind::PerformanceFee)
//
// `add_approver` refuses a key already in either slot (add_approver.rs:54-56), so the generated
// action must propose someone other than the approver setup() installed, or it can never succeed.
// The remove/re-add cycle over all four cases lives in `action_scout_approver_ops`.
// AddApprover.approver = self.user_b.pubkey()
//
// `update_circulating_supply_excluded_balance` reads ctx.remaining_accounts: exactly one ONyc token
// account per ACTIVE owner in the excluded list, in list order (excluded_balance.rs:86-114). With
// none supplied it fails account validation before its handler runs and covers nothing.
// UpdateCirculatingSupplyExcludedBalance.remaining_accounts = self.excluded_owner_atas()
//
// ---- programV5: Prop AMM ---------------------------------------------------------------------
// One pair state per offer, and both sides share ONE offer: `validate_canonical_offer`
// (`prop_amm/validation.rs:43`) always derives `[OFFER, asset_mint, onyc_mint]` -- asset leg first
// -- whichever direction the swap runs. So the buy (usdc in) and the sell (ONyc in) both resolve to
// the main usdc -> onyc offer, and to its single `PropAmmPairState`. Global, not qualified.
// prop_amm_pair_state = self.prop_amm_pair_pda()
// ConfigurePropAmm.asset_mint = self.mint_usdc
//
// `curve_exponent_scaled` must be in [1_000, 100_000] AND a multiple of 1_000; `cadence_wave_scaled`
// must be <= 50_000 AND a multiple of 1_000 (`prop_amm/config.rs:123-140`). A raw fuzzer u32
// satisfies neither in ~1 draw in 10^5, so the action would never configure anything. Deriving both
// from the still-free `cadence_threshold` keeps them fuzzed across their whole legal domain instead
// of pinning them to the defaults -- the curve exponent and the cadence wave are the two knobs the
// pricing model is most sensitive to.
// ConfigurePropAmm.curve_exponent_scaled = 1_000 + (cadence_threshold % 100) * 1_000
// ConfigurePropAmm.cadence_wave_scaled = (cadence_threshold % 51) * 1_000
//
// The SELL side runs the mints the other way round from every other instruction in this harness:
// token_in is ONyc and token_out is the asset (`resolve_swap_side`, `prop_amm/validation.rs:28`),
// so the global `token_in_mint = mint_usdc` default is backwards for it and must be overridden.
// QuoteSwapSell.token_in_mint = self.mint_onyc
// QuoteSwapSell.token_out_mint = self.mint_usdc
// QuoteSwapSell.redemption_vault_token_out_account = scout_ata(&self.redemption_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID)
// OpenSwapSell.token_in_mint = self.mint_onyc
// OpenSwapSell.token_out_mint = self.mint_usdc
//
// The swaps are user-signed and the boss holds no usdc/ONyc, so leaving `user = self.payer` makes
// every swap fail on an empty balance. Same `token_in_amount`-low-bit actor selection as
// `take_offer`, so the two swap sides trade against the same pools as the v1 path.
// OpenSwapBuy.user = signer:self.pick_user(token_in_amount)
// OpenSwapBuy.user_token_in_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_usdc, &SPL_TOKEN_ID)
// OpenSwapBuy.user_token_out_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_onyc, &SPL_TOKEN_ID)
// OpenSwapSell.user = signer:self.pick_user(token_in_amount)
// OpenSwapSell.user_token_in_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_onyc, &SPL_TOKEN_ID)
// OpenSwapSell.user_token_out_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_usdc, &SPL_TOKEN_ID)
//
// Vault legs. `buy.rs:256-297` / `sell.rs` re-derive each of these as the canonical ATA of its
// authority for that side's mint and reject anything else, so there is exactly one right answer per
// account and the direction decides which mint.
// OpenSwapBuy.offer_vault_token_in_account = scout_ata(&self.offer_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID)
// OpenSwapBuy.offer_vault_token_out_account = scout_ata(&self.offer_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID)
// OpenSwapBuy.redemption_vault_token_in_account = scout_ata(&self.redemption_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID)
// OpenSwapSell.redemption_vault_token_in_account = scout_ata(&self.redemption_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID)
// OpenSwapSell.redemption_vault_token_out_account = scout_ata(&self.redemption_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID)
//
// Proceeds and fee vaults: one PDA per `ConfigurableVaultKind`, token account denominated in that
// side's token_in. The fee vault differs per side (BuyFee vs SellFee); the proceeds vault is shared,
// so it needs BOTH a usdc and an ONyc ATA (created in setup section 12a).
// OpenSwapBuy.prop_amm_proceeds_vault = self.cv_pda(CvKind::PropAmmProceeds)
// OpenSwapBuy.prop_amm_proceeds_token_in_account = self.cv_ata(CvKind::PropAmmProceeds, &self.mint_usdc)
// OpenSwapBuy.prop_amm_buy_fee_vault = self.cv_pda(CvKind::PropAmmBuyFee)
// OpenSwapBuy.prop_amm_buy_fee_token_in_account = self.cv_ata(CvKind::PropAmmBuyFee, &self.mint_usdc)
// OpenSwapSell.prop_amm_proceeds_vault = self.cv_pda(CvKind::PropAmmProceeds)
// OpenSwapSell.prop_amm_proceeds_token_in_account = self.cv_ata(CvKind::PropAmmProceeds, &self.mint_onyc)
// OpenSwapSell.prop_amm_sell_fee_vault = self.cv_pda(CvKind::PropAmmSellFee)
// OpenSwapSell.prop_amm_sell_fee_token_in_account = self.cv_ata(CvKind::PropAmmSellFee, &self.mint_onyc)
//
// `burn_for_nav_increase` burns ONyc out of the BUFFER reserve to hold NAV after the asset base is
// written down. A raw fuzzer u64 fails on essentially every draw — the adjustment has to be at most
// TVL (`burn_for_nav_increase.rs:238`) and the resulting burn at most the reserve vault's balance
// (`:181`), a window of order 1e11 base units inside a u64. Bound it to a fraction of what the
// reserve can actually cover, from live state, and rotate the fraction so both the "burn some" and
// the "nothing to burn" outcomes stay reachable.
// BurnForNavIncrease.asset_adjustment_amount = self.nav_burn_amount_next()
//
// The Prop AMM sell size has to be bounded for the same reason the fulfil amount does: the legal
// window sits between the ONyc fee floor and the redemption vault's balance, and a raw u64 misses
// it every time.
// OpenSwapSell.token_in_amount = self.swap_sell_amount_next()
// QuoteSwapSell.token_in_amount = self.swap_sell_amount_next()
//
// `delete_offer_vector` takes the vector's START TIME, not its index, so it must be handed one that
// exists or it can never succeed.
// DeleteOfferVector.vector_start_time = self.pick_offer_vector_start_time_next()
// SCOUT:BINDINGS:END

// SCOUT:PRELUDE:BEGIN
// ---------------------------------------------------------------------------------------------
// Program IDs and PDA seeds.
//
// Seeds are copied from the target's `constants::seeds` (onre-sol/programs/onreapp/src/
// constants.rs). extract.py reports them as `unresolvable seed seeds::STATE` etc. because they are
// module constants, not literals, so every PDA in every action is bound from here.
// ---------------------------------------------------------------------------------------------
pub const SPL_TOKEN_ID: Pubkey = Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const SPL_TOKEN_2022_ID: Pubkey = Pubkey::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
pub const ATA_PROGRAM_ID: Pubkey = Pubkey::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
pub const ED25519_PROGRAM_ID: Pubkey = Pubkey::from_str_const("Ed25519SigVerify111111111111111111111111111");
pub const INSTRUCTIONS_SYSVAR_ID: Pubkey = Pubkey::from_str_const("Sysvar1nstructions1111111111111111111111111");
pub const BPF_LOADER_UPGRADEABLE_ID: Pubkey = Pubkey::from_str_const("BPFLoaderUpgradeab1e11111111111111111111111");

pub const SEED_STATE: &[u8] = b"state";
pub const SEED_OFFER: &[u8] = b"offer";
pub const SEED_OFFER_VAULT_AUTHORITY: &[u8] = b"offer_vault_authority";
pub const SEED_PERMISSIONLESS_AUTHORITY: &[u8] = b"permissionless-1";
pub const SEED_MINT_AUTHORITY: &[u8] = b"mint_authority";
pub const SEED_REDEMPTION_OFFER: &[u8] = b"redemption_offer";
pub const SEED_REDEMPTION_OFFER_VAULT_AUTHORITY: &[u8] = b"redemption_offer_vault_authority";
pub const SEED_REDEMPTION_REQUEST: &[u8] = b"redemption_request";

// ---- programV5 seeds -------------------------------------------------------------------------
pub const SEED_MARKET_STATS: &[u8] = b"market_stats";
pub const SEED_CIRC_SUPPLY_EXCLUDED_ACCOUNTS: &[u8] = b"circ_supply_excl_accounts";
pub const SEED_CIRC_SUPPLY_EXCLUDED_BALANCE: &[u8] = b"circ_supply_excl_balance";
pub const SEED_BUFFER_STATE: &[u8] = b"buffer_state";
pub const SEED_RESERVE_VAULT_AUTHORITY: &[u8] = b"reserve_vault_authority";
pub const SEED_CONFIGURABLE_VAULT: &[u8] = b"configurable_vault";
pub const SEED_PROP_AMM_PAIR_STATE: &[u8] = b"prop_amm_pair";

/// The nine `ConfigurableVaultKind` seed suffixes, indexed by `ConfigurableVaultKind::as_u8()`
/// (`state.rs:120-148`). The generated enum has no `seed()` method — that lives on the program's
/// own type, not on the IDL-derived one — so the mapping is restated here and the two are pinned
/// together by `t_v5_configurable_vault_seed_table`.
pub const SCOUT_CV_SEEDS: [&[u8]; 9] = [
    b"offer_fee",
    b"management_fee",
    b"performance_fee",
    b"prop_amm_buy_fee",
    b"offer_proceeds",
    b"prop_amm_proceeds",
    b"permissionless_offer_fee",
    b"redemption_fee",
    b"prop_amm_sell_fee",
];

pub use onreapp::types::ConfigurableVaultKind as CvKind;

/// `ConfigurableVaultKind` by discriminant, so a fuzzer-chosen u64 can select one.
pub fn scout_vault_kind(sel: u64) -> onreapp::types::ConfigurableVaultKind {
    use onreapp::types::ConfigurableVaultKind as K;
    match sel % 9 {
        0 => K::OfferFee,
        1 => K::ManagementFee,
        2 => K::PerformanceFee,
        3 => K::PropAmmBuyFee,
        4 => K::OfferProceeds,
        5 => K::PropAmmProceeds,
        6 => K::PermissionlessOfferFee,
        7 => K::RedemptionFee,
        _ => K::PropAmmSellFee,
    }
}

/// The seed suffix of a `ConfigurableVaultKind`.
pub fn scout_vault_kind_seed(kind: onreapp::types::ConfigurableVaultKind) -> &'static [u8] {
    use onreapp::types::ConfigurableVaultKind as K;
    match kind {
        K::OfferFee => SCOUT_CV_SEEDS[0],
        K::ManagementFee => SCOUT_CV_SEEDS[1],
        K::PerformanceFee => SCOUT_CV_SEEDS[2],
        K::PropAmmBuyFee => SCOUT_CV_SEEDS[3],
        K::OfferProceeds => SCOUT_CV_SEEDS[4],
        K::PropAmmProceeds => SCOUT_CV_SEEDS[5],
        K::PermissionlessOfferFee => SCOUT_CV_SEEDS[6],
        K::RedemptionFee => SCOUT_CV_SEEDS[7],
        K::PropAmmSellFee => SCOUT_CV_SEEDS[8],
    }
}

/// Decimals chosen so the two legs differ — a same-decimals pair hides every scaling bug in
/// `calculate_token_out_amount` / `process_redemption_core`, which both convert across decimals.
pub const USDC_DECIMALS: u8 = 6;
pub const ONYC_DECIMALS: u8 = 9;

/// Starting balances. Large enough that a fuzzer-chosen u64 amount is usually affordable, small
/// enough that the u128 intermediates in the price math do not saturate on every call.
pub const USER_USDC_START: u64 = 1_000_000_000_000; // 1_000_000 USDC
pub const USER_ONYC_START: u64 = 100_000_000_000; // 100 ONyc
pub const VAULT_USDC_START: u64 = 1_000_000_000_000;

/// Upper bound on the redemption-request id space a property will walk. Beyond this the walk
/// returns None (check skipped) rather than a truncated sum -- a silently capped total would
/// under-report the claims against the vault and turn a real shortfall into a clean run.
pub const SCOUT_REQUEST_SCAN_CAP: u64 = 512;

/// Capacity of each property's registry of redemption requests it has seen created.
///
/// The predicate grammar admits only pure expressions plus a handful of trusted `ctx` reads, so a
/// property cannot derive a request PDA itself; it has to be handed the address at creation time.
/// If the ring wraps, the predicates bail out rather than sum a partial set — under-counting the
/// claims against the vault would turn a real shortfall into a clean run.
pub const SCOUT_REQ_CAP: usize = 32;

/// `RedemptionRequest`, borsh: 8 disc | offer 32 | request_id u64 8 | redeemer 32 | amount u64.
pub const SCOUT_REQ_AMOUNT_OFFSET: usize = 80;
pub const SCOUT_REQ_MIN_LEN: usize = 88;
// programV5 appends `fulfilled_amount: u64` after `bump`
// (redemption/redemption_offer_state.rs:130-137).
pub const SCOUT_REQ_FULFILLED_OFFSET: usize = 89;
pub const SCOUT_REQ_FULFILLED_END: usize = 97;
/// `RedemptionRequest.offer` — the redemption offer a request is locked against.
///
/// Lets a solvency predicate scope its sum from ON-CHAIN data instead of trusting that whoever
/// appended to its registry appended only the right requests. With this check a mis-registration
/// can only UNDER-count (a miss), never inflate the claim and manufacture a shortfall against a
/// vault that never received the funds (a false positive).
pub const SCOUT_REQ_OFFER_OFFSET: usize = 8;
pub const SCOUT_REQ_OFFER_END: usize = 40;

/// Anchor's closed-account sentinel. `#[account(close = ...)]` zeroes an account's lamports and
/// overwrites its 8-byte discriminator with this marker; the DATA may still be readable until the
/// runtime purges it. A solvency/conservation predicate that treats "account_data returned Ok" as
/// "the request is open" would therefore sum a retired request against an aggregate that has
/// already dropped it — an OVER-count, the direction that manufactures violations.
pub const SCOUT_CLOSED_ACCOUNT_DISCRIMINATOR: [u8; 8] = [255, 255, 255, 255, 255, 255, 255, 255];

/// `spl_token::state::Account`: mint 32 | owner 32 | amount u64 at 64.
pub const SCOUT_TOKEN_AMOUNT_OFFSET: usize = 64;
pub const SCOUT_TOKEN_MIN_LEN: usize = 72;

/// `RedemptionOffer`, borsh: 8 disc | offer 32 | token_in 32 | token_out 32 | executed u128 16
/// -> requested_redemptions u128 at 120.
pub const SCOUT_RO_REQUESTED_OFFSET: usize = 120;
pub const SCOUT_RO_REQUESTED_MID: usize = 128;
pub const SCOUT_RO_MIN_LEN: usize = 136;

/// `State`, borsh: 8 disc | boss 32 | proposed_boss 32 | is_killed 1 | onyc_mint 32 (73..105)
/// | admins [Pubkey;20] 640 | approver1 32 | approver2 32 | bump 1 -> max_supply u64 at 810.
pub const SCOUT_STATE_ONYC_MINT_OFFSET: usize = 73;
pub const SCOUT_STATE_ONYC_MINT_END: usize = 105;
pub const SCOUT_STATE_MAX_SUPPLY_OFFSET: usize = 810;
pub const SCOUT_STATE_MAX_SUPPLY_END: usize = 818;
// BufferState (buffer/state.rs:13-24): onyc_mint 8..40 | gross_apr 40..48 |
// previous_supply 48..56 | management_fee_bps 56..58 | performance_fee_bps 58..60 |
// performance_fee_high_watermark 60..68 | high_watermark_enabled 68 | last_accrual_timestamp
// 69..77 | bump 77 | reserved.
pub const SCOUT_BUF_GROSS_APR: usize = 40;
pub const SCOUT_BUF_PREVIOUS_SUPPLY: usize = 48;
pub const SCOUT_BUF_MGMT_FEE_BPS: usize = 56;
pub const SCOUT_BUF_PERF_FEE_BPS: usize = 58;
pub const SCOUT_BUF_HIGH_WATERMARK: usize = 60;
pub const SCOUT_BUF_HWM_ENABLED: usize = 68;
pub const SCOUT_BUF_LAST_ACCRUAL: usize = 69;
pub const SCOUT_BUF_MIN_LEN: usize = 78;

// MarketStats (state.rs:56-77): apy 8..16 | circulating_supply 16..24 | nav 24..32 |
// nav_adjustment 32..40 | tvl 40..48 | last_updated_at 48..56 | last_updated_slot 56..64 | bump 64.
pub const SCOUT_MS_APY: usize = 8;
pub const SCOUT_MS_CIRCULATING: usize = 16;
pub const SCOUT_MS_NAV: usize = 24;
pub const SCOUT_MS_NAV_ADJ: usize = 32;
pub const SCOUT_MS_TVL: usize = 40;
pub const SCOUT_MS_UPDATED_AT: usize = 48;
pub const SCOUT_MS_MIN_LEN: usize = 65;

// CirculatingSupplyExcludedBalance (state.rs:88-101): amount 8..16 | last_updated_at 16..24 |
// last_updated_slot 24..32 | bump 32.
pub const SCOUT_EXB_AMOUNT: usize = 8;
pub const SCOUT_EXB_MIN_LEN: usize = 33;

// programV5 appends `worker: Pubkey`, `max_mint_amount: u64`, `main_offer: Pubkey` after
// `max_supply` (state.rs:9-37). Pinned by `t_v5_state_layout_offsets`.
pub const SCOUT_STATE_WORKER_OFFSET: usize = 818;
pub const SCOUT_STATE_WORKER_END: usize = 850;
pub const SCOUT_STATE_MAIN_OFFER_OFFSET: usize = 858;
pub const SCOUT_STATE_MAIN_OFFER_END: usize = 890;

/// `spl_token::state::Mint`: mint_authority COption<Pubkey> 36 -> supply u64 at 36..44.
pub const SCOUT_MINT_SUPPLY_OFFSET: usize = 36;
pub const SCOUT_MINT_SUPPLY_END: usize = 44;

/// `Offer` (zero_copy, repr(C)): 8 disc | token_in_mint 32 (8..40) | token_out_mint 32 (40..72).
pub const SCOUT_OFFER_IN_MINT_OFFSET: usize = 8;
pub const SCOUT_OFFER_IN_MINT_END: usize = 40;
pub const SCOUT_OFFER_OUT_MINT_END: usize = 72;

/// Capacity of P-0005's registry of offer accounts.
pub const SCOUT_OFFER_CAP: usize = 16;

/// Capacity of P-0007's registry of ONyc-denominated redemption requests, pooled across EVERY
/// redemption offer whose token_in is ONyc — they all share one vault token account.
pub const SCOUT_P7_CAP: usize = 48;

/// `Offer` (zero_copy, repr(C), align 8): within-struct token_in_mint 0..32, token_out_mint 32..64,
/// vectors [OfferVector;10] 64..464 (5 x u64 each), fee_basis_points 464..466. Plus the 8-byte
/// account discriminator -> 472..474. Confirmed against the observed 608-byte account length.
pub const SCOUT_OFFER_FEE_OFFSET: usize = 472;
pub const SCOUT_OFFER_FEE_END: usize = 474;

/// `constants::MAX_ALLOWED_FEE_BPS` — the documented 10% ceiling on offer fees.
pub const SCOUT_MAX_ALLOWED_FEE_BPS: u16 = 1000;

pub fn scout_pda(seeds: &[&[u8]], program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(seeds, program_id).0
}

/// Associated-token address. Every token account in this program is constrained with
/// `associated_token::{mint, authority, token_program}`, so a plain random pubkey is always
/// rejected — each one has to be minted at exactly this address.
pub fn scout_ata(owner: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), token_program.as_ref(), mint.as_ref()],
        &ATA_PROGRAM_ID,
    )
    .0
}

/// Mint a pre-funded SPL token account at its canonical ATA address.
pub fn scout_mk_ata(
    ctx: &mut TestContext,
    owner: &Pubkey,
    mint: &Pubkey,
    amount: u64,
) -> Pubkey {
    let addr = scout_ata(owner, mint, &SPL_TOKEN_ID);
    ctx.create_token_account()
        .pubkey(addr)
        .mint(*mint)
        .token_owner(*owner)
        .amount(amount)
        .create()
        .expect("setup: create ATA");
    addr
}

// ---------------------------------------------------------------------------------------------
// Token-2022 transfer-fee mints — a FIXTURE CAPABILITY, not a convenience.
//
// The program's OFFER path explicitly refuses a fee-bearing mint on either leg
// (`token_utils.rs:374,378`). The REDEMPTION path has no such guard: `has_transfer_fee` has
// exactly two call sites in the entire program and both are inside `execute_token_operations`,
// which only `take_offer` / `take_offer_permissionless` reach. `create_redemption_request` calls
// `transfer_tokens` directly.
//
// Without a fee-bearing mint in the world, amplifier #8 (amount != accounting) is structurally
// UNOBSERVABLE on the redemption path — a clean campaign would mean "the fixture cannot express
// it", not "the program is safe". That distinction is the whole reason these exist.
// ---------------------------------------------------------------------------------------------

/// Transfer fee on `mint_fee`, in basis points. 1% is large enough that the shortfall is obvious
/// in a log line and small enough to stay far from the `maximum_fee` clamp.
pub const SCOUT_T22_FEE_BPS: u16 = 100;
pub const FEE_DECIMALS: u8 = 6;

/// Build a Token-2022 mint carrying a live `TransferFeeConfig`, owned by the Token-2022 program.
///
/// Both epoch slots carry the same fee, so the effective rate does not depend on which epoch the
/// SVM happens to be in — `has_transfer_fee` reads `get_epoch_fee(clock.epoch)`, and a fixture
/// whose fee silently switched off at an epoch boundary would produce an unreproducible campaign.
pub fn scout_mk_t22_fee_mint(
    ctx: &mut TestContext,
    authority: &Pubkey,
    decimals: u8,
    supply: u64,
    fee_bps: u16,
) -> Pubkey {
    use spl_token_2022_interface::extension::{
        transfer_fee::TransferFeeConfig, BaseStateWithExtensionsMut, ExtensionType,
        StateWithExtensionsMut,
    };
    use solana_program_option::COption;
    use spl_token_2022_interface::state::Mint as T22Mint;

    let addr = Keypair::new().pubkey();
    let len =
        ExtensionType::try_calculate_account_len::<T22Mint>(&[ExtensionType::TransferFeeConfig])
            .expect("setup: t22 mint account len");
    let mut data = vec![0u8; len];
    {
        let mut st = StateWithExtensionsMut::<T22Mint>::unpack_uninitialized(&mut data)
            .expect("setup: t22 unpack_uninitialized");
        let cfg = st
            .init_extension::<TransferFeeConfig>(true)
            .expect("setup: init TransferFeeConfig");
        for fee in [&mut cfg.older_transfer_fee, &mut cfg.newer_transfer_fee] {
            fee.epoch = 0u64.into();
            fee.maximum_fee = u64::MAX.into();
            fee.transfer_fee_basis_points = fee_bps.into();
        }
        st.base = T22Mint {
            mint_authority: COption::Some(*authority),
            supply,
            decimals,
            is_initialized: true,
            freeze_authority: COption::None,
        };
        st.pack_base();
        st.init_account_type().expect("setup: t22 init_account_type");
    }
    ctx.create_account()
        .pubkey(addr)
        .owner(SPL_TOKEN_2022_ID)
        .data(&data)
        .create()
        .expect("setup: create t22 fee mint");
    addr
}

/// Mint a pre-funded **Token-2022** account at its canonical ATA address.
///
/// A token account for a mint carrying `TransferFeeConfig` must itself carry `TransferFeeAmount`,
/// or the Token-2022 program rejects every transfer touching it. Building this by hand rather
/// than through `create_token_account()` is what that requirement forces.
pub fn scout_mk_t22_ata(
    ctx: &mut TestContext,
    owner: &Pubkey,
    mint: &Pubkey,
    amount: u64,
) -> Pubkey {
    use spl_token_2022_interface::extension::{
        transfer_fee::TransferFeeAmount, BaseStateWithExtensionsMut, ExtensionType,
        StateWithExtensionsMut,
    };
    use solana_program_option::COption;
    use spl_token_2022_interface::state::{Account as T22Account, AccountState as T22AccountState};

    let addr = scout_ata(owner, mint, &SPL_TOKEN_2022_ID);
    let len =
        ExtensionType::try_calculate_account_len::<T22Account>(&[ExtensionType::TransferFeeAmount])
            .expect("setup: t22 token account len");
    let mut data = vec![0u8; len];
    {
        let mut st = StateWithExtensionsMut::<T22Account>::unpack_uninitialized(&mut data)
            .expect("setup: t22 account unpack_uninitialized");
        st.init_extension::<TransferFeeAmount>(true)
            .expect("setup: init TransferFeeAmount");
        st.base = T22Account {
            mint: *mint,
            owner: *owner,
            amount,
            delegate: COption::None,
            state: T22AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };
        st.pack_base();
        st.init_account_type().expect("setup: t22 account type");
    }
    ctx.create_account()
        .pubkey(addr)
        .owner(SPL_TOKEN_2022_ID)
        .data(&data)
        .create()
        .expect("setup: create t22 ATA");
    addr
}

/// setup() must abort loudly on a failed prerequisite: a silently-skipped step leaves a world that
/// looks built but is not, and every downstream action then fails for a reason that has nothing to
/// do with the action.
pub fn scout_expect_ok(label: &str, out: anyhow::Result<TxOutcome>) {
    match out {
        Ok(o) if o.is_success() => {}
        Ok(o) => panic!(
            "setup step `{}` failed (code {:?})\nlogs:\n{}",
            label,
            o.error_code(),
            o.logs().join("\n")
        ),
        Err(e) => panic!("setup step `{}` errored: {e:?}", label),
    }
}

// ---------------------------------------------------------------------------------------------
// Clock control.
//
// Every price this program quotes is a function of `Clock::get()` — `calculate_step_price_at`
// snaps to the end of the current `price_fix_duration` window, and `find_active_vector_at` picks a
// vector by `start_time <= now`. A harness that never moves the clock quotes ONE price forever and
// silently deletes the entire class of bugs that only appears when the price moves between two
// user actions. So: pin the clock to a known epoch in setup, and expose advancing it as an action.
// ---------------------------------------------------------------------------------------------
pub const SCOUT_GENESIS_TS: i64 = 1_700_000_000;

pub fn scout_now(ctx: &TestContext) -> u64 {
    ctx.svm.get_sysvar::<crucible_fuzzer::anchor_lang::prelude::Clock>().unix_timestamp as u64
}

pub fn scout_set_time(ctx: &mut TestContext, unix_timestamp: i64) {
    let mut clock = ctx.svm.get_sysvar::<crucible_fuzzer::anchor_lang::prelude::Clock>();
    clock.unix_timestamp = unix_timestamp;
    ctx.set_sysvar(&clock);
}

/// Create the `State` account and wire up governance.
///
/// Shared by `setup()` and the `close_state` action hook: `close_state` deliberately deallocates
/// the state PDA, which would otherwise brick every remaining action in the iteration and read as
/// a coverage collapse rather than as the destructive-by-design instruction it is. Re-running this
/// afterwards restores the world WITHOUT weakening the action — close_state still really executes
/// and still really covers its own lines.
pub fn scout_bootstrap_state(
    ctx: &mut TestContext,
    program_id: Pubkey,
    boss: &Keypair,
    redemption_admin: &Pubkey,
    approver: &Pubkey,
    mint_onyc: Pubkey,
    state_pda: Pubkey,
    mint_authority_pda: Pubkey,
    offer_vault_authority: Pubkey,
) {
    let program_data = scout_pda(&[program_id.as_ref()], &BPF_LOADER_UPGRADEABLE_ID);
    scout_expect_ok(
        "initialize",
        ctx.program(program_id)
            .call(instruction::Initialize {})
            .accounts(accounts::Initialize {
                state: state_pda,
                mint_authority: mint_authority_pda,
                offer_vault_authority,
                boss: boss.pubkey(),
                program_data: Some(program_data),
                onyc_mint: mint_onyc,
            })
            .signers(&[boss])
            .send(),
    );
    // programV5 replaced the `redemption_admin` role with a single `worker` (State.worker), which
    // fulfils/cancels redemptions AND settles BUFFER. The harness keeps the actor named
    // `redemption_admin` so every existing binding and test still reads, but installs it as the
    // worker.
    scout_expect_ok(
        "set_worker",
        ctx.program(program_id)
            .call(instruction::SetWorker {
                new_worker: *redemption_admin,
            })
            .accounts(accounts::SetWorker {
                state: state_pda,
                boss: boss.pubkey(),
            })
            .signers(&[boss])
            .send(),
    );
    scout_expect_ok(
        "add_approver",
        ctx.program(program_id)
            .call(instruction::AddApprover {
                approver: *approver,
            })
            .accounts(accounts::AddApprover {
                state: state_pda,
                boss: boss.pubkey(),
            })
            .signers(&[boss])
            .send(),
    );
}

// ---------------------------------------------------------------------------------------------
// Ed25519 precompile instruction.
//
// Hand-built rather than pulled from a Solana crate because the target parses the layout itself
// (`utils/ed25519_parser.rs`) and additionally REQUIRES all three instruction indices to be
// u16::MAX — i.e. every field must live in this instruction's own data. The account list must be
// empty; `verify_approval_message_generic` rejects the instruction outright otherwise.
//
// Layout (16-byte header, then the fixed-position payload):
//   0      num_signatures = 1
//   1      padding
//   2..4   signature_offset            4..6   signature_instruction_index   = u16::MAX
//   6..8   public_key_offset           8..10  public_key_instruction_index  = u16::MAX
//   10..12 message_data_offset        12..14  message_data_size
//   14..16 message_instruction_index  = u16::MAX
// ---------------------------------------------------------------------------------------------
pub fn scout_ed25519_instruction(
    pubkey: &Pubkey,
    signature: &[u8; 64],
    message: &[u8],
) -> solana_instruction::Instruction {
    const HEADER: usize = 16;
    let pubkey_offset = HEADER;
    let signature_offset = pubkey_offset + 32;
    let message_offset = signature_offset + 64;

    let mut data = Vec::with_capacity(message_offset + message.len());
    data.push(1u8); // num_signatures
    data.push(0u8); // padding
    data.extend_from_slice(&(signature_offset as u16).to_le_bytes());
    data.extend_from_slice(&u16::MAX.to_le_bytes());
    data.extend_from_slice(&(pubkey_offset as u16).to_le_bytes());
    data.extend_from_slice(&u16::MAX.to_le_bytes());
    data.extend_from_slice(&(message_offset as u16).to_le_bytes());
    data.extend_from_slice(&(message.len() as u16).to_le_bytes());
    data.extend_from_slice(&u16::MAX.to_le_bytes());
    data.extend_from_slice(pubkey.as_ref());
    data.extend_from_slice(signature);
    data.extend_from_slice(message);

    solana_instruction::Instruction {
        program_id: ED25519_PROGRAM_ID,
        accounts: vec![],
        data,
    }
}

// ---------------------------------------------------------------------------------------------
// Fixture helpers.
//
// These live in SCOUT:PRELUDE rather than beside the generated actions because everything outside
// a SCOUT region is deleted verbatim on the next `scout regen`. A second `impl` block at file
// scope is regeneration-safe and `#[fuzz_fixture]` only inspects the block it is attached to, so
// nothing here is mistaken for an action.
// ---------------------------------------------------------------------------------------------
impl OnreappFixture {
    /// Pick one of the two non-privileged actors from a fuzzer-chosen value.
    ///
    /// Derived from the action's own amount argument so the FUZZER controls who acts. Hard-wiring
    /// a single user would make every adversary-value property vacuous: with one actor, every
    /// transfer is self-to-self and no one can end richer at anyone else's expense.
    pub fn pick_user(&self, sel: u64) -> Keypair {
        if sel & 1 == 0 {
            self.user_a.insecure_clone()
        } else {
            self.user_b.insecure_clone()
        }
    }

    pub fn pick_user_pk(&self, sel: u64) -> Pubkey {
        if sel & 1 == 0 {
            self.user_a.pubkey()
        } else {
            self.user_b.pubkey()
        }
    }

    /// Address of request `id` under an arbitrary redemption offer.
    pub fn request_pda_of(&self, ro: &Pubkey, id: u64) -> Pubkey {
        scout_pda(
            &[SEED_REDEMPTION_REQUEST, ro.as_ref(), &id.to_le_bytes()],
            &self.program_id,
        )
    }

    /// `request_counter` of an arbitrary redemption offer.
    pub fn request_counter_of(&self, ro: &Pubkey) -> Option<u64> {
        let data = self.ctx.account_data(ro).ok()?;
        if data.len() < 146 { return None; }
        Some(u64::from_le_bytes(data[138..146].try_into().ok()?))
    }

    /// Open requests under an arbitrary redemption offer, as `(id, redeemer, amount)`.
    pub fn open_requests_of(&self, ro: &Pubkey) -> Option<Vec<(u64, Pubkey, u64)>> {
        let counter = self.request_counter_of(ro)?;
        if counter > SCOUT_REQUEST_SCAN_CAP { return None; }
        let mut out = Vec::new();
        for id in 0..counter {
            let pda = self.request_pda_of(ro, id);
            let data = match self.ctx.account_data(&pda) {
                Ok(d) if d.len() >= SCOUT_REQ_MIN_LEN => d,
                _ => continue,
            };
            let redeemer = Pubkey::new_from_array(data[48..80].try_into().ok()?);
            let amount = u64::from_le_bytes(
                data[SCOUT_REQ_AMOUNT_OFFSET..SCOUT_REQ_MIN_LEN].try_into().ok()?,
            );
            // programV5 tracks PARTIAL fulfilment: `requested_redemptions` is decremented by each
            // bite (fulfill_redemption_request.rs:546) while the request lives on with its
            // ORIGINAL `amount` and a growing `fulfilled_amount`. What is still owed out of the
            // vault -- and what the offer's counter is supposed to equal -- is the REMAINDER.
            // Reporting the gross amount here made P-0003 read a 500_000 drift after every
            // partial fulfil that is not drift at all.
            let fulfilled = if data.len() >= SCOUT_REQ_FULFILLED_END {
                u64::from_le_bytes(
                    data[SCOUT_REQ_FULFILLED_OFFSET..SCOUT_REQ_FULFILLED_END].try_into().ok()?,
                )
            } else {
                0
            };
            out.push((id, redeemer, amount.saturating_sub(fulfilled)));
        }
        Some(out)
    }

    /// Current ONyc supply, read straight out of the mint account (offset 36 of spl Mint).
    pub fn onyc_supply(&self) -> Option<u64> {
        let data = self.ctx.account_data(&self.mint_onyc).ok()?;
        if data.len() < 44 { return None; }
        Some(u64::from_le_bytes(data[36..44].try_into().ok()?))
    }

    /// Address of the redemption request with a given id under the live redemption offer.
    pub fn request_pda(&self, id: u64) -> Pubkey {
        scout_pda(
            &[
                SEED_REDEMPTION_REQUEST,
                self.redemption_offer_pda.as_ref(),
                &id.to_le_bytes(),
            ],
            &self.program_id,
        )
    }

    /// `RedemptionOffer.request_counter` — the seed the NEXT request will be derived from.
    ///
    /// Read from the chain rather than mirrored harness-side. A mirror would be a second source of
    /// truth that can silently drift from the program, and every property below would then be
    /// measuring the drift rather than the protocol.
    pub fn onchain_request_counter(&self) -> Option<u64> {
        let data = self.ctx.account_data(&self.redemption_offer_pda).ok()?;
        // borsh, no padding: 8 disc | offer 32 | token_in 32 | token_out 32 | executed u128 16
        //   -> requested u128 16 (120..136) | fee_basis_points u16 (136..138) | counter u64 138..146
        if data.len() < 146 { return None; }
        Some(u64::from_le_bytes(data[138..146].try_into().ok()?))
    }

    /// `RedemptionOffer.requested_redemptions` as the program currently records it.
    pub fn onchain_requested_redemptions(&self) -> Option<u128> {
        let data = self.ctx.account_data(&self.redemption_offer_pda).ok()?;
        if data.len() < 136 { return None; }
        Some(u128::from_le_bytes(data[120..136].try_into().ok()?))
    }

    /// Every redemption request account that still exists, as `(id, redeemer, amount)`.
    ///
    /// Ground truth, walked from the chain: fulfil and cancel both `close` the account, so
    /// "the account is still there" IS "the request is still open" — there is no status field to
    /// misread and no shadow ledger to desynchronise.
    ///
    /// Returns `None` rather than a partial answer if the counter has run past `SCOUT_REQUEST_SCAN_CAP`;
    /// a silently truncated sum would under-report and turn a real shortfall into a clean run.
    pub fn open_requests(&self) -> Option<Vec<(u64, Pubkey, u64)>> {
        let counter = self.onchain_request_counter()?;
        if counter > SCOUT_REQUEST_SCAN_CAP { return None; }
        let mut out = Vec::new();
        for id in 0..counter {
            let pda = self.request_pda(id);
            let data = match self.ctx.account_data(&pda) {
                Ok(d) if d.len() >= 88 => d,
                _ => continue, // closed by fulfil/cancel, or never created
            };
            // 8 disc | offer 32 | request_id u64 8 | redeemer 32 (48..80) | amount u64 80..88
            let redeemer = Pubkey::new_from_array(data[48..80].try_into().ok()?);
            let amount = u64::from_le_bytes(data[80..88].try_into().ok()?);
            out.push((id, redeemer, amount));
        }
        Some(out)
    }

    /// Total token_in still locked on behalf of open requests, denominated in the live redemption
    /// offer's token_in (ONyc — the bindings pin `create_redemption_request` to that offer).
    pub fn open_request_total(&self) -> Option<u128> {
        Some(self.open_requests()?.iter().map(|(_, _, a)| *a as u128).sum())
    }

    /// PDA the next `create_redemption_request` will open.
    pub fn next_request_pda(&self) -> Pubkey {
        self.request_pda(self.onchain_request_counter().unwrap_or(0))
    }

    /// The oldest still-open request — what fulfil and cancel are bound to operate on.
    ///
    /// With no open request this returns the NEXT request's address, which does not exist yet, so
    /// the action fails. That is the honest outcome: there is nothing to fulfil or cancel.
    pub fn oldest_request_pda(&self) -> Pubkey {
        match self.open_requests().as_ref().and_then(|v| v.first()) {
            Some((id, _, _)) => self.request_pda(*id),
            None => self.next_request_pda(),
        }
    }

    /// What is still unfulfilled on the oldest open request.
    ///
    /// programV5 made `fulfill_redemption_request` PARTIAL: it takes an `amount` which must be
    /// `> 0` and `<= request.amount - request.fulfilled_amount`
    /// (`fulfill_redemption_request.rs:416-426`). 0 means "there is nothing to fulfil".
    pub fn oldest_request_remaining(&self) -> u64 {
        let pda = self.oldest_request_pda();
        // 8 disc | offer 32 | request_id 8 | redeemer 32 | amount 80..88 | bump 88 | fulfilled 89..97
        match self.ctx.account_data(&pda) {
            Ok(d) if d.len() >= 97 => {
                let amount = u64::from_le_bytes(d[80..88].try_into().unwrap_or([0; 8]));
                let fulfilled = u64::from_le_bytes(d[89..97].try_into().unwrap_or([0; 8]));
                amount.saturating_sub(fulfilled)
            }
            _ => 0,
        }
    }

    /// The `amount` a fulfil action should ask for: the fuzzer's value clamped into
    /// `1..=remaining`, so partial fulfilment stays reachable while the action still succeeds.
    /// Returns 0 when there is nothing open, which the program rejects — the honest outcome.
    pub fn fulfill_amount(&self, sel: u64) -> u64 {
        let remaining = self.oldest_request_remaining();
        if remaining == 0 {
            return 0;
        }
        // Half the draws settle the request outright, half take a partial bite; both branches of
        // `is_fully_fulfilled` (fulfill_redemption_request.rs:540) matter.
        if sel & 1 == 0 {
            remaining
        //
        // The partial bite must be a FRACTION, not `(sel % remaining) + 1`. The payout is
        // `floor(net * price * 10^out_dec / 10^(in_dec + 9))` (redemption_utils.rs:83-99) and this
        // leg is ONyc(9) -> usdc(6), so three decimal places are discarded: at price ~1.0 any bite
        // below ~1_000 base units pays out zero and is rejected by
        // `require!(result > 0, InvalidAmount)` (redemption_utils.rs:102). `sel` is the internal
        // tick, so the old expression handed the very FIRST fulfil `amount = 2` and the action
        // could never succeed. Half the remainder keeps the partial branch reachable and stays
        // clear of the rounding dead zone for every request the harness can open.
        } else {
            (remaining / 2).max(1)
        }
    }

    /// An `asset_adjustment_amount` `burn_for_nav_increase` can actually satisfy.
    ///
    /// At an unchanged NAV the burn it computes is approximately the adjustment itself, so the
    /// binding constraint is the reserve vault's ONyc balance. A quarter of it leaves headroom for
    /// the rounding in `ceil_div_u128` and for the accrual that runs first. Returns 0 when the
    /// reserve is empty, which the program rejects — the honest outcome, and the branch worth
    /// keeping reachable.
    pub fn nav_burn_amount_next(&mut self) -> u64 {
        self.scout_fulfill_tick = self.scout_fulfill_tick.wrapping_add(1);
        let reserve = self.ctx.token_balance(&self.reserve_vault_onyc());
        if reserve == 0 {
            return 0;
        }
        if self.scout_fulfill_tick % 4 == 0 {
            0
        } else {
            (reserve / 4).max(1)
        }
    }

    /// `fulfill_amount` with the internal selector, advanced once per call.
    pub fn fulfill_amount_next(&mut self) -> u64 {
        self.scout_fulfill_tick = self.scout_fulfill_tick.wrapping_add(1);
        let sel = self.scout_fulfill_tick;
        self.fulfill_amount(sel)
    }

    pub fn oldest_request_redeemer(&self) -> Pubkey {
        match self.open_requests().as_ref().and_then(|v| v.first()) {
            Some((_, redeemer, _)) => *redeemer,
            None => self.user_a.pubkey(),
        }
    }

    /// Whether the program's `State` account is currently allocated. `close_state` deallocates it.
    pub fn state_exists(&self) -> bool {
        self.ctx.account_data(&self.state_pda).map(|d| d.len() >= 40).unwrap_or(false)
    }

    /// The (token_in, token_out) pair the generated `make_offer` action uses, chosen from its own
    /// fee argument so the FUZZER decides.
    ///
    /// Variant 3 puts the SAME mint on both legs. That is not a harness contrivance: `make_offer`
    /// relates the two mint arguments nowhere, so this is an ordinary accepted call — and it is the
    /// only way the fuzzer can reach the state P-0005 forbids through a real IDL instruction rather
    /// than a bespoke harness action.
    pub fn make_offer_pair(&self, sel: u16) -> (Pubkey, Pubkey) {
        match sel % 4 {
            0 => (self.mint_play, self.mint_onyc),
            1 => (self.mint_play, self.mint_usdc),
            2 => (self.mint_appr, self.mint_usdc),
            _ => (self.mint_onyc, self.mint_onyc),
        }
    }

    pub fn make_offer_pda_for(&self, sel: u16) -> Pubkey {
        let (a, b) = self.make_offer_pair(sel);
        scout_pda(&[SEED_OFFER, a.as_ref(), b.as_ref()], &self.program_id)
    }

    /// Which vault mint a vault-operation action targets, chosen from a fuzzer value.

    ///
    /// Pinning these to one mint would leave the ONyc redemption vault — the account that actually
    /// custodies user deposits from `create_redemption_request` — untouchable by
    /// `redemption_vault_withdraw`/`_deposit`, and P-0002's whole subject would be unreachable.
    pub fn pick_vault_mint(&self, sel: u64) -> Pubkey {
        if sel & 1 == 0 { self.mint_usdc } else { self.mint_onyc }
    }

    // ---- programV5 role/config readers ---------------------------------------------------------
    //
    // `set_worker` and `set_main_offer` both `require!(new != current)`, so an action pinned to the
    // value setup() installed can NEVER succeed. Both therefore propose the OTHER of two candidates
    // the harness owns — and every consumer reads the CURRENT value off `State` rather than
    // assuming setup()'s, so flipping either one does not silently kill the instructions that
    // depend on it (a self-inflicted coverage collapse, not a finding).

    fn state_pubkey_at(&self, start: usize, end: usize) -> Option<Pubkey> {
        let data = self.ctx.account_data(&self.state_pda).ok()?;
        if data.len() < end {
            return None;
        }
        Some(Pubkey::new_from_array(data[start..end].try_into().ok()?))
    }

    /// `state.worker` — who may fulfil/cancel redemptions and settle BUFFER.
    pub fn state_worker(&self) -> Pubkey {
        self.state_pubkey_at(SCOUT_STATE_WORKER_OFFSET, SCOUT_STATE_WORKER_END)
            .unwrap_or_else(|| self.redemption_admin.pubkey())
    }

    /// The keypair for whichever worker is currently installed.
    pub fn worker_kp(&self) -> Rc<Keypair> {
        if self.state_worker() == self.worker_alt.pubkey() {
            self.worker_alt.clone()
        } else {
            self.redemption_admin.clone()
        }
    }

    /// The worker `set_worker` should propose: always the other one, so the instruction can succeed.
    pub fn other_worker(&self) -> Pubkey {
        if self.state_worker() == self.redemption_admin.pubkey() {
            self.worker_alt.pubkey()
        } else {
            self.redemption_admin.pubkey()
        }
    }

    /// `state.max_supply` — 0 disables the cap.
    pub fn state_max_supply(&self) -> Option<u64> {
        let data = self.ctx.account_data(&self.state_pda).ok()?;
        if data.len() < SCOUT_STATE_MAX_SUPPLY_END {
            return None;
        }
        Some(u64::from_le_bytes(
            data[SCOUT_STATE_MAX_SUPPLY_OFFSET..SCOUT_STATE_MAX_SUPPLY_END].try_into().ok()?,
        ))
    }

    /// `state.main_offer` — the canonical ONyc offer every price-dependent path reads.
    pub fn state_main_offer(&self) -> Pubkey {
        self.state_pubkey_at(SCOUT_STATE_MAIN_OFFER_OFFSET, SCOUT_STATE_MAIN_OFFER_END)
            .unwrap_or(self.offer_pda)
    }

    /// The offer to propose to `set_main_offer`, rotating across its three outcomes: the other
    /// valid ONyc-out offer (accepted), the incumbent (`NoChange`), and the usdc -> fee offer,
    /// whose token_out is not ONyc (`InvalidTokenOutMint`).
    pub fn next_main_offer_candidate(&mut self) -> Pubkey {
        self.scout_fulfill_tick = self.scout_fulfill_tick.wrapping_add(1);
        match self.scout_fulfill_tick % 4 {
            0 => self.state_main_offer(),
            1 => self.offer_fee_pda,
            _ => self.other_main_offer(),
        }
    }

    /// The offer `set_main_offer` should propose. Both candidates have ONyc as token_out, which
    /// `set_main_offer.rs:31-35` requires.
    pub fn other_main_offer(&self) -> Pubkey {
        if self.state_main_offer() == self.offer_pda {
            self.offer_appr_pda
        } else {
            self.offer_pda
        }
    }

    // ---- programV5 state readers ---------------------------------------------------------------
    //
    // The invariant predicate grammar admits no helper calls, so every property reads its inputs
    // through fields or through these, and each returns an Option so that "the account does not
    // exist yet" loses an observation instead of manufacturing one.

    fn read_u64_at(&self, key: &Pubkey, off: usize, min_len: usize) -> Option<u64> {
        let data = self.ctx.account_data(key).ok()?;
        if data.len() < min_len {
            return None;
        }
        Some(u64::from_le_bytes(data[off..off + 8].try_into().ok()?))
    }

    fn read_i64_at(&self, key: &Pubkey, off: usize, min_len: usize) -> Option<i64> {
        let data = self.ctx.account_data(key).ok()?;
        if data.len() < min_len {
            return None;
        }
        Some(i64::from_le_bytes(data[off..off + 8].try_into().ok()?))
    }

    /// `BufferState.previous_supply` — the baseline the NEXT accrual interval charges yield on.
    pub fn buffer_previous_supply(&self) -> Option<u64> {
        self.read_u64_at(&self.buffer_state_pda(), SCOUT_BUF_PREVIOUS_SUPPLY, SCOUT_BUF_MIN_LEN)
    }

    pub fn buffer_last_accrual(&self) -> Option<i64> {
        self.read_i64_at(&self.buffer_state_pda(), SCOUT_BUF_LAST_ACCRUAL, SCOUT_BUF_MIN_LEN)
    }

    pub fn buffer_high_watermark(&self) -> Option<u64> {
        self.read_u64_at(&self.buffer_state_pda(), SCOUT_BUF_HIGH_WATERMARK, SCOUT_BUF_MIN_LEN)
    }

    pub fn market_stats_tvl(&self) -> Option<u64> {
        self.read_u64_at(&self.market_stats_pda(), SCOUT_MS_TVL, SCOUT_MS_MIN_LEN)
    }

    pub fn market_stats_nav(&self) -> Option<u64> {
        self.read_u64_at(&self.market_stats_pda(), SCOUT_MS_NAV, SCOUT_MS_MIN_LEN)
    }

    pub fn market_stats_circulating(&self) -> Option<u64> {
        self.read_u64_at(&self.market_stats_pda(), SCOUT_MS_CIRCULATING, SCOUT_MS_MIN_LEN)
    }

    /// The CACHED excluded balance — what every circulating-supply computation subtracts.
    pub fn excluded_balance_cached(&self) -> Option<u64> {
        self.read_u64_at(&self.excluded_balance_pda(), SCOUT_EXB_AMOUNT, SCOUT_EXB_MIN_LEN)
    }

    /// The LIVE excluded balance: the ONyc actually held by the configured excluded owners.
    pub fn excluded_balance_live(&self) -> u64 {
        self.excluded_owner_atas()
            .iter()
            .map(|ata| self.ctx.token_balance(ata))
            .fold(0u64, |acc, v| acc.saturating_add(v))
    }

    // ---- programV5 PDAs -----------------------------------------------------------------------

    /// The `ConfigurableVault` authority PDA for one vault kind.
    pub fn cv_pda(&self, kind: onreapp::types::ConfigurableVaultKind) -> Pubkey {
        scout_pda(
            &[SEED_CONFIGURABLE_VAULT, scout_vault_kind_seed(kind)],
            &self.program_id,
        )
    }

    /// That vault's ATA for a mint.
    pub fn cv_ata(&self, kind: onreapp::types::ConfigurableVaultKind, mint: &Pubkey) -> Pubkey {
        scout_ata(&self.cv_pda(kind), mint, &SPL_TOKEN_ID)
    }

    /// The mint a vault kind actually accumulates.
    ///
    /// The offer, permissionless-offer and prop-amm-BUY routes all pay their fee and proceeds in
    /// the offer's token_in (usdc here); BUFFER accrual, redemption fulfilment and the prop-amm
    /// SELL route pay in ONyc. A vault asked for the wrong mint holds nothing, so the withdrawal
    /// would only ever reach its `ZeroBalance` branch.
    pub fn cv_mint(&self, kind: CvKind) -> Pubkey {
        match kind {
            CvKind::OfferFee
            | CvKind::PermissionlessOfferFee
            | CvKind::OfferProceeds
            | CvKind::PropAmmBuyFee
            | CvKind::PropAmmProceeds => self.mint_usdc,
            CvKind::ManagementFee
            | CvKind::PerformanceFee
            | CvKind::RedemptionFee
            | CvKind::PropAmmSellFee => self.mint_onyc,
        }
    }

    /// A vault kind that `withdraw_configurable_vault` can actually satisfy, chosen from a fuzzer
    /// value.
    ///
    /// The instruction needs THREE things to line up at once: the vault must have a
    /// `withdrawal_destination` recorded (`withdraw.rs:75-78`), its token account must hold a
    /// positive balance in that kind's mint (`:84`), and the caller must pass the matching
    /// destination ATA. A kind picked blind from the fuzzer's amount satisfies all three about
    /// never — measured 0 successes in 21 selections across a corpus replay. This scans the nine
    /// kinds and prefers one that is ready, starting the scan at the fuzzer's own offset so the
    /// choice still varies; with none ready it falls back to the blind pick, which keeps the
    /// `MissingConfigurableVaultDestination` and `ZeroBalance` rejection branches reachable.
    pub fn pick_withdrawable_kind(&self, sel: u64) -> CvKind {
        let start = (sel % 9) as usize;
        for offset in 0..9usize {
            let kind = scout_vault_kind(((start + offset) % 9) as u64);
            if self.cv_destination(kind) == Pubkey::default() {
                continue;
            }
            let mint = self.cv_mint(kind);
            if self.ctx.token_balance(&self.cv_ata(kind, &mint)) > 0 {
                return kind;
            }
        }
        scout_vault_kind(sel)
    }

    /// The `start_time` of a pricing vector that actually exists on the main offer.
    ///
    /// `delete_offer_vector` deletes BY START TIME, not by index (`lib.rs:241`), so a raw fuzzer
    /// u64 matches an existing vector with probability ~2^-64 and the action can never succeed —
    /// measured 0 successes in 3 selections, and it would have been 0 in any number. The scan reads
    /// the ten slots of the zero-copy `Offer` (8 disc + 32 + 32, then ten 40-byte vectors:
    /// `offer_state.rs:12-33`), takes the live ones, and picks with the fuzzer's own value. With no
    /// vector present it returns the raw value, which keeps the not-found rejection reachable.
    /// The `start_time` for the next `add_offer_vector`: alternately immediate (`None`) and
    /// scheduled a day out. Scheduled vectors are the only ones `delete_offer_vector` may remove.
    pub fn next_vector_start_time(&mut self) -> Option<u64> {
        self.scout_fulfill_tick = self.scout_fulfill_tick.wrapping_add(1);
        if self.scout_fulfill_tick % 2 == 0 {
            None
        } else {
            Some(scout_now(&self.ctx).saturating_add(86_400))
        }
    }

    /// `pick_offer_vector_start_time` with the internal selector, advanced once per call. Binding
    /// the argument removes it from the action signature, so the rotation is the only source of
    /// variation left — the fuzzer still controls how many deletes precede this one, and therefore
    /// which vector it lands on.
    pub fn pick_offer_vector_start_time_next(&mut self) -> u64 {
        self.scout_fulfill_tick = self.scout_fulfill_tick.wrapping_add(1);
        let sel = self.scout_fulfill_tick;
        self.pick_offer_vector_start_time(sel)
    }

    pub fn pick_offer_vector_start_time(&self, sel: u64) -> u64 {
        const VECTORS_OFFSET: usize = 8 + 32 + 32;
        const VECTOR_LEN: usize = 40;
        let data = match self.ctx.account_data(&self.offer_pda) {
            Ok(d) => d,
            Err(_) => return sel,
        };
        let mut live = [0u64; 10];
        let mut n = 0usize;
        for i in 0..10usize {
            let at = VECTORS_OFFSET + i * VECTOR_LEN;
            if data.len() < at + 8 {
                break;
            }
            let start = u64::from_le_bytes(data[at..at + 8].try_into().unwrap_or([0; 8]));
            if start != 0 {
                live[n] = start;
                n += 1;
            }
        }
        if n == 0 {
            return sel;
        }
        // `delete_offer_vector.rs:94` refuses anything not strictly in the future, so prefer a
        // scheduled vector; with none scheduled, return a live one anyway and let the instruction
        // reject it — that rejection is a real branch and worth covering.
        let now = scout_now(&self.ctx);
        let mut future = [0u64; 10];
        let mut m = 0usize;
        for i in 0..n {
            if live[i] > now {
                future[m] = live[i];
                m += 1;
            }
        }
        if m > 0 {
            return future[(sel as usize) % m];
        }
        live[(sel as usize) % n]
    }

    /// Execute a Prop AMM SELL with an explicit size, bypassing the generated action's bound
    /// amount. Only the differential tests need this — the fuzzer's own sell sizes come from
    /// `swap_sell_amount_next`.
    pub fn scout_swap_sell(&mut self, user_sel: u64, token_in_amount: u64, minimum_out: u64) -> bool {
        let user_kp = self.pick_user(user_sel);
        let user = user_kp.pubkey();
        let (mint_onyc, mint_usdc) = (self.mint_onyc, self.mint_usdc);
        let rva = self.redemption_vault_authority;
        let ova = self.offer_vault_authority;
        let pid = self.program_id;
        let accounts = accounts::OpenSwapSell {
            offer: self.offer_pda,
            prop_amm_pair_state: self.prop_amm_pair_pda(),
            redemption_offer: self.redemption_offer_pda,
            state: self.state_pda,
            offer_vault_authority: ova,
            redemption_vault_authority: rva,
            redemption_vault_token_in_account: scout_ata(&rva, &mint_onyc, &SPL_TOKEN_ID),
            redemption_vault_token_out_account: scout_ata(&rva, &mint_usdc, &SPL_TOKEN_ID),
            token_in_mint: mint_onyc,
            token_in_program: SPL_TOKEN_ID,
            token_out_mint: mint_usdc,
            token_out_program: SPL_TOKEN_ID,
            user_token_in_account: scout_ata(&user, &mint_onyc, &SPL_TOKEN_ID),
            user_token_out_account: scout_ata(&user, &mint_usdc, &SPL_TOKEN_ID),
            prop_amm_proceeds_vault: self.cv_pda(CvKind::PropAmmProceeds),
            prop_amm_proceeds_token_in_account: self.cv_ata(CvKind::PropAmmProceeds, &mint_onyc),
            prop_amm_sell_fee_vault: self.cv_pda(CvKind::PropAmmSellFee),
            prop_amm_sell_fee_token_in_account: self.cv_ata(CvKind::PropAmmSellFee, &mint_onyc),
            mint_authority: self.mint_authority_pda,
            buffer_state: self.buffer_state_pda(),
            reserve_vault_onyc_account: self.reserve_vault_onyc(),
            management_fee_vault_onyc_account: self.cv_ata(CvKind::ManagementFee, &mint_onyc),
            performance_fee_vault_onyc_account: self.cv_ata(CvKind::PerformanceFee, &mint_onyc),
            market_stats: self.market_stats_pda(),
            circulating_supply_excluded_balance: self.excluded_balance_pda(),
            instructions_sysvar: INSTRUCTIONS_SYSVAR_ID,
            user,
            main_offer: self.state_main_offer(),
            offer_vault_onyc_account: scout_ata(&ova, &mint_onyc, &SPL_TOKEN_ID),
        };
        self.ctx
            .program(pid)
            .call(instruction::OpenSwapSell { token_in_amount, minimum_out })
            .accounts(accounts)
            .signers(&[&user_kp])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false)
    }

    /// Execute a Prop AMM BUY with an explicit size. See `scout_swap_sell`.
    pub fn scout_swap_buy(&mut self, user_sel: u64, token_in_amount: u64, minimum_out: u64) -> bool {
        let user_kp = self.pick_user(user_sel);
        let user = user_kp.pubkey();
        let (mint_onyc, mint_usdc) = (self.mint_onyc, self.mint_usdc);
        let rva = self.redemption_vault_authority;
        let ova = self.offer_vault_authority;
        let pa = self.permissionless_authority;
        let pid = self.program_id;
        let accounts = accounts::OpenSwapBuy {
            offer: self.offer_pda,
            prop_amm_pair_state: self.prop_amm_pair_pda(),
            redemption_offer: self.redemption_offer_pda,
            state: self.state_pda,
            offer_vault_authority: ova,
            redemption_vault_authority: rva,
            offer_vault_token_in_account: scout_ata(&ova, &mint_usdc, &SPL_TOKEN_ID),
            offer_vault_token_out_account: scout_ata(&ova, &mint_onyc, &SPL_TOKEN_ID),
            redemption_vault_token_in_account: scout_ata(&rva, &mint_usdc, &SPL_TOKEN_ID),
            token_in_mint: mint_usdc,
            token_in_program: SPL_TOKEN_ID,
            token_out_mint: mint_onyc,
            token_out_program: SPL_TOKEN_ID,
            user_token_in_account: scout_ata(&user, &mint_usdc, &SPL_TOKEN_ID),
            user_token_out_account: scout_ata(&user, &mint_onyc, &SPL_TOKEN_ID),
            prop_amm_proceeds_vault: self.cv_pda(CvKind::PropAmmProceeds),
            prop_amm_proceeds_token_in_account: self.cv_ata(CvKind::PropAmmProceeds, &mint_usdc),
            prop_amm_buy_fee_vault: self.cv_pda(CvKind::PropAmmBuyFee),
            prop_amm_buy_fee_token_in_account: self.cv_ata(CvKind::PropAmmBuyFee, &mint_usdc),
            permissionless_authority: pa,
            permissionless_token_in_account: scout_ata(&pa, &mint_usdc, &SPL_TOKEN_ID),
            permissionless_token_out_account: scout_ata(&pa, &mint_onyc, &SPL_TOKEN_ID),
            mint_authority: self.mint_authority_pda,
            buffer_state: self.buffer_state_pda(),
            reserve_vault_onyc_account: self.reserve_vault_onyc(),
            management_fee_vault_onyc_account: self.cv_ata(CvKind::ManagementFee, &mint_onyc),
            performance_fee_vault_onyc_account: self.cv_ata(CvKind::PerformanceFee, &mint_onyc),
            market_stats: self.market_stats_pda(),
            circulating_supply_excluded_balance: self.excluded_balance_pda(),
            instructions_sysvar: INSTRUCTIONS_SYSVAR_ID,
            user,
            main_offer: self.state_main_offer(),
        };
        self.ctx
            .program(pid)
            .call(instruction::OpenSwapBuy { token_in_amount, minimum_out })
            .accounts(accounts)
            .signers(&[&user_kp])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false)
    }

    /// A Prop AMM sell size the pair can actually price, rotating across the interesting sizes.
    ///
    /// `apply_hard_wall_liquidity_factor` rejects outright when the raw pair-asset value exceeds
    /// the redemption vault's balance (`pricing.rs:148-152`), and the redemption fee floor
    /// (`minimum_sell_haircut_onyc`, 5 ONyc by default) rejects anything too small. A raw fuzzer
    /// u64 is outside that window essentially always — measured 0 successes in 20 selections. The
    /// tick walks a fraction of the seller's own ONyc balance and every fourth draw is left raw, so
    /// the over-the-wall rejection stays reachable.
    pub fn swap_sell_amount_next(&mut self) -> u64 {
        self.scout_fulfill_tick = self.scout_fulfill_tick.wrapping_add(1);
        let tick = self.scout_fulfill_tick;
        let held = self
            .ctx
            .token_balance(&scout_ata(&self.pick_user_pk(0), &self.mint_onyc, &SPL_TOKEN_ID));
        if held == 0 {
            return 0;
        }
        match tick % 4 {
            0 => held / 100,
            1 => held / 20,
            2 => held / 4,
            _ => held.saturating_mul(3),
        }
    }

    /// The withdrawal destination a vault currently records, or the default key when the vault
    /// does not exist yet (which `withdraw_configurable_vault` rejects — the honest outcome).
    ///
    /// Layout: 8 disc | kind u8 | withdrawal_destination 32 (9..41) | bump | reserved
    /// (`state.rs:150-160`).
    pub fn cv_destination(&self, kind: CvKind) -> Pubkey {
        match self.ctx.account_data(&self.cv_pda(kind)) {
            Ok(d) if d.len() >= 41 => d[9..41]
                .try_into()
                .map(Pubkey::new_from_array)
                .unwrap_or_default(),
            _ => Pubkey::default(),
        }
    }

    /// A `ConfigurableVaultKind` chosen from a fuzzer value, so one action reaches all nine.
    pub fn pick_vault_kind(&self, sel: u64) -> onreapp::types::ConfigurableVaultKind {
        scout_vault_kind(sel)
    }

    /// The `PropAmmPairState` for the MAIN usdc -> onyc offer.
    ///
    /// There is one pair state per offer (`[PROP_AMM_PAIR_STATE, offer]`,
    /// `prop_amm/config.rs:96`) and setup configures exactly one: the main offer's. Both swap
    /// sides resolve to that same offer, because `validate_canonical_offer`
    /// (`prop_amm/validation.rs:43`) always derives `[OFFER, asset_mint, onyc_mint]` -- asset
    /// first -- regardless of which way round the swap runs. So buy (usdc in, ONyc out) and sell
    /// (ONyc in, usdc out) share one `offer` and one pair state.
    pub fn prop_amm_pair_pda(&self) -> Pubkey {
        scout_pda(&[SEED_PROP_AMM_PAIR_STATE, self.offer_pda.as_ref()], &self.program_id)
    }

    pub fn market_stats_pda(&self) -> Pubkey {
        scout_pda(&[SEED_MARKET_STATS], &self.program_id)
    }

    pub fn excluded_accounts_pda(&self) -> Pubkey {
        scout_pda(&[SEED_CIRC_SUPPLY_EXCLUDED_ACCOUNTS], &self.program_id)
    }

    pub fn excluded_balance_pda(&self) -> Pubkey {
        scout_pda(&[SEED_CIRC_SUPPLY_EXCLUDED_BALANCE], &self.program_id)
    }

    pub fn buffer_state_pda(&self) -> Pubkey {
        scout_pda(&[SEED_BUFFER_STATE], &self.program_id)
    }

    pub fn reserve_vault_authority(&self) -> Pubkey {
        scout_pda(&[SEED_RESERVE_VAULT_AUTHORITY], &self.program_id)
    }

    /// The BUFFER reserve's ONyc ATA.
    pub fn reserve_vault_onyc(&self) -> Pubkey {
        scout_ata(&self.reserve_vault_authority(), &self.mint_onyc, &SPL_TOKEN_ID)
    }

    /// The ONyc ATAs of the excluded owners, in list order -- what
    /// `update_circulating_supply_excluded_balance` expects as its remaining accounts.
    pub fn excluded_owner_atas(&self) -> Vec<Pubkey> {
        self.excluded_owner_list()
            .iter()
            .filter(|owner| **owner != Pubkey::default())
            .map(|owner| scout_ata(owner, &self.mint_onyc, &SPL_TOKEN_ID))
            .collect()
    }

    /// The owner list `set_circulating_supply_excluded_accounts` installs: the two BUFFER-adjacent
    /// program vaults plus the boss, padded with the default key (which the program treats as an
    /// empty slot).
    pub fn excluded_owner_list(&self) -> [Pubkey; 20] {
        let mut owners = [Pubkey::default(); 20];
        owners[0] = self.reserve_vault_authority();
        owners[1] = self.offer_vault_authority;
        owners[2] = self.boss.pubkey();
        owners
    }

    /// The single ATA that custodies every redemption deposit for a mint.
    ///
    /// `seeds::REDEMPTION_OFFER_VAULT_AUTHORITY` carries no per-offer discriminator, so this one
    /// account backs every redemption offer sharing the mint.
    pub fn redemption_vault_ata(&self, mint: &Pubkey) -> Pubkey {
        scout_ata(&self.redemption_vault_authority, mint, &SPL_TOKEN_ID)
    }

    /// Rebuild `State` after `close_state` deallocated it. See `scout_bootstrap_state`.
    pub fn rebuild_state(&mut self) {
        let boss = self.boss.insecure_clone();
        let redemption_admin = self.redemption_admin.pubkey();
        let approver = self.approver.pubkey();
        scout_bootstrap_state(
            &mut self.ctx,
            self.program_id,
            &boss,
            &redemption_admin,
            &approver,
            self.mint_onyc,
            self.state_pda,
            self.mint_authority_pda,
            self.offer_vault_authority,
        );
    }
}
// SCOUT:PRELUDE:END

crucible_idl_gen::declare_fuzz_program!("idls/onreapp.json");

use onreapp::{accounts, instruction};

#[derive(Clone)]
struct OnreappFixture {
    ctx: crate::__scout_crucible_test_context::TestContext,
    program_id: Pubkey,
    payer: Rc<Keypair>,
    // SCOUT:FIELDS:BEGIN
    // --- actors -------------------------------------------------------------------------------
    /// Program authority. Identical to `payer`, so the generator's `boss = self.payer.pubkey()`
    /// default is already correct for every `has_one = boss` instruction.
    boss: Rc<Keypair>,
    /// Two non-privileged actors. Distinct signers are what make an adversary-value-conservation
    /// property meaningful — with one user every transfer is self-to-self and nothing can be stolen.
    user_a: Rc<Keypair>,
    user_b: Rc<Keypair>,
    /// `state.redemption_admin`; the only signer `fulfill_redemption_request` accepts.
    redemption_admin: Rc<Keypair>,
    /// The alternate `state.worker` candidate — see `other_worker`.
    worker_alt: Rc<Keypair>,
    /// `state.approver1`; signs `ApprovalMessage`s for offers with `needs_approval`.
    approver: Rc<Keypair>,

    // --- mints --------------------------------------------------------------------------------
    /// 6 decimals, mint authority stays with `boss` — the program never controls it, so it always
    /// takes the transfer (not burn/mint) path in `execute_token_operations`.
    mint_usdc: Pubkey,
    /// 9 decimals, mint authority moved to the `mint_authority` PDA during setup — the program
    /// controls it, so it takes the burn/mint path. This is `state.onyc_mint`.
    mint_onyc: Pubkey,
    /// 6 decimals, authority left with `boss` and used by nothing in the built world. Exists so
    /// `transfer_mint_authority_to_{program,boss}` and `make_offer` have a live target at fuzz
    /// time instead of being permanently dead against already-configured mints.
    mint_play: Pubkey,

    // --- PDAs ---------------------------------------------------------------------------------
    state_pda: Pubkey,
    mint_authority_pda: Pubkey,
    offer_vault_authority: Pubkey,
    redemption_vault_authority: Pubkey,
    permissionless_authority: Pubkey,
    /// The main offer: usdc -> onyc.
    offer_pda: Pubkey,
    /// The inverse redemption offer: onyc -> usdc.
    redemption_offer_pda: Pubkey,
    /// The REVERSE offer, onyc -> usdc. Exists solely so a redemption offer can be created whose
    /// payout leg is the program-controlled ONyc mint — the only shape in which
    /// `fulfill_redemption_request` reaches `mint_tokens`, and therefore the only shape in which
    /// its hard-coded `token_out_max_supply: 0` is observable.
    offer_rev_pda: Pubkey,
    /// Redemption offer usdc -> onyc, the inverse of `offer_rev_pda`. Fulfilling one of its
    /// requests MINTS ONyc.
    redemption_offer_rev_pda: Pubkey,

    /// The single token account that custodies every ONyc redemption deposit. Precomputed because
    /// a predicate cannot derive an address itself.
    redemption_vault_onyc: Pubkey,

    // --- per-property request registries ------------------------------------------------------
    // Each property keeps its OWN registry rather than sharing one: an isolated single-property
    // replay runs exactly one of the hooks below, so a shared counter would advance a different
    // number of times depending on which property was selected.
    //
    // These record only that a request was CREATED. Whether it is still open is decided in the
    // predicate by reading the account — fulfil and cancel both `close` it, so "the account is
    // still there" IS "the request is still open". No liveness is mirrored, so none can drift.
    /// Every ONyc-denominated redemption request the harness has watched be created, pooled
    /// across offers. `seeds::REDEMPTION_OFFER_VAULT_AUTHORITY` has no per-offer discriminator, so
    /// one token account backs them all — P-0007 asks whether that pooled account stays solvent.
    scout_p7_reqs: [Pubkey; SCOUT_P7_CAP],
    scout_p7_next: usize,

    /// Offer registry, retired with P-0005's and then P-0006's blocks (kept so restoring either
    /// is a one-block edit).
    #[allow(dead_code)]
    scout_p5_offers: [Pubkey; SCOUT_OFFER_CAP],
    #[allow(dead_code)]
    scout_p5_next: usize,

    /// Retired with P-0002's block (kept so restoring it is a one-block edit).
    #[allow(dead_code)]
    scout_p2_reqs: [Pubkey; SCOUT_REQ_CAP],
    #[allow(dead_code)]
    scout_p2_next: usize,
    scout_p3_reqs: [Pubkey; SCOUT_REQ_CAP],
    scout_p3_next: usize,
    /// 6 decimals, authority stays with `boss`. Input leg of the approval-gated offer.
    mint_appr: Pubkey,
    /// An offer with `needs_approval = true`: mint_appr -> onyc. Only reachable through the
    /// compound action that prepends a real Ed25519 instruction.
    offer_appr_pda: Pubkey,

    /// A **Token-2022 mint carrying a live `TransferFeeConfig`** (`SCOUT_T22_FEE_BPS`).
    ///
    /// The program advertises Token-2022 support and refuses fee-bearing mints on the OFFER path
    /// only (`token_utils.rs:374,378`). Without this mint the redemption path's complete absence
    /// of that guard is unobservable — every campaign would come back clean because the world
    /// contains nothing whose transferred amount differs from its requested amount.
    mint_fee: Pubkey,
    /// `make_offer(usdc -> fee)`. Exists solely so the redemption offer below has the `offer`
    /// account its seeds require (`[OFFER, token_out_mint, token_in_mint]`, mints swapped).
    offer_fee_pda: Pubkey,
    /// `make_redemption_offer(fee -> usdc)` — the redemption offer whose token_in charges a
    /// transfer fee. `create_redemption_request` against it is fully permissionless.
    redemption_offer_fee_pda: Pubkey,
    /// Selector for partial vs full fulfilment, advanced once per fulfil action. See
    /// `fulfill_amount_next`.
    scout_fulfill_tick: u64,
    /// programV5 PDAs the INVARIANTS read. Stored rather than derived because the invariant
    /// predicate grammar admits no helper calls and no free functions — a predicate may only read
    /// fields and call `ctx.account_data`, so every address it needs has to be a field.
    buffer_state_acct: Pubkey,
    market_stats_acct: Pubkey,
    excluded_balance_acct: Pubkey,
    // SCOUT:FIELDS:END
}

#[fuzz_fixture]
impl OnreappFixture {
    fn scout_placeholder(&self) -> Pubkey { Pubkey::new_unique() }

    pub fn setup() -> Self {
        let mut ctx = crate::__scout_crucible_test_context::TestContext::new();
        let program_id = Pubkey::new_from_array(onreapp::ID.to_bytes());
        // SCOUT:TARGET-PROGRAM:BEGIN
        crate::__scout_crucible_test_context::TestContext::add_program(&mut ctx, &program_id, SCOUT_TARGET_PROGRAM_ARTIFACT).unwrap();
        // SCOUT:TARGET-PROGRAM:END
        let payer = Rc::new(Keypair::new());
        ctx.create_account().pubkey(payer.pubkey()).lamports(1_000_000_000)
            .owner(system_program::ID).create().unwrap();
        // SCOUT:SETUP-GLUE:BEGIN
        // -----------------------------------------------------------------------------------
        // 0. Compute budget.
        //
        // litesvm's default is one transaction's worth of CUs (200_000) and it IGNORES a
        // `ComputeBudget::SetComputeUnitLimit` instruction inside the transaction -- measured:
        // prepending one still meters "consumed 193629 of 200000". So the only place the limit can
        // be raised is here, on the context.
        //
        // `open_swap_sell` needs it. Measured on the world this setup builds, with the four
        // Prop-AMM vault ATAs pre-created (section 12a) and after a vault-target + 2-day time
        // advance: `consumed 195148 of 200000`. Without the pre-created ATAs the very first sell
        // dies outright ("exceeded CUs meter at BPF instruction"). 195148/200000 is ~2% headroom,
        // which the fuzzer would burn through on the first state that costs slightly more, and the
        // action would read as flaky-unreachable rather than as a real wall. 1_400_000 is the
        // Solana per-transaction maximum, i.e. exactly what a real client would request for a
        // swap this heavy, so nothing here becomes cheaper than it is on-chain.
        // -----------------------------------------------------------------------------------
        ctx = ctx.with_compute_budget(1_400_000);

        // -----------------------------------------------------------------------------------
        // 1. Actors.
        //
        // The generated `payer` above got 1 SOL, which does not survive the number of
        // `init_if_needed` ATAs this program opens (take_offer, mint_to, fulfill and cancel each
        // may create one). Re-create it with a balance that does.
        // -----------------------------------------------------------------------------------
        let boss = payer.clone();
        ctx.create_account()
            .pubkey(boss.pubkey())
            .lamports(1_000_000_000_000)
            .owner(system_program::ID)
            .create()
            .unwrap();

        let user_a = Rc::new(Keypair::new());
        let user_b = Rc::new(Keypair::new());
        let redemption_admin = Rc::new(Keypair::new());
        // The second worker candidate. `set_worker` refuses to install the incumbent, so without a
        // second key the instruction could never succeed; every worker-signed action reads
        // `state.worker` and signs with whichever of the two it names.
        let worker_alt = Rc::new(Keypair::new());
        let approver = Rc::new(Keypair::new());
        for actor in [&user_a, &user_b, &redemption_admin, &worker_alt, &approver] {
            ctx.create_account()
                .pubkey(actor.pubkey())
                .lamports(1_000_000_000_000)
                .owner(system_program::ID)
                .create()
                .unwrap();
        }

        // -----------------------------------------------------------------------------------
        // 2. PDAs.
        // -----------------------------------------------------------------------------------
        let state_pda = scout_pda(&[SEED_STATE], &program_id);
        let mint_authority_pda = scout_pda(&[SEED_MINT_AUTHORITY], &program_id);
        let offer_vault_authority = scout_pda(&[SEED_OFFER_VAULT_AUTHORITY], &program_id);
        let redemption_vault_authority =
            scout_pda(&[SEED_REDEMPTION_OFFER_VAULT_AUTHORITY], &program_id);
        let permissionless_authority =
            scout_pda(&[SEED_PERMISSIONLESS_AUTHORITY], &program_id);

        // -----------------------------------------------------------------------------------
        // 3. Mints.
        //
        // `supply` is seeded to exactly the sum of the balances handed out below. The token
        // program only updates supply on mint/burn, so leaving it at 0 while pre-funding accounts
        // would make `get_circulating_supply` / `get_tvl` / the max-supply cap read a supply that
        // is smaller than the tokens that actually exist — a harness artefact that would show up
        // as a conservation violation with no bug behind it.
        // -----------------------------------------------------------------------------------
        let usdc_supply = USER_USDC_START * 2 + VAULT_USDC_START;
        let onyc_supply = USER_ONYC_START * 2;

        let mint_usdc = ctx
            .create_mint()
            .pubkey(Keypair::new().pubkey())
            .decimals(USDC_DECIMALS)
            .mint_authority(boss.pubkey())
            .supply(usdc_supply)
            .create()
            .unwrap();
        let mint_onyc = ctx
            .create_mint()
            .pubkey(Keypair::new().pubkey())
            .decimals(ONYC_DECIMALS)
            .mint_authority(boss.pubkey())
            .supply(onyc_supply)
            .create()
            .unwrap();
        let mint_play = ctx
            .create_mint()
            .pubkey(Keypair::new().pubkey())
            .decimals(USDC_DECIMALS)
            .mint_authority(boss.pubkey())
            .supply(0)
            .create()
            .unwrap();

        // -----------------------------------------------------------------------------------
        // 4. Pin the clock, then initialize + wire governance.
        //
        // litesvm's `add_program` deploys under BPFLoaderUpgradeable and writes a real ProgramData
        // account with `upgrade_authority_address: None`, which is exactly what
        // `get_upgrade_authority` needs to return `Ok(None)` and let any signer become boss.
        // -----------------------------------------------------------------------------------
        scout_set_time(&mut ctx, SCOUT_GENESIS_TS);
        scout_bootstrap_state(
            &mut ctx,
            program_id,
            &boss,
            &redemption_admin.pubkey(),
            &approver.pubkey(),
            mint_onyc,
            state_pda,
            mint_authority_pda,
            offer_vault_authority,
        );

        // -----------------------------------------------------------------------------------
        // 6. Hand ONyc's mint authority to the program.
        //
        // Done through the real instruction rather than by writing the mint directly, so the
        // `program_controls_mint` == true side of every burn/mint branch is reachable AND this
        // instruction's own happy path is covered.
        // -----------------------------------------------------------------------------------
        scout_expect_ok(
            "transfer_mint_authority_to_program(onyc)",
            ctx.program(program_id)
                .call(instruction::TransferMintAuthorityToProgram {})
                .accounts(accounts::TransferMintAuthorityToProgram {
                    boss: boss.pubkey(),
                    state: state_pda,
                    mint: mint_onyc,
                    mint_authority: mint_authority_pda,
                    token_program: SPL_TOKEN_ID,
                })
                .signers(&[&boss])
                .send(),
        );

        // -----------------------------------------------------------------------------------
        // 7. Token accounts.
        //
        // Pre-minting these is safe against the `init_if_needed` ATAs in make_offer /
        // make_redemption_offer / take_offer: `init_if_needed` accepts an already-correct ATA, so
        // none of those actions is disabled by minting here (contrast with an `init`-only target,
        // which pre-minting WOULD kill permanently).
        // -----------------------------------------------------------------------------------
        for (owner, mint, amount) in [
            (boss.pubkey(), mint_usdc, 0u64),
            (boss.pubkey(), mint_onyc, 0),
            (boss.pubkey(), mint_play, 0),
            (user_a.pubkey(), mint_usdc, USER_USDC_START),
            (user_a.pubkey(), mint_onyc, USER_ONYC_START),
            (user_b.pubkey(), mint_usdc, USER_USDC_START),
            (user_b.pubkey(), mint_onyc, USER_ONYC_START),
            (offer_vault_authority, mint_usdc, 0),
            (offer_vault_authority, mint_onyc, 0),
            (redemption_vault_authority, mint_usdc, VAULT_USDC_START),
            (redemption_vault_authority, mint_onyc, 0),
            (permissionless_authority, mint_usdc, 0),
            (permissionless_authority, mint_onyc, 0),
        ] {
            scout_mk_ata(&mut ctx, &owner, &mint, amount);
        }

        // -----------------------------------------------------------------------------------
        // 8. The main offer: usdc -> onyc, no approval required, permissionless allowed.
        //
        // `needs_approval = false` so the plain generated `action_take_offer` can succeed at all;
        // the approval-required branch needs an ed25519 instruction in the same transaction and is
        // driven by a compound action instead.
        // -----------------------------------------------------------------------------------
        let offer_pda = scout_pda(
            &[SEED_OFFER, mint_usdc.as_ref(), mint_onyc.as_ref()],
            &program_id,
        );
        scout_expect_ok(
            "make_offer(usdc->onyc)",
            ctx.program(program_id)
                .call(instruction::MakeOffer {
                    fee_basis_points: 100,
                    needs_approval: false,
                    allow_permissionless: true,
                })
                .accounts(accounts::MakeOffer {
                    vault_authority: offer_vault_authority,
                    token_in_mint: mint_usdc,
                    token_in_program: SPL_TOKEN_ID,
                    vault_token_in_account: scout_ata(
                        &offer_vault_authority,
                        &mint_usdc,
                        &SPL_TOKEN_ID,
                    ),
                    token_out_mint: mint_onyc,
                    offer: offer_pda,
                    state: state_pda,
                    boss: boss.pubkey(),
                })
                .signers(&[&boss])
                .send(),
        );

        // A pricing vector, without which every take/redeem path dies at `NoActiveVector` and the
        // whole value-flow surface reads as unreachable.
        //   start_time = None -> max(now, base_time); base_time = now => active immediately.
        let now = scout_now(&ctx);
        scout_expect_ok(
            "add_offer_vector(usdc->onyc)",
            ctx.program(program_id)
                .call(instruction::AddOfferVector {
                    start_time: None,
                    base_time: now,
                    base_price: 1_000_000_000, // 1.0 with PRICE_DECIMALS = 9
                    // 5% a year. APR_SCALE is 1_000_000 == 100% (offer_utils.rs:16), so the
                    // V4 fixture's 5_000_000 was FIVE HUNDRED percent — and, more importantly,
                    // `accrue_buffer` compares this field directly against `BufferState.gross_apr`
                    // in the SAME scale (`accrue_buffer.rs:53`), whose maximum is 1_000_000
                    // (`MAX_BUFFER_GROSS_APR`). With the offer above that ceiling `apr_delta` is
                    // permanently 0 and the BUFFER never mints anything: `settle_buffer` succeeds,
                    // covers its early-return path, and every BUFFER property would run green
                    // against code that cannot execute.
                    apr: 50_000,
                    price_fix_duration: 3_600,
                })
                .accounts(accounts::AddOfferVector {
                    offer: offer_pda,
                    token_in_mint: mint_usdc,
                    token_out_mint: mint_onyc,
                    state: state_pda,
                    boss: boss.pubkey(),
                })
                .signers(&[&boss])
                .send(),
        );

        // -----------------------------------------------------------------------------------
        // 8a. programV5: name the MAIN offer.
        //
        // `state.main_offer` is the canonical usdc -> onyc offer every price-dependent path reads
        // (`take_offer.rs:474` loads it whenever token_out is ONyc, and the BUFFER accrual and
        // market-stats refresh both price off it). Until it is set it is `Pubkey::default()`, and
        // `load_main_offer` rejects the mismatch — which fails every ONyc-out take, mint_to,
        // fulfil and swap in the harness. This is programV5's single most load-bearing
        // prerequisite; leaving it out reads exactly like a dozen unrelated account bugs.
        // -----------------------------------------------------------------------------------
        scout_expect_ok(
            "set_main_offer(usdc->onyc)",
            ctx.program(program_id)
                .call(instruction::SetMainOffer {})
                .accounts(accounts::SetMainOffer {
                    state: state_pda,
                    boss: boss.pubkey(),
                    offer: offer_pda,
                })
                .signers(&[&boss])
                .send(),
        );

        // -----------------------------------------------------------------------------------
        // 8a2. programV5: BUFFER, market stats and the circulating-supply cache.
        //
        // `initialize_buffer` is the one instruction that creates `BufferState`, the reserve vault
        // authority and the management/performance `ConfigurableVault`s together with their ONyc
        // ATAs. Everything downstream (settle_buffer, the accrual inside take_offer_v2 /
        // open_swap_*, mint_to) reads those accounts, so without this the whole BUFFER surface is
        // account-validation-dead.
        //
        // `market_stats` and `circulating_supply_excluded_balance` are `init_if_needed` targets of
        // their own instructions, but every value-moving V5 path reads them, so they are created
        // here rather than left to whichever action the fuzzer happens to pick first.
        // -----------------------------------------------------------------------------------
        let reserve_vault_authority = scout_pda(&[SEED_RESERVE_VAULT_AUTHORITY], &program_id);
        let cv = |kind: CvKind| {
            scout_pda(
                &[SEED_CONFIGURABLE_VAULT, scout_vault_kind_seed(kind)],
                &program_id,
            )
        };
        scout_expect_ok(
            "initialize_buffer",
            ctx.program(program_id)
                .call(instruction::InitializeBuffer {})
                .accounts(accounts::InitializeBuffer {
                    state: state_pda,
                    buffer_state: scout_pda(&[SEED_BUFFER_STATE], &program_id),
                    reserve_vault_authority,
                    management_fee_vault: cv(CvKind::ManagementFee),
                    performance_fee_vault: cv(CvKind::PerformanceFee),
                    boss: boss.pubkey(),
                    onyc_mint: mint_onyc,
                    offer: offer_pda,
                    reserve_vault_onyc_account: scout_ata(
                        &reserve_vault_authority,
                        &mint_onyc,
                        &SPL_TOKEN_ID,
                    ),
                    management_fee_vault_onyc_account: scout_ata(
                        &cv(CvKind::ManagementFee),
                        &mint_onyc,
                        &SPL_TOKEN_ID,
                    ),
                    performance_fee_vault_onyc_account: scout_ata(
                        &cv(CvKind::PerformanceFee),
                        &mint_onyc,
                        &SPL_TOKEN_ID,
                    ),
                    token_program: SPL_TOKEN_ID,
                })
                .signers(&[&boss])
                .send(),
        );

        // BUFFER fee configuration.
        //
        // `initialize_buffer` leaves both fee rates at 0, so without this the accrual mints
        // everything to the reserve and the management/performance split — including the entire
        // high-watermark mechanism — is dead code however much yield accrues. 100 bp management /
        // 2000 bp performance are the shapes the program's own unit tests use
        // (`accrual_utils.rs:226`).
        //
        // The gross APR is deliberately NOT set here: `set_buffer_gross_apr` is in the ordinary
        // action pool, so the fuzzer owns it, and leaving it at 0 initially means the "APR not yet
        // configured, accrue nothing" branch is reachable too.
        scout_expect_ok(
            "set_buffer_fee_config",
            ctx.program(program_id)
                .call(instruction::SetBufferFeeConfig {
                    management_fee_basis_points: 100,
                    performance_fee_basis_points: 2_000,
                    performance_fee_high_watermark_enabled: true,
                })
                .accounts(accounts::SetBufferFeeConfig {
                    state: state_pda,
                    boss: boss.pubkey(),
                    main_offer: offer_pda,
                    onyc_mint: mint_onyc,
                    offer_vault_authority,
                    mint_authority: mint_authority_pda,
                    buffer_state: scout_pda(&[SEED_BUFFER_STATE], &program_id),
                    reserve_vault_onyc_account: scout_ata(
                        &reserve_vault_authority,
                        &mint_onyc,
                        &SPL_TOKEN_ID,
                    ),
                    management_fee_vault_onyc_account: scout_ata(
                        &cv(CvKind::ManagementFee),
                        &mint_onyc,
                        &SPL_TOKEN_ID,
                    ),
                    performance_fee_vault_onyc_account: scout_ata(
                        &cv(CvKind::PerformanceFee),
                        &mint_onyc,
                        &SPL_TOKEN_ID,
                    ),
                    token_program: SPL_TOKEN_ID,
                    market_stats: scout_pda(&[SEED_MARKET_STATS], &program_id),
                    circulating_supply_excluded_balance: scout_pda(
                        &[SEED_CIRC_SUPPLY_EXCLUDED_BALANCE],
                        &program_id,
                    ),
                })
                .signers(&[&boss])
                .send(),
        );

        // Fee routing for the two vaults the offer path fills.
        //
        // `withdraw_configurable_vault` refuses any vault whose `withdrawal_destination` is still
        // the default key, so with none configured the instruction can only ever reach its own
        // rejection branch — measured 0 successes in 50 selections. An operator would configure
        // routing for the vaults that actually receive fees, so setup does it for those two and
        // leaves the other seven to `action_scout_set_vault_destination`, which keeps both the
        // "vault already exists" and the "create on first use" branches of
        // `get_or_create_configurable_vault_token_account_pair` reachable.
        for kind in [CvKind::OfferFee, CvKind::OfferProceeds] {
            scout_expect_ok(
                "set_configurable_vault_destination",
                ctx.program(program_id)
                    .call(instruction::SetConfigurableVaultDestination {
                        kind,
                        withdrawal_destination: boss.pubkey(),
                    })
                    .accounts(accounts::SetConfigurableVaultDestination {
                        state: state_pda,
                        boss: boss.pubkey(),
                        configurable_vault: cv(kind),
                    })
                    .signers(&[&boss])
                    .send(),
            );
        }

        // The circulating-supply exclusion list. `update_circulating_supply_excluded_balance`
        // requires this account to exist, and every V2 market-info read consults the cached
        // balance, so the list is installed here rather than left to whichever action fires first.
        // The owners are the program's own ONyc-holding vaults: excluding them is what makes
        // "circulating" differ from "minted" at all, and a list of only the default key would make
        // the excluded balance identically zero and the whole subtraction untested.
        scout_expect_ok(
            "set_circulating_supply_excluded_accounts",
            ctx.program(program_id)
                .call(instruction::SetCirculatingSupplyExcludedAccounts {
                    owners: {
                        let mut owners = [Pubkey::default(); 20];
                        owners[0] = reserve_vault_authority;
                        owners[1] = offer_vault_authority;
                        owners[2] = boss.pubkey();
                        owners
                    },
                })
                .accounts(accounts::SetCirculatingSupplyExcludedAccounts {
                    state: state_pda,
                    boss: boss.pubkey(),
                    excluded_accounts: scout_pda(
                        &[SEED_CIRC_SUPPLY_EXCLUDED_ACCOUNTS],
                        &program_id,
                    ),
                })
                .signers(&[&boss])
                .send(),
        );

        // -----------------------------------------------------------------------------------
        // 8b. An approval-gated offer: mint_appr -> onyc, `needs_approval = true`.
        //
        // Separate from the main offer because `needs_approval` is fixed at creation and cannot be
        // changed afterwards: without a second offer carrying it, `verify_offer_approval`'s entire
        // Some(msg) branch and all of approver_utils is dead code in this harness.
        // -----------------------------------------------------------------------------------
        let mint_appr = ctx
            .create_mint()
            .pubkey(Keypair::new().pubkey())
            .decimals(USDC_DECIMALS)
            .mint_authority(boss.pubkey())
            .supply(USER_USDC_START * 2)
            .create()
            .unwrap();
        for owner in [
            user_a.pubkey(),
            user_b.pubkey(),
            boss.pubkey(),
            offer_vault_authority,
        ] {
            let amount = if owner == user_a.pubkey() || owner == user_b.pubkey() {
                USER_USDC_START
            } else {
                0
            };
            scout_mk_ata(&mut ctx, &owner, &mint_appr, amount);
        }
        let offer_appr_pda = scout_pda(
            &[SEED_OFFER, mint_appr.as_ref(), mint_onyc.as_ref()],
            &program_id,
        );
        scout_expect_ok(
            "make_offer(appr->onyc, needs_approval)",
            ctx.program(program_id)
                .call(instruction::MakeOffer {
                    fee_basis_points: 100,
                    needs_approval: true,
                    allow_permissionless: false,
                })
                .accounts(accounts::MakeOffer {
                    vault_authority: offer_vault_authority,
                    token_in_mint: mint_appr,
                    token_in_program: SPL_TOKEN_ID,
                    vault_token_in_account: scout_ata(
                        &offer_vault_authority,
                        &mint_appr,
                        &SPL_TOKEN_ID,
                    ),
                    token_out_mint: mint_onyc,
                    offer: offer_appr_pda,
                    state: state_pda,
                    boss: boss.pubkey(),
                })
                .signers(&[&boss])
                .send(),
        );
        scout_expect_ok(
            "add_offer_vector(appr->onyc)",
            ctx.program(program_id)
                .call(instruction::AddOfferVector {
                    start_time: None,
                    base_time: now,
                    base_price: 1_000_000_000,
                    apr: 50_000,
                    price_fix_duration: 3_600,
                })
                .accounts(accounts::AddOfferVector {
                    offer: offer_appr_pda,
                    token_in_mint: mint_appr,
                    token_out_mint: mint_onyc,
                    state: state_pda,
                    boss: boss.pubkey(),
                })
                .signers(&[&boss])
                .send(),
        );

        // -----------------------------------------------------------------------------------
        // 9. The inverse redemption offer: onyc -> usdc.
        //
        // Its `offer` account is derived with the mints SWAPPED (seeds = [OFFER, token_out_mint,
        // token_in_mint]), so it resolves back to the offer created above.
        // -----------------------------------------------------------------------------------
        let redemption_offer_pda = scout_pda(
            &[SEED_REDEMPTION_OFFER, mint_onyc.as_ref(), mint_usdc.as_ref()],
            &program_id,
        );
        scout_expect_ok(
            "make_redemption_offer(onyc->usdc)",
            ctx.program(program_id)
                .call(instruction::MakeRedemptionOffer {
                    fee_basis_points: 50,
                    fee_basis_points_prop_amm_sell: 50,
                })
                .accounts(accounts::MakeRedemptionOffer {
                    state: state_pda,
                    offer: offer_pda,
                    redemption_vault_authority,
                    token_in_mint: mint_onyc,
                    token_in_program: SPL_TOKEN_ID,
                    vault_token_in_account: scout_ata(
                        &redemption_vault_authority,
                        &mint_onyc,
                        &SPL_TOKEN_ID,
                    ),
                    token_out_mint: mint_usdc,
                    token_out_program: SPL_TOKEN_ID,
                    vault_token_out_account: scout_ata(
                        &redemption_vault_authority,
                        &mint_usdc,
                        &SPL_TOKEN_ID,
                    ),
                    redemption_offer: redemption_offer_pda,
                    boss: boss.pubkey(),
                })
                .signers(&[&boss])
                .send(),
        );
        // -----------------------------------------------------------------------------------
        // 10. The REVERSE pair: offer onyc -> usdc, and the redemption offer usdc -> onyc.
        //
        // Everything above redeems INTO usdc, which the program does not control, so
        // `execute_redemption_operations` always takes its transfer-from-vault branch and
        // `mint_tokens` is never reached from a redemption. This pair inverts that: its payout leg
        // is ONyc, whose mint authority the program holds, so fulfilling one of its requests goes
        // through `mint_tokens` — the branch `fulfill_redemption_request.rs:274` hands a hard-coded
        // `token_out_max_supply: 0`, where `take_offer.rs:296` and `mint_to` both pass
        // `state.max_supply`. Without this pair that discrepancy is unobservable.
        // -----------------------------------------------------------------------------------
        let offer_rev_pda = scout_pda(
            &[SEED_OFFER, mint_onyc.as_ref(), mint_usdc.as_ref()],
            &program_id,
        );
        scout_expect_ok(
            "make_offer(onyc->usdc)",
            ctx.program(program_id)
                .call(instruction::MakeOffer {
                    fee_basis_points: 100,
                    needs_approval: false,
                    allow_permissionless: false,
                })
                .accounts(accounts::MakeOffer {
                    vault_authority: offer_vault_authority,
                    token_in_mint: mint_onyc,
                    token_in_program: SPL_TOKEN_ID,
                    vault_token_in_account: scout_ata(&offer_vault_authority, &mint_onyc, &SPL_TOKEN_ID),
                    token_out_mint: mint_usdc,
                    offer: offer_rev_pda,
                    state: state_pda,
                    boss: boss.pubkey(),
                })
                .signers(&[&boss])
                .send(),
        );
        scout_expect_ok(
            "add_offer_vector(onyc->usdc)",
            ctx.program(program_id)
                .call(instruction::AddOfferVector {
                    start_time: None,
                    base_time: now,
                    base_price: 1_000_000_000,
                    apr: 50_000,
                    price_fix_duration: 3_600,
                })
                .accounts(accounts::AddOfferVector {
                    offer: offer_rev_pda,
                    token_in_mint: mint_onyc,
                    token_out_mint: mint_usdc,
                    state: state_pda,
                    boss: boss.pubkey(),
                })
                .signers(&[&boss])
                .send(),
        );

        let redemption_offer_rev_pda = scout_pda(
            &[SEED_REDEMPTION_OFFER, mint_usdc.as_ref(), mint_onyc.as_ref()],
            &program_id,
        );
        // programV5 pins every redemption offer's token_in leg to ONyc and to the plain SPL token
        // program (`make_redemption_offer.rs:66-72`: `token_in_mint.key() == state.onyc_mint` and
        // `*token_in_mint.owner == anchor_spl::token::ID`). The usdc -> onyc redemption offer this
        // block used to build is therefore no longer constructible; the underlying onyc -> usdc
        // OFFER above is still built and still useful. `redemption_offer_rev_pda` is kept as a
        // derived address so the field and its readers still compile — the account does not exist.

        // -----------------------------------------------------------------------------------
        // 11. The Token-2022 TRANSFER-FEE pair: offer usdc -> fee, redemption offer fee -> usdc.
        //
        // `has_transfer_fee` has exactly two call sites in the whole program, both inside
        // `execute_token_operations` (`token_utils.rs:374,378`), which only take_offer and
        // take_offer_permissionless reach. Nothing on the redemption path consults it, and
        // `make_redemption_offer` performs no mint validation at all — so a fee-bearing token_in
        // is an accepted, supported configuration whose deposits arrive short.
        //
        // `create_redemption_request` is permissionless, so unlike every other finding on this
        // target the resulting shortfall needs no privileged signer to occur.
        // -----------------------------------------------------------------------------------
        let mint_fee = scout_mk_t22_fee_mint(
            &mut ctx,
            &boss.pubkey(),
            FEE_DECIMALS,
            USER_USDC_START * 2,
            SCOUT_T22_FEE_BPS,
        );
        for owner in [user_a.pubkey(), user_b.pubkey()] {
            scout_mk_t22_ata(&mut ctx, &owner, &mint_fee, USER_USDC_START);
        }
        // The boss leg receives the offer fee on any take; without it `make_offer` has no
        // destination to `init_if_needed` against a Token-2022 mint.
        scout_mk_t22_ata(&mut ctx, &boss.pubkey(), &mint_fee, 0);

        let offer_fee_pda = scout_pda(
            &[SEED_OFFER, mint_usdc.as_ref(), mint_fee.as_ref()],
            &program_id,
        );
        scout_expect_ok(
            "make_offer(usdc->fee)",
            ctx.program(program_id)
                .call(instruction::MakeOffer {
                    fee_basis_points: 100,
                    needs_approval: false,
                    allow_permissionless: false,
                })
                .accounts(accounts::MakeOffer {
                    vault_authority: offer_vault_authority,
                    token_in_mint: mint_usdc,
                    token_in_program: SPL_TOKEN_ID,
                    vault_token_in_account: scout_ata(&offer_vault_authority, &mint_usdc, &SPL_TOKEN_ID),
                    token_out_mint: mint_fee,
                    offer: offer_fee_pda,
                    state: state_pda,
                    boss: boss.pubkey(),
                })
                .signers(&[&boss])
                .send(),
        );

        let redemption_offer_fee_pda = scout_pda(
            &[SEED_REDEMPTION_OFFER, mint_fee.as_ref(), mint_usdc.as_ref()],
            &program_id,
        );
        // Likewise the fee -> usdc redemption offer that made P-0008 observable: programV5's
        // token_in constraints reject BOTH a non-ONyc mint and a Token-2022 mint outright, so the
        // fee-bearing redemption deposit path no longer exists to be tested. The Token-2022
        // `mint_fee` and its usdc -> fee OFFER stay: the offer path is where `has_transfer_fee`
        // actually runs, and that capability must not be lost from the fixture.

        // -----------------------------------------------------------------------------------
        // 12. programV5: the Prop AMM.
        //
        // 12a. The four Prop-AMM `ConfigurableVault` ATAs.
        //
        // Both swaps route their token_in through
        // `get_or_create_configurable_vault_token_account_pair` (`prop_amm/buy.rs:240`, and
        // `prop_amm/sell.rs` via `execute_redemption_operations`), which opens a proceeds ATA and a
        // side-specific fee ATA denominated in that side's token_in -- usdc on the buy, ONyc on the
        // sell. Each creation costs ~13_525 CU inside an instruction that already runs BUFFER
        // accrual and a market-stats refresh, and the two sell-side creations are exactly what put
        // the first `open_swap_sell` over the meter. Pre-creating them is a pure CU optimisation,
        // not a semantic one: `get_or_create_associated_token_account` accepts an already-correct
        // ATA, and the vault PDA itself is still created by the instruction
        // (`configurable_vault/accounts.rs:48-61`), so the init branch stays covered.
        // -----------------------------------------------------------------------------------
        for (kind, mint) in [
            (CvKind::PropAmmProceeds, mint_usdc),
            (CvKind::PropAmmProceeds, mint_onyc),
            (CvKind::PropAmmBuyFee, mint_usdc),
            (CvKind::PropAmmSellFee, mint_onyc),
        ] {
            scout_mk_ata(&mut ctx, &cv(kind), &mint, 0);
        }

        // -----------------------------------------------------------------------------------
        // 12b. Enable the pair.
        //
        // `validate_prop_amm_pair_state` (`prop_amm/validation.rs:74`) rejects with
        // `PropAmmPairDisabled` unless a `PropAmmPairState` exists AND carries `enabled = true`,
        // and every swap account struct reads it with `bump = prop_amm_pair_state.bump`, so an
        // absent account is not even loadable. Only `configure_prop_amm` creates it
        // (`init_if_needed`, boss-signed, `prop_amm/config.rs:92-99`) -- nothing else does -- so
        // without this call all four swap instructions are account-validation-dead and the whole
        // subsystem reads as unreachable.
        //
        // The arguments are the program's own defaults (`prop_amm/config.rs:7-21`), which keeps the
        // fixture on the pricing curve the docs describe. `minimum_sell_haircut_onyc` is 5 ONyc, so
        // a sell of less than that reverts out of `process_redemption_core` -- deliberate; it is the
        // shipped default and users start with 100 ONyc, so ordinary sells clear it.
        //
        // `epoch_start` is stamped from the pinned clock here (`prop_amm/config.rs:168`), which is
        // what makes the epoch/cadence model move only under `action_scout_advance_time`.
        // -----------------------------------------------------------------------------------
        let prop_amm_pair_state_pda = scout_pda(
            &[SEED_PROP_AMM_PAIR_STATE, offer_pda.as_ref()],
            &program_id,
        );
        scout_expect_ok(
            "configure_prop_amm(usdc->onyc)",
            ctx.program(program_id)
                .call(instruction::ConfigurePropAmm {
                    enabled: true,
                    curve_peg_haircut_bps: 700,               // DEFAULT_CURVE_PEG_HAIRCUT_BPS
                    curve_exponent_scaled: 25_000,            // DEFAULT_CURVE_EXPONENT_SCALED
                    cadence_threshold: 20,                    // DEFAULT_CADENCE_THRESHOLD
                    cadence_wave_scaled: 10_000,              // DEFAULT_CADENCE_WAVE_SCALED
                    epoch_duration_seconds: 86_400,           // DEFAULT_EPOCH_DURATION_SECONDS
                    wall_sensitivity_scaled: 20_000,          // DEFAULT_WALL_SENSITIVITY_SCALED
                    minimum_sell_haircut_onyc: 5_000_000_000, // DEFAULT_MINIMUM_SELL_HAIRCUT_ONYC
                })
                .accounts(accounts::ConfigurePropAmm {
                    state: state_pda,
                    offer: offer_pda,
                    asset_mint: mint_usdc,
                    prop_amm_pair_state: prop_amm_pair_state_pda,
                    boss: boss.pubkey(),
                })
                .signers(&[&boss])
                .send(),
        );

        Self {
            ctx,
            program_id,
            payer,
            boss,
            user_a,
            user_b,
            redemption_admin,
            worker_alt,
            approver,
            mint_usdc,
            mint_onyc,
            mint_play,
            state_pda,
            mint_authority_pda,
            offer_vault_authority,
            redemption_vault_authority,
            permissionless_authority,
            offer_pda,
            redemption_offer_pda,
            offer_rev_pda,
            redemption_offer_rev_pda,
            redemption_vault_onyc: scout_ata(&redemption_vault_authority, &mint_onyc, &SPL_TOKEN_ID),
            scout_p7_reqs: [Pubkey::default(); SCOUT_P7_CAP],
            scout_p7_next: 0,
            scout_p5_offers: {
                let mut a = [Pubkey::default(); SCOUT_OFFER_CAP];
                a[0] = offer_pda;
                a[1] = offer_appr_pda;
                a[2] = offer_rev_pda;
                a
            },
            scout_p5_next: 3,
            scout_p2_reqs: [Pubkey::default(); SCOUT_REQ_CAP],
            scout_p2_next: 0,
            scout_p3_reqs: [Pubkey::default(); SCOUT_REQ_CAP],
            scout_p3_next: 0,
            mint_appr,
            offer_appr_pda,
            mint_fee,
            offer_fee_pda,
            redemption_offer_fee_pda,
            scout_fulfill_tick: 0,
            buffer_state_acct: scout_pda(&[SEED_BUFFER_STATE], &program_id),
            market_stats_acct: scout_pda(&[SEED_MARKET_STATS], &program_id),
            excluded_balance_acct: scout_pda(&[SEED_CIRC_SUPPLY_EXCLUDED_BALANCE], &program_id),
        }
        // SCOUT:SETUP-GLUE:END
    }

    #[cfg(feature = "admin_actions")]
    pub fn action_initialize(&mut self) -> bool {
        let state = self.state_pda;
        let mint_authority = self.mint_authority_pda;
        let offer_vault_authority = self.offer_vault_authority;
        let boss = self.boss.pubkey();
        let program = self.program_id;
        let program_data = Some(scout_pda(&[self.program_id.as_ref()], &BPF_LOADER_UPGRADEABLE_ID));
        let onyc_mint = self.mint_onyc;
        let system_program = system_program::ID;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::Initialize {  })
            .accounts(accounts::Initialize {
                state: state,
                mint_authority: mint_authority,
                offer_vault_authority: offer_vault_authority,
                boss: boss,
                program_data: program_data,
                onyc_mint: onyc_mint,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:initialize:BEGIN
            // update shadow-ledger state after successful initialize
            // SCOUT:ACTION-HOOK:initialize:END
        }
        __scout_success
    }
    #[cfg(not(feature = "admin_actions"))]
    pub fn action_initialize(&mut self) -> bool {
        // disabled: build with --features admin_actions to enable
        false
    }

    #[cfg(feature = "admin_actions")]
    pub fn action_initialize_permissionless_authority(&mut self) -> bool {
        let name: String = String::from("permissionless-1");
        let permissionless_authority = self.permissionless_authority;
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let system_program = system_program::ID;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::InitializePermissionlessAuthority { name })
            .accounts(accounts::InitializePermissionlessAuthority {
                permissionless_authority: permissionless_authority,
                state: state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:initialize_permissionless_authority:BEGIN
            // update shadow-ledger state after successful initialize_permissionless_authority
            // SCOUT:ACTION-HOOK:initialize_permissionless_authority:END
        }
        __scout_success
    }
    #[cfg(not(feature = "admin_actions"))]
    pub fn action_initialize_permissionless_authority(&mut self) -> bool {
        // disabled: build with --features admin_actions to enable
        false
    }

    pub fn action_offer_vault_deposit(&mut self, amount: u64) -> bool {
        let state = self.state_pda;
        let vault_authority = self.offer_vault_authority;
        let token_mint = self.mint_usdc;
        let depositor_token_account = scout_ata(&self.pick_user_pk(amount), &self.mint_usdc, &SPL_TOKEN_ID);
        let vault_token_account = scout_ata(&self.offer_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID);
        let __scout_signer_depositor = self.pick_user(amount);
        let depositor = __scout_signer_depositor.pubkey();
        let token_program = SPL_TOKEN_ID;
        let associated_token_program = ATA_PROGRAM_ID;
        let system_program = system_program::ID;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::OfferVaultDeposit { amount })
            .accounts(accounts::OfferVaultDeposit {
                state: state,
                vault_authority: vault_authority,
                token_mint: token_mint,
                depositor_token_account: depositor_token_account,
                vault_token_account: vault_token_account,
                depositor: depositor,
                token_program: token_program,
            })
            .signers(&[&*self.payer, &__scout_signer_depositor])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:offer_vault_deposit:BEGIN
            // update shadow-ledger state after successful offer_vault_deposit
            // SCOUT:ACTION-HOOK:offer_vault_deposit:END
        }
        __scout_success
    }

    pub fn action_offer_vault_withdraw(&mut self, amount: u64) -> bool {
        let vault_authority = self.offer_vault_authority;
        let token_mint = self.mint_usdc;
        let boss_token_account = scout_ata(&self.boss.pubkey(), &self.mint_usdc, &SPL_TOKEN_ID);
        let vault_token_account = scout_ata(&self.offer_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID);
        let boss = self.boss.pubkey();
        let state = self.state_pda;
        let token_program = SPL_TOKEN_ID;
        let associated_token_program = ATA_PROGRAM_ID;
        let system_program = system_program::ID;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::OfferVaultWithdraw { amount })
            .accounts(accounts::OfferVaultWithdraw {
                vault_authority: vault_authority,
                token_mint: token_mint,
                boss_token_account: boss_token_account,
                vault_token_account: vault_token_account,
                boss: boss,
                state: state,
                token_program: token_program,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:offer_vault_withdraw:BEGIN
            // update shadow-ledger state after successful offer_vault_withdraw
            // SCOUT:ACTION-HOOK:offer_vault_withdraw:END
        }
        __scout_success
    }

    pub fn action_redemption_vault_deposit(&mut self, amount: u64) -> bool {
        let redemption_vault_authority = self.redemption_vault_authority;
        let token_mint = self.pick_vault_mint(amount);
        let depositor_token_account = scout_ata(&self.pick_user_pk(amount), &self.pick_vault_mint(amount), &SPL_TOKEN_ID);
        let vault_token_account = scout_ata(&self.redemption_vault_authority, &self.pick_vault_mint(amount), &SPL_TOKEN_ID);
        let __scout_signer_depositor = self.pick_user(amount);
        let depositor = __scout_signer_depositor.pubkey();
        let state = self.state_pda;
        let token_program = SPL_TOKEN_ID;
        let associated_token_program = ATA_PROGRAM_ID;
        let system_program = system_program::ID;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::RedemptionVaultDeposit { amount })
            .accounts(accounts::RedemptionVaultDeposit {
                redemption_vault_authority: redemption_vault_authority,
                token_mint: token_mint,
                depositor_token_account: depositor_token_account,
                vault_token_account: vault_token_account,
                depositor: depositor,
                state: state,
                token_program: token_program,
            })
            .signers(&[&*self.payer, &__scout_signer_depositor])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:redemption_vault_deposit:BEGIN
            // update shadow-ledger state after successful redemption_vault_deposit
            // SCOUT:ACTION-HOOK:redemption_vault_deposit:END
        }
        __scout_success
    }

    pub fn action_redemption_vault_withdraw(&mut self, amount: u64) -> bool {
        let redemption_vault_authority = self.redemption_vault_authority;
        let token_mint = self.pick_vault_mint(amount);
        let boss_token_account = scout_ata(&self.boss.pubkey(), &self.pick_vault_mint(amount), &SPL_TOKEN_ID);
        let vault_token_account = scout_ata(&self.redemption_vault_authority, &self.pick_vault_mint(amount), &SPL_TOKEN_ID);
        let boss = self.boss.pubkey();
        let state = self.state_pda;
        let token_program = SPL_TOKEN_ID;
        let associated_token_program = ATA_PROGRAM_ID;
        let system_program = system_program::ID;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::RedemptionVaultWithdraw { amount })
            .accounts(accounts::RedemptionVaultWithdraw {
                redemption_vault_authority: redemption_vault_authority,
                token_mint: token_mint,
                boss_token_account: boss_token_account,
                vault_token_account: vault_token_account,
                boss: boss,
                state: state,
                token_program: token_program,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:redemption_vault_withdraw:BEGIN
            // update shadow-ledger state after successful redemption_vault_withdraw
            // SCOUT:ACTION-HOOK:redemption_vault_withdraw:END
        }
        __scout_success
    }

    #[cfg(feature = "admin_actions")]
    pub fn action_set_configurable_vault_destination(&mut self) -> bool {
        let kind: onreapp::types::ConfigurableVaultKind = self.pick_vault_kind(0);
        let withdrawal_destination: Pubkey = self.boss.pubkey();
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let configurable_vault = self.cv_pda(kind);
        let system_program = system_program::ID;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::SetConfigurableVaultDestination { kind, withdrawal_destination })
            .accounts(accounts::SetConfigurableVaultDestination {
                state: state,
                boss: boss,
                configurable_vault: configurable_vault,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:set_configurable_vault_destination:BEGIN
            // update shadow-ledger state after successful set_configurable_vault_destination
            // SCOUT:ACTION-HOOK:set_configurable_vault_destination:END
        }
        __scout_success
    }
    #[cfg(not(feature = "admin_actions"))]
    pub fn action_set_configurable_vault_destination(&mut self) -> bool {
        // disabled: build with --features admin_actions to enable
        false
    }

    pub fn action_withdraw_configurable_vault(&mut self, amount: u64) -> bool {
        let kind: onreapp::types::ConfigurableVaultKind = self.pick_withdrawable_kind(amount);
        let state = self.state_pda;
        let caller = self.payer.pubkey();
        let configurable_vault = self.cv_pda(kind);
        let vault_token_account = self.cv_ata(kind, &self.cv_mint(kind));
        let destination = self.cv_destination(kind);
        let destination_token_account = scout_ata(&self.cv_destination(kind), &self.cv_mint(kind), &SPL_TOKEN_ID);
        let mint = self.cv_mint(kind);
        let token_program = SPL_TOKEN_ID;
        let associated_token_program = ATA_PROGRAM_ID;
        let system_program = system_program::ID;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::WithdrawConfigurableVault { kind, amount })
            .accounts(accounts::WithdrawConfigurableVault {
                state: state,
                caller: caller,
                configurable_vault: configurable_vault,
                vault_token_account: vault_token_account,
                destination: destination,
                destination_token_account: destination_token_account,
                mint: mint,
                token_program: token_program,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:withdraw_configurable_vault:BEGIN
            // update shadow-ledger state after successful withdraw_configurable_vault
            // SCOUT:ACTION-HOOK:withdraw_configurable_vault:END
        }
        __scout_success
    }

    pub fn action_make_offer(&mut self, fee_basis_points: u16, needs_approval: bool, allow_permissionless: bool) -> bool {
        let vault_authority = self.offer_vault_authority;
        let token_in_mint = self.make_offer_pair(fee_basis_points).0;
        let token_in_program = SPL_TOKEN_ID;
        let vault_token_in_account = scout_ata(&self.offer_vault_authority, &self.make_offer_pair(fee_basis_points).0, &SPL_TOKEN_ID);
        let token_out_mint = self.make_offer_pair(fee_basis_points).1;
        let offer = self.make_offer_pda_for(fee_basis_points);
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let associated_token_program = ATA_PROGRAM_ID;
        let system_program = system_program::ID;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::MakeOffer { fee_basis_points, needs_approval, allow_permissionless })
            .accounts(accounts::MakeOffer {
                vault_authority: vault_authority,
                token_in_mint: token_in_mint,
                token_in_program: token_in_program,
                vault_token_in_account: vault_token_in_account,
                token_out_mint: token_out_mint,
                offer: offer,
                state: state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:make_offer:BEGIN
            // (the offer registry hook retired with P-0006's block — see SCOUT:INVARIANTS.)
            // SCOUT:ACTION-HOOK:make_offer:END
        }
        __scout_success
    }

    pub fn action_add_offer_vector(&mut self, base_time: u64, base_price: u64, apr: u64, price_fix_duration: u64) -> bool {
        let start_time: Option<u64> = self.next_vector_start_time();
        let token_in_mint = self.mint_usdc;
        let token_out_mint = self.mint_onyc;
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let offer = self.offer_pda;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::AddOfferVector { start_time, base_time, base_price, apr, price_fix_duration })
            .accounts(accounts::AddOfferVector {
                offer: offer,
                token_in_mint: token_in_mint,
                token_out_mint: token_out_mint,
                state: state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:add_offer_vector:BEGIN
            // update shadow-ledger state after successful add_offer_vector
            // SCOUT:ACTION-HOOK:add_offer_vector:END
        }
        __scout_success
    }

    pub fn action_delete_offer_vector(&mut self) -> bool {
        let vector_start_time: u64 = self.pick_offer_vector_start_time_next();
        let token_in_mint = self.mint_usdc;
        let token_out_mint = self.mint_onyc;
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let offer = self.offer_pda;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::DeleteOfferVector { vector_start_time })
            .accounts(accounts::DeleteOfferVector {
                offer: offer,
                token_in_mint: token_in_mint,
                token_out_mint: token_out_mint,
                state: state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:delete_offer_vector:BEGIN
            // update shadow-ledger state after successful delete_offer_vector
            // SCOUT:ACTION-HOOK:delete_offer_vector:END
        }
        __scout_success
    }

    pub fn action_delete_all_offer_vectors(&mut self) -> bool {
        let token_in_mint = self.mint_usdc;
        let token_out_mint = self.mint_onyc;
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let offer = self.offer_pda;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::DeleteAllOfferVectors {  })
            .accounts(accounts::DeleteAllOfferVectors {
                offer: offer,
                token_in_mint: token_in_mint,
                token_out_mint: token_out_mint,
                state: state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:delete_all_offer_vectors:BEGIN
            // update shadow-ledger state after successful delete_all_offer_vectors
            // SCOUT:ACTION-HOOK:delete_all_offer_vectors:END
        }
        __scout_success
    }

    pub fn action_update_offer_fee(&mut self, new_fee_basis_points: u16) -> bool {
        let token_in_mint = self.mint_usdc;
        let token_out_mint = self.mint_onyc;
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let offer = self.offer_pda;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::UpdateOfferFee { new_fee_basis_points })
            .accounts(accounts::UpdateOfferFee {
                offer: offer,
                token_in_mint: token_in_mint,
                token_out_mint: token_out_mint,
                state: state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:update_offer_fee:BEGIN
            // update shadow-ledger state after successful update_offer_fee
            // SCOUT:ACTION-HOOK:update_offer_fee:END
        }
        __scout_success
    }

    pub fn action_update_offer_permissionless_fee(&mut self, new_fee_basis_points_permissionless: u16) -> bool {
        let token_in_mint = self.mint_usdc;
        let token_out_mint = self.mint_onyc;
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let offer = self.offer_pda;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::UpdateOfferPermissionlessFee { new_fee_basis_points_permissionless })
            .accounts(accounts::UpdateOfferPermissionlessFee {
                offer: offer,
                token_in_mint: token_in_mint,
                token_out_mint: token_out_mint,
                state: state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:update_offer_permissionless_fee:BEGIN
            // update shadow-ledger state after successful update_offer_permissionless_fee
            // SCOUT:ACTION-HOOK:update_offer_permissionless_fee:END
        }
        __scout_success
    }

    pub fn action_update_redemption_offer_prop_amm_sell_fee(&mut self, new_fee_basis_points_prop_amm_sell: u16) -> bool {
        let redemption_offer = self.redemption_offer_pda;
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::UpdateRedemptionOfferPropAmmSellFee { new_fee_basis_points_prop_amm_sell })
            .accounts(accounts::UpdateRedemptionOfferPropAmmSellFee {
                redemption_offer: redemption_offer,
                state: state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:update_redemption_offer_prop_amm_sell_fee:BEGIN
            // update shadow-ledger state after successful update_redemption_offer_prop_amm_sell_fee
            // SCOUT:ACTION-HOOK:update_redemption_offer_prop_amm_sell_fee:END
        }
        __scout_success
    }

    pub fn action_set_offer_disabled(&mut self, disabled: bool) -> bool {
        let offer = self.offer_pda;
        let state = self.state_pda;
        let signer = self.payer.pubkey();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::SetOfferDisabled { disabled })
            .accounts(accounts::SetOfferDisabled {
                offer: offer,
                state: state,
                signer: signer,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:set_offer_disabled:BEGIN
            // update shadow-ledger state after successful set_offer_disabled
            // SCOUT:ACTION-HOOK:set_offer_disabled:END
        }
        __scout_success
    }

    pub fn action_take_offer(&mut self, token_in_amount: u64) -> bool {
        let approval_message: Option<onreapp::types::ApprovalMessage> = None;
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let vault_authority = self.offer_vault_authority;
        let vault_token_in_account = scout_ata(&self.offer_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID);
        let vault_token_out_account = scout_ata(&self.offer_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID);
        let token_in_mint = self.mint_usdc;
        let token_in_program = SPL_TOKEN_ID;
        let token_out_mint = self.mint_onyc;
        let token_out_program = SPL_TOKEN_ID;
        let user_token_in_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_usdc, &SPL_TOKEN_ID);
        let user_token_out_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_onyc, &SPL_TOKEN_ID);
        let boss_token_in_account = scout_ata(&self.boss.pubkey(), &self.mint_usdc, &SPL_TOKEN_ID);
        let mint_authority = self.mint_authority_pda;
        let instructions_sysvar = INSTRUCTIONS_SYSVAR_ID;
        let __scout_signer_user = self.pick_user(token_in_amount);
        let user = __scout_signer_user.pubkey();
        let associated_token_program = ATA_PROGRAM_ID;
        let system_program = system_program::ID;
        let offer = self.offer_pda;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::TakeOffer { token_in_amount, approval_message })
            .accounts(accounts::TakeOffer {
                offer: offer,
                state: state,
                boss: boss,
                vault_authority: vault_authority,
                vault_token_in_account: vault_token_in_account,
                vault_token_out_account: vault_token_out_account,
                token_in_mint: token_in_mint,
                token_in_program: token_in_program,
                token_out_mint: token_out_mint,
                token_out_program: token_out_program,
                user_token_in_account: user_token_in_account,
                user_token_out_account: user_token_out_account,
                boss_token_in_account: boss_token_in_account,
                mint_authority: mint_authority,
                instructions_sysvar: instructions_sysvar,
                user: user,
            })
            .signers(&[&*self.payer, &__scout_signer_user])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:take_offer:BEGIN
            // update shadow-ledger state after successful take_offer
            // SCOUT:ACTION-HOOK:take_offer:END
        }
        __scout_success
    }

    pub fn action_take_offer_v2(&mut self, token_in_amount: u64) -> bool {
        let approval_message: Option<onreapp::types::ApprovalMessage> = None;
        let state = self.state_pda;
        let vault_authority = self.offer_vault_authority;
        let vault_token_in_account = scout_ata(&self.offer_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID);
        let vault_token_out_account = scout_ata(&self.offer_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID);
        let token_in_mint = self.mint_usdc;
        let token_in_program = SPL_TOKEN_ID;
        let token_out_mint = self.mint_onyc;
        let token_out_program = SPL_TOKEN_ID;
        let user_token_in_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_usdc, &SPL_TOKEN_ID);
        let user_token_out_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_onyc, &SPL_TOKEN_ID);
        let redemption_offer = self.redemption_offer_pda;
        let redemption_vault_authority = self.redemption_vault_authority;
        let redemption_vault_token_in_account = scout_ata(&self.redemption_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID);
        let offer_proceeds_vault = self.cv_pda(CvKind::OfferProceeds);
        let offer_proceeds_token_in_account = self.cv_ata(CvKind::OfferProceeds, &self.mint_usdc);
        let offer_fee_vault = self.cv_pda(CvKind::OfferFee);
        let offer_fee_token_in_account = self.cv_ata(CvKind::OfferFee, &self.mint_usdc);
        let mint_authority = self.mint_authority_pda;
        let buffer_state = self.buffer_state_pda();
        let reserve_vault_onyc_account = self.reserve_vault_onyc();
        let management_fee_vault_onyc_account = self.cv_ata(CvKind::ManagementFee, &self.mint_onyc);
        let performance_fee_vault_onyc_account = self.cv_ata(CvKind::PerformanceFee, &self.mint_onyc);
        let market_stats = self.market_stats_pda();
        let circulating_supply_excluded_balance = self.excluded_balance_pda();
        let instructions_sysvar = INSTRUCTIONS_SYSVAR_ID;
        let __scout_signer_user = self.pick_user(token_in_amount);
        let user = __scout_signer_user.pubkey();
        let associated_token_program = ATA_PROGRAM_ID;
        let system_program = system_program::ID;
        let main_offer = self.state_main_offer();
        let offer = self.offer_pda;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::TakeOfferV2 { token_in_amount, approval_message })
            .accounts(accounts::TakeOfferV2 {
                offer: offer,
                state: state,
                vault_authority: vault_authority,
                vault_token_in_account: vault_token_in_account,
                vault_token_out_account: vault_token_out_account,
                token_in_mint: token_in_mint,
                token_in_program: token_in_program,
                token_out_mint: token_out_mint,
                token_out_program: token_out_program,
                user_token_in_account: user_token_in_account,
                user_token_out_account: user_token_out_account,
                redemption_offer: redemption_offer,
                redemption_vault_authority: redemption_vault_authority,
                redemption_vault_token_in_account: redemption_vault_token_in_account,
                offer_proceeds_vault: offer_proceeds_vault,
                offer_proceeds_token_in_account: offer_proceeds_token_in_account,
                offer_fee_vault: offer_fee_vault,
                offer_fee_token_in_account: offer_fee_token_in_account,
                mint_authority: mint_authority,
                buffer_state: buffer_state,
                reserve_vault_onyc_account: reserve_vault_onyc_account,
                management_fee_vault_onyc_account: management_fee_vault_onyc_account,
                performance_fee_vault_onyc_account: performance_fee_vault_onyc_account,
                market_stats: market_stats,
                circulating_supply_excluded_balance: circulating_supply_excluded_balance,
                instructions_sysvar: instructions_sysvar,
                user: user,
                main_offer: main_offer,
            })
            .signers(&[&*self.payer, &__scout_signer_user])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:take_offer_v2:BEGIN
            // update shadow-ledger state after successful take_offer_v2
            // SCOUT:ACTION-HOOK:take_offer_v2:END
        }
        __scout_success
    }

    pub fn action_quote_swap_buy(&mut self, token_in_amount: u64) -> bool {
        let offer = self.offer_pda;
        let prop_amm_pair_state = self.prop_amm_pair_pda();
        let state = self.state_pda;
        let token_in_mint = self.mint_usdc;
        let token_out_mint = self.mint_onyc;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::QuoteSwapBuy { token_in_amount })
            .accounts(accounts::QuoteSwapBuy {
                offer: offer,
                prop_amm_pair_state: prop_amm_pair_state,
                state: state,
                token_in_mint: token_in_mint,
                token_out_mint: token_out_mint,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:quote_swap_buy:BEGIN
            // update shadow-ledger state after successful quote_swap_buy
            // SCOUT:ACTION-HOOK:quote_swap_buy:END
        }
        __scout_success
    }

    pub fn action_quote_swap_sell(&mut self) -> bool {
        let token_in_amount: u64 = self.swap_sell_amount_next();
        let offer = self.offer_pda;
        let prop_amm_pair_state = self.prop_amm_pair_pda();
        let state = self.state_pda;
        let redemption_vault_authority = self.redemption_vault_authority;
        let redemption_vault_token_out_account = scout_ata(&self.redemption_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID);
        let token_in_mint = self.mint_onyc;
        let token_out_mint = self.mint_usdc;
        let token_out_program = SPL_TOKEN_ID;
        let market_stats = self.market_stats_pda();
        let redemption_offer = self.redemption_offer_pda;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::QuoteSwapSell { token_in_amount })
            .accounts(accounts::QuoteSwapSell {
                offer: offer,
                prop_amm_pair_state: prop_amm_pair_state,
                redemption_offer: redemption_offer,
                state: state,
                redemption_vault_authority: redemption_vault_authority,
                redemption_vault_token_out_account: redemption_vault_token_out_account,
                token_in_mint: token_in_mint,
                token_out_mint: token_out_mint,
                token_out_program: token_out_program,
                market_stats: market_stats,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:quote_swap_sell:BEGIN
            // update shadow-ledger state after successful quote_swap_sell
            // SCOUT:ACTION-HOOK:quote_swap_sell:END
        }
        __scout_success
    }

    pub fn action_open_swap_buy(&mut self, token_in_amount: u64, minimum_out: u64) -> bool {
        let offer = self.offer_pda;
        let prop_amm_pair_state = self.prop_amm_pair_pda();
        let state = self.state_pda;
        let offer_vault_authority = self.offer_vault_authority;
        let redemption_vault_authority = self.redemption_vault_authority;
        let offer_vault_token_in_account = scout_ata(&self.offer_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID);
        let offer_vault_token_out_account = scout_ata(&self.offer_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID);
        let redemption_vault_token_in_account = scout_ata(&self.redemption_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID);
        let token_in_mint = self.mint_usdc;
        let token_in_program = SPL_TOKEN_ID;
        let token_out_mint = self.mint_onyc;
        let token_out_program = SPL_TOKEN_ID;
        let user_token_in_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_usdc, &SPL_TOKEN_ID);
        let user_token_out_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_onyc, &SPL_TOKEN_ID);
        let prop_amm_proceeds_vault = self.cv_pda(CvKind::PropAmmProceeds);
        let prop_amm_proceeds_token_in_account = self.cv_ata(CvKind::PropAmmProceeds, &self.mint_usdc);
        let prop_amm_buy_fee_vault = self.cv_pda(CvKind::PropAmmBuyFee);
        let prop_amm_buy_fee_token_in_account = self.cv_ata(CvKind::PropAmmBuyFee, &self.mint_usdc);
        let permissionless_authority = self.permissionless_authority;
        let permissionless_token_in_account = scout_ata(&self.permissionless_authority, &self.mint_usdc, &SPL_TOKEN_ID);
        let permissionless_token_out_account = scout_ata(&self.permissionless_authority, &self.mint_onyc, &SPL_TOKEN_ID);
        let mint_authority = self.mint_authority_pda;
        let buffer_state = self.buffer_state_pda();
        let reserve_vault_onyc_account = self.reserve_vault_onyc();
        let management_fee_vault_onyc_account = self.cv_ata(CvKind::ManagementFee, &self.mint_onyc);
        let performance_fee_vault_onyc_account = self.cv_ata(CvKind::PerformanceFee, &self.mint_onyc);
        let market_stats = self.market_stats_pda();
        let circulating_supply_excluded_balance = self.excluded_balance_pda();
        let instructions_sysvar = INSTRUCTIONS_SYSVAR_ID;
        let __scout_signer_user = self.pick_user(token_in_amount);
        let user = __scout_signer_user.pubkey();
        let associated_token_program = ATA_PROGRAM_ID;
        let system_program = system_program::ID;
        let main_offer = self.state_main_offer();
        let redemption_offer = self.redemption_offer_pda;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::OpenSwapBuy { token_in_amount, minimum_out })
            .accounts(accounts::OpenSwapBuy {
                offer: offer,
                prop_amm_pair_state: prop_amm_pair_state,
                redemption_offer: redemption_offer,
                state: state,
                offer_vault_authority: offer_vault_authority,
                redemption_vault_authority: redemption_vault_authority,
                offer_vault_token_in_account: offer_vault_token_in_account,
                offer_vault_token_out_account: offer_vault_token_out_account,
                redemption_vault_token_in_account: redemption_vault_token_in_account,
                token_in_mint: token_in_mint,
                token_in_program: token_in_program,
                token_out_mint: token_out_mint,
                token_out_program: token_out_program,
                user_token_in_account: user_token_in_account,
                user_token_out_account: user_token_out_account,
                prop_amm_proceeds_vault: prop_amm_proceeds_vault,
                prop_amm_proceeds_token_in_account: prop_amm_proceeds_token_in_account,
                prop_amm_buy_fee_vault: prop_amm_buy_fee_vault,
                prop_amm_buy_fee_token_in_account: prop_amm_buy_fee_token_in_account,
                permissionless_authority: permissionless_authority,
                permissionless_token_in_account: permissionless_token_in_account,
                permissionless_token_out_account: permissionless_token_out_account,
                mint_authority: mint_authority,
                buffer_state: buffer_state,
                reserve_vault_onyc_account: reserve_vault_onyc_account,
                management_fee_vault_onyc_account: management_fee_vault_onyc_account,
                performance_fee_vault_onyc_account: performance_fee_vault_onyc_account,
                market_stats: market_stats,
                circulating_supply_excluded_balance: circulating_supply_excluded_balance,
                instructions_sysvar: instructions_sysvar,
                user: user,
                main_offer: main_offer,
            })
            .signers(&[&*self.payer, &__scout_signer_user])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:open_swap_buy:BEGIN
            // update shadow-ledger state after successful open_swap_buy
            // SCOUT:ACTION-HOOK:open_swap_buy:END
        }
        __scout_success
    }

    pub fn action_open_swap_sell(&mut self, minimum_out: u64) -> bool {
        let token_in_amount: u64 = self.swap_sell_amount_next();
        let state = self.state_pda;
        let offer_vault_authority = self.offer_vault_authority;
        let redemption_vault_authority = self.redemption_vault_authority;
        let redemption_vault_token_in_account = scout_ata(&self.redemption_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID);
        let redemption_vault_token_out_account = scout_ata(&self.redemption_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID);
        let token_in_mint = self.mint_onyc;
        let token_in_program = SPL_TOKEN_ID;
        let token_out_mint = self.mint_usdc;
        let token_out_program = SPL_TOKEN_ID;
        let user_token_in_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_onyc, &SPL_TOKEN_ID);
        let user_token_out_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_usdc, &SPL_TOKEN_ID);
        let prop_amm_proceeds_vault = self.cv_pda(CvKind::PropAmmProceeds);
        let prop_amm_proceeds_token_in_account = self.cv_ata(CvKind::PropAmmProceeds, &self.mint_onyc);
        let prop_amm_sell_fee_vault = self.cv_pda(CvKind::PropAmmSellFee);
        let prop_amm_sell_fee_token_in_account = self.cv_ata(CvKind::PropAmmSellFee, &self.mint_onyc);
        let mint_authority = self.mint_authority_pda;
        let buffer_state = self.buffer_state_pda();
        let reserve_vault_onyc_account = self.reserve_vault_onyc();
        let management_fee_vault_onyc_account = self.cv_ata(CvKind::ManagementFee, &self.mint_onyc);
        let performance_fee_vault_onyc_account = self.cv_ata(CvKind::PerformanceFee, &self.mint_onyc);
        let market_stats = self.market_stats_pda();
        let circulating_supply_excluded_balance = self.excluded_balance_pda();
        let instructions_sysvar = INSTRUCTIONS_SYSVAR_ID;
        let __scout_signer_user = self.pick_user(token_in_amount);
        let user = __scout_signer_user.pubkey();
        let associated_token_program = ATA_PROGRAM_ID;
        let system_program = system_program::ID;
        let main_offer = self.state_main_offer();
        let offer_vault_onyc_account = scout_ata(&self.offer_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID);
        let offer = self.offer_pda;
        let prop_amm_pair_state = self.prop_amm_pair_pda();
        let redemption_offer = self.redemption_offer_pda;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::OpenSwapSell { token_in_amount, minimum_out })
            .accounts(accounts::OpenSwapSell {
                offer: offer,
                prop_amm_pair_state: prop_amm_pair_state,
                redemption_offer: redemption_offer,
                state: state,
                offer_vault_authority: offer_vault_authority,
                redemption_vault_authority: redemption_vault_authority,
                redemption_vault_token_in_account: redemption_vault_token_in_account,
                redemption_vault_token_out_account: redemption_vault_token_out_account,
                token_in_mint: token_in_mint,
                token_in_program: token_in_program,
                token_out_mint: token_out_mint,
                token_out_program: token_out_program,
                user_token_in_account: user_token_in_account,
                user_token_out_account: user_token_out_account,
                prop_amm_proceeds_vault: prop_amm_proceeds_vault,
                prop_amm_proceeds_token_in_account: prop_amm_proceeds_token_in_account,
                prop_amm_sell_fee_vault: prop_amm_sell_fee_vault,
                prop_amm_sell_fee_token_in_account: prop_amm_sell_fee_token_in_account,
                mint_authority: mint_authority,
                buffer_state: buffer_state,
                reserve_vault_onyc_account: reserve_vault_onyc_account,
                management_fee_vault_onyc_account: management_fee_vault_onyc_account,
                performance_fee_vault_onyc_account: performance_fee_vault_onyc_account,
                market_stats: market_stats,
                circulating_supply_excluded_balance: circulating_supply_excluded_balance,
                instructions_sysvar: instructions_sysvar,
                user: user,
                main_offer: main_offer,
                offer_vault_onyc_account: offer_vault_onyc_account,
            })
            .signers(&[&*self.payer, &__scout_signer_user])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:open_swap_sell:BEGIN
            // update shadow-ledger state after successful open_swap_sell
            // SCOUT:ACTION-HOOK:open_swap_sell:END
        }
        __scout_success
    }

    pub fn action_take_offer_permissionless(&mut self, token_in_amount: u64) -> bool {
        let approval_message: Option<onreapp::types::ApprovalMessage> = None;
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let vault_authority = self.offer_vault_authority;
        let vault_token_in_account = scout_ata(&self.offer_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID);
        let vault_token_out_account = scout_ata(&self.offer_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID);
        let permissionless_authority = self.permissionless_authority;
        let permissionless_token_in_account = scout_ata(&self.permissionless_authority, &self.mint_usdc, &SPL_TOKEN_ID);
        let permissionless_token_out_account = scout_ata(&self.permissionless_authority, &self.mint_onyc, &SPL_TOKEN_ID);
        let token_in_mint = self.mint_usdc;
        let token_in_program = SPL_TOKEN_ID;
        let token_out_mint = self.mint_onyc;
        let token_out_program = SPL_TOKEN_ID;
        let user_token_in_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_usdc, &SPL_TOKEN_ID);
        let user_token_out_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_onyc, &SPL_TOKEN_ID);
        let boss_token_in_account = scout_ata(&self.boss.pubkey(), &self.mint_usdc, &SPL_TOKEN_ID);
        let mint_authority = self.mint_authority_pda;
        let instructions_sysvar = INSTRUCTIONS_SYSVAR_ID;
        let __scout_signer_user = self.pick_user(token_in_amount);
        let user = __scout_signer_user.pubkey();
        let associated_token_program = ATA_PROGRAM_ID;
        let system_program = system_program::ID;
        let offer = self.offer_pda;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::TakeOfferPermissionless { token_in_amount, approval_message })
            .accounts(accounts::TakeOfferPermissionless {
                offer: offer,
                state: state,
                boss: boss,
                vault_authority: vault_authority,
                vault_token_in_account: vault_token_in_account,
                vault_token_out_account: vault_token_out_account,
                permissionless_authority: permissionless_authority,
                permissionless_token_in_account: permissionless_token_in_account,
                permissionless_token_out_account: permissionless_token_out_account,
                token_in_mint: token_in_mint,
                token_in_program: token_in_program,
                token_out_mint: token_out_mint,
                token_out_program: token_out_program,
                user_token_in_account: user_token_in_account,
                user_token_out_account: user_token_out_account,
                boss_token_in_account: boss_token_in_account,
                mint_authority: mint_authority,
                instructions_sysvar: instructions_sysvar,
                user: user,
            })
            .signers(&[&*self.payer, &__scout_signer_user])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:take_offer_permissionless:BEGIN
            // update shadow-ledger state after successful take_offer_permissionless
            // SCOUT:ACTION-HOOK:take_offer_permissionless:END
        }
        __scout_success
    }

    pub fn action_take_offer_permissionless_v2(&mut self, token_in_amount: u64) -> bool {
        let state = self.state_pda;
        let vault_authority = self.offer_vault_authority;
        let vault_token_in_account = scout_ata(&self.offer_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID);
        let vault_token_out_account = scout_ata(&self.offer_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID);
        let permissionless_authority = self.permissionless_authority;
        let permissionless_token_in_account = scout_ata(&self.permissionless_authority, &self.mint_usdc, &SPL_TOKEN_ID);
        let permissionless_token_out_account = scout_ata(&self.permissionless_authority, &self.mint_onyc, &SPL_TOKEN_ID);
        let token_in_mint = self.mint_usdc;
        let token_in_program = SPL_TOKEN_ID;
        let token_out_mint = self.mint_onyc;
        let token_out_program = SPL_TOKEN_ID;
        let user_token_in_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_usdc, &SPL_TOKEN_ID);
        let user_token_out_account = scout_ata(&self.pick_user_pk(token_in_amount), &self.mint_onyc, &SPL_TOKEN_ID);
        let redemption_offer = self.redemption_offer_pda;
        let redemption_vault_authority = self.redemption_vault_authority;
        let redemption_vault_token_in_account = scout_ata(&self.redemption_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID);
        let offer_proceeds_vault = self.cv_pda(CvKind::OfferProceeds);
        let offer_proceeds_token_in_account = self.cv_ata(CvKind::OfferProceeds, &self.mint_usdc);
        let permissionless_offer_fee_vault = self.cv_pda(CvKind::PermissionlessOfferFee);
        let permissionless_offer_fee_token_in_account = self.cv_ata(CvKind::PermissionlessOfferFee, &self.mint_usdc);
        let mint_authority = self.mint_authority_pda;
        let buffer_state = self.buffer_state_pda();
        let reserve_vault_onyc_account = self.reserve_vault_onyc();
        let management_fee_vault_onyc_account = self.cv_ata(CvKind::ManagementFee, &self.mint_onyc);
        let performance_fee_vault_onyc_account = self.cv_ata(CvKind::PerformanceFee, &self.mint_onyc);
        let market_stats = self.market_stats_pda();
        let circulating_supply_excluded_balance = self.excluded_balance_pda();
        let __scout_signer_user = self.pick_user(token_in_amount);
        let user = __scout_signer_user.pubkey();
        let associated_token_program = ATA_PROGRAM_ID;
        let system_program = system_program::ID;
        let main_offer = self.state_main_offer();
        let offer = self.offer_pda;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::TakeOfferPermissionlessV2 { token_in_amount })
            .accounts(accounts::TakeOfferPermissionlessV2 {
                offer: offer,
                state: state,
                vault_authority: vault_authority,
                vault_token_in_account: vault_token_in_account,
                vault_token_out_account: vault_token_out_account,
                permissionless_authority: permissionless_authority,
                permissionless_token_in_account: permissionless_token_in_account,
                permissionless_token_out_account: permissionless_token_out_account,
                token_in_mint: token_in_mint,
                token_in_program: token_in_program,
                token_out_mint: token_out_mint,
                token_out_program: token_out_program,
                user_token_in_account: user_token_in_account,
                user_token_out_account: user_token_out_account,
                redemption_offer: redemption_offer,
                redemption_vault_authority: redemption_vault_authority,
                redemption_vault_token_in_account: redemption_vault_token_in_account,
                offer_proceeds_vault: offer_proceeds_vault,
                offer_proceeds_token_in_account: offer_proceeds_token_in_account,
                permissionless_offer_fee_vault: permissionless_offer_fee_vault,
                permissionless_offer_fee_token_in_account: permissionless_offer_fee_token_in_account,
                mint_authority: mint_authority,
                buffer_state: buffer_state,
                reserve_vault_onyc_account: reserve_vault_onyc_account,
                management_fee_vault_onyc_account: management_fee_vault_onyc_account,
                performance_fee_vault_onyc_account: performance_fee_vault_onyc_account,
                market_stats: market_stats,
                circulating_supply_excluded_balance: circulating_supply_excluded_balance,
                user: user,
                main_offer: main_offer,
            })
            .signers(&[&*self.payer, &__scout_signer_user])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:take_offer_permissionless_v2:BEGIN
            // update shadow-ledger state after successful take_offer_permissionless_v2
            // SCOUT:ACTION-HOOK:take_offer_permissionless_v2:END
        }
        __scout_success
    }

    pub fn action_propose_boss(&mut self) -> bool {
        let new_boss: Pubkey = self.boss.pubkey();
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::ProposeBoss { new_boss })
            .accounts(accounts::ProposeBoss {
                state: state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:propose_boss:BEGIN
            // update shadow-ledger state after successful propose_boss
            // SCOUT:ACTION-HOOK:propose_boss:END
        }
        __scout_success
    }

    #[cfg(feature = "admin_actions")]
    pub fn action_accept_boss(&mut self) -> bool {
        let state = self.state_pda;
        let new_boss = self.payer.pubkey();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::AcceptBoss {  })
            .accounts(accounts::AcceptBoss {
                state: state,
                new_boss: new_boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:accept_boss:BEGIN
            // update shadow-ledger state after successful accept_boss
            // SCOUT:ACTION-HOOK:accept_boss:END
        }
        __scout_success
    }
    #[cfg(not(feature = "admin_actions"))]
    pub fn action_accept_boss(&mut self) -> bool {
        // disabled: build with --features admin_actions to enable
        false
    }

    #[cfg(feature = "admin_actions")]
    pub fn action_add_admin(&mut self) -> bool {
        let new_admin: Pubkey = self.user_a.pubkey();
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::AddAdmin { new_admin })
            .accounts(accounts::AddAdmin {
                state: state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:add_admin:BEGIN
            // update shadow-ledger state after successful add_admin
            // SCOUT:ACTION-HOOK:add_admin:END
        }
        __scout_success
    }
    #[cfg(not(feature = "admin_actions"))]
    pub fn action_add_admin(&mut self) -> bool {
        // disabled: build with --features admin_actions to enable
        false
    }

    #[cfg(feature = "admin_actions")]
    pub fn action_remove_admin(&mut self) -> bool {
        let admin_to_remove: Pubkey = self.user_a.pubkey();
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::RemoveAdmin { admin_to_remove })
            .accounts(accounts::RemoveAdmin {
                state: state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:remove_admin:BEGIN
            // update shadow-ledger state after successful remove_admin
            // SCOUT:ACTION-HOOK:remove_admin:END
        }
        __scout_success
    }
    #[cfg(not(feature = "admin_actions"))]
    pub fn action_remove_admin(&mut self) -> bool {
        // disabled: build with --features admin_actions to enable
        false
    }

    #[cfg(feature = "admin_actions")]
    pub fn action_clear_admins(&mut self) -> bool {
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::ClearAdmins {  })
            .accounts(accounts::ClearAdmins {
                state: state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:clear_admins:BEGIN
            // update shadow-ledger state after successful clear_admins
            // SCOUT:ACTION-HOOK:clear_admins:END
        }
        __scout_success
    }
    #[cfg(not(feature = "admin_actions"))]
    pub fn action_clear_admins(&mut self) -> bool {
        // disabled: build with --features admin_actions to enable
        false
    }

    pub fn action_transfer_mint_authority_to_program(&mut self) -> bool {
        let boss = self.boss.pubkey();
        let state = self.state_pda;
        let mint = self.mint_play;
        let mint_authority = self.mint_authority_pda;
        let token_program = SPL_TOKEN_ID;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::TransferMintAuthorityToProgram {  })
            .accounts(accounts::TransferMintAuthorityToProgram {
                boss: boss,
                state: state,
                mint: mint,
                mint_authority: mint_authority,
                token_program: token_program,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:transfer_mint_authority_to_program:BEGIN
            // update shadow-ledger state after successful transfer_mint_authority_to_program
            // SCOUT:ACTION-HOOK:transfer_mint_authority_to_program:END
        }
        __scout_success
    }

    pub fn action_transfer_mint_authority_to_boss(&mut self) -> bool {
        let boss = self.boss.pubkey();
        let state = self.state_pda;
        let mint = self.mint_play;
        let mint_authority = self.mint_authority_pda;
        let token_program = SPL_TOKEN_ID;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::TransferMintAuthorityToBoss {  })
            .accounts(accounts::TransferMintAuthorityToBoss {
                boss: boss,
                state: state,
                mint: mint,
                mint_authority: mint_authority,
                token_program: token_program,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:transfer_mint_authority_to_boss:BEGIN
            // update shadow-ledger state after successful transfer_mint_authority_to_boss
            // SCOUT:ACTION-HOOK:transfer_mint_authority_to_boss:END
        }
        __scout_success
    }

    pub fn action_set_kill_switch(&mut self, enable: bool) -> bool {
        let state = self.state_pda;
        let signer = self.payer.pubkey();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::SetKillSwitch { enable })
            .accounts(accounts::SetKillSwitch {
                state: state,
                signer: signer,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:set_kill_switch:BEGIN
            // update shadow-ledger state after successful set_kill_switch
            // SCOUT:ACTION-HOOK:set_kill_switch:END
        }
        __scout_success
    }

    pub fn action_set_onyc_mint(&mut self) -> bool {
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let onyc_mint = self.mint_onyc;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::SetOnycMint {  })
            .accounts(accounts::SetOnycMint {
                state: state,
                boss: boss,
                onyc_mint: onyc_mint,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:set_onyc_mint:BEGIN
            // update shadow-ledger state after successful set_onyc_mint
            // SCOUT:ACTION-HOOK:set_onyc_mint:END
        }
        __scout_success
    }

    pub fn action_set_worker(&mut self) -> bool {
        let new_worker: Pubkey = self.other_worker();
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::SetWorker { new_worker })
            .accounts(accounts::SetWorker {
                state: state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:set_worker:BEGIN
            // update shadow-ledger state after successful set_worker
            // SCOUT:ACTION-HOOK:set_worker:END
        }
        __scout_success
    }

    #[cfg(feature = "admin_actions")]
    pub fn action_initialize_buffer(&mut self) -> bool {
        let state = self.state_pda;
        let buffer_state = self.buffer_state_pda();
        let reserve_vault_authority = self.reserve_vault_authority();
        let management_fee_vault = self.cv_pda(CvKind::ManagementFee);
        let performance_fee_vault = self.cv_pda(CvKind::PerformanceFee);
        let boss = self.boss.pubkey();
        let onyc_mint = self.mint_onyc;
        let offer = self.offer_pda;
        let reserve_vault_onyc_account = self.reserve_vault_onyc();
        let management_fee_vault_onyc_account = self.cv_ata(CvKind::ManagementFee, &self.mint_onyc);
        let performance_fee_vault_onyc_account = self.cv_ata(CvKind::PerformanceFee, &self.mint_onyc);
        let token_program = SPL_TOKEN_ID;
        let associated_token_program = ATA_PROGRAM_ID;
        let system_program = system_program::ID;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::InitializeBuffer {  })
            .accounts(accounts::InitializeBuffer {
                state: state,
                buffer_state: buffer_state,
                reserve_vault_authority: reserve_vault_authority,
                management_fee_vault: management_fee_vault,
                performance_fee_vault: performance_fee_vault,
                boss: boss,
                onyc_mint: onyc_mint,
                offer: offer,
                reserve_vault_onyc_account: reserve_vault_onyc_account,
                management_fee_vault_onyc_account: management_fee_vault_onyc_account,
                performance_fee_vault_onyc_account: performance_fee_vault_onyc_account,
                token_program: token_program,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:initialize_buffer:BEGIN
            // update shadow-ledger state after successful initialize_buffer
            // SCOUT:ACTION-HOOK:initialize_buffer:END
        }
        __scout_success
    }
    #[cfg(not(feature = "admin_actions"))]
    pub fn action_initialize_buffer(&mut self) -> bool {
        // disabled: build with --features admin_actions to enable
        false
    }

    pub fn action_settle_buffer(&mut self) -> bool {
        let state = self.state_pda;
        let __scout_signer_worker = self.worker_kp().insecure_clone();
        let worker = __scout_signer_worker.pubkey();
        let onyc_mint = self.mint_onyc;
        let mint_authority = self.mint_authority_pda;
        let token_program = SPL_TOKEN_ID;
        let system_program = system_program::ID;
        let main_offer = self.state_main_offer();
        let buffer_state = self.buffer_state_pda();
        let reserve_vault_onyc_account = self.reserve_vault_onyc();
        let management_fee_vault_onyc_account = self.cv_ata(CvKind::ManagementFee, &self.mint_onyc);
        let performance_fee_vault_onyc_account = self.cv_ata(CvKind::PerformanceFee, &self.mint_onyc);
        let market_stats = self.market_stats_pda();
        let circulating_supply_excluded_balance = self.excluded_balance_pda();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::SettleBuffer {  })
            .accounts(accounts::SettleBuffer {
                state: state,
                worker: worker,
                onyc_mint: onyc_mint,
                mint_authority: mint_authority,
                token_program: token_program,
                main_offer: main_offer,
                buffer_state: buffer_state,
                reserve_vault_onyc_account: reserve_vault_onyc_account,
                management_fee_vault_onyc_account: management_fee_vault_onyc_account,
                performance_fee_vault_onyc_account: performance_fee_vault_onyc_account,
                market_stats: market_stats,
                circulating_supply_excluded_balance: circulating_supply_excluded_balance,
            })
            .signers(&[&*self.payer, &__scout_signer_worker])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:settle_buffer:BEGIN
            // update shadow-ledger state after successful settle_buffer
            // SCOUT:ACTION-HOOK:settle_buffer:END
        }
        __scout_success
    }

    pub fn action_set_main_offer(&mut self) -> bool {
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let offer = self.next_main_offer_candidate();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::SetMainOffer {  })
            .accounts(accounts::SetMainOffer {
                state: state,
                boss: boss,
                offer: offer,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:set_main_offer:BEGIN
            // update shadow-ledger state after successful set_main_offer
            // SCOUT:ACTION-HOOK:set_main_offer:END
        }
        __scout_success
    }

    #[cfg(feature = "admin_actions")]
    pub fn action_configure_prop_amm(&mut self, enabled: bool, curve_peg_haircut_bps: u16, cadence_threshold: u32, epoch_duration_seconds: i64, wall_sensitivity_scaled: u32, minimum_sell_haircut_onyc: u64) -> bool {
        let curve_exponent_scaled: u32 = 1_000 + (cadence_threshold % 100) * 1_000;
        let cadence_wave_scaled: u32 = (cadence_threshold % 51) * 1_000;
        let state = self.state_pda;
        let asset_mint = self.mint_usdc;
        let boss = self.boss.pubkey();
        let system_program = system_program::ID;
        let offer = self.offer_pda;
        let prop_amm_pair_state = self.prop_amm_pair_pda();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::ConfigurePropAmm { enabled, curve_peg_haircut_bps, curve_exponent_scaled, cadence_threshold, cadence_wave_scaled, epoch_duration_seconds, wall_sensitivity_scaled, minimum_sell_haircut_onyc })
            .accounts(accounts::ConfigurePropAmm {
                state: state,
                offer: offer,
                asset_mint: asset_mint,
                prop_amm_pair_state: prop_amm_pair_state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:configure_prop_amm:BEGIN
            // update shadow-ledger state after successful configure_prop_amm
            // SCOUT:ACTION-HOOK:configure_prop_amm:END
        }
        __scout_success
    }
    #[cfg(not(feature = "admin_actions"))]
    pub fn action_configure_prop_amm(&mut self, enabled: bool, curve_peg_haircut_bps: u16, cadence_threshold: u32, epoch_duration_seconds: i64, wall_sensitivity_scaled: u32, minimum_sell_haircut_onyc: u64) -> bool {
        // disabled: build with --features admin_actions to enable
        false
    }

    pub fn action_set_buffer_gross_apr(&mut self, gross_yield: u64) -> bool {
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let main_offer = self.state_main_offer();
        let onyc_mint = self.mint_onyc;
        let offer_vault_authority = self.offer_vault_authority;
        let mint_authority = self.mint_authority_pda;
        let buffer_state = self.buffer_state_pda();
        let reserve_vault_onyc_account = self.reserve_vault_onyc();
        let management_fee_vault_onyc_account = self.cv_ata(CvKind::ManagementFee, &self.mint_onyc);
        let performance_fee_vault_onyc_account = self.cv_ata(CvKind::PerformanceFee, &self.mint_onyc);
        let token_program = SPL_TOKEN_ID;
        let system_program = system_program::ID;
        let market_stats = self.market_stats_pda();
        let circulating_supply_excluded_balance = self.excluded_balance_pda();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::SetBufferGrossApr { gross_yield })
            .accounts(accounts::SetBufferGrossApr {
                state: state,
                boss: boss,
                main_offer: main_offer,
                onyc_mint: onyc_mint,
                offer_vault_authority: offer_vault_authority,
                mint_authority: mint_authority,
                buffer_state: buffer_state,
                reserve_vault_onyc_account: reserve_vault_onyc_account,
                management_fee_vault_onyc_account: management_fee_vault_onyc_account,
                performance_fee_vault_onyc_account: performance_fee_vault_onyc_account,
                token_program: token_program,
                market_stats: market_stats,
                circulating_supply_excluded_balance: circulating_supply_excluded_balance,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:set_buffer_gross_apr:BEGIN
            // update shadow-ledger state after successful set_buffer_gross_apr
            // SCOUT:ACTION-HOOK:set_buffer_gross_apr:END
        }
        __scout_success
    }

    #[cfg(feature = "admin_actions")]
    pub fn action_set_buffer_fee_config(&mut self, management_fee_basis_points: u16, performance_fee_basis_points: u16, performance_fee_high_watermark_enabled: bool) -> bool {
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let main_offer = self.state_main_offer();
        let onyc_mint = self.mint_onyc;
        let offer_vault_authority = self.offer_vault_authority;
        let mint_authority = self.mint_authority_pda;
        let buffer_state = self.buffer_state_pda();
        let reserve_vault_onyc_account = self.reserve_vault_onyc();
        let management_fee_vault_onyc_account = self.cv_ata(CvKind::ManagementFee, &self.mint_onyc);
        let performance_fee_vault_onyc_account = self.cv_ata(CvKind::PerformanceFee, &self.mint_onyc);
        let token_program = SPL_TOKEN_ID;
        let system_program = system_program::ID;
        let market_stats = self.market_stats_pda();
        let circulating_supply_excluded_balance = self.excluded_balance_pda();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::SetBufferFeeConfig { management_fee_basis_points, performance_fee_basis_points, performance_fee_high_watermark_enabled })
            .accounts(accounts::SetBufferFeeConfig {
                state: state,
                boss: boss,
                main_offer: main_offer,
                onyc_mint: onyc_mint,
                offer_vault_authority: offer_vault_authority,
                mint_authority: mint_authority,
                buffer_state: buffer_state,
                reserve_vault_onyc_account: reserve_vault_onyc_account,
                management_fee_vault_onyc_account: management_fee_vault_onyc_account,
                performance_fee_vault_onyc_account: performance_fee_vault_onyc_account,
                token_program: token_program,
                market_stats: market_stats,
                circulating_supply_excluded_balance: circulating_supply_excluded_balance,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:set_buffer_fee_config:BEGIN
            // update shadow-ledger state after successful set_buffer_fee_config
            // SCOUT:ACTION-HOOK:set_buffer_fee_config:END
        }
        __scout_success
    }
    #[cfg(not(feature = "admin_actions"))]
    pub fn action_set_buffer_fee_config(&mut self, management_fee_basis_points: u16, performance_fee_basis_points: u16, performance_fee_high_watermark_enabled: bool) -> bool {
        // disabled: build with --features admin_actions to enable
        false
    }

    pub fn action_burn_for_nav_increase(&mut self) -> bool {
        let asset_adjustment_amount: u64 = self.nav_burn_amount_next();
        let state = self.state_pda;
        let buffer_state = self.buffer_state_pda();
        let boss = self.boss.pubkey();
        let main_offer = self.state_main_offer();
        let onyc_mint = self.mint_onyc;
        let offer_vault_authority = self.offer_vault_authority;
        let reserve_vault_authority = self.reserve_vault_authority();
        let reserve_vault_onyc_account = self.reserve_vault_onyc();
        let management_fee_vault = self.cv_pda(CvKind::ManagementFee);
        let management_fee_vault_onyc_account = self.cv_ata(CvKind::ManagementFee, &self.mint_onyc);
        let performance_fee_vault = self.cv_pda(CvKind::PerformanceFee);
        let performance_fee_vault_onyc_account = self.cv_ata(CvKind::PerformanceFee, &self.mint_onyc);
        let mint_authority = self.mint_authority_pda;
        let token_program = SPL_TOKEN_ID;
        let system_program = system_program::ID;
        let market_stats = self.market_stats_pda();
        let circulating_supply_excluded_balance = self.excluded_balance_pda();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::BurnForNavIncrease { asset_adjustment_amount })
            .accounts(accounts::BurnForNavIncrease {
                state: state,
                buffer_state: buffer_state,
                boss: boss,
                main_offer: main_offer,
                onyc_mint: onyc_mint,
                offer_vault_authority: offer_vault_authority,
                reserve_vault_authority: reserve_vault_authority,
                reserve_vault_onyc_account: reserve_vault_onyc_account,
                management_fee_vault: management_fee_vault,
                management_fee_vault_onyc_account: management_fee_vault_onyc_account,
                performance_fee_vault: performance_fee_vault,
                performance_fee_vault_onyc_account: performance_fee_vault_onyc_account,
                mint_authority: mint_authority,
                token_program: token_program,
                market_stats: market_stats,
                circulating_supply_excluded_balance: circulating_supply_excluded_balance,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:burn_for_nav_increase:BEGIN
            // update shadow-ledger state after successful burn_for_nav_increase
            // SCOUT:ACTION-HOOK:burn_for_nav_increase:END
        }
        __scout_success
    }

    pub fn action_deposit_reserve_vault(&mut self, amount: u64) -> bool {
        let state = self.state_pda;
        let buffer_state = self.buffer_state_pda();
        let reserve_vault_authority = self.reserve_vault_authority();
        let onyc_mint = self.mint_onyc;
        let depositor_onyc_account = scout_ata(&self.pick_user_pk(amount), &self.mint_onyc, &SPL_TOKEN_ID);
        let reserve_vault_onyc_account = self.reserve_vault_onyc();
        let __scout_signer_depositor = self.pick_user(amount);
        let depositor = __scout_signer_depositor.pubkey();
        let token_program = SPL_TOKEN_ID;
        let associated_token_program = ATA_PROGRAM_ID;
        let system_program = system_program::ID;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::DepositReserveVault { amount })
            .accounts(accounts::DepositReserveVault {
                state: state,
                buffer_state: buffer_state,
                reserve_vault_authority: reserve_vault_authority,
                onyc_mint: onyc_mint,
                depositor_onyc_account: depositor_onyc_account,
                reserve_vault_onyc_account: reserve_vault_onyc_account,
                depositor: depositor,
                token_program: token_program,
            })
            .signers(&[&*self.payer, &__scout_signer_depositor])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:deposit_reserve_vault:BEGIN
            // update shadow-ledger state after successful deposit_reserve_vault
            // SCOUT:ACTION-HOOK:deposit_reserve_vault:END
        }
        __scout_success
    }

    pub fn action_withdraw_reserve_vault(&mut self, amount: u64) -> bool {
        let state = self.state_pda;
        let buffer_state = self.buffer_state_pda();
        let reserve_vault_authority = self.reserve_vault_authority();
        let onyc_mint = self.mint_onyc;
        let boss_onyc_account = scout_ata(&self.boss.pubkey(), &self.mint_onyc, &SPL_TOKEN_ID);
        let reserve_vault_onyc_account = self.reserve_vault_onyc();
        let boss = self.boss.pubkey();
        let token_program = SPL_TOKEN_ID;
        let associated_token_program = ATA_PROGRAM_ID;
        let system_program = system_program::ID;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::WithdrawReserveVault { amount })
            .accounts(accounts::WithdrawReserveVault {
                state: state,
                buffer_state: buffer_state,
                reserve_vault_authority: reserve_vault_authority,
                onyc_mint: onyc_mint,
                boss_onyc_account: boss_onyc_account,
                reserve_vault_onyc_account: reserve_vault_onyc_account,
                boss: boss,
                token_program: token_program,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:withdraw_reserve_vault:BEGIN
            // update shadow-ledger state after successful withdraw_reserve_vault
            // SCOUT:ACTION-HOOK:withdraw_reserve_vault:END
        }
        __scout_success
    }

    pub fn action_mint_to(&mut self, amount: u64) -> bool {
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let onyc_mint = self.mint_onyc;
        let boss_onyc_account = scout_ata(&self.boss.pubkey(), &self.mint_onyc, &SPL_TOKEN_ID);
        let mint_authority = self.mint_authority_pda;
        let token_program = SPL_TOKEN_ID;
        let associated_token_program = ATA_PROGRAM_ID;
        let system_program = system_program::ID;
        let main_offer = self.state_main_offer();
        let buffer_state = self.buffer_state_pda();
        let reserve_vault_onyc_account = self.reserve_vault_onyc();
        let management_fee_vault_onyc_account = self.cv_ata(CvKind::ManagementFee, &self.mint_onyc);
        let performance_fee_vault_onyc_account = self.cv_ata(CvKind::PerformanceFee, &self.mint_onyc);
        let market_stats = self.market_stats_pda();
        let circulating_supply_excluded_balance = self.excluded_balance_pda();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::MintTo { amount })
            .accounts(accounts::MintTo {
                state: state,
                boss: boss,
                onyc_mint: onyc_mint,
                boss_onyc_account: boss_onyc_account,
                mint_authority: mint_authority,
                token_program: token_program,
                main_offer: main_offer,
                buffer_state: buffer_state,
                reserve_vault_onyc_account: reserve_vault_onyc_account,
                management_fee_vault_onyc_account: management_fee_vault_onyc_account,
                performance_fee_vault_onyc_account: performance_fee_vault_onyc_account,
                market_stats: market_stats,
                circulating_supply_excluded_balance: circulating_supply_excluded_balance,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:mint_to:BEGIN
            // update shadow-ledger state after successful mint_to
            // SCOUT:ACTION-HOOK:mint_to:END
        }
        __scout_success
    }

    pub fn action_get_nav(&mut self) -> bool {
        let token_in_mint = self.mint_usdc;
        let token_out_mint = self.mint_onyc;
        let offer = self.offer_pda;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::GetNav {  })
            .accounts(accounts::GetNav {
                offer: offer,
                token_in_mint: token_in_mint,
                token_out_mint: token_out_mint,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:get_nav:BEGIN
            // update shadow-ledger state after successful get_nav
            // SCOUT:ACTION-HOOK:get_nav:END
        }
        __scout_success
    }

    pub fn action_get_apy(&mut self) -> bool {
        let token_in_mint = self.mint_usdc;
        let token_out_mint = self.mint_onyc;
        let offer = self.offer_pda;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::GetApy {  })
            .accounts(accounts::GetApy {
                offer: offer,
                token_in_mint: token_in_mint,
                token_out_mint: token_out_mint,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:get_apy:BEGIN
            // update shadow-ledger state after successful get_apy
            // SCOUT:ACTION-HOOK:get_apy:END
        }
        __scout_success
    }

    pub fn action_get_nav_adjustment(&mut self) -> bool {
        let token_in_mint = self.mint_usdc;
        let token_out_mint = self.mint_onyc;
        let offer = self.offer_pda;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::GetNavAdjustment {  })
            .accounts(accounts::GetNavAdjustment {
                offer: offer,
                token_in_mint: token_in_mint,
                token_out_mint: token_out_mint,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:get_nav_adjustment:BEGIN
            // update shadow-ledger state after successful get_nav_adjustment
            // SCOUT:ACTION-HOOK:get_nav_adjustment:END
        }
        __scout_success
    }

    pub fn action_get_tvl(&mut self) -> bool {
        let token_in_mint = self.mint_usdc;
        let token_out_mint = self.mint_onyc;
        let vault_authority = self.offer_vault_authority;
        let vault_token_out_account = scout_ata(&self.offer_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID);
        let token_out_program = SPL_TOKEN_ID;
        let offer = self.offer_pda;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::GetTvl {  })
            .accounts(accounts::GetTvl {
                offer: offer,
                token_in_mint: token_in_mint,
                token_out_mint: token_out_mint,
                vault_authority: vault_authority,
                vault_token_out_account: vault_token_out_account,
                token_out_program: token_out_program,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:get_tvl:BEGIN
            // update shadow-ledger state after successful get_tvl
            // SCOUT:ACTION-HOOK:get_tvl:END
        }
        __scout_success
    }

    pub fn action_get_circulating_supply(&mut self) -> bool {
        let onyc_mint = self.mint_onyc;
        let state = self.state_pda;
        let vault_authority = self.offer_vault_authority;
        let onyc_vault_account = scout_ata(&self.offer_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID);
        let token_program = SPL_TOKEN_ID;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::GetCirculatingSupply {  })
            .accounts(accounts::GetCirculatingSupply {
                onyc_mint: onyc_mint,
                state: state,
                vault_authority: vault_authority,
                onyc_vault_account: onyc_vault_account,
                token_program: token_program,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:get_circulating_supply:BEGIN
            // update shadow-ledger state after successful get_circulating_supply
            // SCOUT:ACTION-HOOK:get_circulating_supply:END
        }
        __scout_success
    }

    pub fn action_get_circulating_supply_v2(&mut self) -> bool {
        let onyc_mint = self.mint_onyc;
        let state = self.state_pda;
        let circulating_supply_excluded_balance = self.excluded_balance_pda();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::GetCirculatingSupplyV2 {  })
            .accounts(accounts::GetCirculatingSupplyV2 {
                onyc_mint: onyc_mint,
                state: state,
                circulating_supply_excluded_balance: circulating_supply_excluded_balance,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:get_circulating_supply_v2:BEGIN
            // update shadow-ledger state after successful get_circulating_supply_v2
            // SCOUT:ACTION-HOOK:get_circulating_supply_v2:END
        }
        __scout_success
    }

    pub fn action_refresh_market_stats(&mut self) -> bool {
        let main_offer = self.state_main_offer();
        let token_in_mint = self.mint_usdc;
        let state = self.state_pda;
        let onyc_mint = self.mint_onyc;
        let circulating_supply_excluded_balance = self.excluded_balance_pda();
        let market_stats = self.market_stats_pda();
        let signer = self.payer.pubkey();
        let system_program = system_program::ID;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::RefreshMarketStats {  })
            .accounts(accounts::RefreshMarketStats {
                main_offer: main_offer,
                token_in_mint: token_in_mint,
                state: state,
                onyc_mint: onyc_mint,
                circulating_supply_excluded_balance: circulating_supply_excluded_balance,
                market_stats: market_stats,
                signer: signer,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:refresh_market_stats:BEGIN
            // update shadow-ledger state after successful refresh_market_stats
            // SCOUT:ACTION-HOOK:refresh_market_stats:END
        }
        __scout_success
    }

    pub fn action_get_tvl_v2(&mut self) -> bool {
        let token_in_mint = self.mint_usdc;
        let token_out_mint = self.mint_onyc;
        let state = self.state_pda;
        let circulating_supply_excluded_balance = self.excluded_balance_pda();
        let offer = self.offer_pda;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::GetTvlV2 {  })
            .accounts(accounts::GetTvlV2 {
                offer: offer,
                token_in_mint: token_in_mint,
                token_out_mint: token_out_mint,
                state: state,
                circulating_supply_excluded_balance: circulating_supply_excluded_balance,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:get_tvl_v2:BEGIN
            // update shadow-ledger state after successful get_tvl_v2
            // SCOUT:ACTION-HOOK:get_tvl_v2:END
        }
        __scout_success
    }

    pub fn action_set_circulating_supply_excluded_accounts(&mut self) -> bool {
        let owners = self.excluded_owner_list();
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let excluded_accounts = self.excluded_accounts_pda();
        let system_program = system_program::ID;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::SetCirculatingSupplyExcludedAccounts { owners })
            .accounts(accounts::SetCirculatingSupplyExcludedAccounts {
                state: state,
                boss: boss,
                excluded_accounts: excluded_accounts,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:set_circulating_supply_excluded_accounts:BEGIN
            // update shadow-ledger state after successful set_circulating_supply_excluded_accounts
            // SCOUT:ACTION-HOOK:set_circulating_supply_excluded_accounts:END
        }
        __scout_success
    }

    pub fn action_update_circulating_supply_excluded_balance(&mut self) -> bool {
        let state = self.state_pda;
        let onyc_mint = self.mint_onyc;
        let excluded_accounts = self.excluded_accounts_pda();
        let circulating_supply_excluded_balance = self.excluded_balance_pda();
        let token_program = SPL_TOKEN_ID;
        let signer = self.payer.pubkey();
        let system_program = system_program::ID;
        let __scout_remaining_accounts = self.excluded_owner_atas();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::UpdateCirculatingSupplyExcludedBalance {  })
            .accounts(accounts::UpdateCirculatingSupplyExcludedBalance {
                state: state,
                onyc_mint: onyc_mint,
                excluded_accounts: excluded_accounts,
                circulating_supply_excluded_balance: circulating_supply_excluded_balance,
                token_program: token_program,
                signer: signer,
            })
            .remaining_accounts(__scout_remaining_accounts)
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:update_circulating_supply_excluded_balance:BEGIN
            // update shadow-ledger state after successful update_circulating_supply_excluded_balance
            // SCOUT:ACTION-HOOK:update_circulating_supply_excluded_balance:END
        }
        __scout_success
    }

    pub fn action_add_approver(&mut self) -> bool {
        let approver: Pubkey = self.user_b.pubkey();
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::AddApprover { approver })
            .accounts(accounts::AddApprover {
                state: state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:add_approver:BEGIN
            // update shadow-ledger state after successful add_approver
            // SCOUT:ACTION-HOOK:add_approver:END
        }
        __scout_success
    }

    pub fn action_remove_approver(&mut self) -> bool {
        let approver: Pubkey = self.approver.pubkey();
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::RemoveApprover { approver })
            .accounts(accounts::RemoveApprover {
                state: state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:remove_approver:BEGIN
            // update shadow-ledger state after successful remove_approver
            // SCOUT:ACTION-HOOK:remove_approver:END
        }
        __scout_success
    }

    #[cfg(feature = "admin_actions")]
    pub fn action_configure_max_supply(&mut self, max_supply: u64) -> bool {
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::ConfigureMaxSupply { max_supply })
            .accounts(accounts::ConfigureMaxSupply {
                state: state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:configure_max_supply:BEGIN
            // update shadow-ledger state after successful configure_max_supply
            // SCOUT:ACTION-HOOK:configure_max_supply:END
        }
        __scout_success
    }
    #[cfg(not(feature = "admin_actions"))]
    pub fn action_configure_max_supply(&mut self, max_supply: u64) -> bool {
        // disabled: build with --features admin_actions to enable
        false
    }

    pub fn action_configure_max_mint_amount(&mut self, max_mint_amount: u64) -> bool {
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::ConfigureMaxMintAmount { max_mint_amount })
            .accounts(accounts::ConfigureMaxMintAmount {
                state: state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:configure_max_mint_amount:BEGIN
            // update shadow-ledger state after successful configure_max_mint_amount
            // SCOUT:ACTION-HOOK:configure_max_mint_amount:END
        }
        __scout_success
    }

    pub fn action_close_state(&mut self) -> bool {
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let system_program = system_program::ID;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::CloseState {  })
            .accounts(accounts::CloseState {
                state: state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:close_state:BEGIN
            // Nothing to record. close_state deallocates the State PDA by design; recovery is
            // exposed as `action_scout_rebuild_state` (SCOUT:EXTRA-ACTIONS) rather than done here,
            // because a hook region may hold only pure assignments — no calls, no conditionals.
            // SCOUT:ACTION-HOOK:close_state:END
        }
        __scout_success
    }

    pub fn action_make_redemption_offer(&mut self, fee_basis_points: u16) -> bool {
        let fee_basis_points_prop_amm_sell: u16 = 50;
        let state = self.state_pda;
        let redemption_vault_authority = self.redemption_vault_authority;
        let token_in_mint = self.mint_onyc;
        let token_in_program = SPL_TOKEN_ID;
        let vault_token_in_account = scout_ata(&self.redemption_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID);
        let token_out_mint = self.mint_play;
        let token_out_program = SPL_TOKEN_ID;
        let vault_token_out_account = scout_ata(&self.redemption_vault_authority, &self.mint_play, &SPL_TOKEN_ID);
        let redemption_offer = scout_pda(&[SEED_REDEMPTION_OFFER, self.mint_onyc.as_ref(), self.mint_play.as_ref()], &self.program_id);
        let __scout_signer_boss = self.boss.insecure_clone();
        let boss = __scout_signer_boss.pubkey();
        let associated_token_program = ATA_PROGRAM_ID;
        let system_program = system_program::ID;
        let offer = scout_pda(&[SEED_OFFER, self.mint_play.as_ref(), self.mint_onyc.as_ref()], &self.program_id);
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::MakeRedemptionOffer { fee_basis_points, fee_basis_points_prop_amm_sell })
            .accounts(accounts::MakeRedemptionOffer {
                state: state,
                offer: offer,
                redemption_vault_authority: redemption_vault_authority,
                token_in_mint: token_in_mint,
                token_in_program: token_in_program,
                vault_token_in_account: vault_token_in_account,
                token_out_mint: token_out_mint,
                token_out_program: token_out_program,
                vault_token_out_account: vault_token_out_account,
                redemption_offer: redemption_offer,
                boss: boss,
            })
            .signers(&[&*self.payer, &__scout_signer_boss])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:make_redemption_offer:BEGIN
            // update shadow-ledger state after successful make_redemption_offer
            // SCOUT:ACTION-HOOK:make_redemption_offer:END
        }
        __scout_success
    }

    pub fn action_set_redemption_offer_disabled(&mut self, disabled: bool) -> bool {
        let redemption_offer = self.redemption_offer_pda;
        let state = self.state_pda;
        let signer = self.payer.pubkey();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::SetRedemptionOfferDisabled { disabled })
            .accounts(accounts::SetRedemptionOfferDisabled {
                redemption_offer: redemption_offer,
                state: state,
                signer: signer,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:set_redemption_offer_disabled:BEGIN
            // update shadow-ledger state after successful set_redemption_offer_disabled
            // SCOUT:ACTION-HOOK:set_redemption_offer_disabled:END
        }
        __scout_success
    }

    pub fn action_create_redemption_request(&mut self, amount: u64) -> bool {
        let state = self.state_pda;
        let redemption_offer = self.redemption_offer_pda;
        let offer = self.offer_pda;
        let redemption_request = self.next_request_pda();
        let __scout_signer_redeemer = self.pick_user(amount);
        let redeemer = __scout_signer_redeemer.pubkey();
        let redemption_vault_authority = self.redemption_vault_authority;
        let token_in_mint = self.mint_onyc;
        let redeemer_token_account = scout_ata(&self.pick_user_pk(amount), &self.mint_onyc, &SPL_TOKEN_ID);
        let vault_token_account = scout_ata(&self.redemption_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID);
        let token_program = SPL_TOKEN_ID;
        let associated_token_program = ATA_PROGRAM_ID;
        let system_program = system_program::ID;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::CreateRedemptionRequest { amount })
            .accounts(accounts::CreateRedemptionRequest {
                state: state,
                redemption_offer: redemption_offer,
                offer: offer,
                redemption_request: redemption_request,
                redeemer: redeemer,
                redemption_vault_authority: redemption_vault_authority,
                token_in_mint: token_in_mint,
                redeemer_token_account: redeemer_token_account,
                vault_token_account: vault_token_account,
                token_program: token_program,
            })
            .signers(&[&*self.payer, &__scout_signer_redeemer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:create_redemption_request:BEGIN
            // Each property keeps its OWN registry: an isolated single-property replay runs
            // exactly one of these hooks, so a shared counter would advance a different number of
            // times depending on which property was selected.
            scout_run_property!("P-0003", {
                self.scout_p3_reqs[self.scout_p3_next % SCOUT_REQ_CAP] = redemption_request;
                self.scout_p3_next = self.scout_p3_next.saturating_add(1);
                self.scout_p7_reqs[self.scout_p7_next % SCOUT_P7_CAP] = redemption_request;
                self.scout_p7_next = self.scout_p7_next.saturating_add(1);
            });
            // P-0007's block is retired, but its registry stays fed under P-0003's gate so the
            // pooled-solvency test (c5_pool_stays_solvent_without_the_drain) still has its subject.
            // SCOUT:ACTION-HOOK:create_redemption_request:END
        }
        __scout_success
    }

    pub fn action_fulfill_redemption_request(&mut self) -> bool {
        let amount: u64 = self.fulfill_amount_next();
        let state = self.state_pda;
        let offer = self.offer_pda;
        let redemption_offer = self.redemption_offer_pda;
        let redemption_request = self.oldest_request_pda();
        let redemption_vault_authority = self.redemption_vault_authority;
        let vault_token_in_account = scout_ata(&self.redemption_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID);
        let vault_token_out_account = scout_ata(&self.redemption_vault_authority, &self.mint_usdc, &SPL_TOKEN_ID);
        let token_in_mint = self.mint_onyc;
        let token_in_program = SPL_TOKEN_ID;
        let token_out_mint = self.mint_usdc;
        let token_out_program = SPL_TOKEN_ID;
        let user_token_out_account = scout_ata(&self.oldest_request_redeemer(), &self.mint_usdc, &SPL_TOKEN_ID);
        let offer_proceeds_vault = self.cv_pda(CvKind::OfferProceeds);
        let offer_proceeds_token_in_account = self.cv_ata(CvKind::OfferProceeds, &self.mint_onyc);
        let redemption_fee_vault = self.cv_pda(CvKind::RedemptionFee);
        let redemption_fee_token_in_account = self.cv_ata(CvKind::RedemptionFee, &self.mint_onyc);
        let mint_authority = self.mint_authority_pda;
        let redeemer = self.oldest_request_redeemer();
        let __scout_signer_worker = self.worker_kp().insecure_clone();
        let worker = __scout_signer_worker.pubkey();
        let buffer_state = self.buffer_state_pda();
        let reserve_vault_onyc_account = self.reserve_vault_onyc();
        let management_fee_vault_onyc_account = self.cv_ata(CvKind::ManagementFee, &self.mint_onyc);
        let performance_fee_vault_onyc_account = self.cv_ata(CvKind::PerformanceFee, &self.mint_onyc);
        let associated_token_program = ATA_PROGRAM_ID;
        let system_program = system_program::ID;
        let offer_vault_authority = self.offer_vault_authority;
        let offer_vault_onyc_account = scout_ata(&self.offer_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID);
        let market_stats = self.market_stats_pda();
        let circulating_supply_excluded_balance = self.excluded_balance_pda();
        let main_offer = self.state_main_offer();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::FulfillRedemptionRequest { amount })
            .accounts(accounts::FulfillRedemptionRequest {
                state: state,
                offer: offer,
                redemption_offer: redemption_offer,
                redemption_request: redemption_request,
                redemption_vault_authority: redemption_vault_authority,
                vault_token_in_account: vault_token_in_account,
                vault_token_out_account: vault_token_out_account,
                token_in_mint: token_in_mint,
                token_in_program: token_in_program,
                token_out_mint: token_out_mint,
                token_out_program: token_out_program,
                user_token_out_account: user_token_out_account,
                offer_proceeds_vault: offer_proceeds_vault,
                offer_proceeds_token_in_account: offer_proceeds_token_in_account,
                redemption_fee_vault: redemption_fee_vault,
                redemption_fee_token_in_account: redemption_fee_token_in_account,
                mint_authority: mint_authority,
                redeemer: redeemer,
                worker: worker,
                buffer_state: buffer_state,
                reserve_vault_onyc_account: reserve_vault_onyc_account,
                management_fee_vault_onyc_account: management_fee_vault_onyc_account,
                performance_fee_vault_onyc_account: performance_fee_vault_onyc_account,
                offer_vault_authority: offer_vault_authority,
                offer_vault_onyc_account: offer_vault_onyc_account,
                market_stats: market_stats,
                circulating_supply_excluded_balance: circulating_supply_excluded_balance,
                main_offer: main_offer,
            })
            .signers(&[&*self.payer, &__scout_signer_worker])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:fulfill_redemption_request:BEGIN
            // No shadow ledger to maintain — every property below walks the request
            // accounts themselves, so "the account still exists" IS "the request is open".
            // SCOUT:ACTION-HOOK:fulfill_redemption_request:END
        }
        __scout_success
    }

    pub fn action_cancel_redemption_request(&mut self) -> bool {
        let state = self.state_pda;
        let redemption_offer = self.redemption_offer_pda;
        let redemption_request = self.oldest_request_pda();
        let __scout_signer_signer = self.redemption_admin.insecure_clone();
        let signer = __scout_signer_signer.pubkey();
        let redeemer = self.oldest_request_redeemer();
        let worker = self.state_worker();
        let redemption_vault_authority = self.redemption_vault_authority;
        let token_in_mint = self.mint_onyc;
        let vault_token_account = scout_ata(&self.redemption_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID);
        let redeemer_token_account = scout_ata(&self.oldest_request_redeemer(), &self.mint_onyc, &SPL_TOKEN_ID);
        let token_program = SPL_TOKEN_ID;
        let system_program = system_program::ID;
        let associated_token_program = ATA_PROGRAM_ID;
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::CancelRedemptionRequest {  })
            .accounts(accounts::CancelRedemptionRequest {
                state: state,
                redemption_offer: redemption_offer,
                redemption_request: redemption_request,
                signer: signer,
                redeemer: redeemer,
                worker: worker,
                redemption_vault_authority: redemption_vault_authority,
                token_in_mint: token_in_mint,
                vault_token_account: vault_token_account,
                redeemer_token_account: redeemer_token_account,
                token_program: token_program,
            })
            .signers(&[&*self.payer, &__scout_signer_signer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:cancel_redemption_request:BEGIN
            // No shadow ledger to maintain — every property below walks the request
            // accounts themselves, so "the account still exists" IS "the request is open".
            // SCOUT:ACTION-HOOK:cancel_redemption_request:END
        }
        __scout_success
    }

    pub fn action_update_redemption_offer_fee(&mut self, new_fee_basis_points: u16) -> bool {
        let redemption_offer = self.redemption_offer_pda;
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::UpdateRedemptionOfferFee { new_fee_basis_points })
            .accounts(accounts::UpdateRedemptionOfferFee {
                redemption_offer: redemption_offer,
                state: state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:update_redemption_offer_fee:BEGIN
            // update shadow-ledger state after successful update_redemption_offer_fee
            // SCOUT:ACTION-HOOK:update_redemption_offer_fee:END
        }
        __scout_success
    }

    pub fn action_update_redemption_offer_vault_target(&mut self, new_vault_target_bps: u16) -> bool {
        let redemption_offer = self.redemption_offer_pda;
        let state = self.state_pda;
        let boss = self.boss.pubkey();
        let __scout_success = self.ctx
            .program(self.program_id)
            .call(instruction::UpdateRedemptionOfferVaultTarget { new_vault_target_bps })
            .accounts(accounts::UpdateRedemptionOfferVaultTarget {
                redemption_offer: redemption_offer,
                state: state,
                boss: boss,
            })
            .signers(&[&*self.payer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if __scout_success {
            // SCOUT:ACTION-HOOK:update_redemption_offer_vault_target:BEGIN
            // update shadow-ledger state after successful update_redemption_offer_vault_target
            // SCOUT:ACTION-HOOK:update_redemption_offer_vault_target:END
        }
        __scout_success
    }

    // SCOUT:EXTRA-ACTIONS:BEGIN
    /// Move the clock forward. NOT an IDL instruction — no real program exposes "advance time",
    /// but essentially every guard in this one reads `Clock::get()`.
    ///
    /// Without this the harness quotes a single price forever: `calculate_step_price_at` snaps to
    /// the end of the current `price_fix_duration` window, `find_active_vector_at` selects by
    /// `start_time <= now`, and `add_offer_vector` refuses any `start_time` at or below the latest
    /// existing one. A frozen clock therefore deletes the whole "value moved between two user
    /// actions" bug class AND makes a second vector unaddable, which is exactly where the
    /// interesting pricing behaviour lives.
    pub fn action_scout_advance_time(&mut self, seconds: u32) -> bool {
        // Bounded so the fuzzer explores real durations (sub-window, multi-window, multi-year)
        // instead of spending its budget on timestamps that overflow the price math immediately.
        let delta = 1i64 + (seconds as i64 % 40_000_000);
        let now = scout_now(&self.ctx) as i64;
        scout_set_time(&mut self.ctx, now.saturating_add(delta));
        self.ctx.advance_slots(1);
        true
    }

    /// Price the self-referential onyc->onyc offer, if one exists.
    ///
    /// Deliberately does NOT create it. The only creator is the generated `make_offer` action,
    /// whose (token_in, token_out) pair the fuzzer selects via `make_offer_pair` — so the state
    /// P-0005 forbids is reached through the real IDL instruction, and a minimized counterexample
    /// names `make_offer` rather than a bespoke harness action.
    ///
    /// Priced below 1.0, which is an ordinary `base_price` for a pair where one unit of the input
    /// buys more than one of the output; it only becomes a money printer because both legs are the
    /// same token.
    pub fn action_scout_price_same_mint_offer(&mut self, price: u64) -> bool {
        let m = self.mint_onyc;
        let offer = scout_pda(&[SEED_OFFER, m.as_ref(), m.as_ref()], &self.program_id);
        let boss = self.boss.insecure_clone();
        let state_pda = self.state_pda;
        let now = scout_now(&self.ctx);
        // `.max(1)` rather than `1 + ..` so a caller asking for exactly 0.5 gets 500_000_000.
        let base_price = (price % 2_000_000_000).max(1);
        self.ctx
            .program(self.program_id)
            .call(instruction::AddOfferVector {
                start_time: None,
                base_time: now,
                base_price,
                apr: 0,
                price_fix_duration: 3_600,
            })
            .accounts(accounts::AddOfferVector {
                offer,
                token_in_mint: m,
                token_out_mint: m,
                state: state_pda,
                boss: boss.pubkey(),
            })
            .signers(&[&boss])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false)
    }

    /// Take the same-mint offer. Separate from the generated action, whose bindings pin it to the
    /// usdc -> onyc pair.
    pub fn action_scout_take_same_mint_offer(&mut self, amount: u64, sel: u8) -> bool {
        let m = self.mint_onyc;
        let offer = scout_pda(&[SEED_OFFER, m.as_ref(), m.as_ref()], &self.program_id);
        let user_kp = self.pick_user(sel as u64);
        let user = user_kp.pubkey();
        let amount = if sel & 4 != 0 { amount } else { amount % (USER_ONYC_START / 4) + 1 };
        let va = self.offer_vault_authority;
        let (state_pda, boss_pk, ma) = (self.state_pda, self.boss.pubkey(), self.mint_authority_pda);
        let vault_ata = scout_ata(&va, &m, &SPL_TOKEN_ID);
        let uata = scout_ata(&user, &m, &SPL_TOKEN_ID);
        let boss_ata = scout_ata(&boss_pk, &m, &SPL_TOKEN_ID);
        self.ctx
            .program(self.program_id)
            .call(instruction::TakeOffer { token_in_amount: amount, approval_message: None })
            .accounts(accounts::TakeOffer {
                offer,
                state: state_pda,
                boss: boss_pk,
                vault_authority: va,
                vault_token_in_account: vault_ata,
                vault_token_out_account: vault_ata,
                token_in_mint: m,
                token_in_program: SPL_TOKEN_ID,
                token_out_mint: m,
                token_out_program: SPL_TOKEN_ID,
                user_token_in_account: uata,
                user_token_out_account: uata,
                boss_token_in_account: boss_ata,
                mint_authority: ma,
                instructions_sysvar: INSTRUCTIONS_SYSVAR_ID,
                user,
            })
            .signers(&[&user_kp])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false)
    }

    /// Take the REVERSE offer (ONyc in, usdc out) through the LEGACY `take_offer`.
    ///
    /// Two things only this action reaches.
    ///
    /// 1. The BURN side of `execute_token_operations`. Every other take in the harness has ONyc as
    ///    token_out, so the program MINTS; here ONyc is token_in and the program controls that
    ///    mint, so the net input is burned instead.
    /// 2. The documented BUFFER baseline exemption. `docs/BUFFER_ACCRUAL.md` states that legacy
    ///    `take_offer`/`take_offer_permissionless` do NOT update the BUFFER baseline, and that
    ///    "offer executions that only burn ONyc as token in are not wired into the BUFFER baseline
    ///    path". A burn that leaves `BufferState.previous_supply` above the real supply is the one
    ///    direction of that exemption that creates value out of nothing on the next accrual, and
    ///    without this action the harness cannot produce that state at all.
    ///
    /// The vault must hold usdc to pay out, which `offer_vault_deposit` and ordinary takes supply.
    pub fn action_scout_take_offer_rev(&mut self, amount: u64, sel: u8) -> bool {
        let user_kp = self.pick_user(sel as u64);
        let user = user_kp.pubkey();
        // Bounded so the trade is usually affordable; bit 2 leaves a quarter of draws raw so the
        // insufficient-funds and overflow branches stay reachable.
        let amount = if sel & 4 != 0 { amount } else { amount % (USER_ONYC_START / 8) + 1 };
        let ova = self.offer_vault_authority;
        let (state_pda, boss_pk, ma) = (self.state_pda, self.boss.pubkey(), self.mint_authority_pda);
        let (mint_onyc, mint_usdc) = (self.mint_onyc, self.mint_usdc);
        let offer = self.offer_rev_pda;
        self.ctx
            .program(self.program_id)
            .call(instruction::TakeOffer { token_in_amount: amount, approval_message: None })
            .accounts(accounts::TakeOffer {
                offer,
                state: state_pda,
                boss: boss_pk,
                vault_authority: ova,
                vault_token_in_account: scout_ata(&ova, &mint_onyc, &SPL_TOKEN_ID),
                vault_token_out_account: scout_ata(&ova, &mint_usdc, &SPL_TOKEN_ID),
                token_in_mint: mint_onyc,
                token_in_program: SPL_TOKEN_ID,
                token_out_mint: mint_usdc,
                token_out_program: SPL_TOKEN_ID,
                user_token_in_account: scout_ata(&user, &mint_onyc, &SPL_TOKEN_ID),
                user_token_out_account: scout_ata(&user, &mint_usdc, &SPL_TOKEN_ID),
                boss_token_in_account: scout_ata(&boss_pk, &mint_onyc, &SPL_TOKEN_ID),
                mint_authority: ma,
                instructions_sysvar: INSTRUCTIONS_SYSVAR_ID,
                user,
            })
            .signers(&[&user_kp])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false)
    }

    /// Configure `state.max_supply` — the ONyc minting cap.
    ///
    /// `configure_max_supply` is admin-gated out of the generated pool (it reconfigures the world),
    /// so without this the cap is 0 forever and every `max_supply > 0` branch of `mint_tokens`
    /// (token_utils.rs:234-245) is dead code.
    ///
    /// The cap is ALWAYS set at or above the current supply. `configure_max_supply` itself performs
    /// no such check, so a lower cap is legal on-chain — but a harness that set one would make
    /// `supply > max_supply` true immediately and by construction, and P-0004 would be reporting the
    /// harness's own choice rather than a mint that overshot. Fixing the floor here means any later
    /// excess was necessarily produced by minting, which is exactly the question the property asks.
    pub fn action_scout_configure_max_supply(&mut self, headroom: u64) -> bool {
        let supply = match self.onyc_supply() {
            Some(v) => v,
            None => return false,
        };
        // Headroom spans zero (cap exactly at supply: any mint at all overshoots) up to a slack
        // large enough that ordinary activity stays under it.
        let max_supply = supply.saturating_add(headroom % 1_000_000_000_000);
        let boss = self.boss.insecure_clone();
        self.ctx
            .program(self.program_id)
            .call(instruction::ConfigureMaxSupply { max_supply })
            .accounts(accounts::ConfigureMaxSupply { state: self.state_pda, boss: boss.pubkey() })
            .signers(&[&boss])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false)
    }

    /// Open a redemption request against the `onyc -> play` redemption offer that
    /// `action_make_redemption_offer` creates.
    ///
    /// Its token_in is ONyc, exactly like the offer `setup()` built — and because
    /// `seeds::REDEMPTION_OFFER_VAULT_AUTHORITY` carries no per-offer discriminator, BOTH offers'
    /// collateral lands in the same token account. Without this action only one offer ever funds
    /// that vault and P-0007 degenerates into P-0002; with it, the pooled-solvency question is
    /// actually posed.
    pub fn action_scout_create_request_play(&mut self, amount: u64, sel: u8) -> bool {
        let ro = scout_pda(
            &[SEED_REDEMPTION_OFFER, self.mint_onyc.as_ref(), self.mint_play.as_ref()],
            &self.program_id,
        );
        let id = match self.request_counter_of(&ro) {
            Some(v) => v,
            None => return false, // the offer does not exist yet
        };
        let user_kp = self.pick_user(sel as u64);
        let user = user_kp.pubkey();
        let amount = if sel & 4 != 0 { amount } else { amount % (USER_ONYC_START / 8) + 1 };
        let request = self.request_pda_of(&ro, id);
        let rva = self.redemption_vault_authority;
        let (state_pda, mint_onyc) = (self.state_pda, self.mint_onyc);
        let redeemer_ata = scout_ata(&user, &mint_onyc, &SPL_TOKEN_ID);
        let vault_ata = scout_ata(&rva, &mint_onyc, &SPL_TOKEN_ID);
        let ok = self
            .ctx
            .program(self.program_id)
            .call(instruction::CreateRedemptionRequest { amount })
            .accounts(accounts::CreateRedemptionRequest {
                state: state_pda,
                redemption_offer: ro,
                // programV5 cross-checks `redemption_offer.offer == offer.key()`. The onyc -> play
                // redemption offer's underlying offer has the mints SWAPPED.
                offer: scout_pda(
                    &[SEED_OFFER, self.mint_play.as_ref(), self.mint_onyc.as_ref()],
                    &self.program_id,
                ),
                redemption_request: request,
                redeemer: user,
                redemption_vault_authority: rva,
                token_in_mint: mint_onyc,
                redeemer_token_account: redeemer_ata,
                vault_token_account: vault_ata,
                token_program: SPL_TOKEN_ID,
            })
            .signers(&[&user_kp])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if ok {
            // kept fed even with P-0007 retired: c5_pool_stays_solvent_without_the_drain reads it
            self.scout_p7_reqs[self.scout_p7_next % SCOUT_P7_CAP] = request;
            self.scout_p7_next = self.scout_p7_next.saturating_add(1);
        }
        ok
    }

    // The reverse (usdc-in) and transfer-fee (Token-2022-in) redemption request/cancel actions
    // were removed with programV5: `make_redemption_offer.rs:66-72` pins every redemption offer's
    // token_in to `state.onyc_mint` under the plain SPL token program, so neither offer can exist
    // and neither action could ever succeed. Their subjects are pinned instead by
    // `t_p0008_fee_bearing_redemption_offer_is_unconstructible` and
    // `t_v5_redemption_token_in_must_be_onyc`.

    /// Take the `usdc -> fee` offer. This is the POSITIVE CONTROL for P-0008: the program refuses
    /// it at `token_utils.rs:378` because `token_out` is fee-bearing. A run in which this returns
    /// `true` means the fixture's mint stopped being fee-bearing, not that the guard was removed.
    pub fn action_scout_take_fee_offer(&mut self, amount: u64) -> bool {
        let user_kp = self.user_a.insecure_clone();
        let user = user_kp.pubkey();
        let ova = self.offer_vault_authority;
        let (state_pda, offer, mint_usdc, mint_fee, boss) = (
            self.state_pda,
            self.offer_fee_pda,
            self.mint_usdc,
            self.mint_fee,
            self.boss.pubkey(),
        );
        self.ctx
            .program(self.program_id)
            .call(instruction::TakeOffer { token_in_amount: amount, approval_message: None })
            .accounts(accounts::TakeOffer {
                state: state_pda,
                offer,
                vault_authority: ova,
                mint_authority: self.mint_authority_pda,
                token_in_mint: mint_usdc,
                token_in_program: SPL_TOKEN_ID,
                vault_token_in_account: scout_ata(&ova, &mint_usdc, &SPL_TOKEN_ID),
                user_token_in_account: scout_ata(&user, &mint_usdc, &SPL_TOKEN_ID),
                boss_token_in_account: scout_ata(&boss, &mint_usdc, &SPL_TOKEN_ID),
                token_out_mint: mint_fee,
                token_out_program: SPL_TOKEN_2022_ID,
                vault_token_out_account: scout_ata(&ova, &mint_fee, &SPL_TOKEN_2022_ID),
                user_token_out_account: scout_ata(&user, &mint_fee, &SPL_TOKEN_2022_ID),
                instructions_sysvar: INSTRUCTIONS_SYSVAR_ID,
                user,
                boss,
            })
            .signers(&[&user_kp])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false)
    }

    /// Raw SPL/Token-2022 balance at `addr` (offset 64 is `amount` in both layouts; the Token-2022
    /// extensions live after the 165-byte base, so the base offsets are shared).
    pub fn tok_amt(&self, addr: &Pubkey) -> u64 {
        self.ctx
            .account_data(addr)
            .ok()
            .filter(|d| d.len() >= SCOUT_TOKEN_MIN_LEN)
            .and_then(|d| {
                d[SCOUT_TOKEN_AMOUNT_OFFSET..SCOUT_TOKEN_MIN_LEN]
                    .try_into()
                    .ok()
                    .map(u64::from_le_bytes)
            })
            .unwrap_or(0)
    }

    // `action_scout_fulfill_rev` was removed with programV5. Its whole point was the redemption
    // payout leg being ONyc so `fulfill_redemption_request` reached `mint_tokens` with the
    // hard-coded `token_out_max_supply: 0` (old P-0004). V5 pins every redemption offer's token_in
    // to `state.onyc_mint` (`make_redemption_offer.rs:66-70`), so the payout leg can never be ONyc
    // and the reverse redemption offer is not constructible at all.

    /// Re-create `State` after `close_state` deallocated it.
    ///
    /// `close_state` is destructive by design and is NOT admin-gated by the generator, so it sits
    /// in the ordinary action pool. Without a way back, every action after it in the chain fails
    /// for want of a world rather than for any reason of its own. Exposing recovery as an action
    /// lets the fuzzer climb out on its own; doing it in close_state's hook is not possible, as a
    /// hook region may contain only pure assignments.
    ///
    /// A no-op (and reports failure) when `State` already exists, so it cannot silently reset a
    /// live world mid-chain.
    pub fn action_scout_rebuild_state(&mut self) -> bool {
        if self.state_exists() {
            return false;
        }
        self.rebuild_state();
        true
    }

    /// Drive `add_approver` / `remove_approver` across the whole key space, not just the one
    /// approver setup installed.
    ///
    /// The IDL arg is a bare `Pubkey`, so the generator cannot invent values for it and the static
    /// binding pins it to a single key — which leaves four branches structurally unreachable:
    /// `InvalidApprover` (both instructions reject `Pubkey::default()`), the approver2 slot, and
    /// `BothApproversFilled`. Selecting the key from a fuzzer byte reaches all of them.
    pub fn action_scout_approver_ops(&mut self, sel: u8, remove: bool) -> bool {
        let approver = match sel % 4 {
            0 => Pubkey::default(),      // -> InvalidApprover on both instructions
            1 => self.approver.pubkey(), // the one setup installed
            2 => self.user_a.pubkey(),   // fills approver2, then -> BothApproversFilled
            _ => self.user_b.pubkey(),
        };
        let boss = self.boss.insecure_clone();
        let outcome = if remove {
            self.ctx
                .program(self.program_id)
                .call(instruction::RemoveApprover { approver })
                .accounts(accounts::RemoveApprover { state: self.state_pda, boss: boss.pubkey() })
                .signers(&[&boss])
                .send()
        } else {
            self.ctx
                .program(self.program_id)
                .call(instruction::AddApprover { approver })
                .accounts(accounts::AddApprover { state: self.state_pda, boss: boss.pubkey() })
                .signers(&[&boss])
                .send()
        };
        outcome.map(|o| o.is_success()).unwrap_or(false)
    }

    /// Name a configurable vault's withdrawal destination.
    ///
    /// `withdraw_configurable_vault` refuses every vault whose `withdrawal_destination` is still
    /// the default key (`configurable_vault/withdraw.rs:75-78`), and nothing in the ordinary action
    /// pool sets one: `set_configurable_vault_destination` is admin-gated out of it. Without this
    /// the withdraw instruction can only ever reach its own rejection branch, and the routing of
    /// every fee and proceeds vault — nine of them — is untested.
    ///
    /// Deliberately NOT done in `setup()`. `set_configurable_vault_destination` is
    /// `init_if_needed` on the vault PDA, so pre-creating all nine would take the create branch of
    /// `get_or_create_configurable_vault_token_account_pair`
    /// (`configurable_vault/accounts.rs:33`) permanently out of reach — the branch every v2 take
    /// and every swap runs on ITS first use. As an action the fuzzer chooses the order, so both the
    /// create-on-first-use path and the already-exists path stay live.
    ///
    /// The destination alternates between the boss and `user_a` off the selector: a vault that can
    /// only ever pay one fixed key cannot express "paid the wrong destination".
    pub fn action_scout_set_vault_destination(&mut self, sel: u8) -> bool {
        let kind = self.pick_vault_kind(sel as u64);
        let destination = if sel & 16 == 0 {
            self.boss.pubkey()
        } else {
            self.user_a.pubkey()
        };
        let boss = self.boss.insecure_clone();
        let vault = self.cv_pda(kind);
        let state_pda = self.state_pda;
        self.ctx
            .program(self.program_id)
            .call(instruction::SetConfigurableVaultDestination { kind, withdrawal_destination: destination })
            .accounts(accounts::SetConfigurableVaultDestination {
                state: state_pda,
                boss: boss.pubkey(),
                configurable_vault: vault,
            })
            .signers(&[&boss])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false)
    }

    /// Call `update_circulating_supply_excluded_balance` with a DELIBERATELY wrong remaining-account
    /// list.
    ///
    /// The instruction cross-checks its remaining accounts three ways
    /// (`circulating_supply/excluded_balance.rs:86-116`): the count must equal the number of active
    /// owners, each account's mint must be ONyc, and each account's owner must be the owner at the
    /// same index. The generated action always passes a correct list — as it must, since that is
    /// the only way the instruction ever succeeds — so all three rejection branches are dead. This
    /// action walks the three malformations.
    ///
    /// It is expected to FAIL every time; that is the point. The lines it covers are the guards.
    pub fn action_scout_excluded_balance_malformed(&mut self, sel: u8) -> bool {
        let mut atas = self.excluded_owner_atas();
        if atas.is_empty() {
            return false;
        }
        match sel % 3 {
            // too few accounts -> the count check
            0 => {
                atas.pop();
            }
            // right count, wrong MINT: swap in a usdc account of the same owner
            1 => {
                atas[0] = scout_ata(&self.reserve_vault_authority(), &self.mint_usdc, &SPL_TOKEN_ID);
            }
            // right count and mint, wrong OWNER for that index
            _ => {
                atas[0] = scout_ata(&self.user_a.pubkey(), &self.mint_onyc, &SPL_TOKEN_ID);
            }
        }
        let signer = self.payer.insecure_clone();
        let (state_pda, mint_onyc) = (self.state_pda, self.mint_onyc);
        let excluded_accounts = self.excluded_accounts_pda();
        let balance = self.excluded_balance_pda();
        self.ctx
            .program(self.program_id)
            .call(instruction::UpdateCirculatingSupplyExcludedBalance {})
            .accounts(accounts::UpdateCirculatingSupplyExcludedBalance {
                state: state_pda,
                onyc_mint: mint_onyc,
                excluded_accounts,
                circulating_supply_excluded_balance: balance,
                token_program: SPL_TOKEN_ID,
                signer: signer.pubkey(),
            })
            .remaining_accounts(atas)
            .signers(&[&signer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false)
    }

    /// Disable an offer as someone who is neither boss nor admin.
    ///
    /// `authorize_targeted_disable` (`targeted_disable.rs:12-19`) is asymmetric: DISABLING accepts
    /// the boss or any admin, ENABLING accepts only the boss. The generated actions always sign as
    /// boss, so the `UnauthorizedToDisableOffer` branch — the one that decides who may halt a
    /// market — is never executed. This signs as an ordinary user and expects refusal.
    ///
    /// `sel` picks which of the two targeted-disable instructions to attack.
    pub fn action_scout_disable_as_stranger(&mut self, sel: u8) -> bool {
        let user = self.pick_user(sel as u64);
        let (state_pda, offer, redemption_offer) =
            (self.state_pda, self.offer_pda, self.redemption_offer_pda);
        let outcome = if sel & 2 == 0 {
            self.ctx
                .program(self.program_id)
                .call(instruction::SetOfferDisabled { disabled: true })
                .accounts(accounts::SetOfferDisabled {
                    state: state_pda,
                    offer,
                    signer: user.pubkey(),
                })
                .signers(&[&user])
                .send()
        } else {
            self.ctx
                .program(self.program_id)
                .call(instruction::SetRedemptionOfferDisabled { disabled: true })
                .accounts(accounts::SetRedemptionOfferDisabled {
                    state: state_pda,
                    redemption_offer,
                    signer: user.pubkey(),
                })
                .signers(&[&user])
                .send()
        };
        outcome.map(|o| o.is_success()).unwrap_or(false)
    }

    /// Call `set_kill_switch` as someone other than the boss.
    ///
    /// The generated action always signs as `boss`, so `boss_signed` is always true and the
    /// `require!(boss_signed || admin_signed, UnauthorizedToEnable)` branch can never fail. An
    /// admin can ENABLE but only the boss can DISABLE, which is the asymmetry worth exercising —
    /// and it needs a signer who is an admin, and one who is neither.
    pub fn action_scout_kill_switch_as(&mut self, sel: u8, enable: bool) -> bool {
        let signer = match sel % 3 {
            0 => self.boss.insecure_clone(),
            1 => self.user_a.insecure_clone(),           // admin iff add_admin ran first
            _ => self.redemption_admin.insecure_clone(), // never an admin
        };
        self.ctx
            .program(self.program_id)
            .call(instruction::SetKillSwitch { enable })
            .accounts(accounts::SetKillSwitch { state: self.state_pda, signer: signer.pubkey() })
            .signers(&[&signer])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false)
    }

    /// `propose_boss` with a fuzzer-chosen nominee, including `Pubkey::default()`.
    ///
    /// The static binding nominates the incumbent (deliberately — see SCOUT:BINDINGS), which never
    /// reaches `InvalidBossAddress`. This does, without ever handing authority to a key the harness
    /// cannot sign with.
    pub fn action_scout_propose_boss(&mut self, sel: u8) -> bool {
        let new_boss = if sel % 2 == 0 { Pubkey::default() } else { self.boss.pubkey() };
        let boss = self.boss.insecure_clone();
        self.ctx
            .program(self.program_id)
            .call(instruction::ProposeBoss { new_boss })
            .accounts(accounts::ProposeBoss { state: self.state_pda, boss: boss.pubkey() })
            .signers(&[&boss])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false)
    }

    /// Take the approval-gated offer (`mint_appr -> onyc`, `needs_approval = true`).
    ///
    /// This has to be a COMPOUND action. `verify_approval_message_generic` loads the instruction
    /// at `current_index - 1` off the Instructions sysvar and demands it be an Ed25519 program
    /// instruction carrying the same message, so a single-instruction `.send()` can never satisfy
    /// it — the generated `action_take_offer` reaches `ApprovalRequired` and stops. Here the
    /// Ed25519 instruction and the take are pushed into ONE transaction.
    ///
    /// `expiry_delta` and `sel` are fuzzer-controlled so both sides of every approval guard are
    /// reachable: an expired message, a message bound to the wrong user, and the valid case.
    pub fn action_take_offer_with_approval(
        &mut self,
        token_in_amount: u64,
        expiry_delta: i32,
        sel: u8,
    ) -> bool {
        let user_kp = self.pick_user(sel as u64);
        let user = user_kp.pubkey();
        let now = scout_now(&self.ctx);

        // A raw u64 from the fuzzer exceeds the user's balance in ~every draw, so the transaction
        // would die at InsufficientFunds before `verify_offer_approval` is ever consulted — this
        // action would be dispatched forever and never once exercise the branch it exists for.
        // Bit 2 of `sel` keeps a quarter of the draws unclamped so the overflow / insufficient-funds
        // paths stay reachable too.
        let token_in_amount = if sel & 4 != 0 {
            token_in_amount
        } else {
            token_in_amount % (USER_USDC_START / 4) + 1
        };

        // The message the program will be handed. `expiry_delta` reaches back before `now`, which
        // is the only way to cover the `Expired` branch.
        let expiry_unix = (now as i64).saturating_add(expiry_delta as i64).max(0) as u64;
        // Half the draws bind the approval to the OTHER user, covering `WrongUser`.
        let bound_user = if sel & 2 == 0 { user } else { self.pick_user_pk((sel as u64) + 1) };

        let msg = onreapp::types::ApprovalMessage {
            program_id: self.program_id,
            user_pubkey: bound_user,
            expiry_unix,
        };
        let mut message_bytes = Vec::with_capacity(72);
        message_bytes.extend_from_slice(msg.program_id.as_ref());
        message_bytes.extend_from_slice(msg.user_pubkey.as_ref());
        message_bytes.extend_from_slice(&msg.expiry_unix.to_le_bytes());

        // A real Ed25519 signature by the registered approver. litesvm runs with sigverify off, so
        // the precompile itself is not re-verified here — signing for real keeps the harness
        // faithful in the direction that matters and means no property may claim anything about
        // forged signatures (see PROPERTIES.md / NOTES.md).
        let signature = self.approver.sign_message(&message_bytes);
        let ed25519_ix = scout_ed25519_instruction(&self.approver.pubkey(), &signature.into(), &message_bytes);

        self.ctx.pending_instructions.push(ed25519_ix);
        let queued = self
            .ctx
            .program(self.program_id)
            .call(instruction::TakeOffer {
                token_in_amount,
                approval_message: Some(msg),
            })
            .accounts(accounts::TakeOffer {
                offer: self.offer_appr_pda,
                state: self.state_pda,
                boss: self.boss.pubkey(),
                vault_authority: self.offer_vault_authority,
                vault_token_in_account: scout_ata(&self.offer_vault_authority, &self.mint_appr, &SPL_TOKEN_ID),
                vault_token_out_account: scout_ata(&self.offer_vault_authority, &self.mint_onyc, &SPL_TOKEN_ID),
                token_in_mint: self.mint_appr,
                token_in_program: SPL_TOKEN_ID,
                token_out_mint: self.mint_onyc,
                token_out_program: SPL_TOKEN_ID,
                user_token_in_account: scout_ata(&user, &self.mint_appr, &SPL_TOKEN_ID),
                user_token_out_account: scout_ata(&user, &self.mint_onyc, &SPL_TOKEN_ID),
                boss_token_in_account: scout_ata(&self.boss.pubkey(), &self.mint_appr, &SPL_TOKEN_ID),
                mint_authority: self.mint_authority_pda,
                instructions_sysvar: INSTRUCTIONS_SYSVAR_ID,
                user,
            })
            .signers(&[&user_kp])
            .add_transaction();
        if queued.is_err() {
            self.ctx.pending_instructions.clear();
            return false;
        }
        self.ctx
            .send_batch()
            .ok()
            .flatten()
            .map(|o| o.is_success())
            .unwrap_or(false)
    }
    // SCOUT:EXTRA-ACTIONS:END
}

#[invariant_test]
fn invariant_test(_f: &mut OnreappFixture) {
    scout_check_session!();
    // SCOUT:INVARIANTS:BEGIN
    // P-0002 is CONFIRMED (report-p0002.md) and its block has been retired from the live
    // harness: a confirmed property keeps firing on the same known defect and buries every other
    // property's first finding under duplicates. The regression is preserved as a deterministic
    // test instead — scout_reachability::t_p0002_vault_drain_strands_open_requests.

    // SCOUT:INVARIANT:P-0003:BEGIN
    // Aggregate equals sum of parts: `RedemptionOffer.requested_redemptions` against what the
    // requests that actually exist still have OUTSTANDING (`amount - fulfilled_amount` —
    // programV5 fulfils partially). Spans every path that can retire a request, not only the two that do so
    // today — a path that closed one without decrementing, or decremented by an amount other than
    // the one it locked, shows up here and nowhere else.
    //
    // What is shadow and what is not: the AMOUNTS are read live from the request accounts, but
    // the MEMBERSHIP of the summed set is shadow state (`scout_p3_reqs` / `scout_p3_next`), fed by
    // a success-gated hook on `create_redemption_request`. Membership can therefore drift, and the
    // guards below exist so that every way it can drift LOSES an observation rather than
    // manufacturing one:
    //   * Wrap: the hook writes at `scout_p3_next % SCOUT_REQ_CAP`, so the first overwrite happens
    //     only once the counter exceeds CAP; `scout_p3_next > SCOUT_REQ_CAP` disables the check on
    //     every state in which a slot could have been clobbered.
    //   * Duplicates: earlier slots are rescanned read-only, so one address cannot be summed twice
    //     against an aggregate that counts it once.
    //   * Retirement is not shadowed at all — both `cancel` and `fulfil` close the request account,
    //     so account existence IS openness and a stale registry entry simply reads as absent.
    //
    // The comparison is done in u128 on BOTH sides. Summing u64 amounts into a u64 accumulator and
    // asserting the aggregate's high half is zero would be strictly stronger than the property:
    // SCOUT_REQ_CAP u64 addends can legitimately exceed u64::MAX, and that state would be reported
    // as a violation rather than compared.
    fn invariant_p_0003(f: &mut OnreappFixture) {
        if f.scout_p3_next > SCOUT_REQ_CAP {
            return;
        }
        let offer_data = match f.ctx.account_data(&f.redemption_offer_pda) {
            Ok(d) => d,
            Err(_) => return,
        };
        if offer_data.len() < SCOUT_RO_MIN_LEN {
            return;
        }
        let rbuf: [u8; 16] = match offer_data[SCOUT_RO_REQUESTED_OFFSET..SCOUT_RO_MIN_LEN].try_into()
        {
            Ok(b) => b,
            Err(_) => return,
        };
        let recorded = u128::from_le_bytes(rbuf);

        let mut summed: u128 = 0;
        let reqs = f.scout_p3_reqs;
        for idx in 0..SCOUT_REQ_CAP {
            let pda = reqs[idx];
            if pda == Pubkey::default() {
                continue;
            }
            // De-duplicate against earlier slots, read-only: summing one address twice against an
            // aggregate that counts it once would manufacture a violation.
            let mut dup = false;
            for j in 0..idx {
                if reqs[j] == pda {
                    dup = true;
                }
            }
            if dup {
                continue;
            }
            let data = match f.ctx.account_data(&pda) {
                Ok(d) => d,
                Err(_) => continue,
            };
            if data.len() < SCOUT_REQ_FULFILLED_END {
                continue;
            }
            // A CLOSED request is not an open claim. Anchor's close path stamps this sentinel over
            // the discriminator, and the data can outlive the close, so length alone does not mean
            // live — summing a retired request would over-count against the aggregate.
            if data[0..8] == SCOUT_CLOSED_ACCOUNT_DISCRIMINATOR {
                continue;
            }
            // Scope from ON-CHAIN data: only requests the program itself records as belonging to
            // THIS offer are part of its aggregate. A mis-registered address then under-counts.
            let offer_bytes: [u8; 32] =
                match data[SCOUT_REQ_OFFER_OFFSET..SCOUT_REQ_OFFER_END].try_into() {
                    Ok(b) => b,
                    Err(_) => continue,
                };
            if Pubkey::new_from_array(offer_bytes) != f.redemption_offer_pda {
                continue;
            }
            let buf: [u8; 8] = match data[SCOUT_REQ_AMOUNT_OFFSET..SCOUT_REQ_MIN_LEN].try_into() {
                Ok(b) => b,
                Err(_) => continue,
            };
            // programV5: a request is retired in BITES. `requested_redemptions` is decremented by
            // each `amount` fulfilled (fulfill_redemption_request.rs:546) while the request lives
            // on carrying its ORIGINAL total and a growing `fulfilled_amount`, so what the offer's
            // counter must equal is the sum of the REMAINDERS. Summing the gross totals would
            // report every partially-fulfilled request as drift.
            let fulfilled: [u8; 8] =
                match data[SCOUT_REQ_FULFILLED_OFFSET..SCOUT_REQ_FULFILLED_END].try_into() {
                    Ok(b) => b,
                    Err(_) => continue,
                };
            let remaining =
                u64::from_le_bytes(buf).saturating_sub(u64::from_le_bytes(fulfilled));
            summed = summed.saturating_add(remaining as u128);
        }
        scout_check!(
            "P-0003",
            "requested-redemptions-equals-sum-of-open-requests",
            recorded == summed,
            "P-0003: redemption offer {} records requested_redemptions={} but the requests that \
             still exist and name it sum to {}.",
            f.redemption_offer_pda,
            recorded,
            summed
        );
    }
    scout_run_property!("P-0003", invariant_p_0003(fixture));
    // SCOUT:INVARIANT:P-0003:END

    // P-0007 is retired from the live harness: it is a VALID and strictly stronger statement of
    // redemption solvency than the confirmed P-0002, but every violation it produced was P-0002's
    // `redemption_vault_withdraw` drain, so leaving it armed only duplicates a written-up finding.
    // The cross-offer contamination it was written to test is REFUTED by
    // scout_reachability::c5_pool_stays_solvent_without_the_drain. Restore it once P-0002 is fixed:
    // it is the right long-term guard, since custody is pooled whether or not the accounting is.

    // P-0006 is CONFIRMED (report-p0006.md) and its block has been retired from the live
    // harness, as with P-0002/P-0004/P-0005. Regression preserved as
    // scout_reachability::t_p0006_update_offer_fee_bypasses_the_fee_ceiling.

    // P-0005 is CONFIRMED (report-p0005.md) and its block has been retired from the live
    // harness, as with P-0002 and P-0004. Regression preserved as
    // scout_reachability::t_p0005_same_mint_offer_mints_free_tokens.

    // P-0004 is CONFIRMED (report-p0004.md) and its block has been retired from the live
    // harness, for the same reason as P-0002: a confirmed property keeps firing on a known defect
    // and buries every other property's first finding. Regression preserved as
    // scout_reachability::t_p0004_redemption_fulfilment_bypasses_max_supply.

    // SCOUT:INVARIANT:P-0004:BEGIN
    // The configured supply cap holds against EVERY minter, not just the ones that remembered.
    //
    // RE-ARMED for programV5. P-0004's V4 counterexample was the redemption payout path handing
    // `mint_tokens` a hard-coded `0` cap; that path cannot exist any more, because a redemption
    // offer's token_in is pinned to ONyc and its payout leg therefore can never be the
    // program-controlled mint. The STATEMENT is unchanged and is still the right net — programV5
    // added three new minting paths (BUFFER accrual, the Prop AMM buy, `take_offer_v2`), and the
    // mirror ("this call site passes the cap") is exactly what missed it last time.
    //
    // `validate_max_supply` (`token_utils.rs:268`) is called by `mint_to`, both takes, the Prop AMM
    // buy and the BUFFER accrual, each passing `state.max_supply` itself. The net costs one account
    // read and covers all of them at once.
    fn invariant_p_0004(f: &mut OnreappFixture) {
        // Reads are inlined byte reads, not helper calls: the predicate grammar admits only
        // `ctx.account_data` and pure methods.
        let state_data = match f.ctx.account_data(&f.state_pda) {
            Ok(d) => d,
            Err(_) => return,
        };
        if state_data.len() < SCOUT_STATE_MAX_SUPPLY_END {
            return;
        }
        let cap = u64::from_le_bytes(
            match state_data[SCOUT_STATE_MAX_SUPPLY_OFFSET..SCOUT_STATE_MAX_SUPPLY_END].try_into() {
                Ok(b) => b,
                Err(_) => return,
            },
        );
        if cap == 0 {
            return; // 0 disables the cap
        }
        // SPL Mint: supply is a u64 at offset 36.
        let mint_data = match f.ctx.account_data(&f.mint_onyc) {
            Ok(d) => d,
            Err(_) => return,
        };
        if mint_data.len() < 44 {
            return;
        }
        let supply = u64::from_le_bytes(match mint_data[36..44].try_into() {
            Ok(b) => b,
            Err(_) => return,
        });
        scout_check!(
            "P-0004",
            "onyc-supply-never-exceeds-configured-cap",
            supply <= cap,
            "P-0004: ONyc supply {} exceeds the configured max_supply {} by {}.",
            supply,
            cap,
            supply.saturating_sub(cap)
        );
    }
    scout_run_property!("P-0004", invariant_p_0004(fixture));
    // SCOUT:INVARIANT:P-0004:END


    // SCOUT:INVARIANT:P-0010:BEGIN
    // The cached excluded balance must never exceed the live ONyc supply.
    //
    // `calculate_circulating_supply` is `total_supply.checked_sub(excluded_supply)` and ERRORS with
    // `ArithmeticUnderflow` on failure (`market_stats.rs:217-221`). `excluded_supply` is a CACHE
    // that any signer refreshes and nothing forces to be fresh; `total_supply` is live and FALLS on
    // every redemption burn, Prop AMM sell and `burn_for_nav_increase`. Once the cache sits above
    // the supply, every consumer of `recompute_market_stats` fails at once — `refresh_market_stats`,
    // `get_tvl_v2`, `get_circulating_supply_v2`, and the Prop AMM sell path's hard wall, which is
    // what stands between a seller and the redemption vault.
    //
    // Liveness rather than value: the claim is that the protocol cannot be pushed into that state
    // at all. Recovery exists (`update_circulating_supply_excluded_balance` is permissionless) but
    // needs every excluded owner's ATA supplied in list order, so the window is not self-healing.
    fn invariant_p_0010(f: &mut OnreappFixture) {
        let cache_data = match f.ctx.account_data(&f.excluded_balance_acct) {
            Ok(d) => d,
            Err(_) => return,
        };
        if cache_data.len() < SCOUT_EXB_MIN_LEN {
            return;
        }
        let cached = u64::from_le_bytes(
            match cache_data[SCOUT_EXB_AMOUNT..SCOUT_EXB_AMOUNT + 8].try_into() {
                Ok(b) => b,
                Err(_) => return,
            },
        );
        let mint_data = match f.ctx.account_data(&f.mint_onyc) {
            Ok(d) => d,
            Err(_) => return,
        };
        if mint_data.len() < 44 {
            return;
        }
        let supply = u64::from_le_bytes(match mint_data[36..44].try_into() {
            Ok(b) => b,
            Err(_) => return,
        });
        scout_check!(
            "P-0010",
            "excluded-balance-cache-never-above-supply",
            cached <= supply,
            "P-0010: the cached excluded balance is {} against a live ONyc supply of {}; every \
             market-stats consumer now fails with ArithmeticUnderflow.",
            cached,
            supply
        );
    }
    scout_run_property!("P-0010", invariant_p_0010(fixture));
    // SCOUT:INVARIANT:P-0010:END
    // SCOUT:INVARIANTS:END
}
