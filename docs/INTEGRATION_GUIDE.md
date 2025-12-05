# Onre Program Integration Guide

Simple guide for integrating NAV and APY queries into your application.

---

## Quick Overview

The Onre program provides **read-only view instructions** to query market data. Use the program IDL and standard Anchor client libraries to make these calls.

**Program ID (Mainnet):** `[INSERT_PROGRAM_ID_HERE]`

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

**Instruction:** `get_tvl`

**Returns:** Total tokens locked in vault (raw amount with token decimals)

**Accounts:**
```typescript
{
  offer: PublicKey,               // PDA: ["offer", tokenInMint, tokenOutMint]
  tokenInMint: PublicKey,
  tokenOutMint: PublicKey,
  vaultTokenOutAccount: PublicKey, // ATA: (tokenOutMint, vaultAuthority)
  tokenOutProgram: PublicKey       // Usually TOKEN_PROGRAM_ID
}
```

**Example:**
```typescript
import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";

// Derive vault authority PDA
const [vaultAuthority] = PublicKey.findProgramAddressSync(
  [Buffer.from("offer_vault_authority")],
  program.programId
);

// Derive vault token account
const vaultTokenOutAccount = getAssociatedTokenAddressSync(
  tokenOutMint,
  vaultAuthority,
  true,
  TOKEN_PROGRAM_ID
);

const tvl = await program.methods
  .getTvl()
  .accounts({
    tokenInMint,
    tokenOutMint,
    vaultTokenOutAccount,
    tokenOutProgram: TOKEN_PROGRAM_ID
  })
  .view();

console.log(`TVL: ${tvl.toString()}`);
```

---

### 4. Get Circulating Supply

**Instruction:** `get_circulating_supply`

**Returns:** Current circulating supply of ONyc

**Accounts:**
```typescript
{
  state: PublicKey,           // PDA: ["state"]
  onycMint: PublicKey,        // From state.onyc_mint
  onycVaultAccount: PublicKey, // ATA: (onycMint, vaultAuthority)
  tokenProgram: PublicKey      // Usually TOKEN_PROGRAM_ID
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

// Derive vault authority
const [vaultAuthority] = PublicKey.findProgramAddressSync(
  [Buffer.from("offer_vault_authority")],
  program.programId
);

// Derive ONyc vault account
const onycVaultAccount = getAssociatedTokenAddressSync(
  onycMint,
  vaultAuthority,
  true,
  TOKEN_PROGRAM_ID
);

const supply = await program.methods
  .getCirculatingSupply()
  .accounts({
    onycVaultAccount,
    tokenProgram: TOKEN_PROGRAM_ID
  })
  .view();

console.log(`Circulating Supply: ${supply.toString()}`);
```

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

**Reference Scripts:** `scripts/market_info/get-nav.ts`, `scripts/market_info/get-apy.ts`
