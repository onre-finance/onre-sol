use crate::{
    constants::seeds::STATE,
    model::{seeds::QUOTER, Quoter, QuoterKind},
    state::State,
    OnreError,
};
use anchor_lang::prelude::*;

/// Records the created account and its complete initial configuration.
#[event]
pub struct QuoterCreatedEvent {
    pub boss: Pubkey,
    pub quoter: Pubkey,
    pub configuration: Quoter,
    pub timestamp: i64,
}

/// Creates reusable dispatch configuration. Prop AMM pair state is initialized separately.
#[derive(Accounts)]
#[instruction(kind: QuoterKind, instance_id: u64)]
pub struct CreateQuoter<'info> {
    #[account(
        init,
        payer = boss,
        space = 8 + Quoter::INIT_SPACE,
        seeds = [
            QUOTER,
            &[kind as u8],
            &instance_id.to_le_bytes()
        ],
        bump
    )]
    pub quoter: Box<Account<'info, Quoter>>,

    #[account(
        seeds = [STATE],
        bump = state.bump,
        has_one = boss @ OnreError::InvalidBoss
    )]
    pub state: Box<Account<'info, State>>,

    #[account(mut)]
    pub boss: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_quoter(ctx: Context<CreateQuoter>, kind: QuoterKind, instance_id: u64) -> Result<()> {
    ctx.accounts.quoter.set_inner(Quoter {
        kind,
        instance_id,
        enabled: true,
        bump: ctx.bumps.quoter,
    });
    emit!(QuoterCreatedEvent {
        boss: ctx.accounts.boss.key(),
        quoter: ctx.accounts.quoter.key(),
        configuration: (**ctx.accounts.quoter).clone(),
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
