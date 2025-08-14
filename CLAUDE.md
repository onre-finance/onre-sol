# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Solana program built with the Anchor framework for the OnRe App - a tokenized (re)insurance platform. The program manages ONe tokens representing fractional ownership in regulated (re)insurance pools, facilitating minting, burning, and offer-based token exchanges with dynamic pricing.

## Architecture

### Core Components
- **Solana Program** (`programs/onreapp/`): Core smart contract logic written in Rust using Anchor framework
- **TypeScript Client** (`scripts/`): Client-side scripts for program interactions
- **Tests** (`tests/`): Jest-based test suite using solana-bankrun
- **Cross-chain Integration** (`scripts/cross_chain_transfer/`): CCTP (Circle Cross-Chain Transfer Protocol) integration for USDC transfers between Ethereum and Solana

### Program Structure
The Solana program is organized into:
- `lib.rs`: Main program entry point with instruction definitions
- `state.rs`: Account structures (Offer, State) 
- `instructions/`: Business logic for each program instruction
- `contexts/`: Account validation and constraints for instructions

### Key Features
- **Dynamic Pricing**: Offer prices change linearly over time based on configurable parameters
- **Dual Token Offers**: Support for offers with one or two buy tokens
- **Access Controls**: Boss-based authorization system
- **Event Emission**: On-chain events for off-chain tracking

## Development Commands

### Building and Testing
```bash
# Build the Anchor program
anchor build

# Run tests (copies program binary to test fixtures)
npm test
# or
anchor test

# Build with program binary copy for tests
npm run anchor-build
```

### Program Management
```bash
# Set program ID for different environments
npm run set-program:dev    # Development
npm run set-program:test   # Test
npm run set-program:prod   # Production

# Update program ID in config files
anchor keys sync
```

### Cross-chain Operations
```bash
# Transfer USDC between Ethereum and Solana using CCTP v2
npm run transfer-usdc-v2
```

## Testing

Tests use Jest with solana-bankrun for fast local blockchain simulation. Test files are in `tests/` and use the program binary from `tests/fixtures/onreapp.so`.

Key test patterns:
- Use `test_helper.ts` for common test utilities
- Tests run with 30-second timeout due to blockchain operations
- Program binary must be built before running tests

## Program Instructions

### Core Instructions
- `make_offer_one`/`make_offer_two`: Create offers with dynamic pricing
- `take_offer_one`/`take_offer_two`: Accept offers at current price
- `close_offer_one`/`close_offer_two`: Close and cleanup offers
- `initialize`: Set up program state with initial boss
- `set_boss`: Update boss authority
- `get_nav`: Get current Net Asset Value
- `get_current_offer`: Get current offer ID

### Dynamic Pricing Model
Offers use linear interpolation between start/end amounts over the offer duration, with discrete pricing intervals defined by `price_fix_duration`.

## Key Files and Directories

- `Anchor.toml`: Anchor framework configuration
- `programs/onreapp/src/`: Solana program source code
- `scripts/`: Client interaction scripts and utilities
- `tests/`: Test suite with Jest configuration
- `target/`: Build outputs including program IDL and types
- `scripts/cross_chain_transfer/`: CCTP integration for cross-chain USDC transfers

## Program ID
Current program ID: `onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe`

The program ID is managed through keypair files in `~/.config/solana/` and synchronized using `anchor keys sync`.