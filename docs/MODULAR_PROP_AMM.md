# Modular Prop AMM execution

The permissionless `PropAmm` quoter supports both directions through `take_offer`.
Each direction has its own OfferConfig, fees, and vault references. Both offers
can select the same quoter and its pair-bound PropAmmState to share pressure.
A reverse offer is optional: a standalone Prop AMM sell or buy offer can execute
without any corresponding opposite offer. Its pressure accumulates and expires
through the normal epoch decay; opposite-flow relief occurs only when a matching
offer exists and explicitly uses that tracker. Creation and execution do not
require a counterpart account.
Separate quoter instances have separate accounting even if they use the same
custody. Permissioned offers still require `Nav`; worker execution remains
unsupported. Legacy Prop AMM entrypoints, accounts and tracker behavior are unchanged.

## Pressure link and symmetric pricing

`OfferConfig.pressure_state` explicitly selects the PropAmmState that receives
successful trade accounting, independently of quote selection. At creation,
provide `prop_amm_state` to attach the tracker, or omit it for untracked NAV.
The boss can later call `set_offer_pressure_state` to attach, detach, or switch a
NAV offer's tracker. This emits `OfferPressureStateSetEvent` and never resets
tracker history. PropAmm pricing requires its own canonical state and cannot
switch to another quoter's tracker or detach it.

Both mint addresses must match the tracker's immutable asset/managed pair.
ONyc -> USDG sell pressure can receive buy relief only from USDG -> ONyc;
USDC -> ONyc and USDG -> another managed mint cannot link to that tracker.
The state PDA and stored bump are also checked. Execution must supply exactly
the stored tracker; omitting it, substituting another tracker, or supplying a
tracker for an unlinked offer fails. Sharing a liquidity vault does not link
pressure. A linked NAV offer retains NAV pricing while recording its executed
volume and directional trade count. Tracking does not require its parent pricing
quoter to be selected or enabled; the boss controls the offer's stored link.

Input fees are charged before quoting. The percentage fee rounds up and the
configured input-token minimum still applies.

| Direction | Asset value used for curve and accounting | NAV output reduced by curve |
| --- | --- | --- |
| Asset to managed | Entire net asset input, including refill and proceeds | Net asset input converted to managed tokens |
| Managed to asset | Raw NAV asset output before haircut | Net managed input converted to asset tokens |

For PropAmm pricing, both directions use the same curve with directional net
pressure: `max(buys - sells, 0)` for buys and `max(sells - buys, 0)` for sells.
Effective volume is decayed previous directional net pressure plus current
directional net pressure plus this trade's asset value. The dynamic wall is
`actual_asset_balance * 10000 / (10000 + sensitivity * effective_volume / actual_asset_balance)`,
with integer divisions, floored at one asset base unit. Effective pricing
liquidity is the minimum of that wall and the hard-wall reserve. The reserve is
actual liquidity for a zero refill target, or `min(actual, token-wide TVL target)`
otherwise. Utilization is this trade's asset value divided by effective liquidity.
The resulting factor multiplies the NAV output in the output mint's base units.

A PropAmm buy requires a configured, funded liquidity vault, just like a sell.
This trade's asset value must fit the current asset balance even if its discounted
output would fit. Pending asset input is not counted as existing pricing liquidity;
prefund a new Prop AMM vault before its first trade. NAV buys may omit liquidity.

The haircut is the greater of the utilization curve and cadence-wave haircut.
Output rounds down. Zero output, insufficient raw liquidity, or output below
`minimum_amount_out` fails. Cadence uses the prior count for the selected direction.
Opposite trades offset current net volume but do not erase cadence or already
carried previous-epoch pressure. Both directional counts advance on linked trades.
The integer curve and cadence math is shared with the legacy implementation;
the symmetric policy and tracker are modular and leave legacy behavior unchanged.

## Fixed epoch boundaries

The modular tracker uses scheduled boundaries, independently of the
legacy Solana tracker. For epoch duration `E` and stored start `S`:

- Before `S + E`, previous net pressure decays linearly and current net pressure
  contributes in full.
- From `S + E` through `S + 2E`, the old current net pressure becomes previous
  pressure and decays from the scheduled `S + E` boundary, even before a trade
  updates storage. Cadence count for quoting is zero in the new epoch.
- At or after `S + 2E`, old pressure expires. Recording advances to the latest
  scheduled boundary rather than resetting the start to the trade timestamp.

Quotes do not mutate the tracker. Successful execution records net buy volume or
raw sell value with checked `u128` aggregates and checked buy/sell counters. Any
later failure rolls back accounting, BUFFER accrual and token movements together.

## Buy-side liquidity refill

Refill works for all supported buy quoters: permissioned `Nav`, permissionless
`NavPermissionless`, and permissionless `PropAmm`. A NAV buy may omit liquidity;
a PropAmm buy must select one of the [supported custody authorities](VAULT_CONFIG_OPERATIONS.md).
Only a generic Liquidity VaultConfig has a modular `refill_target_bps`; legacy
configurable vaults and redemption custody use zero, which disables refill.

For a nonzero target, after BUFFER accrual and before the user's managed-token
mint, execution calculates:

```
TVL = floor(circulating_managed_supply * canonical_NAV / 10^9)
target_asset_units = floor(TVL * target_bps * 10^asset_decimals
                          / (10_000 * 10^managed_decimals))
refill = min(net_asset_input, max(target_asset_units - selected_balance, 0))
proceeds = net_asset_input - refill
```

TVL here is in managed-token base units before asset decimal conversion.
ONyc uses `State::main_offer` NAV and the canonical cached excluded balance, with
the existing cache freshness assumptions. Other managed mints use full mint
supply and the enabled USD instance-zero Pricer; per-mint supply-exclusion
configuration is not implemented. When the offer selects another Pricer, supply
`buffer_pricer` as the canonical instance-zero Pricer even if BUFFER is absent.
An initialized per-mint BUFFER retains its existing canonical Pricer requirement.

Curve hard-wall valuation in both directions uses pre-execution supply before
BUFFER accrual and user issuance/burn. Buy refill uses post-BUFFER, pre-user-mint supply. Both use
token-wide NAV rather than an offer's independently selected Pricer. Checked
arithmetic rejects unrepresentable intermediate values.

The selected asset ATA is created at the user's expense only for a positive
refill. Excess input goes to the Proceeds ATA. To have buys refill the funds used
by sells, configure both offers to select the same liquidity vault. Sharing a
quoter alone does not link their custody references.

`OfferExecutedEvent.liquidity_refill_amount` reports the actual refill, and
`proceeds_amount` reports the remainder. Existing slippage, deadline, kill-switch,
mint-authority, issuance-limit, fee and transfer-fee controls remain enforced.
ONyc still shares its original BUFFER accounts with V2; other managed tokens use
their initialized per-mint BUFFER. No live market configuration is created by
this implementation, and the CLI still uses V2 entrypoints.

## Modular layout compatibility

`OfferConfig` now stores the pressure-state reference (262 body bytes, 270 with
discriminator). `PropAmmState` stores previous net buy pressure and buy cadence
alongside the sell fields (203 body bytes, 211 with discriminator). Earlier modular
draft accounts require recreation or an explicit migration before this layout can
be used. No migration instruction is provided. Legacy Offer, PropAmmPairState,
ConfigurableVault, BUFFER and V2 account layouts and entrypoints are unchanged.
Clients must rebuild instructions against the regenerated IDL/types.

Production snapshot checks, representative transaction links, and the existing
intentional market getter removal are recorded in [MODULAR_PROP_AMM_VALIDATION.md](MODULAR_PROP_AMM_VALIDATION.md).
