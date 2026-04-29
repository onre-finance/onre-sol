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
| `V` | effective sell volume | Previous epoch pressure after decay plus current net sell pressure plus this sell's raw value. |
| `W` | dynamic wall position | Vault-based effective pool size after applying net sell pressure. |
| `raw` | `result.token_out_amount` before hard wall | Stablecoin output from normal redemption pricing after redemption fee. |
| `out` | final `result.token_out_amount` | Actual stablecoin output transferred to the seller. |
| `h_min` | `min_liquidation_haircut_bps / 10_000` | Flat sell-side haircut. Default is `50`, or 0.5%. |
| `h_peg` | `curve_peg_haircut_bps / 10_000` | Additional haircut when utilization is exactly 1. Default is `700`, or 7%. |
| `e` | `curve_exponent_scaled / 10_000` | Haircut curve exponent. Default is `25_000`, or 2.5. |
| `S` | `HARD_WALL_SCALE` | Fixed-point scale, currently `1_000_000_000_000`. |

All on-chain math is integer fixed-point math. In formulas below, values are shown as real numbers for readability.

ONYC always uses 9 decimals and supported stablecoins use 6 decimals. The dynamic wall tracker stores pressure in stablecoin base units, not ONYC base units:

```text
1 ONYC at NAV = 1.00 stable
ONYC amount = 1_000_000_000
stable value = 1_000_000
```

So:

- selling ONYC records the raw stablecoin redemption value before dampening
- buying ONYC records the net stablecoin input used to buy ONYC
- both sides are comparable because both are stored in stablecoin base units

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

So buys self-heal the redemption vault until the TVL-based hard-wall target is reached. The buy net stablecoin amount is also recorded as current-epoch buy relief. It can reduce net sell pressure, but it cannot create negative pressure.

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

### Step 2: Calculate Net Sell Pressure

Prop AMM tracks sell pressure across the current and previous epoch:

```text
curr_net = max(0, curr_sell_value_stable - curr_buy_value_stable)
effective_volume = decayed_prev_net + curr_net + raw
```

The current sell's own `raw` stable value is included in `effective_volume`, so the order pays against the pressure it creates. ONYC buy pressure relief is the stablecoin net input paid by the buyer, not the 9-decimal ONYC amount minted.

If an epoch rolls over, current net sell pressure moves into `prev_net_sell_value_stable`; if two or more epochs elapsed, pressure resets to zero. Previous pressure decays linearly through the next epoch.

### Step 3: Calculate the Dynamic Wall

The dynamic wall is based on the actual redemption vault balance and effective sell pressure:

```text
W = L / (1 + wall_sensitivity * (effective_volume / L))
```

With the default `wall_sensitivity_scaled = 20_000`, `wall_sensitivity = 2.0`.

### Step 4: Calculate the TVL Hard-Wall Reserve

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

Here `R = 10,000 USDC` means the fixed hard-wall reserve target is `10,000 USDC`. When dynamic wall sensitivity is enabled, the dynamic wall `W` is the denominator used by the sell curve. The fixed reserve still guards the target calculation and preserves the disabled-sensitivity fallback behavior.

### Step 5: Apply Endpoint Dampening

This is the core hard-wall logic:

```text
effective_liquidity = W
u = raw / effective_liquidity
haircut = h_min + h_peg * u^e
liquidity_factor = 1 - haircut
out = raw * liquidity_factor
```

The implementation rejects the sell if:

```text
L == 0
R == 0
raw > L
```

The dynamic wall is a pricing input, not a hard execution cap. If `raw` is at or below the actual vault balance, the order can execute even when `raw` is greater than the pressure-adjusted wall. In that case the haircut can reach or exceed `1`, the liquidity factor saturates at `0`, and `out` becomes `0`. The sell still has to satisfy `minimum_out`, so a zero-output sell only succeeds when the caller explicitly allows `minimum_out = 0`.

