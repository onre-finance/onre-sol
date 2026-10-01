# PROPERTIES

Discovered invariants for this program. `SHOULD-HOLD` = expected to always hold; `EXPLORATORY` = worth fuzzing but may be refined/dropped.

| ID | Property | Tag | Category | Status |
|----|----------|-----|----------|--------|
| P-0001 | `RedemptionOffer.request_counter` is monotonically non-decreasing (only ever increased). | EXPLORATORY | monotonicity | blocked |
| P-0002 | The redemption vault token account for a mint always holds at least the sum of the amounts locked by open (neither fulfilled nor cancelled) redemption requests denominated in that mint. | SHOULD-HOLD | solvency | confirmed |
| P-0003 | `RedemptionOffer.requested_redemptions` equals the sum of `amount - fulfilled_amount` over that offer's open redemption requests. | SHOULD-HOLD | conservation | survived |
| P-0004 | While state.max_supply is non-zero, the supply of state.onyc_mint never exceeds it. | SHOULD-HOLD | bounds | survived |
| P-0005 | No Offer may have token_in_mint equal to token_out_mint. | SHOULD-HOLD | structural-integrity | confirmed |
| P-0006 | Every live Offer.fee_basis_points is at most MAX_ALLOWED_FEE_BPS (1000). | SHOULD-HOLD | bounds | confirmed |
| P-0007 | The shared redemption vault token account for a mint covers the open requests of ALL redemption offers whose token_in is that mint, not just one offer's. | SHOULD-HOLD | solvency | blocked |
| P-0008 | The redemption vault for a token_in mint that charges a TRANSFER FEE still covers the open requests locked against it. | SHOULD-HOLD | solvency | confirmed |
| P-0009 | `BufferState.previous_supply` never exceeds the live supply of `state.onyc_mint`. | SHOULD-HOLD | conservation | confirmed |
| P-0010 | The cached circulating-supply excluded balance never exceeds the live supply of `state.onyc_mint`. | SHOULD-HOLD | dos/liveness | survived |

## Reporting scope

`Status` above is the harness lifecycle, **not** the report scope — a property can be `confirmed` and
still be excluded from the deliverable. This engagement excludes defects that require a `boss` or
`redemption_admin` signature at the moment value moves; those are trust-model observations. All
properties stay armed and all regression tests stay live regardless.

| property | confirmed a defect? | reported |
|---|---|---|
| P-0002 | yes | **no** — boss signs the drain and receives the funds (`out-of-scope/report-p0002.md`) |
| P-0004 | yes | **no** — `redemption_admin` signs every mint past the cap (`out-of-scope/report-p0004.md`) |
| P-0005 | yes | **yes** — `report-p0005.md`; the drain is `take_offer`, permissionless |
| P-0006 | yes | **no** — boss raises the fee, boss collects it (`out-of-scope/report-p0006.md`) |
| P-0008 | yes | **yes** — `report-p0008.md`; the deposit that breaks solvency is `create_redemption_request`, permissionless |
| P-0003 | no | n/a — armed; survived 3.4M+ executions before the fixture grew, regression now also pinned by `t_p0003_requested_redemptions_tracks_open_requests` |
| P-0001, P-0007 | not implemented / retired | n/a |

## P-0001 — request_counter monotonicity

`request_counter` is written by `create_redemption_request` (increment) and `make_redemption_offer`
(initialise to 0) — `field_effects` writer set, cross-checked against
`create_redemption_request.rs:189-194` and `make_redemption_offer.rs` respectively. Since
`make_redemption_offer` is `#[account(init, ...)]` it cannot target an existing offer, so the reset
should be unreachable for a live offer. Kept as EXPLORATORY: it is close to a mirror, and its only
real content is "nothing else resets the counter."

**Status `blocked`: stated but not implemented, deliberately.** The invariant predicate grammar
admits no helper calls, so a property needs its own registry of request addresses fed from a
success-gated action hook — and each property must own a SEPARATE registry, because an isolated
single-property replay runs exactly one hook and a shared counter would advance a different number
of times depending on which property was selected. Carrying a third registry for an acknowledged
near-mirror costs fixture size and per-iteration throughput for very little net. Worth implementing
only if a future change gives `request_counter` a second writer.

Reset would be serious rather than cosmetic: the counter is a **PDA seed**
(`create_redemption_request.rs:59-63`), so a rewind makes the next request derive an address that is
already occupied, permanently bricking new requests for that offer.

## P-0002 — redemption vault coverage  (the one worth breaking)

