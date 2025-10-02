use crate::constants::seeds;
use crate::state::{State, MAX_ADMINS};
use crate::AccountInfo;
use anchor_lang::prelude::*;

/// Account structure for clearing all admins from the program state
///
/// This struct defines the accounts required to remove all admin privileges
/// by clearing the entire admin list. Only the boss can perform this operation.
#[derive(Accounts)]
pub struct ClearAdmins<'info> {
    /// Program state account containing the admin list to be cleared
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

    /// The boss account authorized to clear all admin privileges
    #[account(mut)]
    pub boss: Signer<'info>,
}

/// Removes all admin privileges by clearing the entire admin list
///
/// This instruction allows the boss to revoke admin privileges from all accounts
/// by resetting the admin array to default (empty) values. This is useful for
/// emergency admin management or complete admin list restructuring.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
///
/// # Returns
/// * `Ok(())` - Always succeeds when called by authorized boss
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss account must match the one stored in program state
///
/// # Effects
/// - Sets all admin array entries to default (empty) public keys
/// - Revokes admin privileges from all previously authorized accounts
/// - Does not affect the boss account's authority
pub fn clear_admins(ctx: Context<ClearAdmins>) -> Result<()> {
    let state = &mut ctx.accounts.state;

    // Clear all admins
    for i in 0..MAX_ADMINS {
        state.admins[i] = Pubkey::default();
    }

    Ok(())
}
