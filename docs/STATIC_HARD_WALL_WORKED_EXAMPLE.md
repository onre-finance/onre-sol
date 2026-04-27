# Static Hard Wall Worked Example

This note applies the endpoint-style formulas to concrete numbers, so the order-splitting problem is visible in actual outputs.

The goal is to compare:

- `3 x 1,000 ONYC` sells
- `1 x 3,000 ONYC` sell

under a reserve setup of:

- hard-wall reserve target: `10,000`
- actual reserves right now: `5,000`

To keep the example simple, assume:

- `NAV = 1`
- `1 ONYC = 1 USD` before hard-wall dampening
- sell fee = `0`
- linear weight = `20%`
- exponent = `3`

So:

$$
penalty(u) = 0.20u + 0.80u^3
$$

and the endpoint liquidity factor is:

$$
liquidity\_factor(u) = 1 - penalty(u)
$$

The payout formula is:

$$
P(q) = q \times liquidity\_factor(u)
$$

---

## 1. Case A: Original Spec-Style Formula

This is the formula from the earlier spec, where utilization is:

$$
u = \frac{order\_size}{actual\_liquidity}
$$

Since current actual liquidity is `5,000`, we get:

### Sequential Sells: `q = 1,000`, then `1,000`, then `1,000`

The first sell uses:

$$
u_1 = \frac{1,000}{5,000} = 0.20
$$

Penalty:

$$
penalty(u_1) = 0.20 \times 0.20 + 0.80 \times 0.20^3
$$

$$
= 0.0400 + 0.0064
$$

$$
= 0.0464
$$

Liquidity factor:

$$
liquidity\_factor = 1 - 0.0464 = 0.9536
$$

First payout:

$$
P_1 = 1,000 \times 0.9536 = 953.6
$$

New liquidity:

$$
L_1 = 5,000 - 953.6 = 4,046.4
$$

The second sell is repriced against the smaller vault:

$$
u_2 = \frac{1,000}{4,046.4} \approx 0.247133
$$

$$
penalty(u_2) = 0.20 \times 0.247133 + 0.80 \times 0.247133^3 \approx 0.061502
$$

$$
liquidity\_factor_2 \approx 1 - 0.061502 = 0.938498
$$

$$
P_2 = 1,000 \times 0.938498 \approx 938.498
$$

New liquidity:

$$
L_2 = 4,046.4 - 938.498 \approx 3,107.902
$$

The third sell is repriced again:

$$
u_3 = \frac{1,000}{3,107.902} \approx 0.321761
$$

$$
penalty(u_3) = 0.20 \times 0.321761 + 0.80 \times 0.321761^3 \approx 0.091002
$$

$$
liquidity\_factor_3 \approx 1 - 0.091002 = 0.908998
$$

$$
P_3 = 1,000 \times 0.908998 \approx 908.998
$$

Total payout:

$$
P_{split} = P_1 + P_2 + P_3
$$

$$
\approx 953.6 + 938.498 + 908.998
$$

$$
\approx 2,801.097
$$

### Large Sell: `q = 3,000`

$$
u = \frac{3,000}{5,000} = 0.60
$$

Penalty:

$$
penalty(0.60) = 0.20 \times 0.60 + 0.80 \times 0.60^3
$$

$$
= 0.1200 + 0.1728
$$

$$
= 0.2928
$$

Liquidity factor:

$$
liquidity\_factor = 1 - 0.2928 = 0.7072
$$

Payout:

$$
P(3,000) = 3,000 \times 0.7072 = 2,121.6
$$

### Result

Three sequential sells:

$$
2,801.097
$$

One large sell:

$$
2,121.6
$$

Difference:

$$
2,801.097 - 2,121.6 \approx 679.497
$$

Relative advantage from splitting:

$$
\frac{679.497}{2,121.6} \approx 32.0\%
$$

That is a very large exploit surface.

---

## 2. Case B: Static Hard-Wall Endpoint Formula

Now keep the endpoint-style formula, but use the hard-wall reserve target as the denominator:

$$
u = \frac{order\_size}{hard\_wall\_reserve}
$$

Here:

$$
hard\_wall\_reserve = 10,000
$$

So the vault is already only `50%` full, but the endpoint formula still prices each order in isolation against the full hard-wall reserve.

### Small Sell: `q = 1,000`

$$
u = \frac{1,000}{10,000} = 0.10
$$

Penalty:

$$
penalty(0.10) = 0.20 \times 0.10 + 0.80 \times 0.10^3
$$

$$
= 0.0200 + 0.0008
$$

$$
= 0.0208
$$

Liquidity factor:

$$
liquidity\_factor = 1 - 0.0208 = 0.9792
$$

