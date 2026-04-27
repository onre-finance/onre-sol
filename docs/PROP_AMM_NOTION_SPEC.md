# Prop AMM Pricing Engine

## 2. The Pricing Engine

The program is the sole authority for ONyc pricing. Price is derived from the NAV baseline, explicit buy/sell fees, and the TVL-Anchored Hard-Wall Reserve Model.

### 2.1 Baseline Pricing & Fees

Granular fees are applied to both directions to capture protocol revenue and manage churn.

**Buy Price (`price_buy`):**

$$
price\_buy = NAV \times \left(1 + \frac{buy\_fee\_bps}{10,000}\right)
$$

**Raw Sell Value (`raw_sell_value`):**

Before hard-wall dampening, the sell path applies the redemption offer fee:

$$
net\_margin = 1 - \frac{redemption\_fee\_bps}{10,000}
$$

$$
raw\_sell\_value = NAV \times amount\_{ONyc} \times net\_margin
$$

If the redemption offer PDA is valid but uninitialized, `redemption_fee_bps = 0`.

---

### 2.2 TVL-Anchored Hard-Wall Reserve

The sell output is anchored to a target redemption reserve derived from TVL, not only the current vault balance.

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

### 2.3 Effective Liquidity

Let:

$$
L = actual\_liquidity
$$

Where `actual_liquidity` is the current redemption vault balance for the output stablecoin.

The current implementation clips liquidity to the TVL-derived hard-wall reserve:

$$
effective\_liquidity = \min(L, R)
$$

The program rejects sells when `L = 0`, `R = 0`, or `raw_sell_value >= effective_liquidity`. That keeps every accepted sell below the effective reserve.

---

### 2.4 Order Utilization

The current curve is an endpoint dampening curve. It calculates utilization from the current order's raw sell value:

$$
u = \frac{raw\_sell\_value}{effective\_liquidity}
$$

Where:

$$
u = 0.10
$$

means this order consumes 10% of effective liquidity before dampening, and:

$$
u \rightarrow 1
$$

means the order is close to the maximum size accepted by the current curve.

---

### 2.5 Penalty Function

The penalty function combines a linear term and a nonlinear cliff term.

$$
w = \frac{linear\_weight\_bps}{10,000}
$$

$$
penalty(u) = w \times u + (1 - w) \times u^{base\_exponent}
$$

Default parameters:

$$
linear\_weight\_bps = 2,000
$$

$$
w = 0.20
$$

$$
base\_exponent = 3
$$

So the default penalty curve is:

$$
penalty(u) = 0.20u + 0.80u^3
$$

---

### 2.6 Final Sell Output

The program converts the penalty into a liquidity factor:

$$
liquidity\_factor = 1 - penalty(u)
$$

Then it applies that factor directly to the raw sell value:

$$
final\_output = raw\_sell\_value \times liquidity\_factor
$$

So, in code terms:

```text
effective_liquidity = min(actual_liquidity, hard_wall_reserve)
u = raw_sell_value / effective_liquidity
penalty = linear_weight * u + nonlinear_weight * u^base_exponent
final_output = raw_sell_value * (1 - penalty)
```

This is intentionally simple and cheap to compute.

---

### 2.7 Order Splitting

The current endpoint dampening curve does not prevent order splitting. A single large sell receives a higher utilization and a larger penalty than several smaller sells with the same total raw sell value.

That behavior is known and accepted for the current implementation. A separate anti-splitting mechanism is expected to address this later.
