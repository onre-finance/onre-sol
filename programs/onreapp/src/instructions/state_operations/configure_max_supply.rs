use crate::constants::seeds;
use crate::state::State;
use anchor_lang::prelude::*;

/// Event emitted when the ONyc maximum supply is successfully configured
///
/// Provides transparency for tracking max supply configuration changes.
#[event]
pub struct MaxSupplyConfigured {
    /// The previous maximum supply cap (0 = no cap)
    pub old_max_supply: u64,
    /// The new maximum supply cap (0 = no cap)
    pub new_max_supply: u64,
}

/// Account structure for configuring the ONyc token maximum supply
///
/// This struct defines the accounts required to set or update the maximum
/// supply cap for ONyc tokens. Only the boss can configure this setting.
#[derive(Accounts)]
pub struct ConfigureMaxSupply<'info> {
    /// Program state account containing the max supply configuration
    ///
    /// Must be mutable to allow max supply updates and have the boss account
    /// as the authorized signer for supply cap management.
    #[account(
        mut,
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss
    )]
    pub state: Account<'info, State>,

    /// The boss account authorized to configure the max supply
    pub boss: Signer<'info>,
}

/// Configures the maximum supply cap for ONyc token minting
///
/// This instruction allows the boss to set or update the maximum supply cap
/// that restricts ONyc token minting. When set to a non-zero value, all minting
/// operations will be validated against this cap to prevent unbounded inflation.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `max_supply` - The maximum supply cap in base units (0 = no cap)
///
/// # Returns
/// * `Ok(())` - If the max supply is successfully configured
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss account must match the one stored in program state
///
/// # Effects
/// - Updates the program state's max_supply field
/// - All future minting operations will validate against this cap
/// - Setting to 0 removes the cap (unlimited minting)
///
/// # Events
/// * `MaxSupplyConfigured` - Emitted with old and new max supply values
pub fn configure_max_supply(ctx: Context<ConfigureMaxSupply>, max_supply: u64) -> Result<()> {
    let state = &mut ctx.accounts.state;

    let old_max_supply = state.max_supply;
    state.max_supply = max_supply;

    msg!(
        "Max supply configured: {} (previous: {})",
        max_supply,
        old_max_supply
    );

    emit!(MaxSupplyConfigured {
        old_max_supply,
        new_max_supply: max_supply,
    });

    Ok(())
}
