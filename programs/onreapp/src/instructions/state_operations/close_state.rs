use anchor_lang::prelude::*;

use crate::constants::seeds;
use crate::state::State;

/// Event emitted when the state account is successfully closed
///
/// Provides transparency for tracking the closure of the program's main state account.
#[event]
pub struct StateClosedEvent {
    /// The PDA address of the state account that was closed
    pub state_pda: Pubkey,
    /// The boss account that initiated the closure and received the rent
    pub boss: Pubkey,
}

/// Account structure for closing the program state account
///
/// This struct defines the accounts required to permanently close the program's
/// main state account and transfer its rent balance back to the boss.
/// Only the boss can close the state account.
#[derive(Accounts)]
pub struct CloseState<'info> {
    /// The state account to be closed and its rent reclaimed
    ///
    /// This account is validated as a PDA derived from the "state" seed.
    /// The account will be closed and its rent transferred to the boss.
    #[account(
        mut,
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss,
        close = boss
    )]
    pub state: Account<'info, State>,

    /// The boss account authorized to close the state and receive rent
    ///
    /// Must match the boss stored in the state account.
    /// This signer will receive the rent from the closed state account.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// System program required for account closure and rent transfer
    pub system_program: Program<'info, System>,
}

/// Permanently closes the program's state account and reclaims its rent balance
///
/// This instruction removes the program's main state account and transfers its rent
/// balance back to the boss. The state account is permanently deleted and cannot
/// be recovered. All program configuration and governance settings are lost.
///
/// This operation effectively disables the program, as most instructions require
/// the state account to function. Use with extreme caution.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
///
/// # Returns
/// * `Ok(())` - If the state is successfully closed and rent reclaimed
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss account must match the one stored in program state
///
/// # Effects
/// - State account is permanently deleted
/// - Rent balance is transferred to the boss
/// - Program becomes effectively non-functional
///
/// # Events
/// * `StateClosedEvent` - Emitted with state PDA and boss details
pub fn close_state(ctx: Context<CloseState>) -> Result<()> {
    emit!(StateClosedEvent {
        state_pda: ctx.accounts.state.key(),
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}
