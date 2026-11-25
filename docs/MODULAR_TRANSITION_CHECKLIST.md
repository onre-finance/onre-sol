# Transition checklist: legacy ONyc offers → modular offers

Do the steps in order. Legacy offers keep working until step 15, when they are retired.
Details are in [MODULAR_TAKE_OFFER.md](MODULAR_TAKE_OFFER.md).

## A. Upgrade (legacy keeps working)

1. Update every `mint_to` caller (Squads templates, scripts, backend) to pass the nine new trailing accounts, using the program ID as `None`. The current program ignores extra accounts, so this is safe before the upgrade. Saved transactions with the old account list fail after it.
2. Deploy the program upgrade. ONyc stays on the legacy BUFFER (`State::onyc_per_mint_buffer_active = false`), so legacy offers, the legacy BUFFER and `mint_to` behave as before.

## B. Register ONyc as a managed token (legacy keeps working)

3. `register_managed_token` for ONyc. Keep minting **direct** (program PDA as mint authority). Do not enable a mint multisig yet, because legacy instructions only mint directly.
4. `set_managed_token_mint_limits` to the **same** values as `State` `max_supply` / `max_mint_amount`. Until step 15, keep the two sets equal: modular paths use the per-mint caps and legacy paths use the `State` caps.
5. `create_pricer` (USD, instance 0) and `add_pricer_vector` with the **same** price as the main offer's current vector. Until step 15, keep them aligned, because both write the legacy market stats.
6. `set_managed_token_supply_exclusions` with the same accounts as the legacy exclusion list. It writes the legacy cache too. From now on, change exclusions only through this instruction.
7. Select ONyc's market stats: `set_managed_token_market_stats` with the legacy `["market_stats"]` singleton, or `initialize_managed_token_market_stats` for a per-mint account.
8. `initialize_managed_token_buffer` for ONyc, and create the ATAs of the vaults it references. It stays inactive until step 15.

## C. Run modular offers next to the legacy ones

9. Create the modular offer stack:
   - `create_quoter`
   - `create_vault_config` for the fee, proceeds and liquidity vaults
   - `create_fee_config`
   - `create_offer_config` for USDC → ONyc and USDG → ONyc buys and for worker sell (redemption) offers

   Then fund the vaults (`deposit_vault_config`).
10. Switch the frontend and integrators to the modular `take_offer`. For ONyc, always pass **both** the legacy and the per-mint BUFFER accounts plus `legacy_market_stats`. The same account list then works before and after the switch.
11. Switch redemptions to worker offers: `create_fulfillment_request` → `fulfill_fulfillment_request`. Test a small buy, sell and redemption on mainnet.
12. Stop creating new legacy redemption requests (frontend/backend). Fulfill or cancel every pending legacy request. Fulfillment needs the legacy offer vectors, while cancel does not.

## D. Switch (one boss transaction)

13. Make sure the kill switch is off and that no legacy redemption requests are pending (step 12).
14. Build **one** Squads transaction:
    1. `switch_onyc_buffer_to_per_mint`: final legacy accrual, then the per-mint BUFFER takes over the APR, fees and high-water mark. This cannot be undone.
    2. `delete_all_offer_vectors` for **every** legacy ONyc offer, including `State::main_offer`.

    Doing both in one transaction leaves no window in which a legacy trade can mint ONyc without accruing.
15. Execute it. Legacy `take_offer_v2`, `take_offer_permissionless_v2`, Prop AMM buy and sell, and redemption fulfillment now fail because there is no active vector. `mint_to` now uses the per-mint BUFFER and caps.

## E. After the switch

16. Worker jobs:
    - replace `settle_buffer` with `settle_managed_token_buffer`;
    - replace `refresh_market_stats` (it now fails) with `refresh_managed_token_supply_exclusions` + `refresh_managed_token_market_stats`.
17. Settings now go through the modular instructions:
    - `set_buffer_gross_apr` / `set_buffer_fee_config` → `set_managed_token_buffer_settings`
    - `burn_for_nav_increase` → `burn_managed_token_for_nav_increase`
    - caps → `set_managed_token_mint_limits` only (the `State` caps no longer matter)
18. Move balances out of the legacy vaults:
    - legacy BUFFER reserve (`withdraw_reserve_vault`) and fee vaults (`withdraw_configurable_vault`) → the per-mint BUFFER vaults;
    - legacy offer and redemption vaults (`offer_vault_withdraw`, `redemption_vault_withdraw`) → the modular vaults.
19. Verify:
    - a legacy `take_offer_v2` fails;
    - a modular `take_offer` and `mint_to` succeed;
    - `MarketStats` NAV equals the Pricer NAV;
    - an immediate `settle_managed_token_buffer` mints nothing.
20. Optional, now safe: a mint multisig (e.g. CCIP) with `enable_managed_token_mint_multisig`.
