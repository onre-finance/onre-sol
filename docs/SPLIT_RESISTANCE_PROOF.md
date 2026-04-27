# Why A Static Hard Wall Alone Does Not Prevent Order Splitting

This note explains why a fixed hard-wall denominator is not enough to make the sell curve split-resistant.

## 1. Goal

We want the following property:

```text
One large sell should produce the same total result as many smaller sells
that move the vault through the same start and end state.
```

This property is called:

- path independence
- additivity across state transitions
- split resistance

## 2. State-Based Definition

Let the vault state be `s`.

This can represent, for example:

- remaining reserve
- depletion relative to the hard wall
- normalized reserve health

A sell moves the state:

$$
s_0 \rightarrow s_1
$$

If the user splits the sell, the path becomes:

$$
s_0 \rightarrow s_a \rightarrow s_1
$$

To be split-resistant, the total payout must satisfy:

$$
P(s_0, s_1) = P(s_0, s_a) + P(s_a, s_1)
$$

for every valid intermediate state `s_a`.

This is the key requirement.

---

## 3. Value-Function Pricing Works

Suppose payout is defined by a value function `V(s)`:

$$
P(s_i, s_j) = V(s_i) - V(s_j)
$$

Then:

$$
P(s_0, s_a) + P(s_a, s_1)
$$

$$
= \left(V(s_0) - V(s_a)\right) + \left(V(s_a) - V(s_1)\right)
$$

$$
= V(s_0) - V(s_1)
$$

$$
= P(s_0, s_1)
$$

The intermediate term `V(s_a)` cancels out.

This proves that any pricing rule of the form:

$$
P(s_i, s_j) = V(s_i) - V(s_j)
$$

is split-resistant.

This is exactly why integrated / reserve-value models work.

---

## 4. Endpoint Pricing Does Not Work

Now suppose instead we price each order directly from its own size:

$$
P(s, q) = q \cdot f(s, q)
$$

Where:

- `s` is the current state
- `q` is the size of this order
- `f(s, q)` is an endpoint liquidity factor or effective price

Then one large sell of size `q` gives:

$$
P_{one} = q \cdot f(s_0, q)
$$

If the user splits it into `q_1 + q_2 = q`, then:

$$
P_{split} = q_1 f(s_0, q_1) + q_2 f(s_a, q_2)
$$

For split resistance, we would need:

$$
q f(s_0, q) = q_1 f(s_0, q_1) + q_2 f(s_a, q_2)
$$

for all possible states and all possible splits.

Ordinary endpoint formulas do not satisfy this identity.

So in general:

$$
P_{one} \neq P_{split}
$$

---

## 5. Counterexample With A Static Hard Wall

Now take a very simple endpoint model with a fixed hard wall `H`:

$$
P(q) = q \left(1 - \frac{q}{H}\right)
$$

This already uses a static denominator.

So this example proves that a fixed denominator alone is not enough.

Let:

$$
q = 2x
$$

and compare one trade of size `2x` against two trades of size `x`.

### One Large Trade

$$
P_{one} = 2x \left(1 - \frac{2x}{H}\right)
$$

$$
= 2x - \frac{4x^2}{H}
$$

### Two Smaller Trades

First trade:

$$
P_1 = x \left(1 - \frac{x}{H}\right)
$$

$$
= x - \frac{x^2}{H}
$$

Second trade:

$$
P_2 = x \left(1 - \frac{x}{H}\right)
$$

$$
= x - \frac{x^2}{H}
$$

Total:

$$
P_{split} = P_1 + P_2
$$

$$
= 2x - \frac{2x^2}{H}
$$

Now compare the two:

$$
P_{split} - P_{one}
$$

$$
= \left(2x - \frac{2x^2}{H}\right) - \left(2x - \frac{4x^2}{H}\right)
$$

$$
= \frac{2x^2}{H}
$$

Since:

$$
\frac{2x^2}{H} > 0
$$

we get:

$$
P_{split} > P_{one}
$$

So splitting helps.

And note:

```text
The hard wall H was static the entire time.
```

That is the proof.

---

## 6. Why Static Wall Helps But Is Not Sufficient

A static hard wall does help in one important way:

```text
It fixes the reference point.
```

So instead of repricing everything against a moving denominator like:

$$
u = \frac{order\_size}{actual\_liquidity}
$$

you can reprice against a fixed reference like:

$$
u = \frac{order\_size}{hard\_wall\_reserve}
$$

That makes the curve more stable and easier to reason about.

But it still does not make payouts additive across path segments.

To get split resistance, the pricing rule must depend on the change in state:

$$
P = V(before) - V(after)
$$

not just on the current order's endpoint formula.

---

## 7. General Conclusion

If you want split resistance, you need:

$$
P(s_0, s_1) = P(s_0, s_a) + P(s_a, s_1)
$$

for every intermediate state `s_a`.

That is automatically true if:

$$
P(s_i, s_j) = V(s_i) - V(s_j)
$$

for some value function `V`.

It is generally false for endpoint pricing rules of the form:

$$
P(s, q) = q \cdot f(s, q)
$$

Therefore:

- static hard wall: useful
- endpoint curve only: not split-resistant
- value-function / integrated curve: split-resistant

---

## 8. Practical Takeaway

There are three distinct ideas:

### A. Dynamic denominator

Example:

$$
u = \frac{order\_size}{actual\_liquidity}
$$

This is usually the most vulnerable to splitting.

### B. Static denominator

Example:

$$
u = \frac{order\_size}{hard\_wall\_reserve}
$$

This improves stability, but does not solve splitting by itself.

### C. State-value pricing

Example:

$$
payout = V(before) - V(after)
$$

This is the structure that actually prevents order splitting.

