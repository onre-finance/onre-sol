# Prop AMM Pricing Model

This document explains the current Prop AMM pricing path implemented in:

- `programs/onreapp/src/instructions/prop_amm/quote.rs`
- `programs/onreapp/src/instructions/prop_amm/sell.rs`
- `programs/onreapp/src/instructions/prop_amm/buy.rs`

The important distinction is that buys and sells use different mechanics:

- Buy: normal offer pricing is used. The only Prop AMM change is where incoming stablecoins are routed.
- Sell: normal redemption pricing is used first, then the hard-wall reserve curve converts the raw sell value into the actual stablecoin output.

## Variables

| Symbol | Code variable | Meaning |
| --- | --- | --- |
| `TVL` | `market_stats.tvl` | Protocol TVL from the market stats PDA, stored in ONYC/base precision. |
| `target_bps` | `prop_amm_state.pool_target_bps` | Target redemption liquidity as basis points of TVL. Default is `1500`, or 15%. |
| `R` | `hard_wall_reserve` | TVL-derived hard-wall reserve target, converted into the output token decimals. |
| `L` | `actual_liquidity` | Current redemption vault balance for the stablecoin being paid out. |
| `raw` | `result.token_out_amount` before hard wall | Stablecoin output from normal redemption pricing after redemption fee. |
| `out` | final `result.token_out_amount` | Actual stablecoin output transferred to the seller. |
| `w` | `linear_weight_bps / 10_000` | Linear penalty weight. Default config currently uses `2000`, or 20%. |
| `e` | `base_exponent` | Nonlinear curve exponent. Default config currently uses `3`. |
| `S` | `HARD_WALL_SCALE` | Fixed-point scale, currently `1_000_000_000_000`. |

All on-chain math is integer fixed-point math. In formulas below, values are shown as real numbers for readability.

## Buy Flow

The buy quote is unchanged:

```text
stablecoin input -> process_offer_core(...) -> ONYC output
```

The buy execution does change the routing of incoming stablecoins.

### Step 1: Calculate Target Redemption Liquidity

The target is based on TVL:

```text
target_in_onyc_decimals = TVL * target_bps / 10_000
```

Then the target is converted from ONYC decimals into the stablecoin input mint decimals:

```text
target_liquidity = target_in_onyc_decimals
                 * 10^stablecoin_decimals
                 / 10^onyc_decimals
```

Example:

```text
TVL = 1,000 ONYC
target_bps = 1,500
ONYC decimals = 9
USDC decimals = 6

target_in_onyc_decimals = 1,000e9 * 1,500 / 10,000
                         = 150e9

target_liquidity = 150e9 * 10^6 / 10^9
                 = 150e6 USDC base units
                 = 150 USDC
```

### Step 2: Refill Redemption Vault First

If the redemption vault is below target:

```text
deficit = target_liquidity - current_redemption_vault_balance
refill_amount = min(deficit, buy_net_stablecoin_amount)
```

Any remaining net stablecoin goes to the boss/treasury:

```text
boss_net_amount = buy_net_stablecoin_amount - refill_amount
```

So buys self-heal the redemption vault until the TVL-based hard-wall target is reached.

## Sell Flow Overview

Sell execution has three conceptual stages.

### Step 1: Redemption Fee

The sell first runs normal redemption pricing:

```text
raw = process_redemption_core(
  offer,
  token_in_amount,
  token_in_mint,
  token_out_mint,
  redemption_fee_bps
).token_out_amount
```

The redemption fee comes from the redemption offer if initialized. If the redemption offer PDA is the correct derived address but uninitialized, the fee is treated as zero.

Conceptually:

```text
token_in_net = token_in_amount * (1 - redemption_fee_bps / 10_000)
raw = token_in_net converted into stablecoin output using the offer price
```

Example:

```text
User sells: 1 ONYC
Redemption fee: 500 bps = 5%
NAV/offer price: 1 ONYC = 1 USDC

token_in_net = 1.00 * (1 - 0.05)
             = 0.95 ONYC

raw = 0.95 USDC
```

This `raw` value is not necessarily what the user receives. It is the amount that is passed into the current endpoint dampening curve.

### Step 2: Calculate the Hard-Wall Reserve

The sell path reads `market_stats.tvl` and calculates the hard-wall reserve:

```text
R = TVL * target_bps / 10_000
```

Then it converts from ONYC decimals to the output stablecoin decimals:

```text
R = R_onyc_decimals * 10^token_out_decimals / 10^onyc_decimals
```

Example:

```text
TVL = 66,666.666666666 ONYC
target_bps = 1,500

R = 66,666.666666666 * 0.15
  = 10,000 USDC
```

Here `R = 10,000 USDC` means the curve treats `10,000 USDC` as the full hard-wall reserve target.

### Step 3: Apply Endpoint Dampening

This is the core hard-wall logic:

```text
effective_liquidity = min(L, R)
u = raw / effective_liquidity
penalty = w * u + (1 - w) * u^e
liquidity_factor = 1 - penalty
out = raw * liquidity_factor
```

The implementation rejects the sell if:

```text
L == 0
R == 0
raw >= min(L, R)
out == 0
```

