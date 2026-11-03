use crate::model::{self, seeds::*};
use anchor_lang::prelude::*;

/// Creates reusable dispatch configuration. Prop AMM pair state is initialized separately.
#[derive(Accounts)]
#[instruction(kind: model::QuoterKind, instance_id: u64)]
pub struct CreateQuoter<'info> {
    #[account(
        init,
        payer = boss,
        space = 8 + model::Quoter::INIT_SPACE,
        seeds = [
            QUOTER,
            &[kind as u8],
            &instance_id.to_le_bytes()
        ],
        bump
    )]
    pub quoter: Box<Account<'info, model::Quoter>>,

    #[account(
        seeds = [crate::constants::seeds::STATE],
        bump = state.bump,
        has_one = boss @ crate::OnreError::InvalidBoss
    )]
    pub state: Box<Account<'info, crate::state::State>>,

    #[account(mut)]
    pub boss: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_quoter(
    ctx: Context<CreateQuoter>,
    kind: model::QuoterKind,
    instance_id: u64,
) -> Result<()> {
    ctx.accounts.quoter.set_inner(model::Quoter {
        kind,
        instance_id,
        enabled: true,
        bump: ctx.bumps.quoter,
    });
    Ok(())
}
