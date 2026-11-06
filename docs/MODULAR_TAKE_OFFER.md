# Modular take_offer

`take_offer(gross_input_amount, minimum_amount_out, deadline, approval_message)`
executes the [reusable model](MODULAR_MODEL.md) through one public instruction.
It supports permissioned (`Nav`) and permissionless (`NavPermissionless`) NAV
in both directions. Asset-to-managed currently requires no liquidity-vault reference; managed-to-asset requires a configured
liquidity vault. Worker flows, Prop AMM quoting, and inbound liquidity refill
remain explicitly unsupported. The global kill switch guards execution.

Direction and permissions come from OfferConfig; the pricing algorithm comes from
Quoter. The shared account context lives in `execution/accounts.rs`, validation and
dispatch in `validation.rs`, authorization in `permissions.rs`, vector quoting
in `quote.rs`, mint/burn settlement in `settlement.rs`, selected-vault liquidity custody in `liquidity.rs`, and accrual in `buffer.rs`.
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
offers select `NavPermissionless`. Asset-to-managed has no liquidity vault;
managed-to-asset references a Liquidity VaultConfig, legacy ConfigurableVault, or
the existing redemption authority.
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
  managed-to-asset. No managed-token output inventory vault is required.
- Existing global `state`, `mint_authority`, the user signer, instructions sysvar,
  associated-token program, and system program.
- Canonical `buffer_state`, optional `buffer_pricer` and `main_offer`, and the
  reserve, management-fee, and performance-fee BUFFER token accounts, as below.
- `market_stats` and `circulating_supply_excluded_balance`, used for ONyc reporting.
- Optional `permissionless_authority`, `permissionless_token_in`, and
  `permissionless_token_out`, required by permissionless offers and omitted for
  other flows. The token accounts are the authority's canonical input/output ATAs
  under each mint's token program and are created lazily at the user's expense.
- Optional `prop_amm_state`, `liquidity_vault`, and `liquidity_token`, as described
  below. Redemption custody is supplied through the selected `liquidity_vault`
  reference; no additional legacy liquidity or Prop AMM accounts are required.

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
signs as the intermediary to distribute the input fee and net proceeds, or to burn
net managed input for the opposite direction. Minted managed output or liquidity
asset output goes to the intermediary output ATA. It then transfers exactly this
trade's amount out to the user. Preexisting balances in either intermediary ATA
are not swept, used as extra input, or included in the user's payout.

The boss-selected target markets are USDC -> ONyc and USDG -> ONyc. These are
configured by mint address; symbols are not an on-chain allowlist. A directed pair
and flow have one OfferConfig, so its selected Quoter determines the algorithm.
USDG's intended Prop AMM configuration remains a later layer: selecting PropAmm
currently rejects execution rather than falling back to NAV. This change creates
no live market configuration and leaves generic NAV direction handling intact.

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
Asset-to-managed divides net asset input by NAV. Managed-to-asset multiplies net
managed input by NAV, adjusting decimals and rounding output down.
Percentage fees round up in input
base units; the charged fee is `max(percentage_fee, minimum_fee_amount)`. Zero
input, a fee consuming all input, zero rounded output, and output below
`minimum_amount_out` all fail.

Execution settles BUFFER before changing user supply. Permissioned execution uses
the user as input authority and output recipient. Permissionless execution uses
the intermediary as described above. Asset-to-managed transfers
input fees to the Fee ATA and net input to the Proceeds ATA, then mints managed
output. Managed-to-asset transfers the fee in managed tokens to the Fee ATA, burns
net managed input, and pays the asset output from validated liquidity accounts
to the route recipient. It then records the actual post-execution managed mint supply as the BUFFER
baseline. Managed-token supply and issuance limits apply to both BUFFER accrual
and user issuance. ONyc additionally retains
the stricter nonzero global limits. Gross BUFFER accrual is checked as one
operation before splitting across fee destinations. Every step, including lazy
ATA creation, rolls back if the transaction fails.

`OfferExecutedEvent` records the offer, user, flow, direction, gross/net input,
fee, output, price, zero liquidity refill, proceeds, selected-vault
`liquidity_amount`, and timestamp. Managed-to-asset proceeds are zero.

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

When OfferConfig stores a liquidity-vault reference, provide that exact authority
and its canonical asset-mint ATA. The authority can be a modular Liquidity
VaultConfig (newly created or reused), an existing legacy ConfigurableVault, or
the canonical legacy redemption authority. Creation and execution validate the
configuration's program owner, discriminator, kind, PDA and bump. The stateless
redemption authority is validated by its canonical PDA identity. A legacy
configurable vault retains its original role and withdrawal destination/policy; selecting it explicitly makes its asset ATA available to that
offer. Legacy vaults and the redemption authority have no modular refill target.
Without a reference, liquidity accounts must be omitted; managed-to-asset always
requires a reference.

Only the selected vault's asset balance counts toward solvency. Existing redemption
liquidity is read and consumed only when its redemption authority is explicitly
selected, including for ONyc. The same vault may be explicitly shared by multiple offers; separate vault instances isolate
actual asset balances. Creating an offer does not create or migrate its liquidity.

An uninitialized canonical ATA owned by the system program counts as zero.
Initialized token accounts must match the token program, mint, authority and
canonical ATA address. Payout signs with the selected vault's validated PDA seeds.
Insufficient selected-vault funds roll back BUFFER accrual, fee movement, burn and
all earlier transfers. Other fee and proceeds vault balances are not included in
liquidity; an explicitly selected legacy configurable vault contributes only its
own asset ATA.

New VaultConfig balances can be funded or withdrawn using the
[VaultConfig custody instructions](VAULT_CONFIG_OPERATIONS.md), which also cover
the distinct per-mint BUFFER vault seeds.

A Prop AMM Quoter requires its canonical mutable PropAmmState, bound to the same
Quoter, asset and managed mint. Each new quoter uses its own state, independently
of old offers and their pressure. The two directed offers can intentionally share
one new quoter and its pressure state. Non-Prop AMM paths must omit the pair account.
Prop AMM execution still returns `UnsupportedOfferExecution` until the full
pricing/settlement path is implemented and tested. Worker execution is also
unsupported; supplying an approval cannot enable it.

The modular account list removes the former legacy liquidity authority/ATA and
legacy Prop AMM state positions. Clients must rebuild against the updated IDL.
`OfferExecutedEvent` now reports one `liquidity_amount` instead of a legacy/current
split. V2 instructions and events remain unchanged.

## Pricer vectors

`add_pricer_vector(vector)` requires the existing boss signer and a canonical
Pricer. It fills the first empty slot and requires the new start time to exceed
every stored start time. Up to ten slots are available. Start time cannot
be in the past; base time must be positive and no later than start time. Price and
step duration must be positive. `PricerVectorAddedEvent` includes boss, Pricer,
complete vector, and timestamp. Like other configuration changes, vector append
remains available during the kill switch. Eviction and deletion are deferred.

## Next layers

Liquidity refill, Prop AMM execution,
worker fulfillment, component updates, and legacy-entrypoint disabling remain
separate work. This instruction adds no automatic offer binding or bulk custody
migration. Existing liquidity is used only when explicitly selected for settlement
or moved separately.
