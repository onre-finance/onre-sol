use anchor_lang::prelude::*;

use crate::constants::seeds;
use crate::instructions::KillSwitchState;
use crate::state::{State};

/// Account structure for initializing the program authority.
#[derive(Accounts)]
pub struct InitializeKillSwitchState<'info> {
    /// Program authority account to be initialized.
    #[account(
        init,
        payer = boss,
        space = 8 + KillSwitchState::INIT_SPACE,
        seeds = [seeds::KILL_SWITCH_STATE],
        bump
    )]
    pub kill_switch_state: Account<'info, KillSwitchState>,

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
pub fn initialize_kill_switch_state(ctx: Context<InitializeKillSwitchState>) -> Result<()> {
    ctx.accounts.kill_switch_state.is_killed = false;
    Ok(())
}
