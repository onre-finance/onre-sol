use anchor_lang::prelude::*;
use crate::state::State;

#[derive(Accounts)]
pub struct SetApprover<'info> {
    #[account(mut, has_one = boss)]
    pub state: Account<'info, State>,
    pub boss: Signer<'info>,
}

pub fn set_approver(ctx: Context<SetApprover>, approver: Pubkey) -> Result<()> {
    let state = &mut ctx.accounts.state;
    state.approver = approver;
    Ok(())
}