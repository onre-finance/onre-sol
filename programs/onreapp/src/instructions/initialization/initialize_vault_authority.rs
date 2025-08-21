use anchor_lang::prelude::*;
use crate::constants::seeds;
use crate::state::{State, VaultAuthority};

/// Account structure for initializing the vault authority account.
///
/// This struct defines the accounts required to initialize the vault authority account
/// separately from the main program state. Only the boss can call this.
#[derive(Accounts)]
pub struct InitializeVaultAuthority<'info> {
    /// The vault authority account to initialize, rent paid by `boss`.
    #[account(
        init,
        payer = boss,
        space = 8 + VaultAuthority::INIT_SPACE,
        seeds = [seeds::VAULT_AUTHORITY],
        bump
    )]
    pub vault_authority: Account<'info, VaultAuthority>,

    /// The signer authorizing the initialization, must be the boss.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

/// Initializes the vault authority account.
///
/// Creates and initializes the vault authority account for managing token deposits and withdrawals.
/// Only the boss can call this instruction.
///
/// # Arguments
/// - `ctx`: Context containing the accounts to initialize the vault authority.
///
/// # Returns
/// A `Result` indicating success or failure.
pub fn initialize_vault_authority(_ctx: Context<InitializeVaultAuthority>) -> Result<()> {
    msg!("Vault authority initialized successfully");
    Ok(())
}