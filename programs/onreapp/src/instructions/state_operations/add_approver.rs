use crate::constants::seeds;
use crate::state::State;
use anchor_lang::prelude::*;

/// Event emitted when an approver is successfully added
///
/// Provides transparency for tracking approver changes.
#[event]
pub struct ApproverAddedEvent {
    /// The public key of the newly added approver
    pub approver: Pubkey,
    /// The boss who added the approver
    pub boss: Pubkey,
}

#[derive(Accounts)]
pub struct AddApprover<'info> {
    #[account(mut,
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss)]
    pub state: Account<'info, State>,
    pub boss: Signer<'info>,
}

#[error_code]
pub enum AddApproverError {
    /// Both approver slots are already filled
    #[msg("Both approver slots are already filled")]
    BothApproversFilled,
    #[msg("Invalid approver")]
    InvalidApprover,
    #[msg("Approver already exists")]
    ApproverAlreadyExists,
}

/// Adds a trusted authority for cryptographic approval verification
///
/// This instruction allows the boss to add an approver to one of the two available
/// approver slots. The approver is added to the first empty slot (approver1 or approver2).
/// If both slots are already filled, the instruction will fail.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `approver` - Public key of the account to be added as the approval authority
///
/// # Returns
/// * `Ok(())` - Successfully added approver to an empty slot
/// * `Err(AddApproverError::BothApproversFilled)` - Both approver slots are already filled
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss account must match the one stored in program state
///
/// # Effects
/// - Updates the program state's approver1 or approver2 field
/// - Adds an account that can provide valid approval signatures
/// - Affects all future offer operations requiring approval
pub fn add_approver(ctx: Context<AddApprover>, approver: Pubkey) -> Result<()> {
    let state = &mut ctx.accounts.state;

    if approver == Pubkey::default() {
        return Err(error!(AddApproverError::InvalidApprover));
    }

    if state.approver1 == approver || state.approver2 == approver {
        return Err(error!(AddApproverError::ApproverAlreadyExists));
    }

    // Check if approver1 is empty (default pubkey)
    if state.approver1 == Pubkey::default() {
        state.approver1 = approver;

        emit!(ApproverAddedEvent {
            approver,
            boss: ctx.accounts.boss.key(),
        });

        return Ok(());
    }

    // Check if approver2 is empty
    if state.approver2 == Pubkey::default() {
        state.approver2 = approver;

        emit!(ApproverAddedEvent {
            approver,
            boss: ctx.accounts.boss.key(),
        });

        return Ok(());
    }

    // Both slots are filled
    Err(error!(AddApproverError::BothApproversFilled))
}
