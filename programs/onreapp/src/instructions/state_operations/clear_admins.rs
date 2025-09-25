use crate::constants::seeds;
use crate::state::{State, MAX_ADMINS};
use crate::AccountInfo;
use anchor_lang::prelude::*;

/// Account structure for clearing all admins.
#[derive(Accounts)]
pub struct ClearAdmins<'info> {
    #[account(
        mut,
        has_one = boss,
        seeds = [seeds::STATE],
        bump
    )]
    pub state: Account<'info, State>,

    /// The boss calling this function.
    #[account(mut)]
    pub boss: Signer<'info>,
}

/// Clears all admins from the state.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for clearing all admins.
///
pub fn clear_admins(ctx: Context<ClearAdmins>) -> Result<()> {
    let state = &mut ctx.accounts.state;

    // Clear all admins
    for i in 0..MAX_ADMINS {
        state.admins[i] = Pubkey::default();
    }

    Ok(())
}
