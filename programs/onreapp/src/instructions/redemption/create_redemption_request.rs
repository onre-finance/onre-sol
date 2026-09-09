use crate::constants::seeds;
use crate::instructions::redemption::{
    RedemptionOffer, RedemptionRequest, MAX_REDEMPTION_REQUEST_ID_LEN,
};
use crate::instructions::Offer;
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
    /// Frontend-generated identifier for this request
    pub request_id: String,
}

/// Account structure for creating a redemption request
///
/// This struct defines the accounts required to create a redemption request
/// where users can request to redeem token_out tokens from standard Offer for token_in tokens.
/// Anyone can create a redemption request by paying for the PDA rent.
#[derive(Accounts)]
#[instruction(amount: u64, request_id: String)]
pub struct CreateRedemptionRequest<'info> {
    /// Program state account for kill switch validation
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        constraint = !state.is_killed @ crate::OnreError::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,

    /// The redemption offer account. It remains writable to update aggregate pending redemptions.
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

    /// The original offer associated with the redemption offer.
    pub offer: AccountLoader<'info, Offer>,

    /// The redemption request account
    /// PDA derived from the redemption offer, redeemer, and request ID bytes.
    #[account(
        init,
        payer = redeemer,
        space = 8 + RedemptionRequest::INIT_SPACE,
        seeds = [
            seeds::REDEMPTION_REQUEST,
            redemption_offer.key().as_ref(),
            redeemer.key().as_ref(),
            request_id.as_bytes()
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
            @ crate::OnreError::InvalidMint,
        constraint = *token_in_mint.to_account_info().owner == anchor_spl::token::ID
            @ crate::OnreError::InvalidTokenProgram
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
    #[account(
        constraint = token_program.key() == anchor_spl::token::ID
            @ crate::OnreError::InvalidTokenProgram
    )]
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
/// * `request_id` - Frontend-generated ID, exactly 32 UTF-8 bytes
///
/// # Returns
/// * `Ok(())` - If the redemption request is successfully created
///
/// # Access Control
/// - Anyone can create a redemption request (no admin signature required)
/// - Redeemer pays for the redemption request PDA rent
///
/// # Effects
/// - Creates a new redemption request account derived from the offer, redeemer, and request ID
/// - Transfers token_in tokens from redeemer to redemption vault (locking them)
/// - Updates requested_redemptions in RedemptionOffer
///
/// # Events
/// * `RedemptionRequestCreatedEvent` - Emitted with redemption request details
pub fn create_redemption_request(
    ctx: Context<CreateRedemptionRequest>,
    amount: u64,
    request_id: String,
) -> Result<()> {
    require!(amount > 0, crate::OnreError::InvalidAmount);
    require!(
        request_id.len() == MAX_REDEMPTION_REQUEST_ID_LEN,
        crate::OnreError::InvalidRedemptionRequestId
    );

    // Validate the redemption offer is properly initialized (offer is not default)
    require!(
        ctx.accounts.redemption_offer.offer != Pubkey::default(),
        crate::OnreError::InvalidRedemptionOffer
    );

    // Validate the token_out_mint is properly set
    require!(
        ctx.accounts.redemption_offer.token_out_mint != Pubkey::default(),
        crate::OnreError::InvalidRedemptionOffer
    );
    require_keys_eq!(
        ctx.accounts.redemption_offer.offer,
        ctx.accounts.offer.key(),
        crate::OnreError::OfferMismatch
    );
    ctx.accounts.redemption_offer.require_enabled()?;
    ctx.accounts.offer.load()?.require_enabled()?;

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
    redemption_request.request_id = request_id.clone();
    redemption_request.redeemer = ctx.accounts.redeemer.key();
    redemption_request.amount = amount;
    redemption_request.bump = ctx.bumps.redemption_request;

    // Update requested redemptions in the offer
    ctx.accounts.redemption_offer.requested_redemptions = ctx
        .accounts
        .redemption_offer
        .requested_redemptions
        .checked_add(amount as u128)
        .ok_or(crate::OnreError::ArithmeticOverflow)?;

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
        request_id,
    });

    Ok(())
}
