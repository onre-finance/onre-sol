use crate::state::{State, MAX_ADMINS};
use crate::constants::seeds;
use anchor_lang::prelude::*;

/// Account structure for adding a new admin.
#[derive(Accounts)]
pub struct AddAdmin<'info> {
    #[account(
        mut,
        has_one = boss,
        seeds = [seeds::STATE],
        bump = state.bump
    )]
    pub state: Account<'info, State>,

    /// The signer authorizing the addition, must be the boss.
    #[account(mut)]
    pub boss: Signer<'info>,
}

/// Adds a new admin to the state.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for adding an admin.
/// - `new_admin`: Public key of the new admin to be added.
///
/// # Errors
/// - [`AddAdminErrorCode::AdminAlreadyExists`] if the admin is already in the list.
/// - [`AddAdminErrorCode::MaxAdminsReached`] if the maximum number of admins is reached.
pub fn add_admin(ctx: Context<AddAdmin>, new_admin: Pubkey) -> Result<()> {
    let state = &mut ctx.accounts.state;

    // Check if admin already exists
    require!(
        !state.admins.contains(&new_admin),
        AddAdminErrorCode::AdminAlreadyExists
    );

    // Find first empty slot
    for i in 0..MAX_ADMINS {
        if state.admins[i] == Pubkey::default() {
            state.admins[i] = new_admin;
            return Ok(());
        }
    }

    // If we get here, all slots are full
    Err(AddAdminErrorCode::MaxAdminsReached.into())
}

/// Error codes for add admin operations.
#[error_code]
pub enum AddAdminErrorCode {
    /// Triggered when trying to add an admin that already exists.
    #[msg("Admin already exists in the admin list")]
    AdminAlreadyExists,

    /// Triggered when trying to add more than 20 admins.
    #[msg("Maximum number of admins (20) reached")]
    MaxAdminsReached,
}