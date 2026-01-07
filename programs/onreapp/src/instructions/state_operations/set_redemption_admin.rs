use crate::constants::seeds;
use crate::state::State;
use anchor_lang::prelude::*;

/// Event emitted when the redemption admin is successfully updated
///
/// Provides transparency for tracking redemption admin configuration changes.
#[event]
pub struct RedemptionAdminUpdatedEvent {
    /// The previous redemption admin public key before the update
    pub old_redemption_admin: Pubkey,
    /// The new redemption admin public key after the update
    pub new_redemption_admin: Pubkey,
}

/// Account structure for configuring the redemption admin
///
/// This struct defines the accounts required to set or update the redemption admin
/// address in the program state. Only the boss can configure this setting.
#[derive(Accounts)]
pub struct SetRedemptionAdmin<'info> {
    /// Program state account containing the redemption admin configuration
    ///
    /// Must be mutable to allow redemption admin updates and have the boss account
    /// as the authorized signer for redemption admin configuration management.
    #[account(
        mut,
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss
    )]
    pub state: Account<'info, State>,

    /// The boss account authorized to configure the redemption admin
    pub boss: Signer<'info>,
}

/// Configures the redemption admin address in program state
///
/// This instruction allows the boss to set or update the redemption admin that
/// the program recognizes for managing ONr token mints and redemptions.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `new_redemption_admin` - Public key of the new redemption admin
///
/// # Returns
/// * `Ok(())` - If the redemption admin is successfully configured
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss account must match the one stored in program state
///
/// # Effects
/// - Updates the program state's redemption_admin field
/// - Configures which account is authorized to manage redemptions
///
/// # Events
/// * `RedemptionAdminUpdatedEvent` - Emitted with old and new redemption admin addresses
pub fn set_redemption_admin(
    ctx: Context<SetRedemptionAdmin>,
    new_redemption_admin: Pubkey,
) -> Result<()> {
    let state = &mut ctx.accounts.state;

    // Validate this is not a no-op (setting the same admin)
    require!(
        new_redemption_admin != state.redemption_admin,
        SetRedemptionAdminErrorCode::NoChange
    );

    let old_redemption_admin = state.redemption_admin;
    state.redemption_admin = new_redemption_admin;

    msg!("Redemption admin updated: {}", state.redemption_admin);
    emit!(RedemptionAdminUpdatedEvent {
        old_redemption_admin,
        new_redemption_admin: state.redemption_admin,
    });

    Ok(())
}

/// Error codes for set redemption admin operations
#[error_code]
pub enum SetRedemptionAdminErrorCode {
    /// The new redemption admin is the same as the current one
    #[msg("No change: new redemption admin is the same as current")]
    NoChange,
}
