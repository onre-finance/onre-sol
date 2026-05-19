# Prop AMM Pricing Engine

## 2. The Pricing Engine

The program is the sole authority for ONyc pricing. Price is derived from the NAV baseline, explicit buy/sell fees, and the TVL-Anchored Hard-Wall Reserve Model.

### 2.1 Baseline Pricing & Fees

Granular fees are applied to both directions to capture protocol revenue and manage churn.

**Buy Quote (`token_out_amount`):**

Prop AMM buys use the normal offer engine. The active offer vector determines the current price, the offer fee is deducted from the stablecoin input, and the net input is converted into ONYC output.

$$
\mathrm{buy\_net} = \mathrm{token\_in\_amount} - \left\lceil \frac{\mathrm{token\_in\_amount} \times \mathrm{offer\_fee\_bps}}{10,000} \right\rceil
$$

$$
\mathrm{token\_out\_amount} = \frac{\mathrm{buy\_net} \times 10^{\mathrm{token\_out\_decimals} + 9}}{\mathrm{current\_offer\_price} \times 10^{\mathrm{token\_in\_decimals}}}
$$

**Raw Sell Value (`raw_sell_value_stable`):**

Before hard-wall dampening, the sell path applies the redemption offer fee:

$$
\mathrm{sell\_net} = \mathrm{token\_in\_amount} - \left\lceil \frac{\mathrm{token\_in\_amount} \times \mathrm{redemption\_fee\_bps}}{10,000} \right\rceil
$$

$$
\mathrm{raw\_sell\_value} = \frac{\mathrm{sell\_net} \times \mathrm{current\_offer\_price} \times 10^{\mathrm{token\_out\_decimals}}}{10^{\mathrm{token\_in\_decimals}} \times 10^9}
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
R = TVL \times \frac{\mathrm{pool\_target\_bps}}{10,000}
$$

Where:

$$
\mathrm{pool\_target\_bps} = 1,500
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
L = \mathrm{actual\_liquidity}
$$

Where `actual_liquidity` is the current redemption vault balance for the output stablecoin.

The wall is:

$$
\mathrm{wall} = \frac{L}{1 + \mathrm{wall\_sensitivity} \times \frac{\mathrm{effective\_volume}}{L}}
$$

The program uses this wall as effective liquidity while dynamic wall sensitivity is enabled.

---

### 2.5 Order Utilization

The current curve is an endpoint dampening curve. It calculates utilization from the current order's raw sell value:

$$
u = \frac{\mathrm{raw\_sell\_value}}{\mathrm{effective\_liquidity}}
$$

Where:

$$
u = 0.10
$$

means this order consumes 10% of effective liquidity before dampening, and:

When `u >= 1`, the curve can dampen the sell to zero output. This is allowed as long as the raw stable value is not greater than the actual vault balance and the caller's `minimum_out` permits the final output.

---

### 2.6 Haircut Function

The haircut function is a utilization-based curve.

$$
h_{\mathrm{peg}} = \frac{\mathrm{curve\_peg\_haircut\_bps}}{10,000}
$$

$$
e_{\mathrm{base}} = \frac{\mathrm{curve\_exponent\_scaled}}{10,000}
$$

Cadence can reduce the effective exponent during an active sell-heavy epoch:

$$
\mathrm{quote\_trade\_count} =
\begin{cases}
\mathrm{curr\_sell\_trade\_count}, & \text{if the current epoch is active} \\
0, & \text{otherwise}
\end{cases}
$$

$$
\mathrm{reduction\_scaled}
= 1,000 \times \left\lfloor
\frac{\mathrm{cadence\_sensitivity\_scaled} \times \mathrm{quote\_trade\_count}}
{\mathrm{cadence\_threshold} \times 1,000}
\right\rfloor
$$

$$
\mathrm{effective\_curve\_exponent\_scaled}
= \max\left(
  \mathrm{min\_cadence\_exponent\_scaled},
  \mathrm{curve\_exponent\_scaled} - \mathrm{reduction\_scaled}
\right)
$$

$$
e_{\mathrm{effective}} = \frac{\mathrm{effective\_curve\_exponent\_scaled}}{10,000}
$$

$$
\mathrm{haircut}(u) = h_{\mathrm{peg}} \times u^{e_{\mathrm{effective}}}
$$

Default parameters:

$$
\mathrm{pool\_target\_bps} = 1,500
$$

$$
\mathrm{curve\_peg\_haircut\_bps} = 700
$$

$$
\mathrm{curve\_exponent\_scaled} = 25,000
$$

$$
\mathrm{min\_cadence\_exponent\_scaled} = 1,000
$$

$$
\mathrm{cadence\_threshold} = 20
$$

$$
\mathrm{cadence\_sensitivity\_scaled} = 10,000
$$

$$
\mathrm{epoch\_duration\_seconds} = 86,400
$$

$$
\mathrm{wall\_sensitivity\_scaled} = 20,000
$$

With no cadence adjustment, the default haircut curve is:

$$
\mathrm{haircut}(u) = 0.07u^{2.5}
$$

---

### 2.7 Final Sell Output

The program converts the haircut into a liquidity factor:

$$
\mathrm{liquidity\_factor} = \max(0, 1 - \mathrm{haircut}(u))
$$

Then it applies that factor directly to the raw sell value:

$$
\mathrm{final\_output} = \mathrm{raw\_sell\_value} \times \mathrm{liquidity\_factor}
$$

So, in code terms:

```text
effective_liquidity = min(actual_liquidity, hard_wall_reserve)
if dynamic wall sensitivity is enabled:
  effective_liquidity = dynamic_wall_position
u = raw_sell_value_stable / effective_liquidity
haircut = curve_peg_haircut * u^effective_curve_exponent
final_output = raw_sell_value_stable * max(0, 1 - haircut)
```

The actual redemption vault balance is the hard solvency guard:

```text
reject if raw_sell_value_stable > actual_liquidity
```

The dynamic wall is a pricing input, not a rejection threshold. If pressure pushes `effective_liquidity` far below the order's raw value, the final output can saturate at zero.

The code also contains a legacy fallback where `wall_sensitivity_scaled = 0` uses `min(actual_liquidity, hard_wall_reserve)`, but normal Prop AMM configuration rejects zero wall sensitivity.

Normal Prop AMM configuration also requires `curve_exponent_scaled` to be between `1,000` and `100,000` in `1,000` increments, `min_cadence_exponent_scaled` to be between `1,000` and `10,000` in `1,000` increments, `cadence_threshold > 0`, `cadence_sensitivity_scaled <= 100,000`, and `epoch_duration_seconds > 0`.

This is intentionally simple and cheap to compute.

---

### 2.8 Order Splitting

The current endpoint dampening curve is not mathematically split-proof. A single large sell still receives a higher utilization than smaller chunks.

The dynamic wall reduces the advantage because split orders accumulate global sell pressure. Later chunks see a smaller wall, and the current chunk is priced against the pressure it creates.
