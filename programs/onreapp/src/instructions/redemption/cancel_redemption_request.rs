use crate::constants::seeds;
use crate::instructions::redemption::{RedemptionOffer, RedemptionRequest};
use crate::state::State;
use crate::utils::transfer_tokens;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

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
    /// Original total amount of token_in tokens in the request
    pub original_amount: u64,
    /// Amount of token_in tokens returned to the redeemer
    /// (original_amount - fulfilled_amount; may be less than original_amount for partially fulfilled requests)
    pub returned_amount: u64,
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
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        constraint = !state.is_killed @ CancelRedemptionRequestErrorCode::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,

    /// The redemption offer account
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

    /// The redemption request account to cancel
    /// Account is closed after cancellation and rent is returned to redemption_admin
    #[account(
        mut,
        seeds = [
            seeds::REDEMPTION_REQUEST,
            redemption_request.offer.as_ref(),
            redemption_request.request_id.to_le_bytes().as_ref()
        ],
        bump = redemption_request.bump,
        close = redemption_admin,
        constraint = redemption_request.offer == redemption_offer.key()
            @ CancelRedemptionRequestErrorCode::OfferMismatch
    )]
    pub redemption_request: Account<'info, RedemptionRequest>,

    /// The signer who is cancelling the request
    /// Can be either the redeemer, redemption_admin, or boss
    #[account(mut,
        constraint = signer.key() == state.boss ||
            signer.key() == state.redemption_admin ||
            signer.key() == redemption_request.redeemer
        @ CancelRedemptionRequestErrorCode::Unauthorized
    )]
    pub signer: Signer<'info>,

    /// The redeemer's account (authority for the token account)
    /// CHECK: Must match redemption_request.redeemer
    #[account(
      constraint = redeemer.key() == redemption_request.redeemer
          @ CancelRedemptionRequestErrorCode::InvalidRedeemer
    )]
    pub redeemer: UncheckedAccount<'info>,

    /// Redemption admin receives the rent from closing the redemption request
    /// CHECK: Validated against state.redemption_admin
    #[account(
        mut,
        constraint = redemption_admin.key() == state.redemption_admin
            @ CancelRedemptionRequestErrorCode::InvalidRedemptionAdmin
    )]
    pub redemption_admin: UncheckedAccount<'info>,

    /// Program-derived authority that controls redemption vault token accounts
    ///
    /// This PDA manages the redemption vault token accounts and enables the program
    /// to return locked tokens when requests are cancelled.
    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::REDEMPTION_OFFER_VAULT_AUTHORITY], bump)]
    pub redemption_vault_authority: UncheckedAccount<'info>,

    /// The token mint for token_in (input token)
    #[account(
        constraint = token_in_mint.key() == redemption_offer.token_in_mint
            @ CancelRedemptionRequestErrorCode::InvalidMint
    )]
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Redemption vault's token account serving as the source of locked tokens
    ///
    /// Contains the tokens that were locked when the request was created.
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = redemption_vault_authority,
        associated_token::token_program = token_program
    )]
    pub vault_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Redeemer's token account serving as the destination for returned tokens
    ///
    /// Receives back the tokens that were locked in the redemption request.
    /// Created if needed in case the redeemer closed their account after locking all tokens.
    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = token_in_mint,
        associated_token::authority = redeemer,
        associated_token::token_program = token_program,
    )]
    pub redeemer_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Token program interface for transfer operations
    pub token_program: Interface<'info, TokenInterface>,

    /// System program for account creation and rent payment
    pub system_program: Program<'info, System>,

    /// Associated Token Program for automatic token account creation
    pub associated_token_program: Program<'info, AssociatedToken>,
}

/// Cancels a redemption request
///
/// This instruction cancels a pending redemption request. The request can be cancelled
/// by the redeemer, redemption_admin, or boss. The request must be in pending state.
/// Upon cancellation, the redemption request account is closed and rent is returned
/// to the redemption_admin.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
///
/// # Returns
/// * `Ok(())` - If the redemption request is successfully cancelled
/// * `Err(CancelRedemptionRequestErrorCode::Unauthorized)` - If signer is not authorized
///
/// # Access Control
/// - Signer must be one of: redeemer, redemption_admin, or boss
///
/// # Effects
/// - Closes redemption request account and returns rent to redemption_admin
/// - Returns the unfulfilled token_in tokens (original_amount - fulfilled_amount) from vault to redeemer
/// - Subtracts the returned amount from RedemptionOffer::requested_redemptions
///
/// # Events
/// * `RedemptionRequestCancelledEvent` - Emitted with cancellation details
pub fn cancel_redemption_request(ctx: Context<CancelRedemptionRequest>) -> Result<()> {
    let redemption_request = &ctx.accounts.redemption_request;
    let signer = ctx.accounts.signer.key();

    let original_amount = redemption_request.amount;
    let redeemer = redemption_request.redeemer;

    // Only the unfulfilled remainder is still locked in the vault; return that to redeemer
    let returned_amount = original_amount
        .checked_sub(redemption_request.fulfilled_amount)
        .ok_or(CancelRedemptionRequestErrorCode::ArithmeticUnderflow)?;

    // Return locked tokens from vault to redeemer
    let vault_authority_bump = ctx.bumps.redemption_vault_authority;
    let vault_authority_seeds = &[
        seeds::REDEMPTION_OFFER_VAULT_AUTHORITY,
        &[vault_authority_bump],
    ];
    let vault_authority_signer_seeds = &[vault_authority_seeds.as_slice()];

    transfer_tokens(
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_program,
        &ctx.accounts.vault_token_account,
        &ctx.accounts.redeemer_token_account,
        &ctx.accounts.redemption_vault_authority,
        Some(vault_authority_signer_seeds),
        returned_amount,
    )?;

    // Subtract only the returned (unfulfilled) amount from requested_redemptions
    ctx.accounts.redemption_offer.requested_redemptions = ctx
        .accounts
        .redemption_offer
        .requested_redemptions
        .checked_sub(returned_amount as u128)
        .ok_or(CancelRedemptionRequestErrorCode::ArithmeticUnderflow)?;

    msg!(
        "Redemption request cancelled at: {} original_amount: {} returned_amount: {} by signer: {}",
        ctx.accounts.redemption_request.key(),
        original_amount,
        returned_amount,
        signer
    );

    emit!(RedemptionRequestCancelledEvent {
        redemption_request_pda: ctx.accounts.redemption_request.key(),
        redemption_offer: ctx.accounts.redemption_offer.key(),
        redeemer,
        original_amount,
        returned_amount,
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

    /// Program is in kill switch state
    #[msg("Operation not allowed: program is in kill switch state")]
    KillSwitchActivated,

    /// Arithmetic underflow occurred
    #[msg("Arithmetic underflow")]
    ArithmeticUnderflow,

    /// Invalid mint (doesn't match redemption offer's token_in_mint)
    #[msg("Invalid mint: provided mint doesn't match redemption offer's token_in_mint")]
    InvalidMint,

    /// Invalid redeemer (doesn't match redemption request's redeemer)
    #[msg("Invalid redeemer: provided redeemer doesn't match redemption request's redeemer")]
    InvalidRedeemer,

    /// Invalid redemption admin (doesn't match state.redemption_admin)
    #[msg("Invalid redemption admin: provided account doesn't match state.redemption_admin")]
    InvalidRedemptionAdmin,

    /// Redemption request offer doesn't match provided redemption offer
    #[msg("Offer mismatch: redemption request's offer doesn't match provided redemption offer")]
    OfferMismatch,
}
