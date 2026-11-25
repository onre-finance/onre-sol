# Transition checklist: legacy ONyc offers → modular offers

Do the steps in order. Details are in [MODULAR_TAKE_OFFER.md](MODULAR_TAKE_OFFER.md).

**Zero-downtime rules**

- The legacy offers stay live until every partner has moved. Partners move one at a time, whenever they are ready.
- While both systems run, old and new give the same price, fees and caps, so a partner can switch at any moment and nobody can arbitrage between them.
- Clients build ONyc transactions from on-chain config (the switch flag and `ManagedTokenConfig::mint_multisig`), never from hardcoded account lists. Then no switch needs a client release.
- Every step that changes who can mint or which BUFFER accrues is **one** Squads transaction. If any instruction fails, nothing changes.
- Do not turn on the kill switch during the transition; it stops both systems.

## A. Upgrade (nobody notices)

1. Update every `mint_to` caller (Squads templates, scripts, backend) to pass the nine new trailing accounts, using the program ID as `None`. The current program should ignore extra accounts, so you can do this before the upgrade. Saved transactions with the old account list fail after it.
2. Deploy the program upgrade. ONyc stays on the legacy BUFFER (`State::onyc_per_mint_buffer_active = false`), so legacy offers, the legacy BUFFER and `mint_to` behave exactly as before.

## B. Register ONyc as a managed token (nobody notices)

3. `register_managed_token` for ONyc. Keep minting **direct** (the program PDA stays mint authority), because the legacy instructions can only mint directly.
4. `set_managed_token_mint_limits` to the **same** values as `State` `max_supply` / `max_mint_amount`.
5. `create_pricer` (USD, instance 0) and `add_pricer_vector` with the **same** vector as the main offer (`base_price`, `apr`, `start_time`, `price_fix_duration`).
6. `set_managed_token_supply_exclusions` with the same accounts as the legacy exclusion list. It writes the legacy cache too. From now on, change exclusions only through this instruction.
7. Select ONyc's market stats: `set_managed_token_market_stats` with the legacy `["market_stats"]` singleton, or `initialize_managed_token_market_stats` for a per-mint account.
8. `initialize_managed_token_buffer` for ONyc, and create the ATAs of the vaults it references. It stays inactive until step 18.

## C. Run both side by side (partners move at their own pace)

9. Create the modular offers as mirrors of the legacy ones:
   - `create_quoter` (Prop AMM parameters equal to the legacy `configure_prop_amm`)
   - `create_vault_config` for the fee, proceeds and liquidity vaults
   - `create_fee_config`, with the fees equal to the legacy offer fees (permissioned and permissionless) and a minimum fee of 0
   - `create_offer_config` for USDC → ONyc and USDG → ONyc buys and for worker sell (redemption) offers
10. Fund the modular sell liquidity (`deposit_vault_config`). Keep the legacy redemption vault funded too until step 18.
11. Ship the client and SDK for the modular `take_offer`:
    - same approvers and `ApprovalMessage` for permissioned offers;
    - the same `["permissionless-1"]` intermediary for permissionless offers;
    - for ONyc, always pass **both** the legacy and the per-mint BUFFER accounts plus `legacy_market_stats`, so the step 18 switch needs no client change;
    - pass `mint_multisig` from `ManagedTokenConfig` (`None` while it is unset), so step F needs no client change.
12. Test a small buy, sell and worker redemption (`create_fulfillment_request` → `fulfill_fulfillment_request`) on mainnet.
13. Move your own frontend first, then each partner whenever they are ready. Old and new quote the same price, so nobody sees a gap.
14. Until step 18, change both systems in the **same** transaction:
    - prices: `add_offer_vector` on every legacy ONyc offer + `add_pricer_vector`;
    - fees: `update_offer_fee` / `update_offer_permissionless_fee` + `update_fee_config`;
    - caps: `configure_max_supply` / `configure_max_mint_amount` + `set_managed_token_mint_limits`;
    - exclusions: `set_managed_token_supply_exclusions` only.
