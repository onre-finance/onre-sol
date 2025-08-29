use crate::state::{AdminState, State};
use crate::constants::seeds;
use anchor_lang::prelude::*;

/// Account structure for removing an admin.
#[derive(Accounts)]
#[instruction(admin_to_remove: Pubkey)]
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
    pub boss: Signer<'info>,

    /// Solana System program for account creation.
    pub system_program: Program<'info, System>,
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

    let admin_index = admin_state.admins.iter().position(|&x| x == admin_to_remove);
    
    match admin_index {
        Some(index) => {
            admin_state.admins.swap_remove(index);
        }
        None => {
            return Err(RemoveAdminErrorCode::AdminNotFound.into());
        }
    }

    Ok(())
}

/// Error codes for remove admin operations.
#[error_code]
pub enum RemoveAdminErrorCode {
    /// Triggered when trying to remove an admin that doesn't exist.
    #[msg("Admin not found in the admin list")]
    AdminNotFound,
}