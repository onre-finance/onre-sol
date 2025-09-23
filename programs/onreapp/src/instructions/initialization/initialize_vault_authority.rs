use crate::constants::seeds;
use crate::state::{OfferVaultAuthority, State};
use anchor_lang::prelude::*;

/// Account structure for initializing all vault authority accounts.
///
/// This struct defines the accounts required to initialize vault authority account
/// separately from the main program state. Only the boss can call this.
#[derive(Accounts)]
pub struct InitializeVaultAuthority<'info> {
    /// The buy offer vault authority account to initialize, rent paid by `boss`.
    #[account(
        init,
        payer = boss,
        space = 8 + OfferVaultAuthority::INIT_SPACE,
        seeds = [seeds::OFFER_VAULT_AUTHORITY],
        bump
    )]
    pub buy_offer_vault_authority: Account<'info, OfferVaultAuthority>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// The signer authorizing the initialization, must be the boss.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

/// Initializes all vault authority accounts.
///
/// Creates and initializes all three vault authority accounts for managing token deposits and withdrawals.
/// Only the boss can call this instruction.
///
/// # Arguments
/// - `ctx`: Context containing the accounts to initialize all vault authorities.
///
/// # Returns
/// A `Result` indicating success or failure.
pub fn initialize_vault_authority(_ctx: Context<InitializeVaultAuthority>) -> Result<()> {
    msg!("All vault authorities initialized successfully");
    Ok(())
}
