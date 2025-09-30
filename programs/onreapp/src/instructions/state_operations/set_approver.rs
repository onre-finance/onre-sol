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

pub fn set_approver(ctx: Context<SetApprover>, approver: Pubkey) -> Result<()> {
    let state = &mut ctx.accounts.state;
    state.approver = approver;
    Ok(())
}
