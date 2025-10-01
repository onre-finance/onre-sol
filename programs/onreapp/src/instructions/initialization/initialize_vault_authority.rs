use crate::constants::seeds;
use crate::state::{OfferVaultAuthority, State};
use anchor_lang::prelude::*;

/// Account structure for initializing vault authority account.
#[derive(Accounts)]
pub struct InitializeVaultAuthority<'info> {
    /// The offer vault authority account to initialize, rent paid by `boss`.
    #[account(
        init,
        payer = boss,
        space = 8 + OfferVaultAuthority::INIT_SPACE,
        seeds = [seeds::OFFER_VAULT_AUTHORITY],
        bump
    )]
    pub offer_vault_authority: Account<'info, OfferVaultAuthority>,

    /// Program state, ensures `boss` is authorized.
    #[account(seeds = [seeds::STATE], bump = state.bump, has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// The signer authorizing the initialization, must be the boss.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

/// Initializes vault authority account.
///
/// Creates and initializes vault authority account for managing token deposits and withdrawals.
/// Only the boss can call this instruction.
///
/// # Arguments
/// - `ctx`: Context containing the accounts to initialize vault authority.
///
/// # Returns
/// A `Result` indicating success or failure.
pub fn initialize_vault_authority(ctx: Context<InitializeVaultAuthority>) -> Result<()> {
    ctx.accounts.offer_vault_authority.bump = ctx.bumps.offer_vault_authority;
    msg!("Vault authority initialized successfully");
    Ok(())
}
