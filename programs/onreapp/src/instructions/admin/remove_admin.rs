use crate::state::{AdminState, State};
use crate::constants::seeds;
use anchor_lang::prelude::*;

/// Account structure for removing an admin.
#[derive(Accounts)]
pub struct RemoveAdmin<'info> {
    /// Admin state account containing the list of admins.
    #[account(
        mut,
        seeds = [seeds::ADMIN_STATE],
        bump
    )]
    pub admin_state: Account<'info, AdminState>,

    #[account(
        has_one = boss,
        seeds = [seeds::STATE],
        bump
    )]
    pub state: Account<'info, State>,

    /// The boss calling this function.
    #[account(mut)]
    pub boss: Signer<'info>
}

/// Removes an admin from the admin state.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for removing an admin.
/// - `admin_to_remove`: Public key of the admin to be removed.
///
/// # Errors
/// - [`RemoveAdminErrorCode::AdminNotFound`] if the admin to remove is not in the list.
pub fn remove_admin(ctx: Context<RemoveAdmin>, admin_to_remove: Pubkey) -> Result<()> {
    let admin_state = &mut ctx.accounts.admin_state;

    // Find and remove the admin
    for i in 0..20 {
        if admin_state.admins[i] == admin_to_remove {
            admin_state.admins[i] = Pubkey::default();
            return Ok(());
        }
    }

    // If we get here, admin was not found
    Err(RemoveAdminErrorCode::AdminNotFound.into())
}

/// Error codes for remove admin operations.
#[error_code]
pub enum RemoveAdminErrorCode {
    /// Triggered when trying to remove an admin that doesn't exist.
    #[msg("Admin not found in the admin list")]
    AdminNotFound,
}