# Modular take_offer

`take_offer(gross_input_amount, minimum_amount_out, deadline, approval_message)`
executes the [reusable model](MODULAR_MODEL.md) through one public instruction.
It supports permissioned (`Nav`) and permissionless (`NavPermissionless`) NAV,
plus permissionless [`PropAmm`](MODULAR_PROP_AMM.md), in both directions.
Offers independently select [source and destination vaults](OFFER_VAULT_ROLES.md).
NAV buys allow both absent; sells require a source and PropAmm buys require a
destination. Worker flows remain unsupported. The global kill switch guards execution.

Direction and permissions come from OfferConfig; the pricing algorithm comes from
Quoter. The shared account context lives in `execution/accounts.rs`, validation and
dispatch in `validation.rs`, authorization in `permissions.rs`, vector quoting
in `quote.rs`, symmetric curve/accounting in `prop_amm.rs`, liquidity targets in
`liquidity_target.rs`, mint/burn settlement in `settlement.rs`, selected-vault liquidity custody in `liquidity.rs`, and accrual in `buffer.rs`.
`take_offer.rs` orchestrates these modules and emits the execution event.
Configured extension accounts are validated before dispatch; supplying them does
not enable unfinished paths.

V2 instructions, their data, accounts, events, and legacy layouts remain unchanged.
The retired V1 name is reused with a different ABI; old V1 transactions remain
invalid. The CLI still calls V2. Generated IDL/types expose the new instructions.

## Setup and accounts

Register the managed mint, create its Pricer, append a vector with
`add_pricer_vector`, create the appropriate Quoter, Fee and Proceeds vaults,
a FeeConfig, and an OfferConfig. Permissioned offers select `Nav`; permissionless
offers select `NavPermissionless` or a pair-configured `PropAmm` quoter.
Buy sources optionally supply managed inventory; buy destinations optionally receive
asset refill. Sells require an asset source and optionally transfer net managed
input to a destination instead of burning. PropAmm buys require asset destinations
for curve pricing. Both roles support modular Liquidity, legacy configurable, or
redemption custody. See the linked vault-role document for the complete matrix.
Create a new vault instance first when separate liquidity is needed. The program's existing
mint-authority PDA must control the managed mint in either direction; managed
registration does not transfer authority. The boss selects an external
USD-denominated asset. This path assumes one asset unit equals one USD and introduces no FX oracle.

The generated `TakeOffer` accounts contain:

- `offer_config`, `managed_token`, the explicitly selected `pricer`, `quoter`,
  `fee_config`, `proceeds_vault`, and `fee_vault`.
- Input/output mints and token programs, the user's input/output ATAs, and the
  Fee vault's input-mint ATA and optional Proceeds input-mint ATA. The user's output
  and fee ATAs are created lazily at the user's expense. The Proceeds ATA is
  required and created lazily only for asset-to-managed; it must be omitted for
  managed-to-asset. A managed-token source is optional for buys.
- Existing global `state`, `mint_authority`, the user signer, instructions sysvar,
  associated-token program, and system program.
- Canonical `buffer_state`, optional `buffer_pricer` and `main_offer`, and the
  reserve, management-fee, and performance-fee BUFFER token accounts, as below.
- `market_stats` and `circulating_supply_excluded_balance`, used for ONyc reporting.
- Optional `permissionless_authority`, `permissionless_token_in`, and
  `permissionless_token_out`, required by permissionless offers and omitted for
  other flows. The token accounts are the authority's canonical input/output ATAs
  under each mint's token program and are created lazily at the user's expense.
- Optional `prop_amm_state`, plus independently configured `source_vault` /
  `source_token_out` and `destination_vault` / `destination_token_in`.
  Each vault token account is the canonical ATA for that role's mint.

The managed mint and its token program are selected from the input or output side
according to the stored direction. Both mints are writable because either may be
the managed supply changed by accrual or settlement. Offer PDA seeds use the stored
flow rather than a hardcoded permissioned tag. Anchor optional accounts retain
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
stored flow controls authorization; supplying intermediary accounts cannot bypass
permissioned approval. The trade deadline is inclusive for both flows.

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
configured by mint address; symbols are not an on-chain allowlist. A directed pair
and flow have one OfferConfig, so its selected Quoter determines the algorithm.
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

`OfferExecutedEvent` records the offer, user, flow, direction, gross/net input,
fee, output, price, actual liquidity refill, remaining proceeds, output inventory
`source_amount`, input deposit `destination_amount`, and timestamp. Managed-to-asset proceeds are zero.

## BUFFER coexistence

`buffer_state` is mandatory at the canonical address even when uninitialized.
Supplying an empty substitute cannot bypass an initialized BUFFER.

When the managed side is `State::onyc_mint`, provide the existing `["buffer_state"]`
account and the three legacy BUFFER ATAs. `main_offer` must be `State::main_offer`; it supplies
canonical BUFFER valuation and reporting, independently of the offer's Pricer.
New mint/burn execution and V2 use one supply baseline, watermark, and clock. A separate
per-mint ONyc BUFFER record remains inactive during coexistence. Execution in either direction
also refreshes the existing MarketStats snapshot using the legacy main offer and
the canonical cached excluded balance, with the same freshness assumptions as V2.

Other managed mints use `["buffer", managed_mint]`. When initialized,
`buffer_pricer` must be the enabled canonical USD instance-zero Pricer, and the
three BUFFER token accounts must be the ATAs of the vaults referenced by that
state. Create those ATAs before a positive accrual. The offer may choose a
different Pricer. Accrual reuses existing gross-accrual and fee-split math and emits
`ManagedTokenBufferAccruedEvent` with mint, state, canonical Pricer, mint splits,
old/new supply, watermark, and timestamp. No second ONyc accrual engine is activated.
The optional legacy offer and reporting accounts do not affect other mints.

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
that shared state. Worker execution remains unsupported;
supplying an approval cannot enable it.

The fresh modular account list uses independent source and destination custody
positions. New modular clients use the generated IDL/types; existing V2 and swap
clients retain their current account lists.
`OfferExecutedEvent` now reports `source_amount` and `destination_amount` for the
two custody roles, replacing the former `liquidity_amount`. V2 instructions and events remain unchanged.

## Pricer vectors

`add_pricer_vector(vector)` requires the existing boss signer and a canonical
Pricer. It fills the first empty slot and requires the new start time to exceed
every stored start time. Up to ten slots are available. Start time cannot
be in the past; base time must be positive and no later than start time. Price and
step duration must be positive. `PricerVectorAddedEvent` includes boss, Pricer,
complete vector, and timestamp. Like other configuration changes, vector append
remains available during the kill switch. Eviction and deletion are deferred.

## Next layers

Worker fulfillment, component updates, and legacy-entrypoint disabling remain
separate work. This instruction adds no automatic offer binding or bulk custody
migration. Existing liquidity is used only when explicitly selected for settlement
or moved separately.
