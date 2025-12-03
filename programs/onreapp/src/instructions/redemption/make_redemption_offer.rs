use crate::constants::seeds;
use crate::instructions::redemption::RedemptionOffer;
use crate::instructions::Offer;
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

/// Event emitted when a redemption offer is successfully created
///
/// Provides transparency for tracking redemption offer creation and configuration.
#[event]
pub struct RedemptionOfferCreatedEvent {
    /// The PDA address of the newly created redemption offer
    pub redemption_offer_pda: Pubkey,
    /// Reference to the original offer
    pub offer: Pubkey,
    /// The input token mint for redemptions (ONyc)
    pub token_in_mint: Pubkey,
    /// The output token mint for redemptions (e.g., USDC)
    pub token_out_mint: Pubkey,
}

/// Account structure for creating a redemption offer
///
/// This struct defines the accounts required to initialize a redemption offer
/// where users can redeem ONyc tokens for stable tokens at the current NAV price.
/// The redemption offer is the inverse of the standard Offer.
#[derive(Accounts)]
pub struct MakeRedemptionOffer<'info> {
    /// Program state account containing boss and redemption_admin authorization
    #[account(seeds = [seeds::STATE], bump = state.bump)]
    pub state: Box<Account<'info, State>>,

    /// The original offer that this redemption offer is associated with
    ///
    /// The redemption offer uses the inverse token pair of the original offer.
    /// The offer must be derived from redemption offer token_out_mint (token_in in original offer)
    /// and token_in_mint (token_out in original offer).
    #[account(
        seeds = [
            seeds::OFFER,
            token_out_mint.key().as_ref(),
            token_in_mint.key().as_ref(),
        ],
        bump
    )]
    pub offer: AccountLoader<'info, Offer>,

    /// Program-derived authority that controls redemption offer vault token accounts
    ///
    /// This PDA manages token transfers for redemption operations.
    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::REDEMPTION_OFFER_VAULT_AUTHORITY], bump)]
    pub redemption_vault_authority: UncheckedAccount<'info>,

    /// The input token mint for redemptions
    ///
    /// This is the token_out_mint from the original offer.
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token program interface for the input token
    pub token_in_program: Interface<'info, TokenInterface>,

    /// Vault account for storing input tokens during redemption operations
    ///
    /// Created automatically if needed. Used for holding ONyc tokens before burning.
    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = token_in_mint,
        associated_token::authority = redemption_vault_authority,
        associated_token::token_program = token_in_program
    )]
    pub vault_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The output token mint for redemptions (e.g., USDC)
    ///
    /// This is the token_in_mint from the original offer.
    pub token_out_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token program interface for the output token
    pub token_out_program: Interface<'info, TokenInterface>,

    /// Vault account for storing output tokens (e.g., USDC) for redemption payouts
    ///
    /// Created automatically if needed. Used for distributing stable tokens to redeemers.
    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = token_out_mint,
        associated_token::authority = redemption_vault_authority,
        associated_token::token_program = token_out_program
    )]
    pub vault_token_out_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The redemption offer account storing redemption configuration
    ///
    /// This account is derived from token mint addresses in the same order as Offer
    /// but with the tokens reversed (token_in from Offer becomes token_out here).
    #[account(
        init,
        payer = signer,
        space = 8 + RedemptionOffer::INIT_SPACE,
        seeds = [
            seeds::REDEMPTION_OFFER,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref()
        ],
        bump
    )]
    pub redemption_offer: Account<'info, RedemptionOffer>,

    /// The account creating the redemption offer (must be boss or redemption_admin)
    #[account(
        mut,
        constraint = signer.key() == state.boss || signer.key() == state.redemption_admin
            @ MakeRedemptionOfferErrorCode::Unauthorized
    )]
    pub signer: Signer<'info>,

    /// Associated Token Program for automatic token account creation
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// System program for account creation and rent payment
    pub system_program: Program<'info, System>,
}

/// Creates a redemption offer for converting ONyc back to stable tokens
///
/// This instruction initializes a new redemption offer that allows users to redeem
/// input tokens for output tokens (e.g., USDC) at the current NAV price. The redemption
/// offer is the inverse of the standard Offer - it takes the output token of the standard Offer
/// as input token and provides the input token of the standard Offer as output token.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
///
/// # Returns
/// * `Ok(())` - If the redemption offer is successfully created
/// * `Err(MakeRedemptionOfferErrorCode::Unauthorized)` - If caller is neither boss nor redemption_admin (validated in accounts)
/// * `Err(MakeRedemptionOfferErrorCode::InvalidTokenIn)` - If token_in is not the ONyc mint (validated in accounts)
///
/// # Access Control
/// - Only the boss or redemption_admin can call this instruction
///
/// # Effects
/// - Creates new redemption offer account with reference to the original offer
/// - Initializes vault token accounts if needed for both token_in and token_out
/// - Sets up redemption tracking counters (executed_redemptions, requested_redemptions)
///
/// # Events
/// * `RedemptionOfferCreatedEvent` - Emitted with redemption offer details and configuration
pub fn make_redemption_offer(ctx: Context<MakeRedemptionOffer>) -> Result<()> {
    // Initialize the redemption offer
    let redemption_offer = &mut ctx.accounts.redemption_offer;
    redemption_offer.offer = ctx.accounts.offer.key();
    redemption_offer.token_in_mint = ctx.accounts.token_in_mint.key();
    redemption_offer.token_out_mint = ctx.accounts.token_out_mint.key();
    redemption_offer.executed_redemptions = 0;
    redemption_offer.requested_redemptions = 0;
    redemption_offer.bump = ctx.bumps.redemption_offer;

    msg!(
        "Redemption offer created at: {}",
        ctx.accounts.redemption_offer.key()
    );

    emit!(RedemptionOfferCreatedEvent {
        redemption_offer_pda: ctx.accounts.redemption_offer.key(),
        offer: ctx.accounts.offer.key(),
        token_in_mint: ctx.accounts.token_in_mint.key(),
        token_out_mint: ctx.accounts.token_out_mint.key(),
    });

    Ok(())
}

/// Error codes for redemption offer creation operations
#[error_code]
pub enum MakeRedemptionOfferErrorCode {
    /// Caller is not authorized (must be boss or redemption_admin)
    #[msg("Unauthorized: only boss or redemption_admin can create redemption offers")]
    Unauthorized,
}
