use crate::constants::seeds;
use crate::state::State;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SetApprover<'info> {
    #[account(mut,
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss)]
    pub state: Account<'info, State>,
    pub boss: Signer<'info>,
}

/// Sets the trusted authority for cryptographic approval verification
///
/// This instruction allows the boss to configure which account serves as the
/// trusted authority for providing cryptographic approvals when offers require
/// approval for execution. The approver signs approval messages that are
/// verified during offer operations.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `approver` - Public key of the account to be set as the approval authority
///
/// # Returns
/// * `Ok(())` - Always succeeds when called by authorized boss
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss account must match the one stored in program state
///
/// # Effects
/// - Updates the program state's approver field
/// - Configures which account can provide valid approval signatures
/// - Affects all future offer operations requiring approval
pub fn set_approver(ctx: Context<SetApprover>, approver: Pubkey) -> Result<()> {
    let state = &mut ctx.accounts.state;
    state.approver = approver;
    Ok(())
}
