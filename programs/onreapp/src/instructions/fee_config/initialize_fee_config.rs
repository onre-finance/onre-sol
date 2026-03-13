use crate::constants::seeds;
use crate::state::State;
use anchor_lang::prelude::*;

use super::fee_config_state::{FeeConfig, FeeType};

/// Event emitted when a fee configuration PDA is initialized.
#[event]
pub struct FeeConfigInitializedEvent {
    /// The FeeType
    pub fee_type: FeeType,
    /// The FeeType discriminator (0 = TakeOffer, 1 = FulfillRedemption)
    pub fee_type_discriminator: u8,
    /// The PDA address of the new FeeConfig account
    pub fee_config_pda: Pubkey,
}

/// Account structure for initializing a fee configuration PDA.
///
/// Creates a new FeeConfig account for the specified operation type. Each FeeType
/// gets its own PDA, so there is at most one FeeConfig per operation type.
/// The boss pays for account creation rent.
#[derive(Accounts)]
#[instruction(fee_type: FeeType)]
pub struct InitializeFeeConfig<'info> {
    /// Program state — used to verify boss authority.
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss @ InitializeFeeConfigError::InvalidBoss
    )]
    pub state: Box<Account<'info, State>>,

    /// The FeeConfig PDA to create, derived from `[FEE_CONFIG, fee_type]`.
    #[account(
        init,
        payer = boss,
        space = 8 + FeeConfig::INIT_SPACE,
        seeds = [seeds::FEE_CONFIG, &[fee_type as u8]],
        bump
    )]
    pub fee_config: Account<'info, FeeConfig>,

    /// The boss authority who pays for account creation.
    #[account(mut)]
    pub boss: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Initializes a fee configuration PDA for a given operation type.
///
/// Sets `destination = None` so that fees accumulate in the PDA's own ATA
/// until the boss optionally configures a destination via `set_fee_config_destination`.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `fee_type` - The operation type this config applies to
///
/// # Access Control
/// - Boss only
pub fn initialize_fee_config(
    ctx: Context<InitializeFeeConfig>,
    fee_type: FeeType,
) -> Result<()> {
    let fee_config = &mut ctx.accounts.fee_config;
    fee_config.fee_type = fee_type as u8;
    fee_config.destination = None;
    fee_config.bump = ctx.bumps.fee_config;
    fee_config.reserved = [0u8; 64];

    msg!(
        "Fee config initialized: type={}, pda={}",
        fee_config.fee_type,
        fee_config.key()
    );

    emit!(FeeConfigInitializedEvent {
        fee_type: fee_type,
        fee_type_discriminator: fee_type as u8,
        fee_config_pda: fee_config.key(),
    });

    Ok(())
}

#[error_code]
pub enum InitializeFeeConfigError {
    #[msg("Invalid boss account")]
    InvalidBoss,
}