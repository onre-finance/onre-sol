# Onre Program Integration Guide

Simple guide for integrating NAV and APY queries into your application.

---

## Quick Overview

The Onre program provides **read-only view instructions** to query market data. Use the program IDL and standard Anchor client libraries to make these calls.

For BUFFER integrations, keep in mind that BUFFER accrual does not accept a caller-provided current yield. Instead, `current_yield` is derived from the active APR on `state.main_offer`, even when the surrounding trade or redemption is priced by another offer.

**Program ID (Mainnet):** `onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe`

---

## Operational Kill Switch

Integrations should treat `state.is_killed == true` as an emergency stop for guarded value-moving paths. While active, the program rejects offer takes, Prop AMM quotes/execution, redemption request create/fulfill/cancel, vault deposits/withdrawals, reserve vault deposits/withdrawals, configurable-vault withdrawals, direct `mint_to`, `burn_for_nav_increase`, and BUFFER config updates that would settle accrual.

Read-only market views such as `get_nav`, `get_apy`, `get_tvl_v2`, and `get_circulating_supply_v2` are not kill-switch guarded. Governance and configuration-only instructions also remain callable according to their normal access control.

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

## Available View Instructions

### 1. Get NAV (Current Price)

**Instruction:** `get_nav`

**Returns:** Current price with 9 decimals (divide by `1_000_000_000`)

**Accounts:**
```typescript
{
  offer: PublicKey,        // PDA: ["offer", tokenInMint, tokenOutMint]
  tokenInMint: PublicKey,  // Input token mint (e.g., USDC)
  tokenOutMint: PublicKey  // Output token mint (e.g., ONyc)
}
```

**Example:**
```typescript
const tokenInMint = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"); // USDC
const tokenOutMint = new PublicKey("5Y8NV33Vv7WbnLfq3zBcKSdYPrk7g2KoiQoe7M2tcxp5"); // ONyc

const nav = await program.methods
  .getNav()
  .accounts({
    tokenInMint,
    tokenOutMint
  })
  .view();

const price = nav.toNumber() / 1_000_000_000;
console.log(`Price: ${price}`); // e.g., 1.005
```

---

### 2. Get APY (Annual Yield)

**Instruction:** `get_apy`

**Returns:** APY with 6 decimals (divide by `1_000_000`, multiply by 100 for percentage)

**Accounts:**
```typescript
{
  offer: PublicKey,        // PDA: ["offer", tokenInMint, tokenOutMint]
  tokenInMint: PublicKey,
  tokenOutMint: PublicKey
}
```

**Example:**
```typescript
const apy = await program.methods
  .getApy()
  .accounts({
    tokenInMint,
    tokenOutMint
  })
  .view();

const apyPercent = (apy.toNumber() / 1_000_000) * 100;
console.log(`APY: ${apyPercent.toFixed(2)}%`); // e.g., 10.50%
```

---

### 3. Get TVL (Total Value Locked)

**Recommended instruction:** `get_tvl_v2`

**Returns:** `circulating_supply * current_price / 10^9`

**Accounts:**
```typescript
{
  offer: PublicKey,          // PDA: ["offer", tokenInMint, tokenOutMint]
  tokenInMint: PublicKey,
  tokenOutMint: PublicKey,   // must be the configured ONyc mint
  state: PublicKey,          // PDA: ["state"]
  circulatingSupplyExcludedBalance: PublicKey // PDA: ["circ_supply_excl_balance"]
}
```

**Example:**
```typescript
const [circulatingSupplyExcludedBalance] = PublicKey.findProgramAddressSync(
  [Buffer.from("circ_supply_excl_balance")],
  program.programId
);
const [statePda] = PublicKey.findProgramAddressSync(
  [Buffer.from("state")],
  program.programId
);

const tvl = await program.methods
  .getTvlV2()
  .accounts({
    tokenInMint,
    tokenOutMint,
    state: statePda,
    circulatingSupplyExcludedBalance
  })
  .view();

console.log(`TVL: ${tvl.toString()}`);
```

---

### 4. Get Circulating Supply

**Recommended instruction:** `get_circulating_supply_v2`

**Returns:** Current circulating supply of ONyc

**Accounts:**
```typescript
{
  state: PublicKey,           // PDA: ["state"]
  onycMint: PublicKey,        // From state.onyc_mint
  circulatingSupplyExcludedBalance: PublicKey  // PDA: ["circ_supply_excl_balance"]
}
```

**Example:**
```typescript
// Derive state PDA
const [statePda] = PublicKey.findProgramAddressSync(
  [Buffer.from("state")],
  program.programId
);

// Fetch state to get ONyc mint
const state = await program.account.state.fetch(statePda);
const onycMint = state.onycMint;

const [circulatingSupplyExcludedBalance] = PublicKey.findProgramAddressSync(
  [Buffer.from("circ_supply_excl_balance")],
  program.programId
);

const supply = await program.methods
  .getCirculatingSupplyV2()
  .accounts({
    state: statePda,
    onycMint,
    circulatingSupplyExcludedBalance
  })
  .view();

console.log(`Circulating Supply: ${supply.toString()}`);
```

`get_tvl` retains its pre-V5 account layout and remains a pair-specific legacy
view that subtracts the offer-vault ATA directly. `get_circulating_supply`
remains the legacy ONyc supply view. The V2 views use the cached ONyc
excluded-balance PDA, which is also what `refresh_market_stats` and the Prop AMM
paths use.

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

## Complete Example

```typescript
import { Program, AnchorProvider } from "@coral-xyz/anchor";
import { Connection, PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import idl from "./onreapp.json";

const connection = new Connection("https://api.mainnet-beta.solana.com");
const provider = new AnchorProvider(connection, wallet);
const program = new Program(idl, provider);

// Token mints
const USDC = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
const ONYC = new PublicKey("5Y8NV33Vv7WbnLfq3zBcKSdYPrk7g2KoiQoe7M2tcxp5");

async function getMarketData() {
  // Get NAV
  const nav = await program.methods
    .getNav()
    .accounts({ tokenInMint: USDC, tokenOutMint: ONYC })
    .view();

  const price = nav.toNumber() / 1e9;

  // Get APY
  const apy = await program.methods
    .getApy()
    .accounts({ tokenInMint: USDC, tokenOutMint: ONYC })
    .view();

  const apyPercent = (apy.toNumber() / 1e6) * 100;

  console.log(`Price: ${price}`);
  console.log(`APY: ${apyPercent.toFixed(2)}%`);
}

getMarketData();
```

---

## Notes

- All view instructions are **read-only** (no state changes, no fees)
- No wallet/signing required for view calls
- Accounts are automatically resolved by Anchor if you only pass the required ones
- The `offer` PDA is usually auto-derived by Anchor from the seeds constraint

---

## Need Help?

Check the full IDL for all available instructions and account structures.

For operational examples, use the CLI under `scripts/cli/` or the mainnet browser UI under `scripts/ui/`.
