# Token Flow Routing

This page shows where tokens move for the main V2 offer, redemption, and Prop AMM flows. It focuses on accounting destinations: fee vaults, proceeds vaults, redemption vault liquidity, burns, mints, and user payouts.

Legacy `take_offer` and legacy `take_offer_permissionless` do not use configurable accounting vaults or redemption-vault refill routing. If the program controls the token-in mint, net token-in is burned and fee token-in routes to the boss token-in ATA; otherwise both net token-in and fee token-in route to the boss token-in ATA.

## Kill Switch

Token-moving instructions on this page are guarded by `state.is_killed`. When the kill switch is enabled, offer takes, Prop AMM quotes/execution, redemption request create/fulfill/cancel, vault deposits/withdrawals, reserve vault movement, configurable-vault withdrawals, direct `mint_to`, `burn_for_nav_increase`, and BUFFER config updates that settle accrual reject before moving, minting, or burning tokens.

The kill switch is not a global pause for every instruction. Governance and configuration-only instructions remain available under their normal authority checks.

## Shared Refill Rule

Net asset inflow from `take_offer_v2`, `take_offer_permissionless_v2`, and Prop AMM buy can refill the redemption vault only when all conditions are true:

- a matching `RedemptionOffer` exists
- the matching `RedemptionOffer` is initialized and enabled
- `RedemptionOffer.vault_target_bps > 0`

For `take_offer_v2` and `take_offer_permissionless_v2`, refill also requires that the program does not control the token-in mint. If the program controls token-in, net token-in is burned and refill is forced to zero. Prop AMM buy calculates refill directly from the target and current redemption vault balance.

The cap is:

```text
target = TVL * vault_target_bps / 10_000
target_in_token_in_decimals = target * 10^token_in_decimals / 10^token_out_decimals
headroom = max(0, target_in_token_in_decimals - current_redemption_vault_balance)
refill = min(net_asset_inflow, headroom)
proceeds = net_asset_inflow - refill
```

If `MarketStats` cannot be read, the target calculation resolves to zero. Fees are never part of the refill amount. Fees route to the fee vault for the same business source.

Protocol fees use ceiling rounding. Token-2022 mints with transfer fees are rejected inside the token-operation helpers; failed transactions roll back staged transfers.

## `take_offer_v2`

```mermaid
flowchart TD
    UserIn[User token in] --> Calc[Calculate offer quote]
    Calc --> Fee[Fee amount]
    Calc --> Net[Net token in]
    Calc --> Out[Token out amount]

    Fee --> OfferFee[OfferFee vault]

    Net --> InMintMode{Program controls token in mint}
    InMintMode -- yes --> Burn[Burn net token in]
    InMintMode -- no --> RefillCheck{Redemption refill enabled}
    RefillCheck -- no --> OfferProceeds[OfferProceeds vault]
    RefillCheck -- yes --> Cap[Cap refill by TVL target headroom]
    Cap --> Refill[Redemption vault token in ATA]
    Cap --> Overflow[Overflow]
    Overflow --> OfferProceeds

    Out --> OutMintMode{Program controls token out mint}
    OutMintMode -- yes --> Mint[Mint token out to user]
    OutMintMode -- no --> Pay[Transfer token out from offer vault to user]
```

Notes:

- Refill uses net token-in only.
- `OfferFee` receives the fee regardless of whether net token-in is burned, refilled, or sent to proceeds.
- If token out is ONYC, the buffer is initialized, and the program controls the ONYC mint, buffer accrual can run before minting. Market stats can refresh after execution whenever token out is ONYC.

## `take_offer_permissionless_v2`

```mermaid
flowchart TD
    UserIn[User token in] --> IntermediaryIn[Permissionless token in ATA]
    IntermediaryIn --> Calc[Calculate offer quote]
    Calc --> Fee[Fee amount]
    Calc --> Net[Net token in]
    Calc --> Out[Token out amount]

    Fee --> PermissionlessOfferFee[PermissionlessOfferFee vault]

    Net --> InMintMode{Program controls token in mint}
    InMintMode -- yes --> Burn[Burn net token in]
    InMintMode -- no --> RefillCheck{Redemption refill enabled}
    RefillCheck -- no --> OfferProceeds[OfferProceeds vault]
    RefillCheck -- yes --> Cap[Cap refill by TVL target headroom]
    Cap --> Refill[Redemption vault token in ATA]
    Cap --> Overflow[Overflow]
    Overflow --> OfferProceeds

    Out --> OutMintMode{Program controls token out mint}
    OutMintMode -- yes --> MintToIntermediary[Mint token out to permissionless ATA]
    OutMintMode -- no --> VaultToIntermediary[Transfer token out from offer vault to permissionless ATA]
    MintToIntermediary --> UserOut[Transfer token out to user]
    VaultToIntermediary --> UserOut
```

