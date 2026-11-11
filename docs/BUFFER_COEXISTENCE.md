# BUFFER coexistence and ONyc cutover

ONyc has one authoritative BUFFER at a time. Legacy and modular partners share
its policy, supply baseline, timestamp, watermark and three token vaults until
an explicit boss-authorized cutover. There is no independently accruing copy.

| Phase / mint | Active `buffer_state` | Active token vaults | BUFFER valuation |
| --- | --- | --- | --- |
| ONyc, coexistence | Legacy `["buffer_state"]` | Existing reserve / management / performance ATAs | `State.main_offer` |
| ONyc, after cutover | `["buffer", ONyc]` | Per-mint reserve / management / performance ATAs | Enabled canonical USD Pricer instance zero |
| Other managed mints | `["buffer", mint]` | That mint's three per-mint vault ATAs | Enabled canonical USD Pricer instance zero |

The canonical global `State` selects ONyc's phase. Callers cannot select the other
state by supplying different accounts. The selected BUFFER state account remains
mandatory even when uninitialized. Standalone settlement/configuration requires
initialized state. After ONyc cutover, its active per-mint state must exist.

`initialize_managed_token_buffer` may prepare ONyc's new state and vault configs
before cutover, but they remain inactive. Modular operations continue reading and
updating the original singleton and its settings. Preparing the accounts does not
copy accounting, move funds, or activate a second clock.

## Modular settlement and policy

`settle_managed_token_buffer` is worker-only, using the existing `State.worker`.
It settles through the current timestamp without a user trade. ONyc also refreshes
the existing MarketStats PDA using the active phase's canonical valuation source.

`set_managed_token_buffer_settings(settings)` is boss-only. The settings are:

- `gross_apr`, scale 6, at most 1,000,000;
- `management_fee_basis_points`, at most 10,000;
- `performance_fee_basis_points`, at most 10,000;
- `performance_fee_high_watermark_enabled`.

It first accrues under the existing policy, then applies the next interval's
settings while preserving the just-settled baseline, timestamp and watermark.
An unchanged policy fails with `NoChange`. During coexistence it writes the legacy
ONyc settings, so existing configuration calls and modular calls observe each
other's changes. Both operations honor the kill switch, canonical account/mint
checks, program mint authority, and the stricter nonzero global/per-mint ONyc
issuance caps. Other mints use their per-mint issuance caps.

The shared `ManagedBufferAccounts` context supplies global state, managed-token
registration, writable mint, mint authority, token program, active BUFFER state,
optional canonical Pricer/main offer, and the three active writable vault ATAs.
Supply the worker or boss, system program, MarketStats and excluded-balance cache
as specified by the generated IDL. During coexistence ONyc requires its existing
main offer; after cutover it requires the canonical Pricer and can omit main offer.
Other mints require their canonical Pricer. Positive accrual requires initialized
vault ATAs. The settlement/configuration instructions do not create vault ATAs.

Trade execution uses the same accrual engine and records actual post-trade supply,
including inventory transfers that do not change supply. Alternating legacy and
modular operations at the same timestamp cannot accrue the interval twice.

## One-way cutover

Finish legacy supply-changing workflows, including pending redemption fulfillment
or cancellation, before cutover. Those worker/supply-changing paths also depend
on legacy BUFFER accrual and reject after retirement; modular worker redemption
fulfillment is separate work.

When partners are ready, call `cut_over_onyc_buffer` with the boss signer, both
BUFFER state accounts, the legacy main offer, canonical modular Pricer, all three
old authority/ATA pairs, all three new VaultConfig/ATA pairs, and the generated
IDL's governance, mint, reporting and program accounts.

Prepare the per-mint state/vault configs using `initialize_managed_token_buffer`.
Its Pricer must have the same active APR and current NAV as the legacy main offer
at cutover; a mismatch fails with `IncompatiblePricer`. New vault withdrawal
destinations are those configured by initialization (initially its boss). Supply
exclusions retain their existing cached-balance model: if old BUFFER owners are
excluded, update the configured owners and cached total as part of the migration
transaction so the new ownership preserves the intended excluded supply.

The instruction atomically:

1. Validates boss authority, mint, both canonical states, valuation compatibility,
   vault identities, roles and token accounts.
2. Settles the final legacy interval under the old policy and enforces both legacy
   and per-mint issuance limits.
3. Reloads all three old balances, including the final accrual, and transfers each
   entire balance to its matching per-mint vault. Missing destination ATAs are
   created at the boss's expense. Existing destination balances are retained.
4. Copies the final baseline, timestamp, watermark and all policy settings into
   the per-mint state, preserving its configured vault references and PDA bump.
5. Activates native ONyc BUFFER and marks the legacy state retired.
6. Refreshes MarketStats from the canonical modular Pricer and emits
   `OnycBufferCutOverEvent` with the three transferred amounts and accounting state.

Transfers do not change supply; only the final legacy accrual may mint. Any failure
rolls back accrual, fees, transfers, ATA creation, copied accounting and activation.
The operation cannot be repeated or reversed through these instructions.

Global `State.reserved[0] == 1` is the native ONyc activation marker;
legacy `BufferState.reserved[0] == 1` marks its frozen retirement record.
These use existing reserved bytes and do not change legacy layouts or PDA identities.

Before cutover, existing V2 and legacy swap/BUFFER paths retain their account lists
and behavior. After explicit cutover, legacy trades, accrual, reserve operations
and the legacy market refresh reject with `LegacyBufferRetired`; this prevents
old-state accrual or old-source market refresh alongside native BUFFER. Use modular
trades, modular settlement/settings and per-mint vault custody operations afterward.
No new accounts need to be appended to the legacy instruction lists.

## Validation

Focused integration tests cover shared legacy/native clocks and configuration,
SPL and Token-2022 native accrual, inactive-state substitution, an atomic cutover
with pending accrual and existing balances in all three vaults, post-cutover
modular trading and reporting without the legacy main offer, repeat/legacy-state
rejection, authority and rate/fee limits, aggregate mint caps, and rollback after
a late failure. The cutover fixture checks the actual transaction message against
the 1,232-byte and 64-account limits. The full legacy suites cover the zero-marker
coexistence behavior.
