# Onre Program Integration Guide

Read canonical ONyc market metrics from the `MarketStats` PDA.

---

## Quick Overview

Market metrics are read over RPC from the singleton PDA derived with `["market_stats"]`. A read requires no instruction, transaction, or signature. The account stores a snapshot calculated from `state.main_offer`, not a live calculation for an arbitrary offer.

This branch removes the seven market getter instructions (`get_nav`, `get_apy`, `get_nav_adjustment`, `get_tvl`, `get_tvl_v2`, `get_circulating_supply`, and `get_circulating_supply_v2`). Existing callers must migrate before this build is deployed as an upgrade. Use an IDL that matches the deployed program; the local IDL describes this branch.

For BUFFER integrations, keep in mind that BUFFER accrual does not accept a caller-provided current yield. Instead, `current_yield` is derived from the active APR on `state.main_offer`, even when the surrounding trade or redemption is priced by another offer.

**Program ID (Mainnet):** `onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe`

---

## Operational Kill Switch

Integrations should treat `state.is_killed == true` as an emergency stop for guarded value-moving paths. While active, the program rejects offer takes, Prop AMM quotes/execution, redemption request create/fulfill/cancel, vault deposits/withdrawals, reserve vault deposits/withdrawals, configurable-vault withdrawals, direct `mint_to`, `burn_for_nav_increase`, and BUFFER config updates that would settle accrual.

RPC reads and `refresh_market_stats` remain available while the kill switch is active. Governance and configuration-only instructions also remain callable according to their normal access control.

---

## Getting Started

### 1. Get the IDL

Download the program IDL from:
- Location: `target/idl/onreapp.json`
- Or fetch from chain: `anchor idl fetch <PROGRAM_ID>`

### 2. Install Dependencies

```bash
npm install @coral-xyz/anchor @solana/web3.js
```

### 3. Initialize the Program

```typescript
import { Program, AnchorProvider } from "@coral-xyz/anchor";
import { Connection, PublicKey } from "@solana/web3.js";
import idl from "./onreapp.json";

const connection = new Connection("https://api.mainnet-beta.solana.com");
const provider = new AnchorProvider(connection, wallet);
const program = new Program(idl, provider);
```

---

## Read the MarketStats PDA

```typescript
const [marketStatsPda] = PublicKey.findProgramAddressSync(
  [Buffer.from("market_stats")],
  program.programId
);
const stats = await program.account.marketStats.fetch(marketStatsPda);

// Keep u64/i64 values as BN, bigint, or decimal strings to avoid precision loss.
console.log({
  nav: stats.nav.toString(),
  apy: stats.apy.toString(),
  navAdjustment: stats.navAdjustment.toString(),
  circulatingSupply: stats.circulatingSupply.toString(),
  tvl: stats.tvl.toString(),
  lastUpdatedAt: stats.lastUpdatedAt.toString(),
  lastUpdatedSlot: stats.lastUpdatedSlot.toString(),
});
```

| Field | Meaning and units |
| --- | --- |
| `nav` | Main offer price, scale 9 (`1_000_000_000` = 1) |
| `apy` | Daily-compounded annual yield, scale 6 (`1_000_000` = 100%) |
| `navAdjustment` | Signed price adjustment between vectors, scale 9 |
| `circulatingSupply` | ONyc mint supply minus cached excluded balance, in ONyc base units |
| `tvl` | `circulatingSupply * nav / 1_000_000_000`; scale follows ONyc decimals, not the input mint's decimals |
| `lastUpdatedAt` | Unix seconds of the last recomputation |
| `lastUpdatedSlot` | Slot of the last recomputation |

APY uses `(1 + APR / 365)^365 - 1`. For the first vector, NAV adjustment is its starting step price; later vectors compare their starting step price with the preceding vector's price at the transition.

### Freshness and scope

Fetching the account does not recompute prices or supply. Time, mint supply, main-offer configuration, or excluded balances can change while the stored snapshot remains unchanged. Consumers should check `lastUpdatedAt` and `lastUpdatedSlot` against their own freshness requirements and refresh when needed. The PDA does not store the source offer address; refresh after changing `state.main_offer` before publishing metrics for the new offer.

Supply and TVL use `["circ_supply_excl_balance"]`. If excluded ONyc ATA balances or the configured owner list changed, call `update_circulating_supply_excluded_balance` with the configured owners' ATAs in order **before** `refresh_market_stats`. Both instructions can be included in one transaction. Refreshing market stats alone does not refresh that balance cache, and its timestamp does not certify the exclusion cache's freshness. An uninitialized excluded-balance PDA is treated as zero.

This is the canonical ONyc snapshot. It does not reproduce legacy pair-specific views, automatic subtraction of the offer-vault ATA, or getter-event metadata such as the next price-change timestamp. Configure excluded owners explicitly; read the offer's vectors separately when per-offer pricing details are needed. Use the execution or quote instructions for trading.

### Refresh the snapshot

`refresh_market_stats` is permissionless. The signer pays the transaction fee and any rent needed to create the PDA on its first refresh. The main offer must be configured, output ONyc, and have an active vector; ONyc must use classic SPL Token.

```typescript
import { SystemProgram } from "@solana/web3.js";

const [statePda] = PublicKey.findProgramAddressSync(
  [Buffer.from("state")], program.programId
);
const [circulatingSupplyExcludedBalance] = PublicKey.findProgramAddressSync(
  [Buffer.from("circ_supply_excl_balance")], program.programId
);
const state = await program.account.state.fetch(statePda);
const mainOffer = await program.account.offer.fetch(state.mainOffer);

// If exclusions changed, update their cached balance before this instruction.
await program.methods.refreshMarketStats().accountsStrict({
  mainOffer: state.mainOffer,
  tokenInMint: mainOffer.tokenInMint,
  state: statePda,
  onycMint: state.onycMint,
  circulatingSupplyExcludedBalance,
  marketStats: marketStatsPda,
  signer: provider.wallet.publicKey,
  systemProgram: SystemProgram.programId,
}).rpc();

const refreshed = await program.account.marketStats.fetch(marketStatsPda);
```

