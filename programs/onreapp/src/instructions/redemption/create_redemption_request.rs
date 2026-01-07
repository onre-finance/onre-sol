use crate::constants::seeds;
use crate::instructions::redemption::{RedemptionOffer, RedemptionRequest};
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
    pub redemption_offer_pda: Pubkey,
    /// User requesting the redemption
    pub redeemer: Pubkey,
    /// Amount of token_in tokens requested for redemption
    pub amount: u64,
    /// Unique identifier for this request (counter value used for PDA derivation)
    pub id: u64,
}

/// Account structure for creating a redemption request
///
/// This struct defines the accounts required to create a redemption request
/// where users can request to redeem token_out tokens from standard Offer for token_in tokens.
/// Anyone can create a redemption request by paying for the PDA rent.
#[derive(Accounts)]
pub struct CreateRedemptionRequest<'info> {
    /// Program state account for kill switch validation
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        constraint = !state.is_killed @ CreateRedemptionRequestErrorCode::KillSwitchActivated
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

    /// The redemption request account
    /// PDA derived from redemption_offer and its counter value
    #[account(
        init,
        payer = redeemer,
        space = 8 + RedemptionRequest::INIT_SPACE,
        seeds = [
            seeds::REDEMPTION_REQUEST,
            redemption_offer.key().as_ref(),
            redemption_offer.request_counter.to_le_bytes().as_ref()
        ],
        bump
    )]
    pub redemption_request: Account<'info, RedemptionRequest>,

    /// User requesting the redemption (pays for account creation)
    #[account(mut)]
    pub redeemer: Signer<'info>,

    /// Program-derived authority that controls redemption vault token accounts
    ///
    /// This PDA manages the redemption vault token accounts and enables the program
    /// to hold tokens until redemption requests are fulfilled or cancelled.
    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::REDEMPTION_OFFER_VAULT_AUTHORITY], bump)]
    pub redemption_vault_authority: UncheckedAccount<'info>,

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
/// redemption of input tokens for output tokens at a future time. Anyone can create
/// a redemption request by paying for the PDA rent.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `amount` - Amount of token_in tokens to redeem
///
/// # Returns
/// * `Ok(())` - If the redemption request is successfully created
///
/// # Access Control
/// - Anyone can create a redemption request (no admin signature required)
/// - Redeemer pays for the redemption request PDA rent
///
/// # Effects
/// - Creates new redemption request account (PDA derived from offer and counter)
/// - Transfers token_in tokens from redeemer to redemption vault (locking them)
/// - Increments counter on RedemptionOffer for next request
/// - Updates requested_redemptions in RedemptionOffer
///
/// # Events
/// * `RedemptionRequestCreatedEvent` - Emitted with redemption request details
pub fn create_redemption_request(ctx: Context<CreateRedemptionRequest>, amount: u64) -> Result<()> {
    // Validate the redemption offer is properly initialized (offer is not default)
    require!(
        ctx.accounts.redemption_offer.offer != Pubkey::default(),
        CreateRedemptionRequestErrorCode::InvalidRedemptionOffer
    );

    // Validate the token_out_mint is properly set
    require!(
        ctx.accounts.redemption_offer.token_out_mint != Pubkey::default(),
        CreateRedemptionRequestErrorCode::InvalidRedemptionOffer
    );

    // Capture counter before incrementing (used for PDA derivation)
    let request_id = ctx.accounts.redemption_offer.request_counter;

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
    redemption_request.request_id = request_id;
    redemption_request.redeemer = ctx.accounts.redeemer.key();
    redemption_request.amount = amount;
    redemption_request.bump = ctx.bumps.redemption_request;

    // Update requested redemptions in the offer
    ctx.accounts.redemption_offer.requested_redemptions = ctx
        .accounts
        .redemption_offer
        .requested_redemptions
        .checked_add(amount as u128)
        .ok_or(CreateRedemptionRequestErrorCode::ArithmeticOverflow)?;

    // Increment counter for next request
    ctx.accounts.redemption_offer.request_counter = ctx
        .accounts
        .redemption_offer
        .request_counter
        .checked_add(1)
        .ok_or(CreateRedemptionRequestErrorCode::ArithmeticOverflow)?;

    msg!(
        "Redemption request created at: {} for amount: {} by redeemer: {} (id: {})",
        ctx.accounts.redemption_request.key(),
        amount,
        ctx.accounts.redeemer.key(),
        request_id
    );

    emit!(RedemptionRequestCreatedEvent {
        redemption_request_pda: ctx.accounts.redemption_request.key(),
        redemption_offer_pda: ctx.accounts.redemption_offer.key(),
        redeemer: ctx.accounts.redeemer.key(),
        amount,
        id: request_id,
    });

    Ok(())
}

/// Error codes for redemption request creation operations
#[error_code]
pub enum CreateRedemptionRequestErrorCode {
    /// Redemption system is paused via kill switch
    #[msg("Redemption system is paused: kill switch activated")]
    KillSwitchActivated,

    /// Arithmetic overflow occurred
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,

    /// Invalid mint (doesn't match redemption offer's token_in_mint)
    #[msg("Invalid mint: provided mint doesn't match redemption offer's token_in_mint")]
    InvalidMint,

    /// Invalid redemption offer (not properly initialized)
    #[msg("Invalid redemption offer: offer is not properly initialized")]
    InvalidRedemptionOffer,
}
