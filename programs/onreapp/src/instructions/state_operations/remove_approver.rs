use crate::constants::seeds;
use crate::state::State;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct RemoveApprover<'info> {
    #[account(mut,
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss)]
    pub state: Account<'info, State>,
    pub boss: Signer<'info>,
}

#[error_code]
pub enum RemoveApproverError {
    /// The provided address is not an approver
    #[msg("The provided address is not an approver")]
    NotAnApprover,
}

/// Removes a trusted authority from the approval verification list
///
/// This instruction allows the boss to remove an approver by their public key.
/// The approver must exist in either approver1 or approver2 slot, otherwise
/// the instruction will fail with NotAnApprover error.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `approver` - Public key of the approver to remove
///
/// # Returns
/// * `Ok(())` - Successfully removed the approver
/// * `Err(RemoveApproverError::NotAnApprover)` - The address is not currently an approver
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss account must match the one stored in program state
///
/// # Effects
/// - Sets the matching approver slot (approver1 or approver2) to Pubkey::default()
/// - Removed approver can no longer provide valid approval signatures
/// - Affects all future offer operations requiring approval
pub fn remove_approver(ctx: Context<RemoveApprover>, approver: Pubkey) -> Result<()> {
    let state = &mut ctx.accounts.state;

    // Check if the approver matches approver1
    if state.approver1 == approver && approver != Pubkey::default() {
        state.approver1 = Pubkey::default();
        return Ok(());
    }

    // Check if the approver matches approver2
    if state.approver2 == approver && approver != Pubkey::default() {
        state.approver2 = Pubkey::default();
        return Ok(());
    }

    // The provided address is not an approver
    Err(error!(RemoveApproverError::NotAnApprover))
}
