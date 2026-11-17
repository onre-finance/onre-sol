# Prop AMM Pricing Engine

## 2. The Pricing Engine

The program is the sole authority for ONyc pricing. Price is derived from the NAV baseline, explicit buy/sell fees, and the TVL-Anchored Hard-Wall Reserve Model.

### 2.1 Baseline Pricing & Fees

Granular fees are applied to both directions to capture protocol revenue and manage churn.

**Buy Quote (`token_out_amount`):**

Prop AMM buys use the offer engine with the offer's permissionless fee basis points. The active offer vector determines the current price, the permissionless offer fee is deducted from the asset input, and the net input is converted into ONYC output. A buy quote can exist even when buy execution later rejects because the offer's permissionless mode is disabled.

$$
\mathrm{buy\_net} = \mathrm{token\_in\_amount} - \left\lceil \frac{\mathrm{token\_in\_amount} \times \mathrm{offer\_fee\_bps}}{10,000} \right\rceil
$$

$$
\mathrm{token\_out\_amount} = \frac{\mathrm{buy\_net} \times 10^{\mathrm{token\_out\_decimals} + 9}}{\mathrm{current\_offer\_price} \times 10^{\mathrm{token\_in\_decimals}}}
$$

**Raw Sell Value (`raw_sell_value_stable`):**

Before hard-wall dampening, the sell path applies the greater of `RedemptionOffer.fee_basis_points_prop_amm_sell` and `minimum_sell_haircut_onyc`:

$$
\mathrm{pct\_fee} = \left\lceil \frac{\mathrm{token\_in\_amount} \times \mathrm{fee\_basis\_points\_prop\_amm\_sell}}{10,000} \right\rceil
$$

$$
\mathrm{token\_in\_fee} = \max(\mathrm{pct\_fee}, \mathrm{minimum\_sell\_haircut\_onyc})
$$

$$
\mathrm{sell\_net} = \mathrm{token\_in\_amount} - \mathrm{token\_in\_fee}
$$

$$
\mathrm{raw\_sell\_value} = \frac{\mathrm{sell\_net} \times \mathrm{current\_offer\_price} \times 10^{\mathrm{token\_out\_decimals}}}{10^{\mathrm{token\_in\_decimals}} \times 10^9}
$$

If the redemption offer PDA is valid but uninitialized, `fee_basis_points_prop_amm_sell = 0` and `vault_target_bps = 0`. If the redemption offer exists but is disabled, sells reject. A zero Prop AMM sell fee does not bypass `minimum_sell_haircut_onyc`.

ONYC has 9 decimals. Volume tracking is normalized to the pair asset mint's base units. USDC examples assume 6 decimals:

```text
1 ONYC at NAV = 1.00 pair asset
ONYC base amount = 1_000_000_000
asset base value = 1_000_000
```

So `curr_sell_value_stable`, `curr_buy_value_stable`, `prev_net_sell_value_stable`, and `raw_sell_value_stable` are all denominated in the pair asset mint's base units. This keeps the wall formula compatible with the redemption vault balance for that asset.

---

### 2.2 TVL-Anchored Hard-Wall Reserve

The sell output is anchored to the actual redemption vault balance, the redemption market's TVL reserve target, and current net sell pressure. The actual vault balance remains the solvency guard. When `vault_target_bps > 0`, the TVL-derived target can only reduce the effective hard-wall reserve; it can never let the curve price against more liquidity than the vault actually holds.

**Step 1: Calculate Hard-Wall Reserve (`R`)**

$$
R = TVL \times \frac{\mathrm{pool\_target\_bps}}{10,000}
$$

For example, with a configured target of 1,500 bps:

$$
\mathrm{pool\_target\_bps} = 1,500
$$

then:

$$
R = TVL \times 0.15
$$

`R` is converted into the output token decimals before being used.

The effective fixed reserve is:

$$
R_{\mathrm{effective}} =
\begin{cases}
L, & \text{if } \mathrm{vault\_target\_bps} = 0 \\
\min(L, R), & \text{otherwise}
\end{cases}
$$

where `L` is the current redemption vault token-out balance.

---

### 2.3 Net Sell Pressure

Prop AMM tracks sell pressure over the current and previous epoch.

```text
curr_net = max(0, curr_sell_value_stable - curr_buy_value_stable)
effective_volume = decayed_prev_net + curr_net + raw_sell_value_stable
```

Epochs keep fixed boundaries aligned to the stored start: previous pressure decays linearly from the scheduled boundary, trades roll the start by whole epochs, and history expires after two elapsed epochs. The legacy and modular Prop AMM share these rules.

The current ONYC sell's own raw pair-asset value is included in `effective_volume`, so the sell pays against the pressure it creates. ONYC buys add their net pair-asset input to `curr_buy_value_stable`, which can reduce current-epoch pressure but cannot create negative pressure. The code uses field names ending in `_stable`, but the stored unit is the canonical pair asset's base units.

---

### 2.4 Dynamic Wall

Let:

$$
L = \mathrm{actual\_liquidity}
$$

Where `actual_liquidity` is the current redemption vault balance for the output pair asset.

The wall is:

$$
\mathrm{wall} = \frac{L}{1 + \mathrm{wall\_sensitivity} \times \frac{\mathrm{effective\_volume}}{L}}
$$

The program caps this wall by the fixed reserve:

$$
\mathrm{effective\_liquidity} = \min(\mathrm{wall}, R_{\mathrm{effective}})
$$

