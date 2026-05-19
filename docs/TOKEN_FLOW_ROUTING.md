# Token Flow Routing

This page summarizes token routing after the redemption vault target changes.

Key rule: net stable inflow from `take_offer_v2`, `take_offer_permissionless_v2`, and Prop AMM buy refills the redemption vault only when the `RedemptionOffer` exists and `vault_target_bps > 0`. `make_redemption_offer` initializes `vault_target_bps = 0`, so the default behavior sends all net stable inflow to proceeds.

The refill cap is:

```text
target = TVL * vault_target_bps / 10_000
refill = min(net_stable_inflow, max(0, target - current_redemption_vault_balance))
overflow = net_stable_inflow - refill
```

Fees are never part of the refill calculation. They route to the corresponding fee vault.

## `take_offer_v2`

```mermaid
flowchart TD
    UserStable[User token_in stable] --> Core[process_offer_core]
    Core --> Fee[fee amount]
    Core --> Net[net stable inflow]
    Fee --> OfferFee[OfferFee vault]

    Net --> Check{RedemptionOffer exists<br/>and vault_target_bps > 0?}
    Check -- no --> OfferProceeds[OfferProceeds vault]
    Check -- yes --> Cap[cap refill by TVL target headroom]
    Cap --> Refill[Redemption vault stable ATA]
    Cap --> Overflow[overflow]
    Overflow --> OfferProceeds

    Core --> Out{Program controls token_out mint?}
    Out -- yes --> Mint[Mint token_out to user]
    Out -- no --> VaultOut[Transfer token_out from offer vault to user]
```

## `take_offer_permissionless_v2`

```mermaid
flowchart TD
    UserStable[User token_in stable] --> Intermediary[Permissionless token_in ATA]
    Intermediary --> Core[process_offer_core]
    Core --> Fee[fee amount]
    Core --> Net[net stable inflow]
    Fee --> OfferFee[OfferFee vault]

    Net --> Check{RedemptionOffer exists<br/>and vault_target_bps > 0?}
    Check -- no --> OfferProceeds[OfferProceeds vault]
    Check -- yes --> Cap[cap refill by TVL target headroom]
    Cap --> Refill[Redemption vault stable ATA]
    Cap --> Overflow[overflow]
    Overflow --> OfferProceeds

    Core --> Out{Program controls token_out mint?}
    Out -- yes --> Mint[Mint token_out to permissionless ATA]
    Out -- no --> VaultOut[Transfer token_out from offer vault<br/>to permissionless ATA]
    Mint --> UserOut[Transfer token_out to user]
    VaultOut --> UserOut
```

## Redemption Offer Creation

```mermaid
flowchart TD
    Signer[Boss or redemption_admin] --> Make[make_redemption_offer]
    Make --> Offer[Validate Redemption Offer PDA]
    Make --> VaultIn[Create redemption vault token_in ATA if needed]
    Make --> VaultOut[Create redemption vault token_out ATA if needed]
    Make --> RedOffer[Create RedemptionOffer PDA]
    RedOffer --> Fee[fee_basis_points = caller input]
    RedOffer --> Target[vault_target_bps = 0]
    RedOffer --> Counters[executed/requested redemptions = 0]
```

## Redemption Fulfillment

```mermaid
flowchart TD
    Redeemer[Previously locked token_in<br/>in redemption vault] --> Core[process_redemption_core]
    Core --> Fee[token_in fee]
    Core --> Net[token_in net]
    Fee --> OfferFee[OfferFee vault]

    Net --> InMode{Program controls token_in mint?}
    InMode -- yes --> Burn[Burn net token_in from redemption vault]
    InMode -- no --> OfferProceeds[OfferProceeds vault]

    Core --> Out[token_out amount]
    Out --> OutMode{Program controls token_out mint?}
    OutMode -- yes --> Mint[Mint token_out to redeemer]
    OutMode -- no --> Pay[Transfer token_out from redemption vault to redeemer]

    Core --> Request[Update request fulfilled_amount]
    Request --> Counters[Update RedemptionOffer counters]
```

## Prop AMM Buy

```mermaid
flowchart TD
    UserStable[User stable input] --> Intermediary[Permissionless token_in ATA]
    Intermediary --> Core[process_offer_core]
    Core --> Fee[fee amount]
    Core --> Net[net stable inflow]
    Fee --> AmmFee[PropAmmFee vault]

    Net --> Check{RedemptionOffer exists<br/>and vault_target_bps > 0?}
    Check -- no --> AmmProceeds[PropAmmProceeds vault]
    Check -- yes --> Cap[cap refill by TVL target headroom]
    Cap --> Refill[Redemption vault stable ATA]
    Cap --> Overflow[overflow]
    Overflow --> AmmProceeds

    Net --> RecordBuy[Record Prop AMM buy relief]
    Core --> Out{Program controls token_out mint?}
    Out -- yes --> Mint[Mint token_out to user]
    Out -- no --> Transfer[Transfer token_out from offer vault to user]
```

## Prop AMM Sell

```mermaid
flowchart TD
    UserIn[User token_in] --> Lock[Transfer token_in to redemption vault]
    Lock --> Core[process_redemption_core]
    Core --> Raw[raw stable output]
    Raw --> Wall[Apply hard-wall liquidity curve]
    Wall --> Out[final token_out amount]
    Raw --> RecordSell[Record Prop AMM sell pressure]

    Core --> Fee[token_in fee]
    Core --> Net[token_in net]
    Fee --> AmmFee[PropAmmFee vault]

    Net --> InMode{Program controls token_in mint?}
    InMode -- yes --> Burn[Burn net token_in from redemption vault]
    InMode -- no --> AmmProceeds[PropAmmProceeds vault]

    Out --> OutMode{Program controls token_out mint?}
    OutMode -- yes --> Mint[Mint token_out to user]
    OutMode -- no --> Pay[Transfer token_out from redemption vault to user]
```
