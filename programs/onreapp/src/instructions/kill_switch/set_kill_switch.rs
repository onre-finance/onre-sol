use anchor_lang::prelude::*;

use crate::constants::seeds;
use crate::state::State;

#[derive(Accounts)]
pub struct KillSwitch<'info> {
    #[account(
        mut,
        seeds = [seeds::STATE],
        bump,
    )]
    pub state: Box<Account<'info, State>>,
    pub signer: Signer<'info>,
}

pub fn set_kill_switch(ctx: Context<KillSwitch>, enable: bool) -> Result<()> {
    let state = &mut ctx.accounts.state;
    let signer = &ctx.accounts.signer;

    let boss_signed = state.boss.key() == signer.key() && signer.is_signer;
    let admin_signed = state.admins.contains(signer.key) && signer.is_signer;

    if enable {
        require!(boss_signed || admin_signed, ErrorCode::UnauthorizedToEnable);
        state.is_killed = true;
    } else {
        require!(boss_signed, ErrorCode::OnlyBossCanDisable);
        state.is_killed = false;
    }

    Ok(())
}

#[error_code]
pub enum ErrorCode {
    #[msg("Only boss can disable the kill switch")]
    OnlyBossCanDisable,
    #[msg("Unauthorized to enable the kill switch")]
    UnauthorizedToEnable,
}