Payout for one `1,000` sell:

$$
P(1,000) = 1,000 \times 0.9792 = 979.2
$$

For three such sells:

$$
3 \times 979.2 = 2,937.6
$$

### Large Sell: `q = 3,000`

$$
u = \frac{3,000}{10,000} = 0.30
$$

Penalty:

$$
penalty(0.30) = 0.20 \times 0.30 + 0.80 \times 0.30^3
$$

$$
= 0.0600 + 0.0216
$$

$$
= 0.0816
$$

Liquidity factor:

$$
liquidity\_factor = 1 - 0.0816 = 0.9184
$$

Payout:

$$
P(3,000) = 3,000 \times 0.9184 = 2,755.2
$$

### Result

Three smaller sells:

$$
2,937.6
$$

One large sell:

$$
2,755.2
$$

Difference:

$$
2,937.6 - 2,755.2 = 182.4
$$

Relative advantage from splitting:

$$
\frac{182.4}{2,755.2} \approx 6.62\%
$$

So the static wall helps, but it does not solve the problem.

---

## 3. Why Actual Reserves = 5,000 Still Matters

In the static-wall endpoint formula above, actual reserves were:

$$
actual\_liquidity = 5,000
$$

but the pricing formula used:

$$
u = \frac{order\_size}{10,000}
$$

That means the formula knows the target reserve, but it does not directly price the fact that the vault is already only `50%` full.

So the model still says:

- a `1,000` sell is only `10%` utilization
- a `3,000` sell is only `30%` utilization

even though the vault only contains `5,000` right now.

That is one reason the pricing is too generous.

---

## 4. Summary Table

| Model | `3 x 1,000` sells | `1 x 3,000` sell | Split advantage |
| --- | ---: | ---: | ---: |
| Original spec-style (`q / actual_liquidity`) | `2,801.097` | `2,121.6` | `+679.497` |
| Static-wall endpoint (`q / 10,000`) | `2,937.6` | `2,755.2` | `+182.4` |

---

## 5. Conclusion

Using a static hard wall does improve the endpoint model:

- the exploit gets smaller
- the curve is more stable
- the denominator no longer shrinks with every reserve drop

But it is still not split-resistant.

The reason is that the formula still prices each order independently:

$$
P(q) = q \times f(q)
$$

instead of pricing a state transition:

$$
P = V(before) - V(after)
$$

That is why:

- dynamic endpoint model is bad
- static endpoint model is better
- value-function / integrated model is the one that actually removes the splitting edge

---

## 6. Numeric Illustration From The Current Rust Implementation

Using the current implementation with:

- hard-wall reserve = `10,000`
- actual liquidity = `5,000`
- sell fee = `0`
- `linear_weight_bps = 2,000`
- `base_exponent = 3`

the real outputs from the Rust function are:

### One Sell of `3,000`

```text
raw sell value = 3,000
actual output  = 2,121
```

So:

$$
P_{one} = 2,121
$$

### Three Sells of `1,000`

The same function, applied sequentially with the updated vault balance after each trade, gives:

First sell:

```text
raw sell value = 1,000
actual output  = 953
vault after    = 5,000 - 953 = 4,047
```

Second sell:

```text
raw sell value = 1,000
actual output  = 938
vault after    = 4,047 - 938 = 3,109
```

Third sell:

```text
raw sell value = 1,000
actual output  = 909
vault after    = 3,109 - 909 = 2,200
```

Total:

$$
P_{split} = 953 + 938 + 909 = 2,800
$$

So:

$$
P_{split} > P_{one}
$$

This is the current behavior. The endpoint formula dampens sell output, but it remains open to split orders.

---

## 7. Final Comparison

| Model | `3 x 1,000` sells | `1 x 3,000` sell | Split advantage | Current status |
| --- | ---: | ---: | ---: | --- |
| Dynamic endpoint (`q / actual_liquidity`, repriced each sell) | `2,800` | `2,121` | `+679` | Current Rust behavior when actual liquidity is below target |
| Static-wall endpoint (`q / 10,000`) | `2,937.6` | `2,755.2` | `+182.4` | Behavior when actual liquidity is at or above target |
| Integrated / value-function model | same as one big sell | same as split sells | `0` | Future anti-splitting direction, not current implementation |

---

## 8. Practical Meaning

The difference is structural:

- endpoint model:

$$
P(q) = q \times f(q)
$$

- integrated model:

$$
P = V(before) - V(after)
$$

Only the second one composes correctly across multiple trades.

That is why:

- the current endpoint implementation leaks a split advantage
- a static hard wall reduces the leak but does not remove it
- an integrated/value-function model would remove the leak by construction, but that is not what is implemented right now
