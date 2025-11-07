use crate::constants::seeds;
use crate::state::State;
use anchor_lang::prelude::*;

/// Error codes for the propose_boss instruction
#[error_code]
pub enum ProposeBossErrorCode {
    /// Cannot propose boss to default (system program) address
    InvalidBossAddress,
}

/// Event emitted when a new boss is proposed
///
/// Provides transparency for tracking ownership transfer proposals.
#[event]
pub struct BossProposedEvent {
    /// The current boss's public key
    pub current_boss: Pubkey,
    /// The proposed new boss's public key
    pub proposed_boss: Pubkey,
}

/// Account structure for proposing a new boss
///
/// This struct defines the accounts required to propose a new boss authority.
/// Only the current boss can propose a new boss.
#[derive(Accounts)]
pub struct ProposeBoss<'info> {
    /// Program state account containing the boss authority
    ///
    /// Must be mutable to allow proposed_boss field modification and have the current
    /// boss account as the authorized signer.
    #[account(
        mut,
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss
    )]
    pub state: Account<'info, State>,

    /// The current boss account proposing the ownership transfer
    pub boss: Signer<'info>,
}

/// Proposes a new boss authority for ownership transfer
///
/// This instruction is the first step in a two-step ownership transfer process.
/// The current boss proposes a new boss, which must then accept the proposal
/// using the accept_boss instruction.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `new_boss` - Public key of the account to receive boss authority
///
/// # Returns
/// * `Ok(())` - If the proposal is recorded successfully
/// * `Err(ProposeBossErrorCode::InvalidBossAddress)` - If new_boss is default address
///
/// # Access Control
/// - Only the current boss can call this instruction
/// - Current boss account must match the one stored in program state
///
/// # Effects
/// - Updates the program state's proposed_boss field
/// - Emits BossProposedEvent for transparency
///
/// # Events
/// * `BossProposedEvent` - Emitted with current and proposed boss public keys
pub fn propose_boss(ctx: Context<ProposeBoss>, new_boss: Pubkey) -> Result<()> {
    require!(
        new_boss != Pubkey::default(),
        ProposeBossErrorCode::InvalidBossAddress
    );

    let state = &mut ctx.accounts.state;
    state.proposed_boss = new_boss;

    emit!(BossProposedEvent {
        current_boss: ctx.accounts.boss.key(),
        proposed_boss: new_boss
    });

    Ok(())
}