This is still an endpoint formula, so it is not mathematically split-proof. The dynamic wall makes splitting materially worse because every sell adds global pressure and shrinks the wall for subsequent sells.

## Effective Liquidity

With dynamic wall sensitivity enabled, effective liquidity is the pressure-adjusted wall:

```text
effective_liquidity = dynamic_wall_position
```

When sensitivity is disabled, the legacy fixed hard-wall behavior is:

```text
effective_liquidity = min(actual_liquidity, hard_wall_reserve)
```

Dynamic wall examples with `L = 10,000` and sensitivity `2.0`:

```text
effective_volume = 0      -> W = 10,000
effective_volume = 2,500  -> W = 6,666
effective_volume = 5,000  -> W = 5,000
effective_volume = 10,000 -> W = 3,333
```

This makes the same order more expensive as net sell pressure accumulates.

## Utilization

Utilization is based on the current order's raw sell value and the effective liquidity:

```text
u = raw / effective_liquidity
```

So:

```text
u = 0.10 means this order is 10% of effective liquidity before dampening
u = 0.50 means this order is 50% of effective liquidity before dampening
u = 1.00 means the order receives the flat haircut plus the peg haircut
large u means the order can be dampened to zero output
```

## Haircut Function

The sell-side haircut is:

```text
haircut(u) = h_min + h_peg * u^e
```

Where:

```text
h_min = min_liquidation_haircut_bps / 10_000
h_peg = curve_peg_haircut_bps / 10_000
e = curve_exponent_scaled / 10_000
```

With current defaults:

```text
h_min = 50 / 10,000 = 0.005
h_peg = 700 / 10,000 = 0.07
e = 25,000 / 10,000 = 2.5
```

So:

```text
haircut(u) = 0.005 + 0.07u^2.5
```

Examples:

```text
u = 0.10
haircut = 0.005 + 0.07 * 0.10^2.5
        ~= 0.00522
        ~= 0.52%

u = 0.50
haircut = 0.005 + 0.07 * 0.50^2.5
        ~= 0.01737
        ~= 1.74%

u = 1.00
haircut = 0.005 + 0.07
        = 0.075
        = 7.5%
```

The haircut grows slowly at first, then faster as the order consumes more of the wall. The final liquidity factor is:

```text
liquidity_factor = max(0, 1 - haircut(u))
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
haircut = 0.005 + 0.07 * 0.475^2.5
        ~= 0.0159

liquidity_factor = 1 - 0.0159
                 = 0.9841

out = 4,750 * 0.9841
    ~= 4,674 USDC
```

So the user expected about `5,000 USDC` before fee, has `4,750 USDC` raw value after fee, and actually receives about `4,674 USDC` after hard-wall dampening.

### 3. Vault State Updates

The vault loses the actual output:

```text
vault_after = 10,000 - 4,674
            = 5,326 USDC
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

That means each smaller order receives a smaller utilization and therefore a smaller haircut. However, each sell also increases global net sell pressure, so later split orders see a smaller wall.

The test `test_dynamic_wall_accumulates_sell_pressure_and_buys_relieve_it` documents that sell pressure worsens later sell quotes and ONyc buys relieve current-epoch pressure.

## Reading The Business Scenarios

The generated Markdown/CSV comes from the Rust simulator:

```text
programs/onreapp/examples/prop_amm_business_scenarios.rs
```

It calls the same Rust Prop AMM quote/tracker methods as the program code. Current example command:

```bash
rtk cargo run -p onreapp --example prop_amm_business_scenarios -- \
  --config configs/prop_amm_business_scenarios.toml
```

Report interpretation:

- Each scenario is a sequence of `sell`, `buy`, and `wait` actions.
- `Wall` is the effective liquidity denominator for that row.
- `Output %` is the seller's stable output divided by raw stable value.
- `Pressure After` is the effective sell pressure recorded after the action.
