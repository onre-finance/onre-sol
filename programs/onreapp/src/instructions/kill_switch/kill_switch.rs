use anchor_lang::prelude::*;

use crate::kill_switch_state::KillSwitchState;
use crate::state::{AdminState, State};
use crate::constants::seeds;

#[derive(Accounts)]
pub struct KillSwitch<'info> {
    #[account(
        mut,
        seeds = [seeds::KILL_SWITCH_STATE],
        bump,
    )]
    pub kill_switch_state: Box<Account<'info, KillSwitchState>>,
    #[account(
        seeds = [seeds::ADMIN_STATE],
        bump,
    )]
    pub admin_state: Box<Account<'info, AdminState>>,
    #[account(
        seeds = [seeds::STATE],
        bump,
    )]
    pub state: Box<Account<'info, State>>,
    pub signer: Signer<'info>,
}


pub fn kill_switch(ctx: Context<KillSwitch>, enable: bool) -> Result<()> {
    let state = &ctx.accounts.state;
    let signer = &ctx.accounts.signer;
    let admin_state = &ctx.accounts.admin_state;
    let kill_switch_state = &mut ctx.accounts.kill_switch_state;

    // Who actually signed this tx?
    let boss_signed = state.boss.key() == signer.key() && signer.is_signer;
    let admin_signed = admin_state.admins.contains(signer.key) && signer.is_signer;

    if enable {
        require!(boss_signed || admin_signed, ErrorCode::UnauthorizedToEnable);
        ctx.accounts.kill_switch_state.is_killed = true;
    } else {
        require!(boss_signed, ErrorCode::OnlyBossCanDisable);
        ctx.accounts.kill_switch_state.is_killed = false;
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