Notes:

- `take_offer_permissionless_v2` uses `fee_basis_points_permissionless`, which defaults to `0` for newly created and upgraded offers until the boss calls `update_offer_permissionless_fee`.
- Permissionless execution does not check `needs_approval` or verify an approval signature. The legacy instruction retains but ignores its approval argument and instructions-sysvar account; V2 omits both from its interface.
- The permissionless authority is an intermediary signer. V2 permissionless fees route to `PermissionlessOfferFee`; net token-in otherwise follows the same burn, refill, and proceeds rules as `take_offer_v2`.
- Refill is skipped if the supplied redemption offer account is uninitialized or disabled, the target is zero, `MarketStats` cannot be read, or token-in is program-controlled.

## Redemption Offer Creation

```mermaid
flowchart TD
    Signer[Boss] --> Validate[Validate linked offer]
    Validate --> VaultIn[Create redemption token in ATA]
    Validate --> VaultOut[Create redemption token out ATA]
    VaultIn --> Create[Create RedemptionOffer PDA]
    VaultOut --> Create
    Create --> FeeCheck[Require fee within program cap]
    FeeCheck --> FeeConfig[Set redemption fee bps from input]
    Create --> PropAmmSellFeeConfig[Set Prop AMM sell fee bps to zero]
    Create --> TargetConfig[Set vault target bps to zero]
    Create --> Counters[Set requested and executed redemptions to zero]
    Create --> Enabled[Set disabled flag to false]
```

Notes:

- The redemption market is the ONYC-to-asset side linked to the offer.
- Initial `vault_target_bps` is zero, so new redemption markets do not automatically receive refill inflow until configured.
- Redemption offer creation rejects fee values above the program fee cap.
- `fee_basis_points` is the worker fulfillment fee. `fee_basis_points_prop_amm_sell` is the Prop AMM sell redemption fee set at redemption offer creation and later adjustable by the boss through `update_redemption_offer_prop_amm_sell_fee`.

## Redemption Request Creation

```mermaid
flowchart TD
    Redeemer[User token in] --> Lock[Transfer token in to redemption vault]
    Lock --> Request[Create RedemptionRequest PDA]
    RequestId[Frontend-generated request ID] --> Request
    Request --> Amount[Store requested amount]
    Request --> Requested[Increase requested redemptions]
```

Notes:

- The request locks token-in in the redemption vault.
- Its PDA uses the redemption offer, redeemer, and an exactly 32-byte frontend-generated request ID (a UUID without hyphens); the backend/frontend retain it for later lookup.
- Fulfillment can be partial; the request tracks fulfilled amount.

## Redemption Fulfillment

```mermaid
flowchart TD
    Request[Existing redemption request] --> Calc[Calculate redemption quote]
    Calc --> Fee[Token in fee]
    Calc --> Net[Token in net]
    Calc --> Out[Token out amount]

    Fee --> RedemptionFee[RedemptionFee vault]

    Net --> InMintMode{Program controls token in mint}
    InMintMode -- yes --> Burn[Burn net token in from redemption vault]
    InMintMode -- no --> OfferProceeds[OfferProceeds vault]

    Out --> Pay[Transfer token out from redemption vault to redeemer]

    Calc --> RequestUpdate[Increase fulfilled amount]
    RequestUpdate --> OfferCounters[Update requested and executed redemptions]
```

Notes:

- Fees are paid in token-in from the locked redemption-vault balance and route to `RedemptionFee`.
- If token-in is program-controlled, net token-in is burned.
- If token-in is not program-controlled, net token-in moves to `OfferProceeds`.
- Token-out is paid from pre-funded redemption-vault liquidity; redemption fulfillment does not mint token-out.
- If the fulfillment completes the request, the request account closes.

