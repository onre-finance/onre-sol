use crate::constants::seeds;
use crate::instructions::redemption::{
    RedemptionOffer, RedemptionRequest, RedemptionRequestStatus, UserNonceAccount,
};
use crate::state::State;
use crate::utils::transfer_tokens;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

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
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        constraint = !state.is_killed @ CreateRedemptionRequestErrorCode::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,

    /// The redemption offer account
    #[account(mut)]
    pub redemption_offer: Account<'info, RedemptionOffer>,

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
    pub redemption_request: Account<'info, RedemptionRequest>,

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

    /// Program-derived authority that controls redemption vault token accounts
    ///
    /// This PDA manages the redemption vault token accounts and enables the program
    /// to hold tokens until redemption requests are fulfilled or cancelled.
    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::REDEMPTION_OFFER_VAULT_AUTHORITY], bump)]
    pub redemption_vault_authority: AccountInfo<'info>,

    /// The token mint for token_in (input token)
    #[account(
        constraint = token_in_mint.key() == redemption_offer.token_in_mint
            @ CreateRedemptionRequestErrorCode::InvalidMint
    )]
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Redeemer's token account serving as the source of deposited tokens
    ///
    /// Must have sufficient balance to cover the requested amount.
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = redeemer,
        associated_token::token_program = token_program
    )]
    pub redeemer_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Redemption vault's token account serving as the destination for locked tokens
    ///
    /// Must exist. Stores tokens that are locked until the redemption request is
    /// fulfilled or cancelled.
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = redemption_vault_authority,
        associated_token::token_program = token_program
    )]
    pub vault_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Token program interface for transfer operations
    pub token_program: Interface<'info, TokenInterface>,

    /// Associated Token Program for automatic token account creation
    pub associated_token_program: Program<'info, AssociatedToken>,

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
/// - Transfers token_in tokens from redeemer to redemption vault (locking them)
/// - Increments user's nonce in UserNonceAccount
/// - Updates requested_redemptions in RedemptionOffer
/// - Initializes UserNonceAccount if needed (paid by redeemer)
/// - Initializes vault token account if needed (paid by redeemer)
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

    // Transfer tokens from redeemer to redemption vault (locking them)
    transfer_tokens(
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_program,
        &ctx.accounts.redeemer_token_account,
        &ctx.accounts.vault_token_account,
        &ctx.accounts.redeemer,
        None,
        amount,
    )?;

    // Initialize the redemption request
    let redemption_request = &mut ctx.accounts.redemption_request;
    redemption_request.offer = ctx.accounts.redemption_offer.key();
    redemption_request.redeemer = ctx.accounts.redeemer.key();
    redemption_request.amount = amount;
    redemption_request.status = RedemptionRequestStatus::Pending.as_u8();
    redemption_request.bump = ctx.bumps.redemption_request;

    // Update requested redemptions in the offer
    ctx.accounts.redemption_offer.requested_redemptions = ctx
        .accounts
        .redemption_offer
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

    /// Redemption system is paused via kill switch
    #[msg("Redemption system is paused: kill switch activated")]
    KillSwitchActivated,

    /// Nonce doesn't match user's current nonce
    #[msg("Invalid nonce: provided nonce doesn't match user's current nonce")]
    InvalidNonce,

    /// Arithmetic overflow occurred
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,

    /// Expiration is in the past
    #[msg("Invalid expiration: must be in the future")]
    InvalidExpiration,

    /// Invalid mint (doesn't match redemption offer's token_in_mint)
    #[msg("Invalid mint: provided mint doesn't match redemption offer's token_in_mint")]
    InvalidMint,
}
