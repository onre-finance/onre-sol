use crate::state::{PermissionlessAccount, State};
use anchor_lang::prelude::*;
use anchor_lang::Accounts;

/// Error codes for the initialize instruction.
#[error_code]
pub enum InitializePermissionlessErrorCode {
    /// Error when attempting to initialize a permissionless account with an invalid name
    InvalidPermissionlessAccountName,
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
        InitializePermissionlessErrorCode::InvalidPermissionlessAccountName
    );
    let permissionless_account = &mut ctx.accounts.permissionless_account;
    permissionless_account.name = name_cleaned.to_string();
    Ok(())
}
