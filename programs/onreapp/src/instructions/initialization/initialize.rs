use crate::constants::seeds;
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use anchor_spl::token_interface::Mint;

/// Error codes for the initialize instruction.
#[error_code]
pub enum InitializeErrorCode {
    /// Error when attempting to initialize when boss is already set.
    BossAlreadySet,
}

/// Account structure for initializing the program state.
///
/// This struct defines the accounts required to set up the program’s global state,
/// including the boss’s public key.
///
/// # Preconditions
/// - The `state` account must not exist prior to execution; it will be initialized here.
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// The program state account, initialized with the boss’s public key.
    ///
    /// # Note
    /// - Space is allocated as `8 + State::INIT_SPACE` bytes, where 8 bytes are for the discriminator.
    /// - Seeded with `"state"` and a bump for PDA derivation.
    #[account(
        init,
        payer = boss,
        space = 8 + State::INIT_SPACE,
        seeds = [seeds::STATE],
        bump
    )]
    pub state: Account<'info, State>,

    /// The signer funding and authorizing the state initialization, becomes the boss.
    #[account(mut)]
    pub boss: Signer<'info>,

    pub onyc_mint: InterfaceAccount<'info, Mint>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

/// Initializes the program state with the boss's public key, kill switch disabled, and empty admins.
///
/// Sets the `boss` field in the `state` account to the signer's key if it's not already set.
/// Initializes the kill switch to disabled (false) by default.
/// Initializes the admins array to empty (all zeros).
/// The account is created as a PDA with the seed `"state"`.
///
/// # Arguments
/// - `ctx`: Context containing the accounts to initialize the state.
///
/// # Returns
/// A `Result` indicating success or failure.
pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
    let state = &mut ctx.accounts.state;
    if state.boss != Pubkey::default() {
        return err!(InitializeErrorCode::BossAlreadySet);
    }
    state.boss = ctx.accounts.boss.key();
    state.is_killed = false; // Initialize kill switch as disabled
    state.admins = [Pubkey::default(); crate::state::MAX_ADMINS]; // Initialize empty admins array
    state.onyc_mint = ctx.accounts.onyc_mint.key();
    state.bump = ctx.bumps.state;

    Ok(())
}