**Statement.** For a mint M, let `locked(M)` be the sum of `amount` over every redemption request
that has been created and neither fulfilled nor cancelled, whose `token_in_mint` is M. Then the
associated token account of `seeds::REDEMPTION_OFFER_VAULT_AUTHORITY` for M holds at least
`locked(M)`.

**Why this is a net and not a mirror.** No line of the program checks this, anywhere. The
individual movements are each locally consistent —

* `create_redemption_request.rs:162-170` transfers `amount` of token_in from the redeemer **into**
  that vault account and records the claim,
* `cancel_redemption_request.rs:183-191` transfers `amount` back **out** to the redeemer,
* `fulfill_redemption_request.rs` (via `redemption_utils.rs:195-236`) burns or transfers the same
  `amount` out,

— so each one preserves coverage on its own. The property is a net because it also spans an
instruction that touches the *same account* while knowing nothing about the claims against it:

* `redemption_withdraw.rs:redemption_vault_withdraw` transfers an arbitrary caller-supplied `amount`
  out of `associated_token::authority = redemption_vault_authority` to the boss, with **no
  reference to `requested_redemptions`, to any `RedemptionRequest`, or to any reserve at all**.

The vault authority PDA is derived from `seeds::REDEMPTION_OFFER_VAULT_AUTHORITY` alone — no
per-offer discriminator — so a single token account per mint custodies the deposits of every
redemption offer that shares that mint.

**What a violation means concretely.** Once the vault is drawn below `locked(M)`, the affected
requests can be neither fulfilled nor cancelled: both paths transfer out of that account and fail on
insufficient funds. The redeemer's tokens are gone and their request account cannot even be closed —
stranded, not merely delayed.

**False positives.** The check only reads state that the harness itself caused: `pending_requests`
is appended in `create_redemption_request`'s success-gated action hook and removed in fulfil's and
cancel's, both of which `close` the request account, so an entry is removed exactly once. Boss
deposits (`redemption_vault_deposit`) only ever make the inequality *more* satisfied. The property
therefore cannot fire on a sequence in which the vault was never drained below coverage.

## P-0003 — requested_redemptions equals the sum of its parts

**Statement.** `RedemptionOffer.requested_redemptions` equals the sum of `amount` over that offer's
open requests.

The three writers (`create_redemption_request.rs:181-186` add, `cancel_redemption_request.rs:194-199`
subtract, `fulfill_redemption_request.rs:283-286` subtract) are individually paired with the request
lifecycle. The aggregate net asks the question none of them asks: does the recorded total still
describe the requests that actually exist, across every path that can retire one — including any
path that retires a request without decrementing, or decrements by an amount other than the one it
locked.

**False positives.** Compared against the harness's own shadow ledger, which is updated only on
success. `make_redemption_offer` initialises the field to 0 on a *new* offer (`init`, so never the
one under test).

## P-0004 — the ONyc supply cap is a global bound, not a per-caller one

**Statement.** While `state.max_supply != 0`, the supply of `state.onyc_mint` never exceeds it.

`mint_tokens` (`token_utils.rs:225-245`) is the single choke point for every mint the program
performs, and enforces `current_supply + amount <= max_supply` **only when handed a non-zero cap**.
Its doc states the purpose: *"prevents unbounded inflation when max supply is configured"*
(`token_utils.rs:221-223`).

Writer-set difference over the three callers:

| caller | cap argument |
|---|---|
| `take_offer.rs:296` | `state.max_supply` |
| `mint_authority/mint_to.rs` | `state.max_supply` |
| `fulfill_redemption_request.rs:274` | **`0`** — "No max supply cap for redemptions" |

Asserting the bound inside `mint_tokens` would be a mirror. Asserting it over the mint's persisted
supply after every instruction is the net, and it is blind to which caller minted.

**False positives.** `configure_max_supply` assigns the cap with no floor check, so a cap set *below*
the current supply makes `supply > cap` true by declaration rather than by minting — a genuine
counterexample to the naive statement. Excluded structurally: the harness's only cap-setting action
floors the cap at the supply of the moment, so any later excess was necessarily minted. Burns only
lower the left side. A zero cap is skipped, matching the program's own reading of `0` as "no cap".

**Confirmed** — `out-of-scope/report-p0004.md` (excluded from the deliverable: `redemption_admin`
signs every mint past the cap).

## P-0005 — an offer's two legs must be distinct mints

**Statement.** No `Offer` may have `token_in_mint == token_out_mint`.

`MakeOffer` relates the two mint arguments nowhere, and grepping the whole program for a comparison
between them returns one unrelated hit (`create_redemption_request.rs:154`). The PDA
`seeds = [OFFER, token_in_mint, token_out_mint]` is well defined when they are equal, so even the
address derivation gives no accidental protection.