## Redemption Request Cancellation

```mermaid
flowchart TD
    Request[Existing redemption request] --> Remaining[Calculate unfulfilled amount]
    Remaining --> Return[Transfer remaining token in from redemption vault to redeemer]
    Return --> Requested[Decrease requested redemptions]
    Requested --> Close[Close RedemptionRequest PDA]
```

Notes:

- The redeemer, worker, or boss can cancel.
- Only the unfulfilled remainder is returned.

## Prop AMM Buy

```mermaid
flowchart TD
    Calc[Calculate offer quote] --> Fee[Fee amount]
    UserIn[User asset input] --> IntermediaryIn[Permissionless token in ATA]
    Calc --> Net[Net asset inflow]
    Calc --> Out[ONYC output]

    Fee --> AmmFee[PropAmmBuyFee vault]

    Net --> Split[Calculate refill and proceeds]
    Split --> Refill[Refill amount]
    Split --> Proceeds[Proceeds amount]
    Refill --> RedemptionVault[Redemption vault asset ATA]
    Proceeds --> AmmProceeds[PropAmmProceeds vault]

    Net --> RecordBuy[Record Prop AMM buy relief]

    Out --> OutMintMode{Program controls token out mint}
    OutMintMode -- yes --> Mint[Mint token out to user]
    OutMintMode -- no --> VaultToIntermediary[Transfer token out from offer vault to permissionless ATA]
    VaultToIntermediary --> UserOut[Transfer token out to user]
```

Notes:

- Prop AMM buy uses the offer's `fee_basis_points_permissionless` for fee calculation and routes that fee to `PropAmmBuyFee`. Net inflow routes to `PropAmmProceeds` or redemption-vault refill.
- Prop AMM buy execution requires the offer's permissionless mode to be enabled. A quote can exist even when execution would reject this gate.
- Buy pressure relief records the full net asset inflow, independent of how much was refilled.
- Refill is capped by the redemption market target. Overflow goes to `PropAmmProceeds`.

## Prop AMM Sell

```mermaid
flowchart TD
    Calc[Calculate redemption quote]
    Calc --> RawOut[Raw token out amount]
    RawOut --> Wall[Apply hard wall liquidity curve]
    Wall --> FinalOut[Final token out amount]
    RawOut --> RecordSell[Record raw sell pressure]
    UserIn[User token in] --> Stage[Stage token in in redemption vault]

    Calc --> Fee[Token in fee]
    Calc --> Net[Token in net]

    Fee --> AmmSellFee[PropAmmSellFee vault]

    Net --> InMintMode{Program controls token in mint}
    InMintMode -- yes --> Burn[Burn net token in from redemption vault]
    InMintMode -- no --> AmmProceeds[PropAmmProceeds vault]

    FinalOut --> Pay[Transfer token out from redemption vault to user]
```

Notes:

- Prop AMM sell uses `RedemptionOffer.fee_basis_points_prop_amm_sell` for its percentage redemption fee and routes fees to `PropAmmSellFee`. Net token-in routes to `PropAmmProceeds` or burns, depending on mint authority.
- Sell execution calculates the quote and checks `minimum_out` before staging user token-in in the redemption vault. If the program controls the token-in mint, the net amount is burned from the redemption vault; otherwise it routes to `PropAmmProceeds`.
- Sell execution pays token-out from pre-funded redemption-vault liquidity; it does not mint token-out.
- Sell execution refreshes `MarketStats` before resolving the hard-wall reserve. Quote and execution read TVL for hard-wall reserve only when `vault_target_bps > 0`; when `vault_target_bps == 0`, the hard-wall reserve resolves to the actual vault balance without reading TVL.
- If sell execution later accrues BUFFER before burning ONYC, the hard-wall reserve is not recomputed again after that accrual.
- The hard wall is always bounded by the actual redemption vault token-out balance.
- When `vault_target_bps > 0`, the hard-wall reserve is additionally capped by `TVL * vault_target_bps / 10_000`, converted into token-out decimals. In code: `hard_wall_reserve = min(actual_vault_balance, target_from_tvl)`.
- When `vault_target_bps == 0`, the hard-wall reserve is the actual redemption vault token-out balance.
- Sell pressure records the raw pair-asset output before the hard-wall liquidity factor, not the final paid amount.
