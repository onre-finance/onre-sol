use crate::state::{ProgramAuthority, State};
use anchor_lang::prelude::*;

/// Account structure for initializing the program authority.
#[derive(Accounts)]
pub struct InitializeProgramAuthority<'info> {
    /// Program authority account to be initialized.
    #[account(
        init,
        payer = boss,
        space = 8 + ProgramAuthority::INIT_SPACE,
        seeds = [b"program_authority"],
        bump
    )]
    pub program_authority: Account<'info, ProgramAuthority>,

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
pub fn initialize_program_authority(ctx: Context<InitializeProgramAuthority>) -> Result<()> {
    let program_authority = &mut ctx.accounts.program_authority;
    program_authority.bump = ctx.bumps.program_authority;
    Ok(())
}