15. Wait until legacy volume is zero and every partner has confirmed. Watch `OfferTakenEvent`, `OfferTakenPermissionlessEvent`, the Prop AMM swap events and new legacy redemption requests. Announce the cutoff date.
16. Stop creating legacy redemption requests. Fulfill or cancel every pending one. Fulfillment needs the legacy vectors, but cancelling does not.

## D. Switch (one Squads transaction)

17. Build **one** Squads transaction:
    1. `switch_onyc_buffer_to_per_mint`: final legacy accrual, then the per-mint BUFFER takes over the APR, fees and high-water mark. This cannot be undone.
    2. `delete_all_offer_vectors` for **every** legacy ONyc offer, including `State::main_offer`.

    Keep them in one transaction. Deleting the vectors first would break `mint_to` and `settle_buffer`, while switching first would let a legacy trade mint without accruing.
18. Execute it. Modular `take_offer` keeps working with the same accounts. Legacy `take_offer_v2`, `take_offer_permissionless_v2`, Prop AMM buy and sell, and redemption fulfillment now fail because there is no active vector.

## E. Turn off the old stuff

19. Lock the legacy offers a second time:
    - `set_offer_disabled(true)` on every legacy ONyc offer;
    - `set_redemption_offer_disabled(true)` on every legacy redemption offer, which also blocks new legacy redemption requests.
20. Worker jobs (choose the instruction by the switch flag):
    - replace `settle_buffer` with `settle_managed_token_buffer`;
    - replace `refresh_market_stats` (it now fails) with `refresh_managed_token_supply_exclusions` + `refresh_managed_token_market_stats`.
21. Settings now go through the modular instructions:
    - `set_buffer_gross_apr` / `set_buffer_fee_config` → `set_managed_token_buffer_settings`
    - `burn_for_nav_increase` → `burn_managed_token_for_nav_increase`
    - caps → `set_managed_token_mint_limits` only (the `State` caps no longer matter)
22. Move balances out of the legacy vaults:
    - legacy BUFFER reserve (`withdraw_reserve_vault`) and fee vaults (`withdraw_configurable_vault`) → the per-mint BUFFER vaults;
    - legacy offer and redemption vaults (`offer_vault_withdraw`, `redemption_vault_withdraw`) → the modular vaults.
23. Rebuild any saved `mint_to` Squads transactions. They now need the per-mint accounts, which `buildMintToIx` fills in.
24. Verify:
    - a legacy `take_offer_v2` fails;
    - a modular `take_offer` and `mint_to` succeed;
    - `MarketStats` NAV equals the Pricer NAV;
    - an immediate `settle_managed_token_buffer` mints nothing.

## F. Optional: CCIP mint multisig (any time after step 18)

Not before step 18: legacy instructions only mint directly, so with a multisig they would fail or silently stop minting and burning.

25. Prepare. None of this changes who can mint:
    1. Create an SPL multisig under ONyc's token program (system `create_account` + `InitializeMultisig`) with `m = 1` and these signers:
       - the program's `["mint_authority"]` PDA;
       - the CCIP BurnMint pool's signer PDA;
       - any other minter you want.
    2. Set up the CCIP token pool for ONyc on the Chainlink side.
    3. Check that every client passes `mint_multisig` from `ManagedTokenConfig` (step 11).
26. Make the switch in **one** Squads transaction, in this order:
    1. `transfer_mint_authority_to_boss`: PDA → boss (the Squads vault).
    2. SPL Token `set_authority` (`MintTokens`), signed by the boss: boss → multisig.
    3. `enable_managed_token_mint_multisig` with the multisig. It checks that the multisig is `m = 1`, contains the PDA and holds mint authority.

    Never execute instruction 1 on its own, because then the program cannot mint at all.
27. Verify a modular `take_offer`, `mint_to` and `settle_managed_token_buffer`.
    - Rollback: `disable_managed_token_mint_multisig` alone. It reclaims mint authority to the PDA by signing as the multisig member.
    - From now on, every ONyc instruction that mints must pass the multisig. Before the flip it must be `None` (`UnexpectedMintMultisig`), and after it, it is required (`MintMultisigRequired`). A transaction built just before the flip that lands just after it fails once and must be rebuilt.
    - CCIP mints are outside ONyc's per-mint caps. The BUFFER accrues on them only from the next settle or trade.
