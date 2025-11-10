use crate::constants::{seeds, MAX_ADMINS};
use crate::state::State;
use anchor_lang::prelude::*;

/// Account structure for removing an admin from the program state
///
/// This struct defines the accounts required to revoke admin privileges
/// from a specific account. Only the boss can remove admins.
#[derive(Accounts)]
pub struct RemoveAdmin<'info> {
    /// Program state account containing the admin list to be modified
    ///
    /// Must be mutable to allow admin list modifications and have the
    /// boss account as the authorized signer for admin management.
    #[account(
        mut,
        has_one = boss,
        seeds = [seeds::STATE],
        bump = state.bump
    )]
    pub state: Account<'info, State>,

    /// The boss account authorized to remove admin privileges
    pub boss: Signer<'info>,
}

/// Removes admin privileges from a specific account
///
/// This instruction allows the boss to revoke admin privileges from an account
/// by removing it from the program state's admin list. The admin entry is set
/// to default (empty) value, making the slot available for future additions.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `admin_to_remove` - Public key of the account to lose admin privileges
///
/// # Returns
/// * `Ok(())` - If the admin is successfully removed
/// * `Err(RemoveAdminErrorCode::AdminNotFound)` - If the account is not in the admin list
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss account must match the one stored in program state
///
/// # Effects
/// - Sets the admin array entry to default (empty) public key
/// - Revokes admin privileges from the specified account
/// - Makes the admin slot available for future use
pub fn remove_admin(ctx: Context<RemoveAdmin>, admin_to_remove: Pubkey) -> Result<()> {
    let state = &mut ctx.accounts.state;

    // Find and remove the admin
    for i in 0..MAX_ADMINS {
        if state.admins[i] == admin_to_remove {
            state.admins[i] = Pubkey::default();
            return Ok(());
        }
    }

    // If we get here, admin was not found
    Err(RemoveAdminErrorCode::AdminNotFound.into())
}

/// Error codes for remove admin operations
#[error_code]
pub enum RemoveAdminErrorCode {
    /// The specified account is not present in the admin list
    #[msg("Admin not found in the admin list")]
    AdminNotFound,
}
