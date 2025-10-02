use crate::state::{State, MAX_ADMINS};
use crate::constants::seeds;
use anchor_lang::prelude::*;

/// Account structure for adding a new admin to the program state
///
/// This struct defines the accounts required to add an admin account to the
/// program's admin list. Only the boss can add new admins.
#[derive(Accounts)]
pub struct AddAdmin<'info> {
    /// Program state account containing the admin list
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

    /// The boss account authorized to add new admins
    #[account(mut)]
    pub boss: Signer<'info>,
}

/// Adds a new admin to the program's admin list
///
/// This instruction allows the boss to grant admin privileges to a new account
/// by adding it to the program state's admin list. The admin list supports up
/// to MAX_ADMINS entries and prevents duplicate additions.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `new_admin` - Public key of the account to be granted admin privileges
///
/// # Returns
/// * `Ok(())` - If the admin is successfully added
/// * `Err(AddAdminErrorCode::AdminAlreadyExists)` - If the account is already an admin
/// * `Err(AddAdminErrorCode::MaxAdminsReached)` - If the admin list is full
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss account must match the one stored in program state
///
/// # Effects
/// - Adds the new admin to the first available slot in the admin array
/// - Grants admin privileges for program operations
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

/// Error codes for add admin operations
#[error_code]
pub enum AddAdminErrorCode {
    /// The specified account is already present in the admin list
    #[msg("Admin already exists in the admin list")]
    AdminAlreadyExists,

    /// The admin list has reached its maximum capacity and cannot accept more admins
    #[msg("Maximum number of admins (20) reached")]
    MaxAdminsReached,
}