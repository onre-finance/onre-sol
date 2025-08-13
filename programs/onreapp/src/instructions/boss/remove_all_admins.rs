use crate::state::{AdminState, State};
use anchor_lang::prelude::*;

/// Account structure for removing all admins and resetting to boss only.
#[derive(Accounts)]
pub struct RemoveAllAdmins<'info> {
    /// Admin state account containing the list of admins.
    #[account(
        mut,
        seeds = [b"admin_state"],
        bump
    )]
    pub admin_state: Account<'info, AdminState>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Account<'info, State>,

    /// The boss authorizing the removal of all admins.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Solana System program.
    pub system_program: Program<'info, System>,
}

/// Removes all admins and sets only the boss as admin.
/// Only the boss can call this function.
pub fn remove_all_admins(ctx: Context<RemoveAllAdmins>) -> Result<()> {
    let admin_state = &mut ctx.accounts.admin_state;
    admin_state.admins.clear();
    Ok(())
}