# BUFFER Accrual

This document describes the BUFFER accrual model and the expected state transitions around ONyc supply changes.

## Why BUFFER Exists

BUFFER is the protocol's reserve-growth layer for ONyc supply. It lets the
program recognize a target gross yield over time, mint that growth into a
dedicated reserve, and split configured management and performance fees without
mixing those amounts with offer proceeds, Prop AMM proceeds, or redemption
liquidity.

At a high level, BUFFER is there to keep ONyc supply changes economically
continuous. User-facing instructions can mint, burn, buy, sell, or redeem ONyc
at irregular times, but the reserve yield is time-based. BUFFER settles the
unpaid interval before those supply changes, then stores the post-change supply
as the next baseline. That avoids hidden yield gaps, double-counted accrual, and
ambiguous accounting around supply changes.

The reserve is intentionally separate from redemption liquidity. Redemption
vaults are used to pay users out. BUFFER reserve and fee vaults account for
reserve growth and protocol fees. Keeping those domains separate makes it easier
to reason about solvency, operator withdrawals, market reporting, and audit
trails.

Use BUFFER to answer these questions:

- How much reserve growth has accrued since the last supply baseline?
- Which part of that growth belongs to the reserve versus management or
  performance fees?
- What ONyc supply should the next accrual interval use as its baseline?

## State Fields

BUFFER accrual reads and updates these `BufferState` fields:

- `onyc_mint`
- `gross_apr`
- `previous_supply`
- `management_fee_basis_points`
- `performance_fee_basis_points`
- `performance_fee_high_watermark_enabled`
- `last_accrual_timestamp`
- `performance_fee_high_watermark`

`previous_supply` is the stored supply baseline for the next unpaid accrual interval.

`last_accrual_timestamp` is the start timestamp of that unpaid interval.

`gross_apr` uses scale `1_000_000`, where `1_000_000` is 100%. The
`set_buffer_gross_apr` instruction accepts values from 0 through `1_000_000`.

`performance_fee_high_watermark_enabled` defaults to `true`. When enabled,
performance fees are minted only after the high-water mark has been initialized
and current NAV is at or above it. When disabled, every nonzero BUFFER accrual
mints the configured performance fee regardless of current NAV. The watermark
continues to track the maximum observed NAV while disabled, so re-enabling it
restores the usual all-time-high threshold.

`set_buffer_fee_config` first settles pending accrual under the existing fee and
high-water-mark settings, then applies the new values to the next interval.

## Interval Model

Each accrual interval is handled as:

1. Read `previous_supply`
2. Read `last_accrual_timestamp`
3. Compute elapsed time from `last_accrual_timestamp` to `now`
4. Compute BUFFER accrual using `previous_supply`
5. Mint the accrual
6. Set a new baseline for the next interval

The new baseline after accrual is:

`post_accrual_supply`

In the shared accrual helper, this is written as:

`current_supply_before_mint + gross_mint_amount`

## BUFFER Accrual

Any BUFFER-aware instruction performs one full accrual cycle before applying its own supply change.

The worker-only `settle_buffer` instruction performs the same accrual cycle
without a trade or an additional user/admin mint, then refreshes canonical
market stats. It is the operational fallback for a daily settlement job when no
other BUFFER-aware transaction has settled the interval. The instruction is
blocked by the kill switch and requires configured worker, ONyc mint, main
offer, initialized BUFFER state, and program mint authority.

Inputs:

- stored `previous_supply`
- stored `last_accrual_timestamp`
- current ONyc mint supply before any accrual mint
- active APR and current NAV from `State.main_offer`

BUFFER state is global rather than per offer, so all BUFFER-aware paths use the
same canonical `State.main_offer` for accrual. A trade, Prop AMM pair, or
redemption may use another linked offer to calculate the user's execution
price; that pair-specific offer does not change the APR or NAV used to settle
BUFFER.

Steps:

1. Load `old_previous_supply`
2. Load `current_supply_before_mint`
3. Compute `seconds_elapsed = now - last_accrual_timestamp`
4. Compute `gross_mint_amount`, discounting by the target NAV growth already earned over the interval
5. Enforce `State.max_mint_amount` against the total `gross_mint_amount` when the cap is nonzero
6. Split `gross_mint_amount` into:
   - reserve mint
   - management fee mint
   - performance fee mint
   The performance fee is gated by the stored high-water mark only when
   `performance_fee_high_watermark_enabled` is `true`.
