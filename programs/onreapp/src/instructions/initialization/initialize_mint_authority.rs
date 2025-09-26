use crate::constants::seeds;
use crate::state::{MintAuthority, State};
use anchor_lang::prelude::*;

/// Account structure for initializing mint authority account.
#[derive(Accounts)]
pub struct InitializeMintAuthority<'info> {
    /// The offer mint authority account to initialize, rent paid by `boss`.
    #[account(
        init,
        payer = boss,
        space = 8 + MintAuthority::INIT_SPACE,
        seeds = [seeds::MINT_AUTHORITY],
        bump
    )]
    pub mint_authority: Account<'info, MintAuthority>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// The signer authorizing the initialization, must be the boss.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

/// Initializes mint authority account.
///
/// Creates and initializes mint authority account.
/// Only the boss can call this instruction.
///
/// # Arguments
/// - `ctx`: Context containing the accounts to initialize mint authority.
///
/// # Returns
/// A `Result` indicating success or failure.
pub fn initialize_mint_authority(ctx: Context<InitializeMintAuthority>) -> Result<()> {
    ctx.accounts.mint_authority.bump = ctx.bumps.mint_authority;
    msg!("Mint authority initialized successfully");
    Ok(())
}
