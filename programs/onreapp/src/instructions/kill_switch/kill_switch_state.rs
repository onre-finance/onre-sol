use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct KillSwitchState {
    pub is_killed: bool,
}