7. Mint all parts
8. Update:
   - `performance_fee_high_watermark`
   - `previous_supply = current_supply_before_mint + gross_mint_amount`
   - `last_accrual_timestamp = now`

If `previous_supply == 0`, the accrual path initializes the baseline:

- `previous_supply = current_supply_before_mint`
- `last_accrual_timestamp = now`
- `performance_fee_high_watermark = current_nav`

and performs no accrual mint or performance fee mint.

## BUFFER-Aware ONyc Supply Changes

Implemented BUFFER-aware ONyc supply-changing paths first settle pending accrual,
then apply their own mint or burn, then store the post-change supply and the
accrual timestamp as the next baseline.

This currently applies when the buffer is initialized and the program controls
the ONyc mint, for:

- `mint_to`, which requires `State.main_offer` to be set
- `take_offer_v2` and `take_offer_permissionless_v2` when token out is ONyc
- Prop AMM buy when token out is ONyc
- redemption fulfillment when token in is ONyc and the net amount is burned
- Prop AMM sell when token in is ONyc and the net amount is burned
- `burn_for_nav_increase`

Offer executions that only burn ONyc as token in are not wired into the BUFFER
baseline path. The V1 offer instructions have been removed.

The baseline update is:

1. Accrue pending BUFFER from stored baseline up to `now`
2. Perform the ONyc mint or burn
3. Read or derive the post-change ONyc supply
4. Set:
   - `previous_supply = post_change_supply`
   - `last_accrual_timestamp = now`

## Supply Baseline Update

After a supply-changing operation, the next baseline is always the supply after that operation.

Examples:

- after an accrual mint, baseline becomes post-accrual supply
- after a user buy mint, baseline becomes post-buy supply
- after a redemption burn, baseline becomes post-burn supply
- after a NAV burn, baseline becomes post-burn supply

## Example 1: Initialize BUFFER

Initial state after `initialize_buffer` at `T0`:

- `previous_supply = 0`
- `last_accrual_timestamp = T0`
- ONyc supply = `1,000`

Call a BUFFER accrual path at `T1`.

Steps:

1. Read current supply `1,000`
2. Since `previous_supply == 0`, do not accrue
3. Set:
   - `previous_supply = 1,000`
   - `last_accrual_timestamp = T1`
   - `performance_fee_high_watermark = current_nav_at_T1`

Result:

- current unpaid interval starts at `T1`
- baseline supply for that interval is `1,000`
- performance fees are gated against a real recorded NAV peak instead of the uninitialized zero value

## Example 2: One BUFFER Accrual Cycle

Initial state:

- `previous_supply = 1,000`
- `last_accrual_timestamp = T1`
- current ONyc supply before accrual mint = `1,000`

At `T2`, computed accrual is:

- `gross_mint_amount = 50`
- split into:
  - reserve = `35`
  - management fee = `5`
  - performance fee = `10`

Steps:

1. Accrue using baseline `1,000`
2. Mint total `50`
3. Post-accrual supply becomes `1,050`
4. Set:
   - `previous_supply = 1,050`
   - `last_accrual_timestamp = T2`

Result:

- interval `T1 -> T2` is settled
- next unpaid interval starts at `T2` with baseline `1,050`

The gross accrual is one logical mint operation for `max_mint_amount` purposes,
even though the resulting tokens can be minted to three destinations. A cap of
`80` rejects a gross accrual of `95`, even if the reserve, management-fee, and
performance-fee pieces are each below `80`.

## Example 3: Accrual, Then User Buy

State after prior accrual:

- `previous_supply = 1,050`
- `last_accrual_timestamp = T2`
- ONyc supply = `1,050`

At `T3`, user buys `200` ONyc.

Sequence:

1. Accrue pending BUFFER for `T2 -> T3` using baseline `1,050`
2. Suppose accrued mint is `20`
3. After accrual, supply becomes `1,070`
4. User buy mints `200`
5. Post-buy supply becomes `1,270`
6. Set:
   - `previous_supply = 1,270`
   - `last_accrual_timestamp = T3`

Result:

- interval `T2 -> T3` used supply `1,050`
- next interval starts at `T3` with baseline `1,270`

## Example 4: User Buy, Then Another User Buy

Starting state:

- `previous_supply = 1,270`
- `last_accrual_timestamp = T3`
- ONyc supply = `1,270`

