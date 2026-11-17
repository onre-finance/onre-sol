# Offer source and destination vaults

`OfferConfig` independently selects `source_vault` and `destination_vault`.
A default pubkey means that role is absent. `create_offer_config` accepts both
references as optional accounts and validates their custody identities.

| Direction | Source vault: output-mint ATA | Destination vault: input-mint ATA |
| --- | --- | --- |
| Asset -> managed | Optional; transfer managed inventory from it. Without a source, mint managed output. | Optional for NAV; refill asset liquidity up to its configured TVL percentage. Zero percentage sends all net input to Proceeds. |
| Managed -> asset | Required; pay asset output from it. | Optional; transfer all net managed input here. Without a destination, burn the net input; Prop AMM sells burn only the paid share and retain the haircut (below). |

Input fees always go to the configured Fee vault before net-input settlement.

A Prop AMM sell without a destination burns only the share of the net managed
input `N` it paid for: with raw NAV output `R` and paid output `P`, it burns
`ceil(N × P / R)` and transfers the remaining haircut share to the offer's
`haircut_vault`, a `Haircut`-kind VaultConfig, into its canonical managed-mint ATA
(created at the user's expense on first use). Exactly these offers require a
haircut vault (`HaircutVaultRequired`); every other offer must omit it
(`UnexpectedHaircutVault`). Retained tokens stay in supply, so they count in
circulating supply and TVL and accrue BUFFER yield. `OfferExecutedEvent` reports
the retained amount as `haircut_amount`.
An explicitly configured source must cover the final quoted output. An empty or
insufficient source fails atomically; it never falls back to minting. Mint limits
apply to actual issuance, including BUFFER accrual, not inventory transfers.
Source and destination can reference the same authority: their token accounts
still differ because one holds the output mint and the other holds the input mint.

For buys, the destination's generic Liquidity VaultConfig `refill_target_bps`
retains its existing meaning: a percentage of circulating-supply TVL, not a
percentage of each payment. After BUFFER accrual and before user issuance:

```
refill = min(net_asset_input, max(TVL_asset_target - destination_asset_balance, 0))
proceeds = net_asset_input - refill
```

An absent destination, a zero target, or an already met target produces no refill.
Legacy ConfigurableVault and redemption custody have zero modular refill target.
For sells, the destination receives the entire net managed input regardless of
its target percentage. This preserves managed supply and permits inventory reuse.

Prop AMM pricing is unchanged: buys use destination asset liquidity and its target;
sells use source asset liquidity and its target. Prop AMM buys therefore require
a configured, funded destination even when output comes from managed inventory.
NAV buys allow both references to be absent. Price and pressure tracking remain
independent of custody, and the exact-pair pressure-link checks still apply.

## Execution accounts and authority

Supply `source_vault` with `source_token_out`, its canonical output-mint ATA;
supply `destination_vault` with `destination_token_in`, its canonical input-mint
ATA. An unset role requires both accounts omitted. Runtime checks the exact stored
reference, canonical ATA, mint, token program, authority, and account owner.
Destination ATAs are created lazily only for positive deposits. Empty source ATAs
have zero inventory. Authority substitution and role/mint swaps fail validation.

Either role supports a modular Liquidity VaultConfig, a legacy ConfigurableVault,
or the legacy redemption authority. Source payouts select that authority's actual
PDA signing seeds. Selecting custody preserves its existing withdrawal policy.
It does not implicitly use any other vault's balance. Permissionless offers retain
their existing intermediary route and transfer exactly this execution's output.

Source payout reads inventory after BUFFER accrual, including tokens accrued into
a selected legacy fee vault. BUFFER records actual post-execution supply for all
four mint/burn/inventory cases. Every managed mint, ONyc included, uses its
[per-mint BUFFER](MODULAR_TAKE_OFFER.md#per-mint-buffer); modular execution never
accrues the legacy ONyc singleton. Retained input and inventory output do not
change mint supply. All transfers, fee payments, accounting and ATA creation roll back
if any step fails.

`OfferExecutedEvent.source_amount` reports output paid from inventory;
`destination_amount` reports input deposited to destination. These are in different
mint base units. `liquidity_refill_amount` remains the buy's asset refill and
`proceeds_amount` remains the remaining buy input (zero for sells).

## Fresh modular layout and existing entrypoints

No modular configuration accounts have been created yet, so this defines their
initial layout without a migration requirement. OfferConfig contains source and
destination Pubkeys and is **294 body bytes / 302 allocated bytes**. PropAmmState
is 203 body bytes. New modular clients use the generated CreateOfferConfig,
TakeOffer and OfferExecutedEvent definitions in the updated IDL/types.

The compatibility requirement is to preserve `take_offer_v2`,
`take_offer_permissionless_v2`, `open_swap_buy`, and `open_swap_sell` exactly:
instruction data, account lists/flags, legacy account layouts, PDA identities,
fees, pricing, custody, mint/burn and legacy BUFFER behavior. These
entrypoints retain their handlers/account types and do not read the new vault
references or the per-mint BUFFER. Existing clients for these four paths need no changes for this work.
The CLI continues using V2 and has no modular-offer setup commands.
