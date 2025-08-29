use crate::state::{AdminState, State};
use crate::constants::seeds;
use anchor_lang::prelude::*;

/// Account structure for adding a new admin.
#[derive(Accounts)]
pub struct AddAdmin<'info> {
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

    /// The signer authorizing the addition, must be the boss.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Solana System program for account creation.
    pub system_program: Program<'info, System>,
}

/// Adds a new admin to the admin state.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for adding an admin.
/// - `new_admin`: Public key of the new admin to be added.
///
/// # Errors
/// - [`AddAdminErrorCode::CallerNotAdmin`] if the caller is not in the admin list.
/// - [`AddAdminErrorCode::AdminAlreadyExists`] if the admin is already in the list.
pub fn add_admin(ctx: Context<AddAdmin>, new_admin: Pubkey) -> Result<()> {
    let admin_state = &mut ctx.accounts.admin_state;

    require!(
        !admin_state.admins.contains(&new_admin),
        AddAdminErrorCode::AdminAlreadyExists
    );

    require!(
        admin_state.admins.len() < 20,
        AddAdminErrorCode::MaxAdminsReached
    );

    admin_state.admins.push(new_admin);

    Ok(())
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