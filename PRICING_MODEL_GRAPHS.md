# Pricing Model Graphs

## Current Model

```mermaid
graph TD
    State[State]
    State -->|onyc_mint| ONYC[ONYC Mint]
    State -->|main_offer| MainOffer[Offer PDA]

    OfferUSDC[Offer PDA: USDC -> ONYC]
    OfferUSDG[Offer PDA: USDG -> ONYC]

    OfferUSDC -->|stores| VecUSDC[Pricing vectors]
    OfferUSDC -->|stores| FeeUSDC[buy fee_bps]
    OfferUSDC -->|stores| ApprovalUSDC[approval flags]

    OfferUSDG -->|stores| VecUSDG[Pricing vectors]
    OfferUSDG -->|stores| FeeUSDG[buy fee_bps]
    OfferUSDG -->|stores| ApprovalUSDG[approval flags]

    RedUSDC[RedemptionOffer PDA: ONYC -> USDC]
    RedUSDG[RedemptionOffer PDA: ONYC -> USDG]

    RedUSDC -->|references| OfferUSDC
    RedUSDC -->|stores| RedFeeUSDC[redemption fee_bps]
    RedUSDC -->|stores| RedStatsUSDC[request/executed counters]

    RedUSDG -->|references| OfferUSDG
    RedUSDG -->|stores| RedFeeUSDG[redemption fee_bps]
    RedUSDG -->|stores| RedStatsUSDG[request/executed counters]
```

### Problems

- USDC and USDG can share the same price curve, but pricing vectors are duplicated.
- Updating pricing requires updating multiple offers.
- Route-specific fee and shared pricing are stored in the same object.
- Redemption pricing is modeled through `RedemptionOffer`, even when conceptually it is just sell pricing for a token.
- `open_swap` and `quote_swap` are forced to work through legacy `Offer` semantics.

## Proposed Model

```mermaid
graph TD
    State[State]
    State -->|onyc_mint| ONYC[ONYC Mint]

    BuyPricingUSD[BuyPricing PDA: USD pricing]
    BuyPricingUSD --> BuyVectors[pricing vectors]

    BuyRouteUSDC[BuyRoute PDA: USDC]
    BuyRouteUSDG[BuyRoute PDA: USDG]

    BuyRouteUSDC -->|uses| BuyPricingUSD
    BuyRouteUSDC -->|stores| BuyFeeUSDC[buy fee_bps]
    BuyRouteUSDC -->|stores| BuyFlagsUSDC[enabled / approval / route config]

    BuyRouteUSDG -->|uses| BuyPricingUSD
    BuyRouteUSDG -->|stores| BuyFeeUSDG[buy fee_bps]
    BuyRouteUSDG -->|stores| BuyFlagsUSDG[enabled / approval / route config]

    RedPricingUSD[RedemptionPricing PDA: USD redemption pricing]
    RedPricingUSD --> RedVectors[redemption pricing vectors or rules]

    RedRouteUSDC[RedemptionRoute PDA: USDC]
    RedRouteUSDG[RedemptionRoute PDA: USDG]

    RedRouteUSDC -->|uses| RedPricingUSD
    RedRouteUSDC -->|stores| RedFeeUSDC[redemption fee_bps]
    RedRouteUSDC -->|stores| RedCfgUSDC[vault / destination / route config]

    RedRouteUSDG -->|uses| RedPricingUSD
    RedRouteUSDG -->|stores| RedFeeUSDG[redemption fee_bps]
    RedRouteUSDG -->|stores| RedCfgUSDG[vault / destination / route config]
```

## Quote / Swap Resolution Flow

```mermaid
graph TD
    Quote[quote_swap / open_swap]
    Quote --> Pair[input_mint, output_mint]
    Pair --> ResolveRoute[resolve route]
    ResolveRoute --> Route[BuyRoute or RedemptionRoute]
    Route --> Pricing[shared Pricing PDA]
    Route --> Fee[route fee_bps]
    Route --> Config[route settlement config]
    Pricing --> BaseQuote[base amount from vectors]
    Fee --> FinalQuote[minimum_out / output amount]
    BaseQuote --> FinalQuote
```

## Why This Is Better

- Shared pricing is updated once.
- Fees can still differ per asset pair.
- New assets can be added by creating routes instead of duplicating full offers.
- `quote_swap` becomes: resolve route, load pricing, compute base amount, apply route fee.
- `open_swap` can use the same route resolution model and then execute settlement based on route config.
