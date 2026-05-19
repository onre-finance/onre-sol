# Instruction Token Flows

This document shows one end-to-end routing flow per instruction. Each flow starts with calculation, then decisions, then token splits and final updates.

`vault_target_bps` lives on `RedemptionOffer`. It defaults to `0`, which means stable inflow does not refill the redemption vault and instead goes fully to proceeds.

## `take_offer_v2`

```mermaid
flowchart TD
    Start["take_offer_v2"] --> Calc["Calculate offer price, token_out amount, token_in fee, and token_in net"]
    Calc --> Target{"RedemptionOffer for ONYC to stable exists and vault_target_bps greater than zero?"}
    Target -- "no" --> NoRefill["refill is zero"]
    Target -- "yes" --> Cap["Calculate target as TVL times vault_target_bps divided by 10_000"]
    Cap --> Split["Refill is capped by token_in net and redemption vault headroom"]
    NoRefill --> Split
    Split --> Fee["Send token_in fee to OfferFee vault"]
    Split --> Refill["Send refill amount to redemption vault stable ATA"]
    Split --> Proceeds["Send token_in net minus refill to OfferProceeds vault"]
    Split --> OutDecision{"Program controls token_out mint?"}
    OutDecision -- "yes" --> MintOut["Mint token_out to user"]
    OutDecision -- "no" --> TransferOut["Transfer token_out from offer vault to user"]
    MintOut --> Market["Refresh market stats when token_out is ONYC"]
    TransferOut --> Market
```

## `take_offer_permissionless_v2`

```mermaid
flowchart TD
    Start["take_offer_permissionless_v2"] --> Collect["Transfer full token_in amount from user to permissionless ATA"]
    Collect --> Calc["Calculate offer price, token_out amount, token_in fee, and token_in net"]
    Calc --> Target{"RedemptionOffer for ONYC to stable exists and vault_target_bps greater than zero?"}
    Target -- "no" --> NoRefill["refill is zero"]
    Target -- "yes" --> Cap["Calculate target as TVL times vault_target_bps divided by 10_000"]
    Cap --> Split["Refill is capped by token_in net and redemption vault headroom"]
    NoRefill --> Split
    Split --> Fee["Send token_in fee from permissionless ATA to OfferFee vault"]
    Split --> Refill["Send refill amount from permissionless ATA to redemption vault stable ATA"]
    Split --> Proceeds["Send token_in net minus refill to OfferProceeds vault"]
    Split --> OutDecision{"Program controls token_out mint?"}
    OutDecision -- "yes" --> MintIntermediary["Mint token_out to permissionless ATA"]
    OutDecision -- "no" --> TransferIntermediary["Transfer token_out from offer vault to permissionless ATA"]
    MintIntermediary --> UserOut["Transfer token_out from permissionless ATA to user"]
    TransferIntermediary --> UserOut
    UserOut --> Market["Refresh market stats when token_out is ONYC"]
```

## `make_redemption_offer`

```mermaid
flowchart TD
    Start["make_redemption_offer"] --> Calc["Validate fee and derive redemption offer for token_in to token_out"]
    Calc --> Auth{"Signer is boss or redemption_admin?"}
    Auth -- "no" --> Reject["Reject"]
    Auth -- "yes" --> Create["Create RedemptionOffer PDA"]
    Create --> VaultIn["Create redemption vault token_in ATA if needed"]
    Create --> VaultOut["Create redemption vault token_out ATA if needed"]
    Create --> Fields["Set offer, mints, fee_basis_points, counters, and bump"]
    Fields --> Target["vault_target_bps starts at zero"]
```

## `create_redemption_request`

```mermaid
flowchart TD
    Start["create_redemption_request"] --> Calc["Validate redemption offer and current request counter"]
    Calc --> Lock["Transfer requested token_in from redeemer to redemption vault token_in ATA"]
    Lock --> Request["Create RedemptionRequest PDA"]
    Request --> Fields["Store redeemer, amount, request_id, fulfilled_amount starts at zero, and bump"]
    Fields --> Counters["Increment RedemptionOffer requested_redemptions and request_counter"]
```

