# Modular Prop AMM validation against production

Observed on **2026-11-09, 21:23 UTC**, production program
[`onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe`](https://solscan.io/account/onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe).
Finalized account snapshot: slot **467467063**. Read-only quote simulations:
slot **467467066**, timestamp **1794259397**. These are historical observations,
not current configuration guarantees. No transactions were broadcast.

## Observed routes and configuration

Inspected 35 successful transactions from 14:57 to 21:10 UTC: 21 `get_nav`
CPI calls, seven `open_swap_sell` calls, three USDC `take_offer_permissionless_v2`
buys, and four configurable-vault withdrawals. No USDG buy appeared in this sample.
The sell transactions included flash borrowing and other swap legs; testing the
standalone modular instruction does not validate that entire external route.

Examples:

- [USDG sell at 19:59 UTC](https://solscan.io/tx/2wBMZgaiFm476b5n7UKbbM1Zm9dFy9adRthbwix57cdSaDTtRZC1diJYTmQd8PyLW6z4VikfLFN31dukrJ8QU82D):
  1,813.443530854 ONyc input, 4.533608828 ONyc fee, 2,105.783669 USDG output.
- [USDC buy at 20:08 UTC](https://solscan.io/tx/4KF9JtMXssT1dyUgag19MzuEWPJ3vmWLHw2S8h6UpAHZgAzcu39kvQS7xwLmFDxWk4BrzHCknojFXNUBMcZfk7ry):
  5,000 USDC input, 4,294.997837640 ONyc output, zero input fee.
- [Live NAV integration at 21:10 UTC](https://solscan.io/tx/jJRBvPMo7sA2DM9XbTyoVCwMnGcg1nmayVUCYx6NZmB98hfzNGnRWTRdmXdUBLSs8ZyRDr2vfWnkCvtGE91EeJu):
  a parent valuation transaction CPI-calls OnRe's `get_nav`.

The USDG market used SPL Token ONyc (9 decimals) and Token-2022 USDG (6 decimals).
NAV was 1.164144940 USD. ONyc mint supply was 265,729,412.437347351;
the cached excluded balance was 20,392,629.158112685 ONyc. BUFFER existed with
zero gross APR and zero management/performance fees.

| USDG Prop AMM parameter | Observed value |
| --- | ---: |
| Peg haircut | 3,000 bps |
| Exponent | 21,000 / 10,000 |
| Cadence threshold / wave | 150 / 2,000 |
| Epoch duration | 86,400 seconds |
| Wall sensitivity | 20,000 |
| Sell fee / minimum haircut | 25 bps / 1.5 ONyc |
| Current sells / buys | 161,057.168028 / 0 USDG |
| Previous net sells | 293,328.564770 USDG |
| Prior sell count | 87 |
| Epoch start | 1794178943 |
| Redemption vault USDG balance | 27,661,097.835196 |
| Redemption offer TVL target | 1,000 bps |

The Prop AMM proceeds USDG ATA was absent. Sell liquidity came from the legacy
redemption authority's USDG ATA. The current TVL target exceeded that balance,
so the effective hard-wall cap was actual liquidity. Modular legacy custody has
no modular refill target; selecting it does not import legacy redemption settings.
Future TVL-target/refill policy must be configured explicitly on a modular vault.
The modular ONyc registration account was absent: the new modular paths were
validated locally, not as deployed production instructions.

## Verification performed

Imported the actual mint accounts, legacy offers, pair tracker, vaults, exclusions,
MarketStats and BUFFER into LiteSVM using the rebuilt local SBF binary.
Three deployed `quote_swap_sell` simulations and local legacy quotes matched
exactly. The modular sell with corresponding pricing/fee parameters also matched:

| Gross ONyc input | USDG output at captured state |
| ---: | ---: |
| 1,813.443530854 | 2,105.783113 |
| 12,598.660319631 | 14,627.581755 |
| 100,000 | 115,973.906313 |

These are fresh quotes at the captured state, not reconstructed historical fills;
earlier transaction outputs differ because liquidity, time and pressure differ.
One compact, deterministic integration test retains these three production vectors
without requiring RPC access in CI. Raw snapshots and disposable fork probes were
kept outside the repository.

Additional local snapshot probes verified real Token-2022 USDG transfers, ONyc
mint/burn, redemption-custody debits, shared BUFFER baseline reconciliation, a
standalone sell, and seven sequential sells using the observed transaction sizes.
The probes also checked expired pressure after two epochs, explicit NAV buy
relief, and curve-priced buys. Individual sells in that burst consumed 171,598
to 234,735 compute units, including lazy ATA initialization on the first trade. Governance/user funding and new modular configuration
were prepared locally; deployed legacy custody/state was not migrated on-chain.

For example, a linked 50,000 USDG NAV buy improved the subsequent 1,813.443530854
ONyc sell from 2,105.783113 to 2,105.783291 USDG under the captured profile. A
10,000 USDG curve buy returned 8,589.984928611 ONyc, below its NAV output of
8,589.995675281 ONyc. The small difference reflects this market's large liquidity.

## Intentional market getter removal

Production transactions in this sample still invoked `get_nav` via CPI. This
checkout intentionally removed that entrypoint in commit `430417d`, replacing
market getters with reads from the MarketStats PDA. Replaying the old call against
the rebuilt binary returns Anchor **101, InstructionFallbackNotFound**, as expected
for an unsupported instruction. Legacy `get_nav` compatibility is outside the
supported upgrade scope; this observation is not a Prop AMM defect or deployment
blocker under that scope. Consumers should use the MarketStats PDA and inspect
its update timestamp/slot when assessing freshness. No getter restoration is
required by this implementation.
