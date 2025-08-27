use crate::state::{PermissionlessAccount, State};
use anchor_lang::prelude::*;
use anchor_lang::Accounts;

/// Error codes for the initialize instruction.
#[error_code]
pub enum InitializeErrorCode {
    /// Error when attempting to initialize when boss is already set.
    BossAlreadySet,
    /// Error when attempting to initialize a permissionless account with an invalid name
    InvalidPermissionlessAccountName,
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
        seeds = [b"state"],
        bump
    )]
    pub state: Account<'info, State>,

    /// The signer funding and authorizing the state initialization, becomes the boss.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

/// Initializes the program state with the boss’s public key.
///
/// Sets the `boss` field in the `state` account to the signer’s key if it’s not already set.
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
    Ok(())
}

/// Account structure for initializing a permissionless account.
///
/// This struct defines the accounts required to create a new permissionless account,
/// which can be used as an intermediary authority for routing tokens.
///
/// # Preconditions
/// - Only the boss can initialize permissionless accounts
/// - The permissionless account must not exist prior to execution
#[derive(Accounts)]
pub struct InitializePermissionlessAccount<'info> {
    /// The permissionless account to be created.
    ///
    /// # Note
    /// - Space is allocated as `8 + PermissionlessAccount::INIT_SPACE` bytes
    /// - Seeded with hardcoded "permissionless-1" for PDA derivation
    #[account(
        init,
        payer = boss,
        space = 8 + PermissionlessAccount::INIT_SPACE,
        seeds = [b"permissionless-1"],
        bump
    )]
    pub permissionless_account: Account<'info, PermissionlessAccount>,

    /// The program state account, used to verify boss authorization.
    #[account(has_one = boss)]
    pub state: Account<'info, State>,

    /// The boss account that authorizes and pays for the permissionless account creation.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

/// Initializes a new permissionless account with the provided name.
///
/// Creates a permissionless account that can serve as an intermediary authority
/// for token routing operations. Only the boss can create these accounts.
/// The PDA is always derived using the hardcoded seed "permissionless-1".
///
/// # Arguments
/// - `ctx`: Context containing the accounts for permissionless account creation
/// - `name`: The name to store in the permissionless account (separate from PDA seeds)
///
/// # Returns
/// A `Result` indicating success or failure.
///
/// # Errors
/// - Fails if the caller is not the boss (enforced by `has_one = boss` constraint)
/// - Fails if the permissionless account already exists
pub fn initialize_permissionless_account(
    ctx: Context<InitializePermissionlessAccount>,
    name: String,
) -> Result<()> {
    let name_cleaned = name.trim();
    require!(
        !name_cleaned.is_empty(),
        InitializeErrorCode::InvalidPermissionlessAccountName
    );
    let permissionless_account = &mut ctx.accounts.permissionless_account;
    permissionless_account.name = name_cleaned.to_string();
    Ok(())
}
