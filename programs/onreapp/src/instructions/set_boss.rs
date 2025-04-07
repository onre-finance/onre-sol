use crate::state::State;
use anchor_lang::prelude::*; // Includes `emit!` and `#[event]`
use anchor_lang::Accounts;
use anchor_lang::system_program;

/// Error codes for the set_boss instruction.
#[error_code]
pub enum SetBossErrorCode {
    /// Error when attempting to set the boss to the system program address.
    InvalidBossAddress,
}

/// Event emitted when the boss is updated in the program state.
#[event]
pub struct BossUpdated {
    /// The previous boss’s public key.
    pub old_boss: Pubkey,
    /// The new boss’s public key.
    pub new_boss: Pubkey,
}

/// Account structure for updating the program’s boss.
///
/// This struct defines the accounts required to change the `boss` field in the program state.
///
/// # Preconditions
/// - The `state` account must be initialized prior to execution, via an `initialize` instruction.
/// - The current `boss` must sign the transaction to authorize the change.
#[derive(Accounts)]
pub struct SetBoss<'info> {
    /// The program state account, containing the current boss to be updated.
    ///
    /// # Constraints
    /// - Must be mutable to allow updating the `boss` field.
    /// - The `has_one = boss` constraint ensures only the current boss can modify it.
    #[account(mut, has_one = boss)]
    pub state: Account<'info, State>,

    /// The current boss, signing the transaction to authorize the update.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Solana System program, included for potential rent accounting.
    pub system_program: Program<'info, System>,
}

/// Updates the boss in the program state.
///
/// Sets the `boss` field in the `state` account to a new public key, emitting a `BossUpdated` event
/// for traceability. Only the current boss can call this instruction, enforced by the `has_one = boss` constraint.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the state update.
/// - `new_boss`: The new public key to set as the boss.
///
/// # Errors
/// - [`SetBossErrorCode::InvalidBossAddress`] if the new boss is the system program address.
///
/// # Returns
/// A `Result` indicating success or failure.
pub fn set_boss(ctx: Context<SetBoss>, new_boss: Pubkey) -> Result<()> {
    require!(
        new_boss != Pubkey::default(),
        SetBossErrorCode::InvalidBossAddress
    );

    let state = &mut ctx.accounts.state;
    let old_boss = ctx.accounts.boss.key(); // Capture old boss before update
    state.boss = new_boss;
    emit!(BossUpdated { old_boss, new_boss });
    Ok(())
}
