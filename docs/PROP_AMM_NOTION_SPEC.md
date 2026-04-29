# Prop AMM Pricing Engine

## 2. The Pricing Engine

The program is the sole authority for ONyc pricing. Price is derived from the NAV baseline, explicit buy/sell fees, and the TVL-Anchored Hard-Wall Reserve Model.

### 2.1 Baseline Pricing & Fees

Granular fees are applied to both directions to capture protocol revenue and manage churn.

**Buy Price (`price_buy`):**

$$
price\_buy = NAV \times \left(1 + \frac{buy\_fee\_bps}{10,000}\right)
$$

**Raw Sell Value (`raw_sell_value_stable`):**

Before hard-wall dampening, the sell path applies the redemption offer fee:

$$
net\_margin = 1 - \frac{redemption\_fee\_bps}{10,000}
$$

$$
raw\_sell\_value = NAV \times amount\_{ONyc} \times net\_margin
$$

If the redemption offer PDA is valid but uninitialized, `redemption_fee_bps = 0`.

ONYC has 9 decimals. Stablecoins have 6 decimals. Volume tracking is normalized to stablecoin base units:

```text
1 ONYC at NAV = 1.00 stable
ONYC base amount = 1_000_000_000
stable base value = 1_000_000
```

So `curr_sell_value_stable`, `curr_buy_value_stable`, `prev_net_sell_value_stable`, and `raw_sell_value_stable` are all stablecoin-denominated values. This keeps the wall formula compatible with the redemption vault balance, which is also in stablecoin base units.

---

### 2.2 TVL-Anchored Hard-Wall Reserve

The sell output is anchored to the actual redemption vault balance and current net sell pressure. The TVL reserve target still exists for buy-side refill policy and for disabled-sensitivity fallback behavior.

**Step 1: Calculate Hard-Wall Reserve (`R`)**

$$
R = TVL \times \frac{pool\_target\_bps}{10,000}
$$

Where:

$$
pool\_target\_bps = 1,500
$$

So by default:

$$
R = TVL \times 0.15
$$

`R` is converted into the output token decimals before being used.

---

### 2.3 Net Sell Pressure

Prop AMM tracks sell pressure over the current and previous epoch.

```text
curr_net = max(0, curr_sell_value_stable - curr_buy_value_stable)
effective_volume = decayed_prev_net + curr_net + raw_sell_value_stable
```

The current ONYC sell's own raw stable value is included in `effective_volume`, so the sell pays against the pressure it creates. ONYC buys add their net stablecoin input to `curr_buy_value_stable`, which can reduce current-epoch pressure but cannot create negative pressure.

---

### 2.4 Dynamic Wall

Let:

$$
L = actual\_liquidity
$$

Where `actual_liquidity` is the current redemption vault balance for the output stablecoin.

The wall is:

$$
wall = \frac{L}{1 + wall\_sensitivity \times \frac{effective\_volume}{L}}
$$

The program uses this wall as effective liquidity while dynamic wall sensitivity is enabled.

---

### 2.5 Order Utilization

The current curve is an endpoint dampening curve. It calculates utilization from the current order's raw sell value:

$$
u = \frac{raw\_sell\_value}{effective\_liquidity}
$$

Where:

$$
u = 0.10
$$

means this order consumes 10% of effective liquidity before dampening, and:

When `u >= 1`, the curve can dampen the sell to zero output. This is allowed as long as the raw stable value is not greater than the actual vault balance and the caller's `minimum_out` permits the final output.

---

### 2.6 Haircut Function

The haircut function combines a flat minimum haircut with a utilization-based curve.

$$
h_{min} = \frac{min\_liquidation\_haircut\_bps}{10,000}
$$

$$
h_{peg} = \frac{curve\_peg\_haircut\_bps}{10,000}
$$

$$
e = \frac{curve\_exponent\_scaled}{10,000}
$$

$$
haircut(u) = h_{min} + h_{peg} \times u^e
$$

Default parameters:

$$
min\_liquidation\_haircut\_bps = 50
$$

$$
curve\_peg\_haircut\_bps = 700
$$

$$
curve\_exponent\_scaled = 25,000
$$

So the default haircut curve is:

$$
haircut(u) = 0.005 + 0.07u^{2.5}
$$

---

### 2.7 Final Sell Output

The program converts the haircut into a liquidity factor:

$$
liquidity\_factor = max(0, 1 - haircut(u))
$$

Then it applies that factor directly to the raw sell value:

$$
final\_output = raw\_sell\_value \times liquidity\_factor
$$

So, in code terms:

```text
effective_liquidity = min(actual_liquidity, hard_wall_reserve)
if dynamic wall sensitivity is enabled:
  effective_liquidity = dynamic_wall_position
u = raw_sell_value_stable / effective_liquidity
haircut = min_liquidation_haircut + curve_peg_haircut * u^curve_exponent
final_output = raw_sell_value_stable * max(0, 1 - haircut)
```

The actual redemption vault balance is the hard solvency guard:

```text
reject if raw_sell_value_stable > actual_liquidity
```

The dynamic wall is a pricing input, not a rejection threshold. If pressure pushes `effective_liquidity` far below the order's raw value, the final output can saturate at zero.

This is intentionally simple and cheap to compute.

---

### 2.8 Order Splitting

The current endpoint dampening curve is not mathematically split-proof. A single large sell still receives a higher utilization than smaller chunks.

The dynamic wall reduces the advantage because split orders accumulate global sell pressure. Later chunks see a smaller wall, and the current chunk is priced against the pressure it creates.