At `T4`, user A buys `100`.

Sequence:

1. Accrue pending BUFFER for `T3 -> T4` using `1,270`
2. Suppose accrual mint is `30`
3. Supply after accrual becomes `1,300`
4. User A buy mints `100`
5. Supply becomes `1,400`
6. Set:
   - `previous_supply = 1,400`
   - `last_accrual_timestamp = T4`

At `T5`, user B buys `50`.

Sequence:

1. Accrue pending BUFFER for `T4 -> T5` using `1,400`
2. Suppose accrual mint is `10`
3. Supply after accrual becomes `1,410`
4. User B buy mints `50`
5. Supply becomes `1,460`
6. Set:
   - `previous_supply = 1,460`
   - `last_accrual_timestamp = T5`

## Example 5: Accrual, Then Redemption Burn

State:

- `previous_supply = 1,460`
- `last_accrual_timestamp = T5`
- ONyc supply = `1,460`

At `T6`, a redemption burns `120` ONyc.

Sequence:

1. Accrue pending BUFFER for `T5 -> T6` using `1,460`
2. Suppose accrual mint is `15`
3. Supply after accrual becomes `1,475`
4. Redemption burns `120`
5. Supply becomes `1,355`
6. Set:
   - `previous_supply = 1,355`
   - `last_accrual_timestamp = T6`

## Example 6: Accrual, Then NAV Burn

State:

- `previous_supply = 1,355`
- `last_accrual_timestamp = T6`
- ONyc supply = `1,355`

At `T7`, `burn_for_nav_increase` burns `100`.

Sequence:

1. Accrue pending BUFFER for `T6 -> T7` using `1,355`
2. Suppose accrual mint is `12`
3. Supply after accrual becomes `1,367`
4. NAV burn burns `100`
5. Supply becomes `1,267`
6. Set:
   - `previous_supply = 1,267`
   - `last_accrual_timestamp = T7`

## Example 7: Mint, Burn, Mint Across Multiple Operations

Starting state:

- `previous_supply = 2,000`
- `last_accrual_timestamp = T10`
- ONyc supply = `2,000`

At `T11`, manual mint of `300`.

Sequence:

1. Accrue pending BUFFER for `T10 -> T11` using `2,000`
2. Suppose accrual mint is `40`
3. Supply after accrual becomes `2,040`
4. Manual mint adds `300`
5. Supply becomes `2,340`
6. Set:
   - `previous_supply = 2,340`
   - `last_accrual_timestamp = T11`

At `T12`, redemption burns `90`.

Sequence:

1. Accrue pending BUFFER for `T11 -> T12` using `2,340`
2. Suppose accrual mint is `8`
3. Supply after accrual becomes `2,348`
4. Redemption burn removes `90`
5. Supply becomes `2,258`
6. Set:
   - `previous_supply = 2,258`
   - `last_accrual_timestamp = T12`

At `T13`, user buy mints `60`.

Sequence:

1. Accrue pending BUFFER for `T12 -> T13` using `2,258`
2. Suppose accrual mint is `5`
3. Supply after accrual becomes `2,263`
4. User buy mints `60`
5. Supply becomes `2,323`
6. Set:
   - `previous_supply = 2,323`
   - `last_accrual_timestamp = T13`

## Example 8: Two Operations At The Same Timestamp

Starting state:

- `previous_supply = 5,000`
- `last_accrual_timestamp = T20`

At `T21`, operation A changes supply, and operation B changes supply again in the same block timestamp.

Operation A:

1. Accrue `T20 -> T21` using `5,000`
2. Suppose accrual mint is `25`
3. Perform supply change A, for example mint `100`
4. If starting supply was `5,000`, post-op supply becomes `5,125`
5. Set:
   - `previous_supply = 5,125`
   - `last_accrual_timestamp = T21`

Operation B at the same `T21`:

1. Elapsed time is `0`
2. Pending accrual is `0`
3. Perform supply change B, for example burn `20`
4. Supply becomes `5,105`
5. Set:
   - `previous_supply = 5,105`
   - `last_accrual_timestamp = T21`

## Operational Summary

For each supply-changing instruction:

1. Settle the unpaid interval using stored `previous_supply`
2. Execute the ONyc mint or burn
3. Store the post-change supply as the next `previous_supply`
4. Store `now` as the next `last_accrual_timestamp`