So sell pressure can move the wall below the target, and a configured target can cap surplus vault liquidity above the intended TVL percentage.

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

When `u >= 1`, the curve can dampen the sell to zero output. This is allowed as long as the raw pair-asset value is not greater than the actual vault balance and the caller's `minimum_out` permits the final output.

---

### 2.6 Haircut Function

The haircut function is a utilization-based curve.

$$
h_{\mathrm{peg}} = \frac{\mathrm{curve\_peg\_haircut\_bps}}{10,000}
$$

$$
e_{\mathrm{base}} = \frac{\mathrm{curve\_exponent\_scaled}}{10,000}
$$

The base exponent remains fixed. Cadence instead raises a separate target
haircut during an active sell-heavy epoch:

$$
\mathrm{quote\_trade\_count} =
\begin{cases}
\mathrm{curr\_sell\_trade\_count}, & \text{if the current epoch is active} \\
0, & \text{otherwise}
\end{cases}
$$

$$
\mathrm{ramp} = \min\left(1,
\frac{\mathrm{quote\_trade\_count}}{\mathrm{cadence\_threshold}}
\right)
$$

$$
 y_{\mathrm{quote}} =
 \frac{\mathrm{cadence\_wave\_scaled}}{10,000} \times \mathrm{ramp}
$$

$$
x = \min(u, 1)
$$

$$
\mathrm{eased\_rise}(x) = \frac{8x}{8x + 1 - x}
$$

$$
\mathrm{cadence\_target}(u) =
\min\left(1, \frac{\mathrm{eased\_rise}(x) \times y_{\mathrm{quote}}}{3}\right)
$$

$$
\mathrm{base\_haircut}(u) = h_{\mathrm{peg}} \times u^{e_{\mathrm{base}}}
$$

$$
\mathrm{haircut}(u) =
\max\left(\mathrm{base\_haircut}(u), \mathrm{cadence\_target}(u)\right)
$$

The current sell's raw value is included in pressure immediately. Its
sell-trade-count cadence effect starts with later sells because the trade count
is recorded after the curve and `minimum_out` check. The program evaluates the
cadence target with integer fixed-point arithmetic; `8` and `3` match the Prop
AMM curve tool exactly.

Curve parameters:

$$
\mathrm{vault\_target\_bps} = \mathrm{redemption\_offer.vault\_target\_bps}
$$

$$
\mathrm{curve\_peg\_haircut\_bps} = 700
$$

$$
\mathrm{curve\_exponent\_scaled} = 25,000
$$

$$
\mathrm{cadence\_threshold} = 20
$$

$$
\mathrm{cadence\_wave\_scaled} = 10,000
$$

$$
\mathrm{epoch\_duration\_seconds} = 86,400
$$

$$
\mathrm{wall\_sensitivity\_scaled} = 20,000
$$

$$
\mathrm{minimum\_sell\_haircut\_onyc} = 5,000,000,000
$$

With no prior sells in the active epoch, the default haircut curve is:

$$
\mathrm{haircut}(u) = 0.07u^{2.5}
$$

The on-chain implementation computes the fractional power as:

$$
u^e = 2^{e \times \log_2(u)}
$$

using fixed-point Q40 intermediate math. Integer exponents use repeated
fixed-point multiplication. Fractional exponents approximate `log2` with an
atanh/log series and approximate `exp(frac * ln(2))` with a short Taylor
series. This avoids a lookup table or generic nth-root calculation while keeping
the curve close to the intended fractional exponent.

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
  effective_liquidity = min(dynamic_wall_position, hard_wall_reserve)
u = raw_sell_value_stable / effective_liquidity
base_haircut = curve_peg_haircut * u^curve_exponent
cadence_target = cadence_wave_target(u, prior_sell_count)
haircut = max(base_haircut, cadence_target)
final_output = raw_sell_value_stable * max(0, 1 - haircut)
```

The actual redemption vault balance is the hard solvency guard:

```text
reject if actual_liquidity == 0
reject if hard_wall_reserve == 0
reject if raw_sell_value_stable > actual_liquidity
```

The dynamic wall is a pricing input, not a rejection threshold. If pressure pushes `effective_liquidity` far below the order's raw value, the final output can saturate at zero.

The code also contains a fallback where `wall_sensitivity_scaled = 0` uses `min(actual_liquidity, hard_wall_reserve)`, but normal Prop AMM configuration rejects zero wall sensitivity.

Normal Prop AMM configuration also requires `curve_exponent_scaled` to be
between `1,000` and `100,000` in `1,000` increments,
`cadence_threshold > 0`, `cadence_wave_scaled` to be between `0` and `50,000`
in `1,000` increments, `epoch_duration_seconds > 0`, and
`minimum_sell_haircut_onyc` denominated in ONYC base units.

Because Prop AMM has not been deployed, its state and configuration instruction
contain only the active cadence fields: `cadence_threshold` and
`cadence_wave_scaled`. A configured `cadence_wave_scaled = 0` intentionally
disables cadence and leaves the base curve unchanged.

This is intentionally simple and cheap to compute.

---

### 2.8 Order Splitting

The current endpoint dampening curve is not mathematically split-proof. A single large sell still receives a higher utilization than smaller chunks.

The dynamic wall reduces the advantage because split orders accumulate global sell pressure. Later chunks see a smaller wall, and the current chunk is priced against the pressure it creates.
