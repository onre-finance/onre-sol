# Market Stats Parity Audit

This audit compares the current worktree baseline (`programV4`) against the existing implementation on branch `ralph/market-stats-pda`.

At the start of this iteration, `ralph/market-stats-programv4-parity` points at the same commit as `programV4`, so the `programV4` formulas below are taken from the checked-out source and the parity target is read directly from `git show ralph/market-stats-pda:...`.

## programV4 formulas

| Field | programV4 source path | Formula / behavior |
| --- | --- | --- |
| `apy` | `programs/onreapp/src/instructions/market_info/get_apy.rs` | Uses the active vector APR and computes `APY = (1 + APR / 365)^365 - 1` with scale `1e6`. |
| `circulating_supply` | `programs/onreapp/src/instructions/market_info/get_circulating_supply.rs` | `circulating_supply = onyc_mint.supply - vault_amount`, where an uninitialized or non-token-owned vault ATA is treated as `0`. |
| `nav` | `programs/onreapp/src/instructions/market_info/get_nav.rs` | Finds the active vector at `Clock::get()?.unix_timestamp` and computes the stepped current price with scale `1e9`. |
| `nav_adjustment` | `programs/onreapp/src/instructions/market_info/get_nav_adjustment.rs` | Computes the signed delta at the vector transition point: `price(active_vector.start_time) - price(previous_vector at active_vector.start_time)`. If there is no previous vector, it returns the current vector start price as a positive `i64`. |
| `tvl` | `programs/onreapp/src/instructions/market_info/get_tvl.rs` | `tvl = circulating_supply * nav / 10^PRICE_DECIMALS`, with overflow protection and the same optional-vault behavior as circulating supply. |

## Corresponding paths on `ralph/market-stats-pda`

| Field | Shared recomputation path on `ralph/market-stats-pda` | Storage / write path on `ralph/market-stats-pda` |
| --- | --- | --- |
| `apy` | `programs/onreapp/src/instructions/market_info/market_stats.rs` via `recompute_market_stats()` | Stored in `state::MarketStats.apy` and written from `take_offer`, `take_offer_permissionless`, and `refresh_market_stats`. |
| `circulating_supply` | `programs/onreapp/src/instructions/market_info/market_stats.rs` via `recompute_market_stats()` + `read_optional_token_account_amount()` | Stored in `state::MarketStats.circulating_supply`; written by the same three flows. |
| `nav` | `programs/onreapp/src/instructions/market_info/market_stats.rs` via `recompute_market_stats()` | Stored in `state::MarketStats.nav`; written by the same three flows. |
| `nav_adjustment` | `programs/onreapp/src/instructions/market_info/market_stats.rs` via `recompute_market_stats()` + `calculate_nav_adjustment()` | Stored in `state::MarketStats.nav_adjustment`; written by the same three flows. The read-only `get_nav_adjustment` instruction remains separate and wraps the shared helper to restore a signed external result. |
| `tvl` | `programs/onreapp/src/instructions/market_info/market_stats.rs` via `recompute_market_stats()` + `calculate_tvl()` | Stored in `state::MarketStats.tvl`; written by the same three flows. |

## Confirmed parity status

| Field | Status vs `programV4` | Notes |
| --- | --- | --- |
| `apy` | Matches | The shared branch reuses `calculate_apy_from_apr()` and still derives APY from the active vector APR. |
| `circulating_supply` | Matches | The shared branch extracts the optional vault-reader into `read_optional_token_account_amount()` but keeps the same subtraction semantics. |
| `nav` | Matches | The shared branch still uses the active vector at the current clock time and `calculate_current_step_price()`. |
| `tvl` | Matches | The shared branch factors TVL into `calculate_tvl()` but preserves `circulating_supply * nav / 10^PRICE_DECIMALS`. |
| `nav_adjustment` | Does not match | The stored PDA path diverges from `programV4` semantics in two independent ways described below. |

## Confirmed `nav_adjustment` differences

1. Signedness changes on the stored PDA.
   In `programV4`, `get_nav_adjustment()` returns `i64` and preserves negative transitions.
   On `ralph/market-stats-pda`, `state::MarketStats.nav_adjustment` and `MarketStatsSnapshot.nav_adjustment` are both `u64`, so the stored value cannot represent a negative move.

2. The shared recomputation uses absolute magnitude for stored values.
   `ralph/market-stats-pda` computes stored `nav_adjustment` with `current_price.abs_diff(previous_price)`, which collapses negative and positive transitions to the same magnitude.
   The branch even contains a unit test asserting that behavior for a downward move.

3. The stored PDA compares against `current_time`, not strictly the vector transition time.
   `programV4` computes the current-side value at `active_vector.start_time`, so the result is the jump between vectors.
   `ralph/market-stats-pda` passes `current_time` into the shared `calculate_nav_adjustment()` during PDA recomputation, which makes the stored value drift after the transition when APR is non-zero.

4. The read-only instruction is already compensating for the storage mismatch.
   On `ralph/market-stats-pda`, `get_nav_adjustment()` reconstructs the sign around the shared absolute-magnitude helper so the instruction result can still match `programV4`.
   That means the verified parity gap is in the PDA storage path, not in the read-only instruction contract.

## Source checkpoints

- `programV4` `nav_adjustment` behavior: `programs/onreapp/src/instructions/market_info/get_nav_adjustment.rs`
- `ralph/market-stats-pda` shared snapshot fields and recomputation: `programs/onreapp/src/instructions/market_info/market_stats.rs`
- `ralph/market-stats-pda` PDA schema: `programs/onreapp/src/state.rs`
- `ralph/market-stats-pda` write triggers:
  - `programs/onreapp/src/instructions/offer/take_offer.rs`
  - `programs/onreapp/src/instructions/offer/take_offer_permissionless.rs`
  - `programs/onreapp/src/instructions/market_info/refresh_market_stats.rs`
