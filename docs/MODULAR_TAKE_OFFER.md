# Modular take_offer

`take_offer(gross_input_amount, minimum_amount_out, deadline, approval_message)`
executes the [reusable model](MODULAR_MODEL.md) through one public instruction.
It supports `Nav` for permissioned and permissionless offers, plus permissionless
[`PropAmm`](MODULAR_PROP_AMM.md), in both directions.
Offers independently select [source and destination vaults](OFFER_VAULT_ROLES.md).
NAV buys allow both absent; sells require a source and PropAmm buys require a
destination. It executes Instant offers only; Worker offers execute through
[fulfillment requests](#worker-fulfillment-requests). The global kill switch guards execution.

Direction, access (`Permissioned` or `Permissionless`) and execution (`Instant` or
`Worker`) come from OfferConfig; the pricing algorithm comes from Quoter. The shared
account context lives in `execution/accounts.rs`: `take_offer` and the worker fill
each expose their accounts as one `Execution` view with a `Route` (direct,
intermediary or request escrow), so both run the same pipeline. Validation and
dispatch live in `validation.rs`, authorization in `permissions.rs`, vector quoting
in `quote.rs`, symmetric curve/accounting in `prop_amm.rs`, liquidity targets in
`liquidity_target.rs`, mint/burn settlement in `settlement.rs`, selected-vault liquidity custody in `liquidity.rs`, and accrual in `buffer.rs`.
`take_offer.rs` orchestrates these modules in the shared `execute` and emits the execution event.
Configured extension accounts are validated before dispatch; supplying them does
not enable unfinished paths.

V2 instruction data, accounts, events, and legacy layouts remain unchanged, and
legacy execution keeps its behavior, including the legacy ONyc BUFFER.
The retired V1 name is reused with a different ABI; old V1 transactions remain
invalid. The CLI still calls V2. Generated IDL/types expose the new instructions.

## Setup and accounts

Register the managed mint, create its Pricer, append a vector with
`add_pricer_vector`, create the appropriate Quoter, Fee and Proceeds vaults,
a FeeConfig, and an OfferConfig. Permissioned offers select `Nav`; permissionless
offers select `Nav` or a pair-configured `PropAmm` quoter. Permissioned and
permissionless offers can share one `Nav` quoter and differ by FeeConfig.
Buy sources optionally supply managed inventory; buy destinations optionally receive
asset refill. Sells require an asset source and optionally transfer net managed
input to a destination instead of burning. PropAmm buys require asset destinations
for curve pricing. Both roles support modular Liquidity, legacy configurable, or
redemption custody. See the linked vault-role document for the complete matrix.
Create a new vault instance first when separate liquidity is needed. The program's existing
mint-authority PDA must control the managed mint in either direction; managed
registration does not transfer authority. The boss selects an external
USD-denominated asset. This path assumes one asset unit equals one USD and introduces no FX oracle.

The generated `TakeOffer` accounts contain the items below. Three contiguous runs
are nested account groups, each with its own `try_accounts` stack frame so account
validation stays within the 4 KiB SBF frame limit: `config`
(`TakeOfferConfigAccounts`: offer, managed token, Pricer, Quoter, FeeConfig and the
Proceeds/Fee vaults), `buffer` (`TakeOfferBufferAccounts`: BUFFER state/Pricer,
market stats, excluded-balance cache and the three BUFFER ATAs), and `custody`
(`TakeOfferCustodyAccounts`: pressure state and source/destination vaults/ATAs).
Groups flatten in place, so the transaction's account order is unchanged; Rust and
TypeScript clients pass each group as a nested object. `offer_config` is verified
against the PDA of its stored mint pair and instance ID, and the mint accounts must
equal that pair.

- `offer_config`, `managed_token`, the explicitly selected `pricer`, `quoter`,
  `fee_config`, `proceeds_vault`, and `fee_vault`.
- Input/output mints and token programs, the user's input/output ATAs, and the
  Fee vault's input-mint ATA and optional Proceeds input-mint ATA. The user's output
  and fee ATAs are created lazily at the user's expense. The Proceeds ATA is
  required and created lazily only for asset-to-managed; it must be omitted for
  managed-to-asset. A managed-token source is optional for buys.
- Existing global `state`, `mint_authority`, the user signer, instructions sysvar,
  associated-token program, and system program.
- Optional `legacy_buffer_state`, `legacy_buffer_reserve_token`,
  `legacy_buffer_management_token` and `legacy_buffer_performance_token`: the legacy
  ONyc BUFFER, required for ONyc before the one-way BUFFER switch and ignored
  otherwise (see Per-mint BUFFER).
- Optional `mint_multisig`: required exactly when the managed token mints through a
  multisig and must equal `ManagedTokenConfig::mint_multisig`; omitted for direct
  minting (see Minting authority).
- Canonical `buffer_state`, optional canonical `buffer_pricer`, and the
  reserve, management-fee, and performance-fee BUFFER token accounts, as below.
- `market_stats`, which must be the account selected by `ManagedTokenConfig`
  when one is selected and is ignored otherwise, and
  `circulating_supply_excluded_balance`, used by ONyc targets and reporting.
- Optional `legacy_market_stats`: the `["market_stats"]` singleton, required when
  the managed mint is ONyc and ignored otherwise (see Market stats).
- Optional `permissionless_authority`, `permissionless_token_in`, and
  `permissionless_token_out`, required by permissionless offers and omitted for
  other flows. The token accounts are the authority's canonical input/output ATAs
  under each mint's token program and are created lazily at the user's expense.
- Optional `prop_amm_state`, plus independently configured `source_vault` /
  `source_token_out`, `destination_vault` / `destination_token_in`, and
  `haircut_vault` / `haircut_token_in` (Prop AMM sells without a destination; see
  [OFFER_VAULT_ROLES.md](OFFER_VAULT_ROLES.md)). Each vault token account is the
  canonical ATA for that role's mint.

The managed mint and its token program are selected from the input or output side
according to the stored direction. Both mints are writable because either may be
the managed supply changed by accrual or settlement. Offer PDA seeds use the stored
instance ID rather than a hardcoded tag. Anchor optional accounts retain
fixed positions, using this program's address as the `None` placeholder.

All linked components must be enabled and match the offer's stored references.
The instruction checks canonical offer/managed-registration PDAs, mint decimals,
vault kinds, token programs, and managed mint authority. SPL Token and Token-2022
are supported; currently active transfer-fee mints are rejected.

Permissioned execution requires an Ed25519 verification instruction immediately
before `take_offer`, carrying the existing ApprovalMessage format signed by either
existing approver for this program, user, and an unexpired timestamp.
Permissionless execution requires `approval_message = None`; no Ed25519
verification instruction is needed. A supplied approval fails with `UnexpectedApproval`. The offer's
stored access mode controls authorization; supplying intermediary accounts cannot bypass
permissioned approval. The trade deadline is inclusive for both access modes.

## Permissionless settlement

Provide the same canonical `["permissionless-1"]` intermediary PDA used by V2,
along with its input/output ATAs. This reuses its existing signing identity and
does not introduce a new authority account layout. Its optional stored display
name is not an execution gate, matching V2's address-based validation.

The user signs a gross-input transfer into the intermediary input ATA. The program
signs as the intermediary to distribute input fees and net input to its configured
destination or Proceeds, or to burn net managed input when no sell destination is
set. Source inventory or newly minted managed output goes to the intermediary
output ATA. It then transfers exactly this
trade's amount out to the user. Preexisting balances in either intermediary ATA
are not swept, used as extra input, or included in the user's payout.

The boss-selected target markets are USDC -> ONyc and USDG -> ONyc. These are
configured by mint address; symbols are not an on-chain allowlist. Each OfferConfig
selects one Quoter, which determines the algorithm.
Prop AMM buy and sell offers may select the same quoter/state to share pressure.
Both directions apply the liquidity curve and their directional cadence haircut.
NAV offers may explicitly link to the same pair tracker for relief accounting while
retaining NAV pricing; the boss sets this through `set_offer_pressure_state`. See the linked Prop AMM document for epochs and refill.
No live market configuration is created automatically.

The supported integration baseline is a v0 transaction with address lookup tables.
The implementation must fit v0's 1,232-byte wire limit and 64-account transaction
limit; it does not depend on v1 support or its larger size allowance. Required
signers and top-level invoked programs remain in the static keys. Stable protocol
addresses can share reusable lookup tables; dynamic user accounts can remain inline
or be loaded from an appropriate table.

The tests execute actual v0 lookup-table transactions and enforce both limits.
The existing CLI transaction layer supports this format. For aggregator integration,
verify the complete outer route, including the aggregator and other swap legs,
against both limits. The standalone offer execution fitting does not prove every composed
route fits. The program handler itself does not impose a transaction-version check.

## Price and settlement

Price comes from the offer's selected Pricer, including nonzero instances. The
shared helpers in `utils/pricing.rs` accept vector data and a timestamp, without
loading Offer or Pricer accounts. They ignore empty slots (`start_time == 0`) and
select the latest vector whose start time is no later than the timestamp. Legacy
Offer vectors are adapted by value; their stored layout is unchanged. Neither
Offer nor Pricer stores a separate vector count.
Daily compounding and the snap to the end of the current price window match V2.
The NAV base quote divides net asset input by NAV for buys and multiplies net
managed input by NAV for sells, adjusting decimals and rounding output down.
Prop AMM buys and sells then apply the curve/cadence haircut described in
[`MODULAR_PROP_AMM.md`](MODULAR_PROP_AMM.md).
Percentage fees round up in input
base units; the charged fee is `max(percentage_fee, minimum_fee_amount)`. Zero
input, a fee consuming all input, zero rounded output, and output below
`minimum_amount_out` all fail.

Execution settles BUFFER before changing user supply. Permissioned execution uses
the user as input authority and output recipient. Permissionless execution uses
the intermediary as described above. Asset-to-managed transfers
input fees to the Fee ATA, optionally refills the destination asset ATA, sends
remaining net input to Proceeds, then transfers managed output from the source or
mints if no source is configured. Managed-to-asset transfers fees to the Fee ATA,
then transfers all net managed input to its destination or burns it if absent,
and pays asset output from its required source
to the route recipient. It then records the actual post-execution managed mint supply as the BUFFER
baseline. Managed-token supply and issuance limits apply to both BUFFER accrual
and user issuance. ONyc additionally retains
the stricter nonzero global limits. Gross BUFFER accrual is checked as one
operation before splitting across fee destinations. Every step, including lazy
ATA creation, rolls back if the transaction fails.

`OfferExecutedEvent` records the offer, user, access, execution, direction, gross/net input,
fee, output, price, actual liquidity refill, remaining proceeds, output inventory
`source_amount`, input deposit `destination_amount`, and timestamp. Managed-to-asset proceeds are zero.

## Per-mint BUFFER

Every managed mint uses its own `["buffer", managed_mint]` state; ONyc before its
one-way switch (below) accrues the legacy `["buffer_state"]` singleton instead,
supplied in separate legacy BUFFER accounts. `buffer_state` is mandatory at its
canonical address even when uninitialized;
supplying an empty substitute cannot bypass an initialized BUFFER. When it is
initialized, `buffer_pricer` must be the enabled canonical USD instance-zero
Pricer, and the three BUFFER token accounts must be the ATAs of the vaults
referenced by that state. Create those ATAs before a positive accrual. The offer
may choose a different Pricer; it never drives token-wide accrual. Accrual reuses
the existing gross-accrual and fee-split math and emits
`ManagedTokenBufferAccruedEvent` with mint, state, canonical Pricer, mint splits,
old/new supply, watermark, and timestamp.

### ONyc: one active BUFFER and the one-way switch

ONyc has two BUFFER engines, the legacy singleton and its per-mint BUFFER, and both
would accrue the same supply. Exactly one is active, selected by the one-way
`State::onyc_per_mint_buffer_active` flag (zero, i.e. legacy, on every deployment):

- **Before the switch** the legacy singleton is active. Legacy instructions accrue
  it as before (valued at `State::main_offer`). Modular ONyc `take_offer` accrues it
  too, through four extra accounts: `legacy_buffer_state` and the legacy reserve,
  management-fee and performance-fee vault ATAs (`LegacyBufferAccountsRequired` if
  missing), valued at the canonical Pricer and minted through the configured program
  minter under the effective issuance limits. Every ONyc supply change is therefore
  accrued exactly. The per-mint BUFFER accounts are supplied as usual and stay
  untouched. Per-mint standalone settlement, settings and burn-for-NAV fail with
  `PerMintBufferInactive`.
- **`switch_onyc_buffer_to_per_mint`** (boss only, blocked by the kill switch)
  accrues the legacy singleton up to now, has the initialized per-mint BUFFER take
  over its gross APR, fee rates and high-water mark from the current supply and
  timestamp, and sets the flag. It cannot be undone. Vault balances stay in the
  legacy vaults and are moved manually. `OnycBufferSwitchedToPerMintEvent` records
  the hand-over.
- **After the switch** only the per-mint BUFFER accrues; `take_offer` ignores the
  legacy BUFFER accounts. Clients can therefore supply both account sets for every
  ONyc trade, before and after the switch, and need no change when it happens. Legacy `settle_buffer`,
  `set_buffer_gross_apr`, `set_buffer_fee_config`, `burn_for_nav_increase` and
  `initialize_buffer` fail with `LegacyBufferRetired`; legacy reserve deposits and
  withdrawals keep working. Legacy trades still execute but no longer accrue: their
  mints are under-accrued and their burns over-accrued for the interval until the
  next per-mint accrual.

### Standalone settlement and policy

`settle_managed_token_buffer` is worker-only, using the existing `State.worker`.
It settles the per-mint BUFFER through the current timestamp without a trade and
refreshes the mint's selected MarketStats, if any.

`set_managed_token_buffer_settings(settings)` is boss-only. The settings are:

- `gross_apr`, scale 6, at most 1,000,000;
- `management_fee_basis_points`, at most 10,000;
- `performance_fee_basis_points`, at most 10,000;
- `performance_fee_high_watermark_enabled`.

It first accrues under the existing policy, then applies the next interval's
settings while preserving the just-settled baseline, timestamp, and watermark.
An unchanged policy fails with `NoChange`. Both instructions honor the kill
switch, canonical account and mint checks, program mint authority, and issuance
caps. For ONyc the stricter nonzero of the global and per-mint caps applies;
other mints use their per-mint caps. Per-mint caps default to zero (uncapped)
and are set with the boss-only `set_managed_token_mint_limits(max_supply,
max_mint_amount)`: zero disables a cap, a nonzero `max_supply` must be at least
the current supply (`MaxSupplyBelowCurrentSupply`), identical values fail with
`NoChange`, and `ManagedTokenMintLimitsSetEvent` records old and new values. The
caps bind program mints only; other members of a mint multisig (e.g. a CCIP pool)
mint outside them. Both require initialized state, and positive
accrual requires initialized vault ATAs; neither instruction creates them.

`burn_managed_token_for_nav_increase(asset_adjustment_amount)` is the boss-only
per-mint port of the legacy ONyc `burn_for_nav_increase` and EVM
`burnForNavIncrease`. It accrues the BUFFER first, then burns reserve tokens so the
asset base reduced by the adjustment (USD, in managed-mint base units) keeps the
current canonical NAV: `total_assets = floor(circulating × NAV / 1e9)`,
`required = ceil((total_assets − adjustment) × 1e9 / NAV)`,
`burn = circulating − required`. The reserve vault signs the burn of its ATA. A zero
adjustment, an adjustment above total assets, a zero burn and an insufficient
reserve are rejected; the kill switch blocks it. The BUFFER baseline and market
stats follow the burn, and `ManagedTokenBurnedForNavEvent` records it. For ONyc it
is available only after the one-way switch; before it, the legacy
`burn_for_nav_increase` burns the singleton reserve.

## Market stats

Modular reporting is opt-in per managed mint through
`ManagedTokenConfig::market_stats`. The default (unset) disables it: execution
and settlement skip the refresh and ignore the supplied `market_stats` account.
The boss selects an account with either instruction:

- `initialize_managed_token_market_stats` creates `["market_stats", mint]` and
  selects it.
- `set_managed_token_market_stats` selects any existing program-owned
  MarketStats account, such as the legacy `["market_stats"]` singleton for ONyc.
- `unset_managed_token_market_stats` clears the selection, disabling modular
  reporting without modifying the MarketStats account.

Once selected, the supplied `market_stats` must equal the selection. Each refresh
values the mint at its canonical USD instance-zero Pricer. ONyc subtracts the
canonical cached excluded balance (freshness policy unchanged); other mints report
full supply. ONyc execution additionally always refreshes the legacy `["market_stats"]`
singleton that integrators read, so it is never stale after a modular trade.
`take_offer` requires it as `legacy_market_stats` for ONyc (missing:
`LegacyMarketStatsRequired`; wrong address: `InvalidMarketStatsOwner`), creates it
on first use at the user's expense, and values it with the same canonical Pricer.
When ONyc selects the singleton itself, it is written once. Legacy
`refresh_market_stats` and legacy instructions keep refreshing only the singleton
from `State::main_offer`, so its NAV reflects whichever source refreshed last;
keep the ONyc Pricer and main offer vectors aligned. Standalone BUFFER
`settle_managed_token_buffer` and `set_managed_token_buffer_settings` apply the
same rule with the same required `legacy_market_stats` account, but have no payer:
they refresh the singleton only once it exists.
The optional legacy offer and reporting accounts do not affect other mints.

Between trades, anyone can call `refresh_managed_token_market_stats` to recompute
the selected MarketStats from current supply and the canonical Pricer, so NAV, APY
and TVL do not go stale when no trade or settlement occurs. It applies the same
rules, including the ONyc legacy singleton (created on first use at the caller's
expense), requires the managed token to be enabled, fails with
`MarketStatsNotSelected` when there is nothing to refresh, emits
`ManagedTokenMarketStatsRefreshedEvent`, and is not blocked by the kill switch.

## Minting authority

Modular minting (`take_offer` output and every BUFFER accrual, including
`settle_managed_token_buffer` and `set_managed_token_buffer_settings`) supports two
paths, selected per managed mint by `ManagedTokenConfig::mint_multisig`:

- **Direct** (default): the program's `["mint_authority"]` PDA is the mint
  authority and signs mints itself. No `mint_multisig` account may be supplied
  (`UnexpectedMintMultisig`).
- **Multisig**: the mint authority is a 1-of-N SPL multisig of the mint's token
  program whose signers include the PDA. The program mints by passing the multisig
  as authority and signing as the PDA member, so other members, such as a CCIP token
  pool, can mint independently. The configured multisig must be supplied
  (`MintMultisigRequired`; any other account: `InvalidMintMultisig`).

The path always comes from configuration, never from the mint's current authority;
each instruction requires the program to control the mint in that path, even when it
mints nothing, and applies the same issuance limits on both paths. Legacy instructions
remain direct-only: once a mint's authority moves to a multisig they no longer see
program mint control.

The boss switches with one transaction, after creating the multisig:

1. `transfer_mint_authority_to_boss`;
2. SPL `set_authority` (`MintTokens`) from the boss to the multisig;
3. `enable_managed_token_mint_multisig`, which requires an initialized 1-of-N
   multisig of the mint's token program that contains the PDA and already holds mint
   authority.

`disable_managed_token_mint_multisig` restores direct minting. If the configured
multisig still holds mint authority, the program reclaims it to its PDA by signing as
the multisig member; otherwise the PDA must already be the mint authority. Both
instructions are boss-only and emit `ManagedTokenMintMultisigSetEvent`.

## Conditional liquidity and Prop AMM accounts

Each configured vault role requires its exact authority and canonical role-mint
ATA. Source uses the output mint; destination uses the input mint. Unconfigured
roles must be omitted. Modular Liquidity, legacy ConfigurableVault and canonical
redemption authority identities are supported independently in each role. See
[OFFER_VAULT_ROLES.md](OFFER_VAULT_ROLES.md) for settlement, mint/burn and refill
rules, signing seeds, validation and account-layout compatibility.

Only the source's output balance can fund payouts. No automatic inventory fallback
or balance pooling occurs. A missing destination ATA can be created for a positive
deposit; a missing source ATA is empty inventory. All earlier actions roll back
when settlement fails. Custody can be funded and withdrawn through the existing
[VaultConfig instructions](VAULT_CONFIG_OPERATIONS.md).

A Prop AMM Quoter requires its canonical mutable PropAmmState, bound to the same
Quoter, asset and managed mint. Each new quoter uses its own state, independently
of old offers and their pressure. The two directed offers can intentionally share
one new quoter and its pressure state. NAV offers can explicitly select that same
pair tracker through `OfferConfig.pressure_state` without changing their NAV quote.
Both mints must match the tracker, regardless of quote kind. Execution requires
exactly the stored state; an omitted, substituted, or unsolicited tracker fails.
Prop AMM execution uses curve pricing in both directions, recording both against
that shared state. `take_offer` rejects Worker offers with
`UnsupportedOfferExecution`; supplying an approval cannot enable them.

The fresh modular account list uses independent source and destination custody
positions. New modular clients use the generated IDL/types; existing V2 and swap
clients retain their current account lists.
`OfferExecutedEvent` now reports `source_amount` and `destination_amount` for the
two custody roles, replacing the former `liquidity_amount`. V2 instructions and events remain unchanged.

## Pricer vectors

`add_pricer_vector(vector)` requires the existing boss signer and a canonical
Pricer, and requires the new start time to exceed every stored start time. Start
time cannot be in the past; base time must be positive and no later than start
time. Price must be positive and the step duration (`price_fix_duration`) must be
between 1 second and 1 day, as on EVM.

Before filling a slot it evicts history with the rule shared with legacy offers
and EVM: it keeps the vector active at the current time (the new one if it starts
now), the vector active immediately before it, and every future vector, and clears
all older vectors with `PricerVectorEvictedEvent`. Up to ten slots are available,
so `TooManyVectors` only occurs when active, previous and scheduled vectors fill
them. `PricerVectorAddedEvent` includes boss, Pricer, complete vector, and timestamp.

`delete_pricer_vector(vector_start_time)` removes one vector that has not started
yet (`StartTimeInPast` otherwise, `VectorNotFound` if absent).
`delete_all_pricer_vectors` clears every vector, including the active one; pricing
then fails with `NoActiveVector` until a new vector is added. Both are boss-only
and emit `PricerVectorDeletedEvent` / `AllPricerVectorsDeletedEvent`. Like other
configuration changes, vector management remains available during the kill switch.

## Worker fulfillment requests

Worker offers (`execution = Worker`) take any direction and quoter their access
mode allows, like Instant offers; execution changes only how a trade settles. They
port EVM `LibOnReFulfillment` (where workers are limited to NAV sells): the user
escrows the input, the worker fills it, and either can cancel the rest.

- `create_fulfillment_request(request_id, input_amount, approval_message)`, by the
  user. Requires the offer and every component it uses to be enabled, and the kill
  switch off. Permissioned offers need the same Ed25519 approval as `take_offer`;
  permissionless offers reject one. Escrows the gross input in the request PDA
  `["fulfillment_request", offer_config, user, request_id_le]`'s input-mint ATA.
  Locks no price or fee. The user pays both accounts' rent; an ID can be reused
  once its request closes.
- `fulfill_fulfillment_request(input_amount)`, by `State.worker`. Any amount up to
  the unfilled remainder (`FulfillmentAmountExceedsRemaining` otherwise). Each fill
  runs the shared `take_offer` pipeline on that gross input, with the escrow as
  input and the request user's ATA as output: the offer's quoter (NAV or Prop AMM
  curve, with its pressure recording) at the current price, its fee (the minimum
  applies per fill), BUFFER accrual, refill targets, proceeds, mint or source
  payout, burn, destination or haircut custody, and market stats. A fill and a
  `take_offer` of the same input on the same configuration settle identically.
  It takes the same accounts as `take_offer` except the user's input ATA and the
  permissionless intermediary (the escrow is already a program account); the
  worker pays for created accounts. Emits `OfferExecutedEvent` (user = request user) and
  `FulfillmentRequestFilledEvent`. A full fill closes the request and escrow,
  refunding their rent to the user; any unsolicited escrow surplus goes to the
  fee vault so it cannot block the close. Blocked by the kill switch.
- `cancel_fulfillment_request()`, by the request user or the worker. Returns the
  whole escrow balance to the user (recreating their ATA at the signer's expense
  if needed) and closes the request and escrow, refunding rent to the user. Works
  while the kill switch is active.

## Next layers

Legacy execution
paths stay available alongside modular execution. This instruction adds no automatic offer binding or bulk custody
migration. Existing liquidity is used only when explicitly selected for settlement
or moved separately.
