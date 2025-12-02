use crate::constants::seeds;
use crate::instructions::redemption::{
    RedemptionOffer, RedemptionRequest, RedemptionRequestStatus, UserNonceAccount,
};
use crate::state::State;
use anchor_lang::prelude::*;

/// Event emitted when a redemption request is successfully created
///
/// Provides transparency for tracking redemption requests and their configuration.
#[event]
pub struct RedemptionRequestCreatedEvent {
    /// The PDA address of the newly created redemption request
    pub redemption_request_pda: Pubkey,
    /// Reference to the redemption offer
    pub redemption_offer: Pubkey,
    /// User requesting the redemption
    pub redeemer: Pubkey,
    /// Amount of token_in tokens requested for redemption
    pub amount: u64,
    /// Unix timestamp when the request expires
    pub expires_at: u64,
    /// Nonce used for this request
    pub used_nonce: u64,
    /// New nonce, which should be used for the next request
    pub new_nonce: u64,
}

/// Account structure for creating a redemption request
///
/// This struct defines the accounts required to create a redemption request
/// where users can request to redeem token_out tokens from standard Offer for token_in tokens.
#[derive(Accounts)]
#[instruction(amount: u64, expires_at: u64, nonce: u64)]
pub struct CreateRedemptionRequest<'info> {
    /// Program state account containing redemption_admin authorization
    #[account(seeds = [seeds::STATE], bump = state.bump)]
    pub state: Box<Account<'info, State>>,

    /// The redemption offer account
    #[account(mut)]
    pub redemption_offer: AccountLoader<'info, RedemptionOffer>,

    /// The redemption request account
    #[account(
        init,
        payer = redeemer,
        space = 8 + RedemptionRequest::INIT_SPACE,
        seeds = [
            seeds::REDEMPTION_REQUEST,
            redemption_offer.key().as_ref(),
            redeemer.key().as_ref(),
            nonce.to_le_bytes().as_ref()
        ],
        bump
    )]
    pub redemption_request: AccountLoader<'info, RedemptionRequest>,

    /// User nonce account for preventing replay attacks
    ///
    /// This account tracks the user's current nonce to ensure each request is unique.
    #[account(
        init_if_needed,
        payer = redeemer,
        space = 8 + UserNonceAccount::INIT_SPACE,
        seeds = [
            seeds::NONCE_ACCOUNT,
            redeemer.key().as_ref()
        ],
        bump
    )]
    pub user_nonce_account: Account<'info, UserNonceAccount>,

    /// User requesting the redemption (pays for account creation)
    #[account(mut)]
    pub redeemer: Signer<'info>,

    /// Redemption admin must sign to authorize the request
    #[account(
        constraint = redemption_admin.key() == state.redemption_admin
            @ CreateRedemptionRequestErrorCode::Unauthorized
    )]
    pub redemption_admin: Signer<'info>,

    /// System program for account creation
    pub system_program: Program<'info, System>,
}

/// Creates a redemption request
///
/// This instruction creates a new redemption request that allows users to request
/// redemption of input tokens for output tokens at a future time. The request must
/// be authorized by the redemption admin and uses a nonce to prevent replay attacks.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `amount` - Amount of token_in tokens to redeem
/// * `expires_at` - Unix timestamp when the request expires
/// * `nonce` - User's nonce for replay attack prevention (must match UserNonceAccount)
///
/// # Returns
/// * `Ok(())` - If the redemption request is successfully created
/// * `Err(CreateRedemptionRequestErrorCode::Unauthorized)` - If redemption_admin is not authorized
/// * `Err(CreateRedemptionRequestErrorCode::InvalidNonce)` - If nonce doesn't match user's current nonce
///
/// # Access Control
/// - Requires both redeemer and redemption_admin signatures
/// - Nonce must match the user's current nonce in UserNonceAccount
///
/// # Effects
/// - Creates new redemption request account
/// - Increments user's nonce in UserNonceAccount
/// - Updates requested_redemptions in RedemptionOffer
/// - Initializes UserNonceAccount if needed (paid by redeemer)
///
/// # Events
/// * `RedemptionRequestCreatedEvent` - Emitted with redemption request details
pub fn create_redemption_request(
    ctx: Context<CreateRedemptionRequest>,
    amount: u64,
    expires_at: u64,
    nonce: u64,
) -> Result<()> {
    // Verify nonce matches user's current nonce
    require_eq!(
        ctx.accounts.user_nonce_account.nonce,
        nonce,
        CreateRedemptionRequestErrorCode::InvalidNonce
    );
    require!(
        expires_at > Clock::get()?.unix_timestamp as u64,
        CreateRedemptionRequestErrorCode::InvalidExpiration
    );

    // Initialize the redemption request
    let mut redemption_request = ctx.accounts.redemption_request.load_init()?;
    redemption_request.offer = ctx.accounts.redemption_offer.key();
    redemption_request.redeemer = ctx.accounts.redeemer.key();
    redemption_request.amount = amount;
    redemption_request.expires_at = expires_at;
    redemption_request.status = RedemptionRequestStatus::Pending.as_u8();
    redemption_request.bump = ctx.bumps.redemption_request;

    // Update requested redemptions in the offer
    let mut redemption_offer = ctx.accounts.redemption_offer.load_mut()?;
    redemption_offer.requested_redemptions = redemption_offer
        .requested_redemptions
        .checked_add(amount)
        .ok_or(CreateRedemptionRequestErrorCode::ArithmeticOverflow)?;

    // Increment user's nonce
    ctx.accounts.user_nonce_account.nonce = ctx
        .accounts
        .user_nonce_account
        .nonce
        .checked_add(1)
        .ok_or(CreateRedemptionRequestErrorCode::ArithmeticOverflow)?;

    msg!(
        "Redemption request created at: {} for amount: {} by redeemer: {}",
        ctx.accounts.redemption_request.key(),
        amount,
        ctx.accounts.redeemer.key()
    );

    emit!(RedemptionRequestCreatedEvent {
        redemption_request_pda: ctx.accounts.redemption_request.key(),
        redemption_offer: ctx.accounts.redemption_offer.key(),
        redeemer: ctx.accounts.redeemer.key(),
        amount,
        expires_at,
        used_nonce: nonce,
        new_nonce: ctx.accounts.user_nonce_account.nonce,
    });

    Ok(())
}

/// Error codes for redemption request creation operations
#[error_code]
pub enum CreateRedemptionRequestErrorCode {
    /// Caller is not authorized (redemption_admin mismatch)
    #[msg("Unauthorized: redemption_admin signature required")]
    Unauthorized,

    /// Nonce doesn't match user's current nonce
    #[msg("Invalid nonce: provided nonce doesn't match user's current nonce")]
    InvalidNonce,

    /// Arithmetic overflow occurred
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,

    /// Expiration is in the past
    #[msg("Invalid expiration: must be in the future")]
    InvalidExpiration,
}
