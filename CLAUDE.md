# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands

```bash
# Build the Anchor program
anchor build

# Run all tests (builds program and copies to fixtures)
anchor test

# Run a single test file
npx jest --runInBand tests/path/to/test.spec.ts

# Update program ID after changing keypair
anchor keys sync && anchor build
```

**Note**: Do NOT use `anchor test -- --testNamePattern=...` - it doesn't work.

## Project Overview

This is a Solana smart contract built with Anchor that manages tokenized (re)insurance pools. The program enables:
- **Token Offers**: Dynamic pricing for token exchanges using time-based APR vectors
- **Redemptions**: Converting ONyc tokens back to USDC at NAV price
- **Governance**: Boss/admin hierarchy with kill switch emergency controls

## Architecture

### Program Structure (programs/onreapp/src/)

- **lib.rs**: Entry point defining all program instructions
- **state.rs**: Global `State` account with boss, admins, approvers, kill switch
- **constants.rs**: Seeds for PDAs, limits (MAX_VECTORS=10, MAX_ADMINS=20, PRICE_DECIMALS=9)
- **instructions/**: Organized by domain:
  - `offer/` - Make/take/close offers, manage price vectors
  - `redemption/` - Redemption offers and requests
  - `state_operations/` - Boss transfer, admin management, kill switch
  - `vault_operations/` - Deposit/withdraw tokens to vaults
  - `mint_authority/` - Transfer mint authority to/from program PDA
  - `market_info/` - Read-only queries (NAV, APY, TVL, circulating supply)

### Key Concepts

**Dynamic Pricing**: Offers use `OfferVector` arrays with APR-based compound interest. Price grows over time using `base_price`, `apr` (scale=6, 1_000_000 = 1%), and `price_fix_duration`.

**Authority Structure**:
- `boss`: Primary authority with full control
- `admins[]`: Can enable kill switch
- `redemption_admin`: Manages redemption operations
- `approvers`: Trusted keys for cryptographic approval verification

**PDAs**: Seeds defined in `constants::seeds` - STATE, OFFER, OFFER_VAULT_AUTHORITY, PERMISSIONLESS_AUTHORITY, MINT_AUTHORITY, REDEMPTION_OFFER, etc.

### Test Infrastructure (tests/)

Uses **solana-bankrun** + **anchor-bankrun** for fast local testing without a validator.

- `test_helper.ts`: `TestHelper` class provides utilities for creating mints, token accounts, advancing clock time
- Tests mirror the instruction structure (offer/, redemption/, state_operations/, etc.)
- `onre_program.ts`: Shared program setup

## Client Scripts (scripts/)

Scripts for interacting with deployed programs on mainnet/testnet. Used for debugging and controlling live programs.

### Running Scripts

```bash
# Run any script with tsx
tsx scripts/utils/get-state.ts
tsx scripts/offer/fetch-offer.ts
tsx scripts/market_info/get-nav.ts
```

### Network Configuration

Edit `scripts/utils/script-helper.ts` to switch networks:
- `RPC_URL`: Set to mainnet, devnet, or custom RPC
- `BOSS`: Uncomment the appropriate Squad multisig address
- Token mints: Use `USDC_MINT`/`ONYC_MINT` for mainnet, `*_DEVNET` for devnet, `*_TEST_MAINNET` for test tokens on mainnet

### Script Categories

| Folder | Purpose |
|--------|---------|
| `utils/` | Read-only queries: `get-state.ts`, `get-boss.ts` |
| `offer/` | Offer management: `make-offer.ts`, `fetch-offer.ts`, `add-offer-vector.ts`, `close-offer.ts` |
| `market_info/` | NAV, APY, TVL, circulating supply queries |
| `state_operations/` | Boss transfer, admin/approver management, kill switch |
| `vault_operations/` | Deposit/withdraw tokens to vaults |
| `mint_authority/` | Transfer mint authority to/from program |
| `initialization/` | Initialize program state and permissionless authority |
| `cross_chain_transfer/` | CCTP v1/v2 for cross-chain USDC transfers |

### ScriptHelper Class

All scripts use `ScriptHelper` from `scripts/utils/script-helper.ts`:
- `ScriptHelper.create()` - Read-only operations (no wallet needed)
- `ScriptHelper.createWithLocalWallet("id")` - Uses `~/.config/solana/id.json`
- Builds unsigned transactions as base58 for external signing (Squad multisig)

### Output Format

Scripts that modify state output base58-encoded transactions for signing via Squad or other multisig wallets. Read-only scripts print results directly.

## Common Issues

- **"byte array longer than desired length"**: BN serialization issue - check number formatting
- **"Transaction already processed"**: Bankrun optimizes identical transactions. Differentiate transactions by changing amount or fee payer when running in loops.
- **Testing failures**: Avoid try/catch blocks. Use `await expect(program.methods...).rejects.toThrow("Error message")` for failure cases.

## Token Standards

The program supports both SPL Token and Token-2022 with transfer fee extensions. Test helper provides `createMint2022WithTransferFee()` for Token-2022 mints.