A same-mint offer is meaningless at every price: `calculate_token_out_amount`
(`token_utils.rs:81-122`) gives the taker `token_in_net * 1e9 / price` of the token they just paid,
so below 1.0 it mints value from nothing and above 1.0 it destroys the taker's.

Asserting "the taker did not profit" would be a mirror of a check that does not exist and would need
a shadow of every balance. This asserts the structural precondition instead — read the two fields out
of the persisted account and require them to differ.

**False positives.** Structural ground truth, no shadow state, no economic quantity asserted. A
wrapped registry bails out; unreadable or short accounts are skipped. Equality has no benign reading.

**Confirmed** — `report-p0005.md`.

## P-0007 — pooled redemption-vault solvency  (valid; retired as subsumed)

**Statement.** The one token account backing every redemption offer that shares a `token_in` mint
covers the open requests of **all** of them, not just one offer's.

This is the cross-offer strengthening of the confirmed P-0002. `seeds::REDEMPTION_OFFER_VAULT_AUTHORITY`
has no per-offer discriminator, so custody is pooled per mint while `requested_redemptions` is tracked
per offer — nothing in the program reconciles the two.

**Written to test a specific chain (escalation pass, amplifier #9 shared-key conflation):** can one
offer's fulfilments consume another offer's collateral? Reasoning said no — each of create / cancel /
fulfil moves exactly the request's own `amount` — but that is a claim about three handlers
interacting across two offers, so it went to the fuzzer rather than to my reading.

**Subject proven constructible**, not assumed: `scout_reachability::p7_two_offers_share_one_vault`
builds two ONyc-denominated redemption offers funding the same account (`vault 0 -> 1000000 ->
3000001`). This needed a new action, `action_scout_create_request_play`; without it the property
degenerates into P-0002 and is silently dead.

**Result — the chain is REFUTED, and the property is subsumed.** The isolated campaign fired, but
every counterexample ends in `redemption_vault_withdraw`, i.e. the already-confirmed P-0002 drain.
The decisive experiment, `scout_reachability::c5_pool_stays_solvent_without_the_drain`, interleaves
create/fulfil/cancel across both offers **without** the drain and finds `vault == claimed` exactly, at
every step:

```
 2. create A1    vault=3000000     claimed=3000000     ok
 3. create B1    vault=8000001     claimed=8000001     ok
 4. create A2    vault=15000002    claimed=15000002    ok
 5. create B2    vault=26000003    claimed=26000003    ok
 6. fulfil A1    vault=23000003    claimed=23000003    ok
 7. cancel A2    vault=16000002    claimed=16000002    ok
 8. create A3    vault=18500002    claimed=18500002    ok
 9. fulfil A3    vault=16000002    claimed=16000002    ok
```

Offer B's 16,000,002 stays fully covered throughout offer A's traffic. **No cross-offer
contamination exists.**

**Status `blocked` (retired from the live harness), deliberately.** The property is valid and
reached; leaving it armed only re-reports P-0002. Restore it once P-0002 is fixed — it is the better
long-term guard, because custody is pooled whether or not the accounting is.

## P-0008 — redemption solvency on a transfer-fee token_in  (permissionless by construction)

**Statement.** For the Token-2022 mint `mint_fee`, which carries a live `TransferFeeConfig`, the
redemption vault token account holds at least the sum of `amount` over its open requests.

**Deliberately the same statement as P-0002, over a vault the boss cannot reach.** `pick_vault_mint`
returns only `mint_usdc` or `mint_onyc`, so no `redemption_vault_withdraw` in this harness can touch
`mint_fee`'s vault. Any violation is therefore reachable through **permissionless actions alone** —
which is why it is worth carrying next to a confirmed sibling instead of being folded into it. It is
the first property on this target whose violation survives the engagement's admin-trigger scope rule.

**Why it is a net and not a mirror.** No line of the program compares these two quantities. The
program records the amount a redeemer *asked* to lock — `RedemptionRequest.amount` at
`create_redemption_request.rs:177`, and the same figure into `requested_redemptions` at `:181-186` —
while the vault receives whatever the token program actually delivered. On a fee-bearing mint those
differ by the fee, every time, with no attacker and no unusual state.

The guard that would catch it exists and is applied elsewhere: `has_transfer_fee`
(`token_utils.rs:486`) has exactly **two** call sites in the whole program, both inside
`execute_token_operations` (`:374`, `:378`), which only `take_offer` / `take_offer_permissionless`
reach. `create_redemption_request` calls `transfer_tokens` directly, and `make_redemption_offer`
performs no mint validation at all.

**Fixture capability required.** Amplifier #8 (amount ≠ accounting) is structurally unobservable
without a fee-bearing mint in the world — a clean campaign on the old four-plain-SPL-mint fixture
meant "the fixture cannot express it", not "the program is safe". `scout_mk_t22_fee_mint` /
`scout_mk_t22_ata` exist for exactly this, and
`scout_reachability::t_p0008_control_offer_path_refuses_the_same_mint` is the positive control: the
offer path must still refuse the very same mint, which is what proves the fee is live rather than
that the harness built an ordinary mint.

**False positives.** The registry is appended only on success; both retirement paths close the
request account, so account existence *is* openness. A wrapped ring bails out rather than summing a
partial set. `redemption_vault_deposit` can only raise the left-hand side.

## programV5 — what happened to the V4 ledger

The target was fast-forwarded 175 commits to `origin/master` (`1dbc6ec`), from 38 instructions to
66. Every V4 property was re-examined against the new source; three of the five confirmed ones are
no longer defects, and each is now pinned by a test asserting the programV5 behaviour rather than
the old one.

| id | programV5 status | evidence |
|---|---|---|
| P-0002 | unchanged and still out of scope (boss signs the drain) | not re-tested this round |
| P-0003 | RESTATED. Fulfilment is now PARTIAL, so the aggregate equals the sum of `amount - fulfilled_amount`, not of `amount`. Back to `implemented`; the old counterexample form no longer applies. | `t_p0003_requested_redemptions_tracks_open_requests` |
| P-0004 | RE-ARMED. Its V4 counterexample — the redemption payout path passing a hard-coded `0` cap — cannot exist, because a redemption offer's token_in is pinned to ONyc so its payout leg is never the program-controlled mint. The statement is still the right net over programV5's three NEW minters (BUFFER accrual, Prop AMM buy, `take_offer_v2`). | `t_v5_redemption_token_in_must_be_onyc` |
| P-0005 | ROOT CAUSE PRESENT, EXPLOIT GONE. `make_offer` still relates nothing between its two mints, but anchor-lang 1.1.2 rejects the take with `ConstraintDuplicateMutableAccount` — a same-mint offer necessarily passes one token account in two `mut` slots. A framework mitigation, not a program fix. | `t_p0005_same_mint_offer_is_creatable_but_no_longer_takeable` |
| P-0006 | FIXED. `update_offer_fee.rs:98` now bounds against `MAX_ALLOWED_FEE_BPS`, and the new `update_offer_permissionless_fee` was written with the same bound. All four writers agree. | `t_p0006_fee_ceiling_is_uniform_across_every_writer` |
| P-0008 | FIXED BY CONSTRUCTION. `make_redemption_offer.rs:66-72` requires `token_in_mint == state.onyc_mint` under the plain SPL token program, so a fee-bearing redemption offer cannot be created. | `t_p0008_fee_bearing_redemption_offer_is_unconstructible` |

## P-0009 — the BUFFER baseline must never sit above the live supply

`docs/BUFFER_ACCRUAL.md` ("Supply Baseline Update") promises that after any supply-changing
operation the next baseline is the post-change supply, and the accrual charges yield on that
baseline. The same document then exempts the offer path: legacy `take_offer` /
`take_offer_permissionless` do not update it, and "offer executions that only burn ONyc as token in
are not wired into the BUFFER baseline path". The exemption is wider than the sentence suggests —
`take_offer_v2` gates its baseline write on `should_accrue_onyc_mint`, which is true only when
token_OUT is ONyc (`offer_utils.rs:165-174`), so a v2 take that BURNS ONyc skips it too.

The property is the net the code never checks: `previous_supply <= onyc_supply`. A baseline above
the live supply means the next accrual mints yield against tokens that were already burned. The
accrual is linear in the baseline, so the excess is `minted * (baseline - supply) / baseline`, and
it grows with both the burn size and the time to the next accrual.

## P-0010 — the excluded-balance cache must never exceed the live supply

`calculate_circulating_supply` is `total_supply.checked_sub(excluded_supply)` and ERRORS on failure
(`market_stats.rs:217-221`). The subtrahend is a cache that any signer refreshes and nothing forces
to be fresh; the minuend is live and falls on every redemption burn, Prop AMM sell and
`burn_for_nav_increase`. Once the cache sits above the supply, every consumer of
`recompute_market_stats` fails at once — `refresh_market_stats`, `get_tvl_v2`,
`get_circulating_supply_v2`, and the Prop AMM sell path's hard wall. Recovery is permissionless but
requires every excluded owner's ATA in list order, so the window does not close by itself.
