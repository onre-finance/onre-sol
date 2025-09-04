use crate::constants::seeds;
use crate::state::{
    BuyOfferVaultAuthority, DualRedemptionVaultAuthority, SingleRedemptionVaultAuthority, State,
};
use anchor_lang::prelude::*;

/// Account structure for initializing all vault authority accounts.
///
/// This struct defines the accounts required to initialize all three vault authority accounts
/// separately from the main program state. Only the boss can call this.
#[derive(Accounts)]
pub struct InitializeVaultAuthority<'info> {
    /// The buy offer vault authority account to initialize, rent paid by `boss`.
    #[account(
        init,
        payer = boss,
        space = 8 + BuyOfferVaultAuthority::INIT_SPACE,
        seeds = [seeds::BUY_OFFER_VAULT_AUTHORITY],
        bump
    )]
    pub buy_offer_vault_authority: Account<'info, BuyOfferVaultAuthority>,

    /// The single redemption vault authority account to initialize, rent paid by `boss`.
    #[account(
        init,
        payer = boss,
        space = 8 + SingleRedemptionVaultAuthority::INIT_SPACE,
        seeds = [seeds::SINGLE_REDEMPTION_VAULT_AUTHORITY],
        bump
    )]
    pub single_redemption_vault_authority: Account<'info, SingleRedemptionVaultAuthority>,

    /// The dual redemption vault authority account to initialize, rent paid by `boss`.
    #[account(
        init,
        payer = boss,
        space = 8 + DualRedemptionVaultAuthority::INIT_SPACE,
        seeds = [seeds::DUAL_REDEMPTION_VAULT_AUTHORITY],
        bump
    )]
    pub dual_redemption_vault_authority: Account<'info, DualRedemptionVaultAuthority>,

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