This is an endpoint formula. It dampens large sells, but it is not split-resistant. A user can split a sell into smaller orders and receive more total output than one large order with the same total raw sell value. This is an accepted interim property until a separate anti-splitting mechanism is implemented.

## Effective Liquidity

The curve does not price surplus liquidity above the target reserve more cheaply. It clips the denominator to the smaller of actual liquidity and the TVL-derived hard-wall reserve:

```text
effective_liquidity = min(actual_liquidity, hard_wall_reserve)
```

So if the redemption vault contains more than the target reserve, the curve still behaves as if the vault contained exactly the target reserve:

```text
L = 20,000
R = 10,000
effective_liquidity = 10,000
```

If the redemption vault is below target, the denominator shrinks:

```text
L = 5,000
R = 10,000
effective_liquidity = 5,000
```

This makes sells more expensive when the vault is already below the target reserve.

## Utilization

Utilization is based on the current order's raw sell value and the effective liquidity:

```text
u = raw / effective_liquidity
```

So:

```text
u = 0.10 means this order consumes 10% of effective liquidity before dampening
u = 0.50 means this order consumes 50% of effective liquidity before dampening
u approaches 1.00 near the maximum allowed order size
```

## Penalty Function

The penalty is a blend of a linear component and a nonlinear component:

```text
penalty(u) = w * u + (1 - w) * u^e
```

Where:

```text
w = linear_weight_bps / 10_000
e = base_exponent
```

With current defaults:

```text
w = 2,000 / 10,000 = 0.20
e = 3
```

So:

```text
penalty(u) = 0.20u + 0.80u^3
```

Examples:

```text
u = 0.10
penalty = 0.20 * 0.10 + 0.80 * 0.10^3
        = 0.0200 + 0.0008
        = 0.0208

u = 0.50
penalty = 0.20 * 0.50 + 0.80 * 0.50^3
        = 0.1000 + 0.1000
        = 0.2000

u = 0.90
penalty = 0.20 * 0.90 + 0.80 * 0.90^3
        = 0.1800 + 0.5832
        = 0.7632
```

The penalty grows slowly at first, then much faster near the wall. The final liquidity factor is:

```text
liquidity_factor = 1 - penalty(u)
out = raw * liquidity_factor
```

## Full Sell Example

Assume:

```text
TVL = 66,666.666666666 ONYC
target_bps = 1,500
R = 10,000 USDC
L = 10,000 USDC
redemption_fee_bps = 500
user sells enough ONYC that the normal redemption output before fee would be 5,000 USDC
```

### 1. Apply Redemption Fee

```text
raw = 5,000 * (1 - 500 / 10,000)
    = 5,000 * 0.95
    = 4,750 USDC
```

The user has `4,750 USDC` of raw redemption value to pass through the dampening curve.

### 2. Apply Dampening

The vault is at the target reserve, so:

```text
effective_liquidity = min(10,000, 10,000)
                    = 10,000

u = 4,750 / 10,000
  = 0.475
```

With the default curve:

```text
penalty = 0.20 * 0.475 + 0.80 * 0.475^3
        = 0.095 + 0.0857375
        = 0.1807375

liquidity_factor = 1 - 0.1807375
                 = 0.8192625

out = 4,750 * 0.8192625
    ~= 3,891 USDC
```

So the user expected about `5,000 USDC` before fee, has `4,750 USDC` raw value after fee, and actually receives about `3,891 USDC` after hard-wall dampening.

### 3. Vault State Updates

The vault loses the actual output:

```text
vault_after = 10,000 - 3,891
            = 6,109 USDC
```

The next seller starts from the new lower vault balance.

## Split-Order Behavior

Suppose a user tries:

```text
One sell with raw budget 5,000
```

versus:

```text
Five sells with raw budget 1,000 each
```

The current model prices each order independently:

```text
out = raw * f(raw / min(current_vault, hard_wall_reserve))
```

That means each smaller order receives a smaller utilization and therefore a smaller penalty. Although the vault balance is updated between orders, the endpoint formula is still vulnerable to splitting.

The test `test_hard_wall_curve_is_vulnerable_to_order_splitting` documents this current behavior.

## Reading The Graph

The generated CSV/SVG comes from:

```text
programs/onreapp/examples/hard_wall_curve.rs
```

It calls the same Rust function used on-chain:

```text
apply_hard_wall_reserve_curve_with_params(...)
```

Current example command:

```bash
cargo run -p onreapp --example hard_wall_curve -- \
  --out target/hard_wall_curve_reserve_curve.csv \
  --liquidity 10000000 \
  --hard-wall-reserve 10000000 \
  --points 201 \
  --linear-weight-bps 2000 \
  --base-exponent 3 \
  --redemption-fee-bps 500
```

Graph interpretation:

- X axis: raw sell size as a percentage of the hard-wall reserve `R`.
- Red line: actual payout divided by raw pre-fee sell size.
- Blue dashed line: actual payout divided by fee-adjusted raw sell value.
- Gray line: raw output.
- Green line: actual output.

Current sample with 5% redemption fee:

```text
~1% raw sell  -> 94.82% effective payout
~50% raw sell -> 77.83% effective payout
~90% raw sell -> 31.25% effective payout
~99% raw sell -> 13.91% effective payout
```

The red line includes both the redemption fee and the hard-wall cost.
