use crate::state::{AdminState, State};
use anchor_lang::prelude::*;

/// Account structure for initializing the program authority.
#[derive(Accounts)]
pub struct InitializeAdminState<'info> {
    /// Program authority account to be initialized.
    #[account(
        init,
        payer = boss,
        space = 8 + AdminState::INIT_SPACE,
        seeds = [b"admin_state"],
        bump
    )]
    pub admin_state: Account<'info, AdminState>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Account<'info, State>,

    /// The boss paying for account creation and authorizing the initialization.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Solana System program for account creation.
    pub system_program: Program<'info, System>,
}

/// Initializes the program authority account.
pub fn initialize_admin_state(_ctx: Context<InitializeAdminState>) -> Result<()> {
    Ok(())
}