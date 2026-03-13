use crate::constants::seeds;
use crate::state::State;
use anchor_lang::prelude::*;

use super::fee_config_state::{FeeConfig, FeeType};

/// Event emitted when the fee destination is updated.
#[event]
pub struct FeeConfigDestinationUpdatedEvent {
    /// The FeeType discriminator (0 = TakeOffer, 1 = FulfillRedemption)
    pub fee_type: u8,
    /// The new destination address, or None if fees go to PDA's own ATA
    pub destination: Option<Pubkey>,
    /// The FeeConfig PDA address
    pub fee_config_pda: Pubkey,
}

/// Account structure for updating the fee destination on an existing FeeConfig PDA.
///
/// The boss can set or clear the destination address. When set, fees go directly
/// to the destination's ATA during operations. When cleared (`None`), fees
/// accumulate in the FeeConfig PDA's own ATA.
#[derive(Accounts)]
#[instruction(fee_type: FeeType)]
pub struct SetFeeConfigDestination<'info> {
    /// Program state — used to verify boss authority.
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss @ SetFeeConfigDestinationError::InvalidBoss
    )]
    pub state: Box<Account<'info, State>>,

    /// The FeeConfig PDA to update.
    #[account(
        mut,
        seeds = [seeds::FEE_CONFIG, &[fee_type as u8]],
        bump = fee_config.bump
    )]
    pub fee_config: Account<'info, FeeConfig>,

    /// The boss authority.
    pub boss: Signer<'info>,
}

/// Updates the fee destination for a given fee configuration.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `_fee_type` - The operation type (used for PDA derivation in accounts struct)
/// * `destination` - `Some(address)` to route fees to that address's ATA,
///   or `None` to accumulate in the PDA's own ATA
///
/// # Access Control
/// - Boss only
pub fn set_fee_config_destination(
    ctx: Context<SetFeeConfigDestination>,
    _fee_type: FeeType,
    destination: Option<Pubkey>,
) -> Result<()> {
    ctx.accounts.fee_config.destination = destination;

    msg!(
        "Fee config destination updated: type={}, destination={:?}",
        ctx.accounts.fee_config.fee_type,
        destination
    );

    emit!(FeeConfigDestinationUpdatedEvent {
        fee_type: ctx.accounts.fee_config.fee_type,
        destination,
        fee_config_pda: ctx.accounts.fee_config.key(),
    });

    Ok(())
}

#[error_code]
pub enum SetFeeConfigDestinationError {
    #[msg("Invalid boss account")]
    InvalidBoss,
}