Before initialization, fetching `MarketStats` fails because the account does not exist. Refresh it first, or handle the missing account explicitly in the application.

### CLI

```bash
pnpm cli market fetch --json
pnpm cli market refresh --token-in usdc
```

`market fetch` replaces `nav`, `nav-adjustment`, `apy`, `tvl`, `tvl-v2`, `supply`, and `supply-v2`. It reads all five metrics and freshness metadata without a transaction and prints raw integer values as strings. `market refresh` submits a transaction; the input mint must match the configured main offer.

---

## PDA Derivations

All PDAs use the program ID as the base. Here are the derivation seeds:

### State PDA
```typescript
const [statePda] = PublicKey.findProgramAddressSync(
  [Buffer.from("state")],
  programId
);
```

### Offer PDA
```typescript
const [offerPda] = PublicKey.findProgramAddressSync(
  [
    Buffer.from("offer"),
    tokenInMint.toBuffer(),
    tokenOutMint.toBuffer()
  ],
  programId
);
```

### Offer Vault Authority PDA
```typescript
const [vaultAuthority] = PublicKey.findProgramAddressSync(
  [Buffer.from("offer_vault_authority")],
  programId
);
```

### Redemption Request PDA

`create_redemption_request(amount, requestId)` takes a frontend-generated random ID of exactly 32 UTF-8 bytes. Generate it as `crypto.randomUUID().replaceAll("-", "")`; the request ID is used directly as a PDA seed.

```typescript
const [redemptionRequestPda] = PublicKey.findProgramAddressSync(
  [
    Buffer.from("redemption_request"),
    redemptionOfferPda.toBuffer(),
    redeemer.toBuffer(),
    Buffer.from(requestId, "utf8"),
  ],
  programId,
);
```

Persist `requestId`, `redeemer`, and `redemptionRequestPda` with the request. The worker can use the PDA to fulfill the request; the frontend can use the same tuple to derive it for cancellation.

This is a breaking request-account and instruction change. Resolve every pending redemption request before upgrading.

---

## BUFFER Integration Notes

If your integration touches BUFFER:

- `initialize_buffer` must be given an offer account, and that offer's `token_out_mint` must be the ONyc mint
- `set_main_offer` changes the canonical offer used by every BUFFER accrual and ONYC market-stat refresh path
- `set_buffer_gross_apr` accepts values from 0 through `1_000_000` (0% through 100%), first settles pending BUFFER accrual and refreshes market stats, then updates `gross_apr`
- BUFFER accrual reads `current_yield` and current NAV from the active vector on `state.main_offer`; a separate traded offer only determines user execution pricing
- `set_buffer_gross_apr`, `set_buffer_fee_config`, and `burn_for_nav_increase` require the boss signer to be writable because it pays for lazy `MarketStats` initialization; callers may run permissionless `refresh_market_stats` first or let the first of these boss instructions create the PDA

### Recommended BUFFER Rollout

Recommended rollout sequence for enabling BUFFER on an already-running deployment:

1. upgrade the program
2. let integrators/backend switch to the BUFFER-aware instruction account sets
3. stop using clients built against the legacy fulfillment account set
4. upgrade the program again to remove or disable the legacy paths
5. initialize BUFFER

Operational note:

- `fulfill_redemption_request` is designed to work before BUFFER is initialized
- before BUFFER initialization, the BUFFER-aware path behaves as a no-accrual redemption flow
- after BUFFER is initialized, set `gross_apr` deliberately as part of activation so accrual starts only when you intend it to

### Vault Token Accounts (ATAs)
```typescript
import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";

const vaultTokenAccount = getAssociatedTokenAddressSync(
  tokenMint,           // The token mint
  vaultAuthority,      // The vault authority PDA
  true,                // allowOwnerOffCurve = true
  TOKEN_PROGRAM_ID     // Or TOKEN_2022_PROGRAM_ID
);
```

---

## Token Addresses

**Mainnet:**
- **USDC:** `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`
- **ONyc:** `5Y8NV33Vv7WbnLfq3zBcKSdYPrk7g2KoiQoe7M2tcxp5`
- **USDG:** `2u1tszSeqZ3qBWF3uNGPFc8TzMk2tdiwknnRMWGWjGWH`

---

## Scale Conversions

| Data | Scale | Conversion | Example |
|------|-------|------------|---------|
| NAV/Price | 9 decimals | `value / 1_000_000_000` | `1005000000 → 1.005` |
| APY/APR | 6 decimals | `(value / 1_000_000) * 100` | `105000 → 10.5%` |
| ONyc Amount | 9 decimals | `value / 1_000_000_000` | `1000000000 → 1 ONyc` |
| USDC Amount | 6 decimals | `value / 1_000_000` | `1000000 → 1 USDC` |

---

## Reading without a signer

Only refresh needs a transaction signer. RPC account readers can decode the fetched bytes using the IDL's account coder, or use an Anchor client configured for account reads. Do not use `.view()` or transaction simulation to retrieve these metrics.

---

## Need Help?

Check the full IDL for all available instructions and account structures.

For operational examples, use the CLI under `scripts/cli/` or the mainnet browser UI under `scripts/ui/`.
