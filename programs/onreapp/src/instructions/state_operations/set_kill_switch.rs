use anchor_lang::prelude::*;

use crate::constants::seeds;
use crate::state::State;

/// Event emitted when the kill switch state is changed
///
/// Provides transparency for tracking emergency control changes.
#[event]
pub struct KillSwitchToggledEvent {
    /// Whether the kill switch was enabled (true) or disabled (false)
    pub enabled: bool,
    /// The account that toggled the kill switch
    pub signer: Pubkey,
}

/// Account structure for controlling the program kill switch
///
/// This struct defines the accounts required to enable or disable the emergency
/// kill switch that can halt critical program operations.
#[derive(Accounts)]
pub struct SetKillSwitch<'info> {
    /// Program state account containing the kill switch flag
    ///
    /// Must be mutable to allow kill switch state modifications.
    /// The kill switch prevents offer operations when enabled.
    #[account(
        mut,
        seeds = [seeds::STATE],
        bump = state.bump,
    )]
    pub state: Box<Account<'info, State>>,

    /// The account attempting to modify the kill switch (boss or admin)
    pub signer: Signer<'info>,
}

/// Controls the emergency kill switch for critical program operations
///
/// This instruction manages the program's emergency kill switch which can halt
/// offer operations when activated. The kill switch has asymmetric access control:
/// both boss and admins can enable it, but only the boss can disable it.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `enable` - Whether to enable (true) or disable (false) the kill switch
///
/// # Returns
/// * `Ok(())` - If the kill switch state is successfully updated
/// * `Err(ErrorCode::UnauthorizedToEnable)` - If non-authorized user tries to enable
/// * `Err(ErrorCode::OnlyBossCanDisable)` - If non-boss user tries to disable
///
/// # Access Control
/// - Enable: Boss or any admin can activate the kill switch
/// - Disable: Only the boss can deactivate the kill switch
///
/// # Effects
/// - Updates the program state's is_killed field
/// - When enabled, prevents offer execution operations
/// - Provides emergency halt capability for security incidents
pub fn set_kill_switch(ctx: Context<SetKillSwitch>, enable: bool) -> Result<()> {
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

    emit!(KillSwitchToggledEvent {
        enabled: enable,
        signer: signer.key(),
    });

    Ok(())
}

/// Error codes for kill switch operations
#[error_code]
pub enum ErrorCode {
    /// Only the boss has authority to disable the kill switch
    #[msg("Only boss can disable the kill switch")]
    OnlyBossCanDisable,
    /// Signer is neither boss nor admin and cannot enable the kill switch
    #[msg("Unauthorized to enable the kill switch")]
    UnauthorizedToEnable,
}
