//! Mint Authority Transfer Module
//! 
//! This module provides functionality for transferring mint authority between the boss account
//! and program-derived PDAs. This is a foundational component for implementing burn/mint token
//! operations within the program.
//!
//! # Overview
//! 
//! The mint authority transfer system enables the program to:
//! - Take control of token minting from the boss account
//! - Mint tokens directly instead of transferring from pre-minted vaults
//! - Provide emergency recovery mechanisms for mint authority
//! - Support multiple independent tokens with separate authorities
//!
//! # Architecture
//!
//! Each token gets its own unique mint authority PDA derived from:
//! `[MINT_AUTHORITY, mint_pubkey, program_id]`
//!
//! This ensures:
//! - Each token is independently managed  
//! - No conflicts between different token authorities
//! - Deterministic PDA addresses for client interactions
//! - Clean separation of concerns
//!
//! # Instructions
//!
//! ## `transfer_mint_authority_to_program`
//! Transfers mint authority from the boss to a program PDA, enabling the program
//! to mint tokens for buy offers.
//!
//! ## `transfer_mint_authority_to_boss`
//! Transfers mint authority back from the program PDA to the boss account,
//! serving as an emergency recovery mechanism.
//!
//! # Security Model
//!
//! - **Boss Authorization**: All transfers require boss signature
//! - **Authority Validation**: Current mint authority is validated before transfers
//! - **PDA Signatures**: Program uses proper PDA signatures for authority operations
//! - **Event Emission**: All transfers emit events for transparency
//!
//! # Usage Pattern
//!
//! ```rust
//! // 1. Boss transfers authority to program (setup phase)
//! transfer_mint_authority_to_program(ctx)?;
//!
//! // 2. Program can now mint tokens using the PDA
//! // (implemented in future burn/mint functionality)
//!
//! // 3. Boss can recover authority if needed (emergency/maintenance)
//! transfer_mint_authority_to_boss(ctx)?;
//! ```

pub mod transfer_mint_authority_to_program;
pub mod transfer_mint_authority_to_boss;

pub use transfer_mint_authority_to_program::*;
pub use transfer_mint_authority_to_boss::*;