## `fulfill_redemption_request`

```mermaid
flowchart TD
    Start["fulfill_redemption_request"] --> Calc["Validate remaining request amount and calculate redemption price, token_in fee, token_in net, and token_out amount"]
    Calc --> Accrue{"token_in is ONYC and program controls token_in mint?"}
    Accrue -- "yes" --> Buffer["Accrue BUFFER before burn"]
    Accrue -- "no" --> Split["Split locked token_in from redemption vault"]
    Buffer --> Split
    Split --> InDecision{"Program controls token_in mint?"}
    InDecision -- "yes" --> Burn["Burn token_in net from redemption vault"]
    InDecision -- "no" --> Proceeds["Send token_in net to OfferProceeds vault"]
    Split --> Fee["Send token_in fee to OfferFee vault"]
    Split --> OutDecision{"Program controls token_out mint?"}
    OutDecision -- "yes" --> MintOut["Mint token_out to redeemer"]
    OutDecision -- "no" --> TransferOut["Transfer token_out from redemption vault to redeemer"]
    Burn --> Update["Update request fulfilled_amount"]
    Proceeds --> Update
    Fee --> Update
    MintOut --> Update
    TransferOut --> Update
    Update --> OfferStats["Update RedemptionOffer executed and requested redemption counters"]
    OfferStats --> Market["Refresh market stats when token_in is ONYC and program controls token_in mint"]
```

## `open_swap_buy`

```mermaid
flowchart TD
    Start["open_swap_buy"] --> Calc["Validate canonical buy pair and calculate offer price, token_out amount, token_in fee, and token_in net"]
    Calc --> Collect["Transfer full token_in amount from user to permissionless ATA"]
    Collect --> Target{"RedemptionOffer for ONYC to stable exists and vault_target_bps greater than zero?"}
    Target -- "no" --> NoRefill["refill is zero"]
    Target -- "yes" --> Cap["Calculate target as TVL times vault_target_bps divided by 10_000"]
    Cap --> Split["Refill is capped by token_in net and redemption vault headroom"]
    NoRefill --> Split
    Split --> Fee["Send token_in fee to PropAmmFee vault"]
    Split --> Refill["Send refill amount to redemption vault stable ATA"]
    Split --> Proceeds["Send token_in net minus refill to PropAmmProceeds vault"]
    Split --> Record["Record Prop AMM buy relief"]
    Record --> OutDecision{"Program controls token_out mint?"}
    OutDecision -- "yes" --> MintOut["Mint token_out to user"]
    OutDecision -- "no" --> TransferOut["Transfer token_out from offer vault to user"]
    MintOut --> Market["Refresh market stats when token_out is ONYC"]
    TransferOut --> Market
```

## `open_swap_sell`

```mermaid
flowchart TD
    Start["open_swap_sell"] --> Calc["Validate canonical sell pair and calculate redemption price, token_in fee, token_in net, and raw stable output"]
    Calc --> Wall["Apply hard-wall liquidity curve to raw stable output"]
    Wall --> Record["Record Prop AMM sell pressure using raw stable output"]
    Record --> Collect["Transfer full token_in amount from user to redemption vault token_in ATA"]
    Collect --> Split["Split token_in held by redemption vault"]
    Split --> InDecision{"Program controls token_in mint?"}
    InDecision -- "yes" --> Burn["Burn token_in net from redemption vault"]
    InDecision -- "no" --> Proceeds["Send token_in net to PropAmmProceeds vault"]
    Split --> Fee["Send token_in fee to PropAmmFee vault"]
    Split --> OutDecision{"Program controls token_out mint?"}
    OutDecision -- "yes" --> MintOut["Mint final token_out to user"]
    OutDecision -- "no" --> TransferOut["Transfer final token_out from redemption vault to user"]
    Burn --> Market["Refresh market stats when token_in is ONYC and program controls token_in mint"]
    Proceeds --> Market
    Fee --> Market
    MintOut --> Market
    TransferOut --> Market
```
