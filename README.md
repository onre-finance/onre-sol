# OnreApp Solana Program Documentation

## Overview

The **OnreApp Solana Program** is a custom smart contract built using the [Anchor framework](https://project-serum.github.io/anchor/), designed to manage tokenized (re)insurance pools and related operations on the Solana blockchain. This program facilitates the creation, management, and redemption of **ONe tokens**, which represent fractional ownership in a regulated investment pool specializing in (re)insurance underwriting.

The program integrates blockchain technology to provide **transparency, liquidity, and efficiency** to the traditionally exclusive (re)insurance market. It supports key operations such as minting, burning, and managing offers, while ensuring compliance with Bermuda’s regulatory framework.

## Directory Structure
The program's source code is organized as follows:
```plaintext
programs/onreapp/src/ 
├── contexts/ 
│ ├── mod.rs 
│ └── offer_context.rs 
├── instructions/ 
│ ├── close_offer.rs 
│ ├── initialize.rs 
│ ├── make_offer.rs 
│ ├── mod.rs 
│ ├── set_boss.rs 
│ └── take_offer.rs 
├── lib.rs 
└── state.rs
```

### Key Components

#### 1. `lib.rs`
This is the main entry point for the program. It defines the modules, instructions, and accounts used in the program. It acts as the glue that ties together the contexts, instructions, and state.

#### 2. `state.rs`
This file defines the program's state, including account structures and data models. These structures represent the persistent data stored on-chain, such as:

- **Offer Account**: Tracks details about offers, including terms, participants, and status.
- **Admin Account**: Stores information about the program's administrator and fund manager controls.

#### 3. `contexts/`
This directory contains context definitions for the program's instructions. Contexts define the accounts and constraints required for each instruction.

- **`mod.rs`**: Re-exports the context modules for easier access.
- **`offer_context.rs`**: Defines the context for operations related to offers, such as creating or taking an offer.

#### 4. `instructions/`
This directory contains the core logic for the program's instructions. Each file corresponds to a specific instruction.

- **`initialize.rs`**: Handles the initialization of program accounts.
- **`make_offer.rs`**: Implements the logic for creating an offer.
- **`take_offer.rs`**: Implements the logic for accepting or taking an offer.
- **`close_offer.rs`**: Implements the logic for closing an offer.
- **`set_boss.rs`**: Implements the logic for assigning an administrative role.
- **`mod.rs`**: Re-exports the instruction modules for easier access.

---

## Instructions

### 1. Initialize
**File**: `initialize.rs`

Initializes the program or specific accounts required for its operation. This includes setting up the admin account and preparing the program for subsequent operations.

### 2. Make Offer
**File**: `make_offer.rs`

Allows a user to create an offer. This includes specifying the terms of the offer and storing it on-chain. Offers represent opportunities for investors to participate in the tokenized (re)insurance pool.

### 3. Take Offer
**File**: `take_offer.rs`

Allows a user to accept an existing offer. This involves transferring assets or fulfilling the terms of the offer, enabling participation in the investment pool.

### 4. Close Offer
**File**: `close_offer.rs`

Allows a user to close an existing offer, removing it from the program's state. This is typically used when an offer is no longer valid or has been fulfilled.

### 5. Set Boss
**File**: `set_boss.rs`

Assigns or updates the administrative role for the program. This role is responsible for managing fund operations, including minting and burning tokens.

---

## Token Mechanics

The **ONe token** is the primary asset managed by the program. It represents fractional ownership in the (re)insurance pool and provides investors with a dynamic share of the pool’s value.

### Key Features:
- **Minting and Burning**: Tokens are minted and burned based on fund manager controls.
- **Real-Time NAV Tracking**: Token value reflects the pool’s performance, including premiums earned and claims paid.
- **Liquidity**: Tokens can be traded on secondary markets or redeemed during scheduled windows.

### Redemption Process:
- **Redemption Tokens (ONr)**: Issued for specific dates to facilitate liquidity.
- **Timeline**: Redemption tokens are minted 80 days before the redemption date, with orders closed 70 days before redemption.
- **Direct Liquidity**: Investors can exchange rONe tokens for ONe tokens at the prevailing price on the redemption date.

---

## Regulatory Compliance

The program operates under Bermuda’s regulatory framework, ensuring compliance with local laws and investor protection.

- **KYC/AML Enforcement**: Users must complete KYC verification to participate. Verified addresses are whitelisted for token purchases and redemptions.
- **Segregated Accounts Structure**: Investor funds are legally ring-fenced within a segregated account, managed by Nayms SAC Ltd.

---

## Governance & Security

### Fund Manager Controls:
- Minting and burning of tokens.
- Issuance and redemption of tokens.
- Management of NAV price feeds.
- Multisig approval for all critical operations.

### Smart Contract Security:
- On-chain logs for all minting, burning, and transfers.
- Balances are rounded down to prevent over-allocation.
- Multisig approval ensures secure fund management.

---

## How to Use

1. **Build the Program**  
   Use the Anchor CLI to build the program:
   ```bash
   anchor build
2. **Deploy the Program**
Deploy the program to the Solana blockchain:
anchor deploy
3. **Interact with the Program**
Use a client application or the Anchor CLI to call the program's instructions.

## Conclusion
The OnreApp Solana Program is a pioneering implementation of tokenized (re)insurance pools on the blockchain. By combining the transparency and efficiency of Solana with the institutional-grade compliance of Bermuda’s regulatory framework, it provides investors with a novel way to access stable, yield-generating assets.

This program represents the convergence of blockchain technology and traditional financial instruments, setting a new standard for digital investment products.

## Update Solana Program ID

Drop in the program key into `target/deploy/onreapp-keypair.json` and then:

```zsh
anchor keys sync
anchor build
```

That is going to update the program ID in `anchor.toml` and in the program `lib.rs`.
