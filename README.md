# OnreApp Solana Program

A Solana smart contract built with [Anchor](https://www.anchor-lang.com/) that manages tokenized (re)insurance pools. The program enables the creation, management, and redemption of **ONyc tokens**, which represent fractional ownership in a regulated investment pool specializing in (re)insurance underwriting.

## Quick Start

```bash
# Build the program
anchor build

# Run tests (builds program and copies .so to fixtures)
anchor test

# Run a single test file
npx vitest run tests/path/to/test.spec.ts

# Update program ID after changing keypair
anchor keys sync && anchor build
```

## Program Structure

```
programs/onreapp/src/
├── lib.rs                    # Entry point — all 32 instructions declared here
├── state.rs                  # Global State account (boss, admins, approvers, kill switch)
├── constants.rs              # PDA seeds, limits, decimals
├── utils/                    # Token helpers, ed25519 signature parsing, approver verification
└── instructions/
    ├── initialization/       # initialize, initialize_permissionless_authority
    ├── offer/                # make/take/close offers, manage price vectors, fees
    ├── redemption/           # redemption offers, requests, fulfillment, cancellation
    ├── state_operations/     # Boss transfer, admin/approver management, kill switch, max supply
    ├── vault_operations/     # Deposit/withdraw tokens to offer and redemption vaults
    ├── mint_authority/       # Transfer mint authority to/from program PDA, mint_to
    └── market_info/          # Read-only queries: NAV, APY, TVL, circulating supply, NAV adjustment
```

## Key Concepts

### Dynamic Pricing

Offers use up to 10 `OfferVector` entries with APR-based compound interest. Price grows over time using `base_price`, `apr` (scale = 6, where 1,000,000 = 1%), and `price_fix_duration`.

### Authority Structure

| Role | Description |
|------|-------------|
| `boss` | Primary authority with full control (two-step transfer via propose/accept) |
| `admins[20]` | Can enable the kill switch |
| `redemption_admin` | Manages redemption operations |
| `approvers` | Trusted keys for cryptographic approval verification (ed25519) |

### Token Support

The program supports both **SPL Token** and **Token-2022** with transfer fee extensions.

### Constants

| Constant | Value |
|----------|-------|
| `MAX_VECTORS` | 10 |
| `MAX_ADMINS` | 20 |
| `PRICE_DECIMALS` | 9 |
| `MAX_ALLOWED_FEE_BPS` | 1000 (10%) |

## Instructions

**Initialization**: `initialize`, `initialize_permissionless_authority`

**Offers**: `make_offer`, `add_offer_vector`, `delete_offer_vector`, `delete_all_offer_vectors`, `update_offer_fee`, `take_offer`, `take_offer_permissionless`

**Redemption**: `make_redemption_offer`, `create_redemption_request`, `fulfill_redemption_request`, `cancel_redemption_request`, `update_redemption_offer_fee`

**State Operations**: `propose_boss`, `accept_boss`, `add_admin`, `remove_admin`, `clear_admins`, `set_kill_switch`, `set_onyc_mint`, `set_redemption_admin`, `add_approver`, `remove_approver`, `configure_max_supply`, `close_state`

**Vault Operations**: `offer_vault_deposit`, `offer_vault_withdraw`, `redemption_vault_deposit`, `redemption_vault_withdraw`

**Mint Authority**: `transfer_mint_authority_to_program`, `transfer_mint_authority_to_boss`, `mint_to`

**Market Info** (read-only): `get_nav`, `get_apy`, `get_nav_adjustment`, `get_tvl`, `get_circulating_supply`

## CLI Tool

An interactive CLI for managing deployed programs on mainnet/devnet.

```bash
# Run the CLI
pnpm cli

# Or with a specific network
pnpm script:mainnet-prod tsx scripts/cli/index.ts
```

### Network Environments

| Profile | Cluster | Description |
|---------|---------|-------------|
| `mainnet-prod` | Mainnet | Production program |
| `mainnet-test` | Mainnet | Test program on mainnet |
| `mainnet-dev` | Mainnet | Dev program on mainnet |
| `devnet-test` | Devnet | Test program on devnet |
| `devnet-dev` | Devnet | Dev program on devnet |

Select via `NETWORK` env variable or the `-n` / `--network` flag. Convenience scripts:

```bash
pnpm script:mainnet-prod tsx scripts/some-script.ts
pnpm script:devnet-dev tsx scripts/some-script.ts
```

### Standalone Scripts

Scripts can also be run directly with `tsx`:

```bash
tsx scripts/utils/get-state.ts
tsx scripts/offer/fetch-offer.ts
tsx scripts/market_info/get-nav.ts
```

Scripts that modify state output base58-encoded transactions for signing via Squad multisig. Read-only scripts print results directly.

## Tests

Tests use **Vitest** with **LiteSVM** for fast local testing without a validator.

```bash
# Run all tests
pnpm test

# Run with watch mode
pnpm test:watch

# Run a single test file
npx vitest run tests/offer/take_offer.spec.ts
```

Test structure mirrors the instruction layout:

```
tests/
├── test_helper.ts              # TestHelper class (LiteSVM utilities)
├── onre_program.ts             # Shared program setup
├── offer/                      # Offer instruction tests
├── redemption/                 # Redemption instruction tests
├── state_operations/           # State management tests
├── vault_operations/           # Vault operation tests
├── mint_authority/             # Mint authority tests
└── market_info/                # Market info query tests
```

## Cross-Chain Transfers

The `scripts/cross_chain_transfer/` directory contains CCTP v1 and v2 implementations for cross-chain USDC transfers between Ethereum and Solana.

## Updating the Program ID

```bash
cp ~/.config/solana/<keypair>.json target/deploy/onreapp-keypair.json
anchor keys sync
anchor build
```

Program ID convenience scripts:

```bash
pnpm set-program:dev
pnpm set-program:test
pnpm set-program:prod
```