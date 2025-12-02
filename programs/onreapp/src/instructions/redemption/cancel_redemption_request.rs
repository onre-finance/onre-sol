use crate::constants::seeds;
use crate::instructions::redemption::{
    RedemptionOffer, RedemptionRequest, RedemptionRequestStatus,
};
use crate::state::State;
use anchor_lang::prelude::*;

/// Event emitted when a redemption request is successfully cancelled
///
/// Provides transparency for tracking cancelled redemption requests.
#[event]
pub struct RedemptionRequestCancelledEvent {
    /// The PDA address of the cancelled redemption request
    pub redemption_request_pda: Pubkey,
    /// Reference to the redemption offer
    pub redemption_offer: Pubkey,
    /// User who requested the redemption
    pub redeemer: Pubkey,
    /// Amount of token_in tokens that was requested for redemption
    pub amount: u64,
    /// The signer who cancelled the request
    pub cancelled_by: Pubkey,
}

/// Account structure for cancelling a redemption request
///
/// This struct defines the accounts required to cancel a redemption request.
/// The signer can be either the redeemer, redemption_admin, or boss.
#[derive(Accounts)]
pub struct CancelRedemptionRequest<'info> {
    /// Program state account containing redemption_admin and boss for authorization
    #[account(seeds = [seeds::STATE], bump = state.bump)]
    pub state: Box<Account<'info, State>>,

    /// The redemption offer account
    #[account(mut)]
    pub redemption_offer: AccountLoader<'info, RedemptionOffer>,

    /// The redemption request account to cancel
    #[account(mut)]
    pub redemption_request: AccountLoader<'info, RedemptionRequest>,

    /// The signer who is cancelling the request
    /// Can be either the redeemer, redemption_admin, or boss
    pub signer: Signer<'info>,
}

/// Cancels a redemption request
///
/// This instruction cancels a pending redemption request. The request can be cancelled
/// by the redeemer, redemption_admin, or boss. The request must be in pending state.
/// Upon cancellation, the status is changed to cancelled and the amount is subtracted
/// from the redemption offer's requested_redemptions counter.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
///
/// # Returns
/// * `Ok(())` - If the redemption request is successfully cancelled
/// * `Err(CancelRedemptionRequestErrorCode::Unauthorized)` - If signer is not authorized
/// * `Err(CancelRedemptionRequestErrorCode::InvalidStatus)` - If request is not in pending state
///
/// # Access Control
/// - Signer must be one of: redeemer, redemption_admin, or boss
/// - Request must be in pending state (status = 0)
///
/// # Effects
/// - Changes redemption request status to cancelled (2)
/// - Subtracts amount from RedemptionOffer::requested_redemptions
/// - Does NOT close the redemption request account
///
/// # Events
/// * `RedemptionRequestCancelledEvent` - Emitted with cancellation details
pub fn cancel_redemption_request(ctx: Context<CancelRedemptionRequest>) -> Result<()> {
    let redemption_request = ctx.accounts.redemption_request.load()?;
    let state = &ctx.accounts.state;
    let signer = ctx.accounts.signer.key();

    // Verify authorization - signer must be redeemer, redemption_admin, or boss
    let is_authorized = signer == redemption_request.redeemer
        || signer == state.redemption_admin
        || signer == state.boss;

    require!(
        is_authorized,
        CancelRedemptionRequestErrorCode::Unauthorized
    );

    // Verify the request is in pending state
    require_eq!(
        redemption_request.status,
        RedemptionRequestStatus::Pending.as_u8(),
        CancelRedemptionRequestErrorCode::InvalidStatus
    );

    let amount = redemption_request.amount;
    let redeemer = redemption_request.redeemer;
    drop(redemption_request);

    // Update the redemption request status to cancelled
    let mut redemption_request_mut = ctx.accounts.redemption_request.load_mut()?;
    redemption_request_mut.status = RedemptionRequestStatus::Cancelled.as_u8();

    // Subtract the amount from requested_redemptions in the offer
    let mut redemption_offer = ctx.accounts.redemption_offer.load_mut()?;
    redemption_offer.requested_redemptions = redemption_offer
        .requested_redemptions
        .checked_sub(amount)
        .ok_or(CancelRedemptionRequestErrorCode::ArithmeticUnderflow)?;

    msg!(
        "Redemption request cancelled at: {} for amount: {} by signer: {}",
        ctx.accounts.redemption_request.key(),
        amount,
        signer
    );

    emit!(RedemptionRequestCancelledEvent {
        redemption_request_pda: ctx.accounts.redemption_request.key(),
        redemption_offer: ctx.accounts.redemption_offer.key(),
        redeemer,
        amount,
        cancelled_by: signer,
    });

    Ok(())
}

/// Error codes for redemption request cancellation operations
#[error_code]
pub enum CancelRedemptionRequestErrorCode {
    /// Caller is not authorized (must be redeemer, redemption_admin, or boss)
    #[msg("Unauthorized: signer must be redeemer, redemption_admin, or boss")]
    Unauthorized,

    /// Request is not in pending state
    #[msg("Invalid status: redemption request must be in pending state")]
    InvalidStatus,

    /// Arithmetic underflow occurred
    #[msg("Arithmetic underflow")]
    ArithmeticUnderflow,
}
