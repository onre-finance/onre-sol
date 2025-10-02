use crate::constants::seeds;
use crate::state::State;
use anchor_lang::prelude::*;
// Includes `emit!` and `#[event]`
use anchor_lang::Accounts;

/// Error codes for the set_boss instruction
#[error_code]
pub enum SetBossErrorCode {
    /// Cannot set boss to default (system program) address
    InvalidBossAddress,
}

/// Event emitted when the boss authority is successfully updated
///
/// Provides transparency for tracking ownership transfers and authority changes.
#[event]
pub struct BossUpdated {
    /// The previous boss's public key before the update
    pub old_boss: Pubkey,
    /// The new boss's public key after the update
    pub new_boss: Pubkey,
}

/// Account structure for transferring program ownership to a new boss
///
/// This struct defines the accounts required to change the program's boss authority.
/// Only the current boss can authorize the ownership transfer.
#[derive(Accounts)]
pub struct SetBoss<'info> {
    /// Program state account containing the boss authority to be updated
    ///
    /// Must be mutable to allow boss field modification and have the current
    /// boss account as the authorized signer for ownership transfer.
    #[account(
        mut,
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss
    )]
    pub state: Account<'info, State>,

    /// The current boss account authorizing the ownership transfer
    pub boss: Signer<'info>,

    /// System program for potential account operations
    pub system_program: Program<'info, System>,
}

/// Transfers program ownership to a new boss authority
///
/// This instruction allows the current boss to transfer complete program control
/// to a new account. The new boss will have authority over all program operations
/// including admin management, state changes, and offer configuration.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `new_boss` - Public key of the account to receive boss authority
///
/// # Returns
/// * `Ok(())` - If ownership transfer completes successfully
/// * `Err(SetBossErrorCode::InvalidBossAddress)` - If new_boss is default address
///
/// # Access Control
/// - Only the current boss can call this instruction
/// - Current boss account must match the one stored in program state
///
/// # Effects
/// - Updates the program state's boss field
/// - Transfers all program authority to the new boss
/// - Emits BossUpdated event for transparency
///
/// # Events
/// * `BossUpdated` - Emitted with old and new boss public keys
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
