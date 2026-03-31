# BUFFER Accrual

This document describes the BUFFER accrual model and the expected state transitions around ONyc supply changes.

## State Fields

BUFFER accrual uses these fields from `BufferState`:

- `lowest_supply`
- `last_accrual_timestamp`
- `performance_fee_high_watermark`

`lowest_supply` is the stored supply baseline for the next unpaid accrual interval.

`last_accrual_timestamp` is the start timestamp of that unpaid interval.

## Interval Model

Each accrual interval is handled as:

1. Read `lowest_supply`
2. Read `last_accrual_timestamp`
3. Compute elapsed time from `last_accrual_timestamp` to `now`
4. Compute BUFFER accrual using `lowest_supply`
5. Mint the accrual
6. Set a new baseline for the next interval

The new baseline after accrual is:

`post_accrual_supply`

In the shared accrual helper, this is written as:

`current_supply_before_mint + gross_mint_amount`

## BUFFER Accrual

Any BUFFER-aware instruction performs one full accrual cycle before applying its own supply change.

Inputs:

- stored `lowest_supply`
- stored `last_accrual_timestamp`
- current ONyc mint supply before any accrual mint
- current NAV from the main offer

Steps:

1. Load `previous_lowest_supply`
2. Load `current_supply_before_mint`
3. Compute `seconds_elapsed = now - last_accrual_timestamp`
4. Compute `gross_mint_amount`
5. Split `gross_mint_amount` into:
   - buffer mint
   - management fee mint
   - performance fee mint
6. Mint all parts
7. Update:
   - `performance_fee_high_watermark`
   - `lowest_supply = current_supply_before_mint + gross_mint_amount`
   - `last_accrual_timestamp = now`

If `lowest_supply == 0`, the accrual path initializes the baseline:

- `lowest_supply = current_supply_before_mint`
- `last_accrual_timestamp = now`

and performs no accrual mint.

## Any Other ONyc Supply Change

Any instruction that changes ONyc supply should be handled as:

1. Accrue pending BUFFER from stored baseline up to `now`
2. Perform the ONyc mint or burn
3. Read or derive the post-change ONyc supply
4. Set:
   - `lowest_supply = post_change_supply`
   - `last_accrual_timestamp = now`

This applies to:

- manual ONyc mint
- offer execution when ONyc is minted
- offer execution when ONyc is burned
- redemption fulfillment when ONyc is burned
- `burn_for_nav_increase`

## Supply Baseline Update

After a supply-changing operation, the next baseline is always the supply after that operation.

Examples:

- after an accrual mint, baseline becomes post-accrual supply
- after a user buy mint, baseline becomes post-buy supply
- after a redemption burn, baseline becomes post-burn supply
- after a NAV burn, baseline becomes post-burn supply

## Example 1: Initialize BUFFER

Initial state:

- `lowest_supply = 0`
- `last_accrual_timestamp = 0`
- ONyc supply = `1,000`

Call any BUFFER-aware instruction at `T1`.

Steps:

1. Read current supply `1,000`
2. Since `lowest_supply == 0`, do not accrue
3. Set:
   - `lowest_supply = 1,000`
   - `last_accrual_timestamp = T1`

Result:

- current unpaid interval starts at `T1`
- baseline supply for that interval is `1,000`

## Example 2: One BUFFER Accrual Cycle

Initial state:

- `lowest_supply = 1,000`
- `last_accrual_timestamp = T1`
- current ONyc supply before accrual mint = `1,000`

At `T2`, computed accrual is:

- `gross_mint_amount = 50`
- split into:
  - buffer = `35`
  - management fee = `5`
  - performance fee = `10`

Steps:

1. Accrue using baseline `1,000`
2. Mint total `50`
3. Post-accrual supply becomes `1,050`
4. Set:
   - `lowest_supply = 1,050`
   - `last_accrual_timestamp = T2`

Result:

- interval `T1 -> T2` is settled
- next unpaid interval starts at `T2` with baseline `1,050`

## Example 3: Accrual, Then User Buy

State after prior accrual:

- `lowest_supply = 1,050`
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
   - `lowest_supply = 1,270`
   - `last_accrual_timestamp = T3`

Result:

- interval `T2 -> T3` used supply `1,050`
- next interval starts at `T3` with baseline `1,270`

## Example 4: User Buy, Then Another User Buy

Starting state:

- `lowest_supply = 1,270`
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
   - `lowest_supply = 1,400`
   - `last_accrual_timestamp = T4`

At `T5`, user B buys `50`.

Sequence:

1. Accrue pending BUFFER for `T4 -> T5` using `1,400`
2. Suppose accrual mint is `10`
3. Supply after accrual becomes `1,410`
4. User B buy mints `50`
5. Supply becomes `1,460`
6. Set:
   - `lowest_supply = 1,460`
   - `last_accrual_timestamp = T5`

## Example 5: Accrual, Then Redemption Burn

State:

- `lowest_supply = 1,460`
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
   - `lowest_supply = 1,355`
   - `last_accrual_timestamp = T6`

## Example 6: Accrual, Then NAV Burn

State:

- `lowest_supply = 1,355`
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
   - `lowest_supply = 1,267`
   - `last_accrual_timestamp = T7`

## Example 7: Mint, Burn, Mint Across Multiple Operations

Starting state:

- `lowest_supply = 2,000`
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
   - `lowest_supply = 2,340`
   - `last_accrual_timestamp = T11`

At `T12`, redemption burns `90`.

Sequence:

1. Accrue pending BUFFER for `T11 -> T12` using `2,340`
2. Suppose accrual mint is `8`
3. Supply after accrual becomes `2,348`
4. Redemption burn removes `90`
5. Supply becomes `2,258`
6. Set:
   - `lowest_supply = 2,258`
   - `last_accrual_timestamp = T12`

At `T13`, user buy mints `60`.

Sequence:

1. Accrue pending BUFFER for `T12 -> T13` using `2,258`
2. Suppose accrual mint is `5`
3. Supply after accrual becomes `2,263`
4. User buy mints `60`
5. Supply becomes `2,323`
6. Set:
   - `lowest_supply = 2,323`
   - `last_accrual_timestamp = T13`

## Example 8: Two Operations At The Same Timestamp

Starting state:

- `lowest_supply = 5,000`
- `last_accrual_timestamp = T20`

At `T21`, operation A changes supply, and operation B changes supply again in the same block timestamp.

Operation A:

1. Accrue `T20 -> T21` using `5,000`
2. Suppose accrual mint is `25`
3. Perform supply change A, for example mint `100`
4. If starting supply was `5,000`, post-op supply becomes `5,125`
5. Set:
   - `lowest_supply = 5,125`
   - `last_accrual_timestamp = T21`

Operation B at the same `T21`:

1. Elapsed time is `0`
2. Pending accrual is `0`
3. Perform supply change B, for example burn `20`
4. Supply becomes `5,105`
5. Set:
   - `lowest_supply = 5,105`
   - `last_accrual_timestamp = T21`

## Operational Summary

For each supply-changing instruction:

1. Settle the unpaid interval using stored `lowest_supply`
2. Execute the ONyc mint or burn
3. Store the post-change supply as the next `lowest_supply`
4. Store `now` as the next `last_accrual_timestamp`
