use crate::constants::{seeds, MAX_ALLOWED_FEE_BPS};
use crate::instructions::redemption::RedemptionOffer;
use crate::state::State;
use anchor_lang::prelude::*;

/// Event emitted when a redemption offer's fee is successfully updated
///
/// Provides transparency for tracking fee changes and redemption offer configuration modifications.
#[event]
pub struct RedemptionOfferFeeUpdatedEvent {
    /// The PDA address of the redemption offer whose fee was updated
    pub redemption_offer_pda: Pubkey,
    /// Previous fee in basis points (10000 = 100%)
    pub old_fee_basis_points: u16,
    /// New fee in basis points (10000 = 100%)
    pub new_fee_basis_points: u16,
    /// The boss account that authorized the fee update
    pub boss: Pubkey,
}

/// Account structure for updating a redemption offer's fee configuration
///
/// This struct defines the accounts required to modify the fee basis points
/// charged when fulfilling redemption requests. Only the boss can update redemption offer fees.
#[derive(Accounts)]
pub struct UpdateRedemptionOfferFee<'info> {
    /// The redemption offer account whose fee will be updated
    #[account(
        mut,
        seeds = [
            seeds::REDEMPTION_OFFER,
            redemption_offer.token_in_mint.as_ref(),
            redemption_offer.token_out_mint.as_ref()
        ],
        bump = redemption_offer.bump
    )]
    pub redemption_offer: Account<'info, RedemptionOffer>,

    /// Program state account containing boss authorization
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss @ UpdateRedemptionOfferFeeErrorCode::Unauthorized
    )]
    pub state: Account<'info, State>,

    /// The boss account authorized to update redemption offer fees
    pub boss: Signer<'info>,
}

/// Updates the fee configuration for an existing redemption offer
///
/// This instruction allows the boss to modify the fee charged when fulfilling
/// redemption requests for a specific redemption offer. The fee applies to all future
/// redemption fulfillments and is deducted from the token_in amount before calculating
/// token_out exchange amounts.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `new_fee_basis_points` - New fee in basis points (10000 = 100%, 500 = 5%)
///
/// # Returns
/// * `Ok(())` - If the fee is successfully updated
/// * `Err(UpdateRedemptionOfferFeeErrorCode::InvalidFee)` - If fee exceeds 10000 basis points
/// * `Err(UpdateRedemptionOfferFeeErrorCode::Unauthorized)` - If caller is not the boss
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss account must match the one stored in program state
///
/// # Effects
/// - Updates the redemption offer's fee_basis_points field
/// - Affects all future redemption fulfillments for this offer
/// - Does not modify existing redemption requests
///
/// # Events
/// * `RedemptionOfferFeeUpdatedEvent` - Emitted with old and new fee values
pub fn update_redemption_offer_fee(
    ctx: Context<UpdateRedemptionOfferFee>,
    new_fee_basis_points: u16,
) -> Result<()> {
    // Validate fee is within valid range (0-1000 basis points = 0-10%)
    require!(
        new_fee_basis_points <= MAX_ALLOWED_FEE_BPS,
        UpdateRedemptionOfferFeeErrorCode::InvalidFee
    );

    let redemption_offer = &mut ctx.accounts.redemption_offer;

    // Validate this is not a no-op (setting the same fee)
    require!(
        new_fee_basis_points != redemption_offer.fee_basis_points,
        UpdateRedemptionOfferFeeErrorCode::NoChange
    );

    // Store old fee for event
    let old_fee_basis_points = redemption_offer.fee_basis_points;

    // Update the fee
    redemption_offer.fee_basis_points = new_fee_basis_points;

    msg!(
        "Redemption offer fee updated for offer: {}, old fee: {}, new fee: {}",
        ctx.accounts.redemption_offer.key(),
        old_fee_basis_points,
        new_fee_basis_points
    );

    emit!(RedemptionOfferFeeUpdatedEvent {
        redemption_offer_pda: ctx.accounts.redemption_offer.key(),
        old_fee_basis_points,
        new_fee_basis_points,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}

/// Error codes for update redemption offer fee operations
#[error_code]
pub enum UpdateRedemptionOfferFeeErrorCode {
    /// Caller is not authorized (must be boss)
    #[msg("Unauthorized: only boss can update redemption offer fee")]
    Unauthorized,

    /// Fee basis points exceeds maximum allowed value of 1000 (10%)
    #[msg("Invalid fee: fee_basis_points must be <= 1000")]
    InvalidFee,

    /// The new fee is the same as the current fee
    #[msg("No change: new fee is the same as current fee")]
    NoChange,
}
