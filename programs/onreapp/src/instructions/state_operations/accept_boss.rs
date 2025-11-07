use crate::constants::seeds;
use crate::state::State;
use anchor_lang::prelude::*;

/// Error codes for the accept_boss instruction
#[error_code]
pub enum AcceptBossErrorCode {
    /// No boss transfer has been proposed
    NoBossProposal,
    /// The signer is not the proposed boss
    NotProposedBoss,
}

/// Event emitted when the boss authority is successfully transferred
///
/// Provides transparency for tracking ownership transfers and authority changes.
#[event]
pub struct BossAcceptedEvent {
    /// The previous boss's public key before the update
    pub old_boss: Pubkey,
    /// The new boss's public key after the update
    pub new_boss: Pubkey,
}

/// Account structure for accepting boss authority
///
/// This struct defines the accounts required to complete the ownership transfer.
/// Only the proposed boss can accept and complete the transfer.
#[derive(Accounts)]
pub struct AcceptBoss<'info> {
    /// Program state account containing the boss and proposed_boss
    ///
    /// Must be mutable to allow boss field modification.
    #[account(
        mut,
        seeds = [seeds::STATE],
        bump = state.bump,
    )]
    pub state: Account<'info, State>,

    /// The proposed new boss account accepting the ownership transfer
    pub new_boss: Signer<'info>,
}

/// Accepts and completes the boss authority transfer
///
/// This instruction is the second step in a two-step ownership transfer process.
/// The proposed boss must sign this transaction to accept the transfer and become
/// the new boss authority.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
///
/// # Returns
/// * `Ok(())` - If ownership transfer completes successfully
/// * `Err(AcceptBossErrorCode::NoBossProposal)` - If no proposal exists
/// * `Err(AcceptBossErrorCode::NotProposedBoss)` - If signer is not the proposed boss
///
/// # Access Control
/// - Only the proposed boss can call this instruction
/// - A proposal must have been previously made via propose_boss
///
/// # Effects
/// - Updates the program state's boss field to the new boss
/// - Clears the proposed_boss field (resets to default)
/// - Transfers all program authority to the new boss
/// - Emits BossUpdatedEvent for transparency
///
/// # Events
/// * `BossAcceptedEvent` - Emitted with old and new boss public keys
pub fn accept_boss(ctx: Context<AcceptBoss>) -> Result<()> {
    let state = &mut ctx.accounts.state;

    // Check that a proposal exists
    require!(
        state.proposed_boss != Pubkey::default(),
        AcceptBossErrorCode::NoBossProposal
    );

    // Check that the signer is the proposed boss
    require!(
        ctx.accounts.new_boss.key() == state.proposed_boss,
        AcceptBossErrorCode::NotProposedBoss
    );

    let old_boss = state.boss;
    state.boss = state.proposed_boss;
    state.proposed_boss = Pubkey::default(); // Clear the proposal

    emit!(BossAcceptedEvent {
        old_boss,
        new_boss: state.boss
    });

    Ok(())
}
