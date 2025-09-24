use crate::constants::seeds;
use crate::instructions::Offer;
use crate::state::State;
use crate::utils::MAX_BASIS_POINTS;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

/// Event emitted when an offer is created.
#[event]
pub struct OfferMadeEvent {
    pub offer_pda: Pubkey,
    pub fee_basis_points: u64,
    pub boss: Pubkey,
}

/// Account structure for creating an offer.
///
/// This struct defines the accounts required to initialize an offer where the boss provides
/// token_in in exchange for token_out. The price can change dynamically over the offer's duration.
///
/// # Preconditions
/// - All Associated Token Accounts (ATAs) must be initialized prior to execution.
#[derive(Accounts)]
pub struct MakeOffer<'info> {
    /// The offer vault authority PDA that controls vault token accounts
    #[account(seeds = [seeds::OFFER_VAULT_AUTHORITY], bump)]
    /// CHECK: This is safe as it's a PDA used for signing
    pub vault_authority: UncheckedAccount<'info>,

    /// Mint of the token_in for the offer.
    pub token_in_mint: InterfaceAccount<'info, Mint>,
    pub token_in_program: Interface<'info, TokenInterface>,

    /// Vault token_in account, used to transfer tokens to a program owned account for burning
    /// when program has mint authority.
    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = token_in_mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = token_in_program
    )]
    pub vault_token_in_account: InterfaceAccount<'info, TokenAccount>,

    /// Mint of the token_out for the offer.
    pub token_out_mint: InterfaceAccount<'info, Mint>,

    /// The offer account within the OfferAccount, rent paid by `boss`. Already initialized in initialize.
    #[account(
        init,
        payer = boss,
        space = 8 + Offer::INIT_SPACE,
        seeds = [
            seeds::OFFERS,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref()
        ],
        bump
    )]
    pub offer: AccountLoader<'info, Offer>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Account<'info, State>,

    /// The signer funding and authorizing the offer creation.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Associated Token Program for automatic token account creation
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

/// Creates an offer.
///
/// Initializes an offer where the boss provides token_in in exchange for token_out.
/// The price of the token_out can change dynamically over the offer's duration.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the offer.
/// - `fee_basis_points`: Fee in basis points (e.g., 500 = 5%) charged when taking the offer.
///
/// # Errors
/// - [`MakeOfferErrorCode::AccountFull`] if the OfferAccount is full.
/// - [`MakeOfferErrorCode::InvalidFee`] if fee_basis_points > 10000.
pub fn make_offer(ctx: Context<MakeOffer>, fee_basis_points: u64) -> Result<()> {
    // Validate fee is within valid range (0-10000 basis points = 0-100%)
    require!(
        fee_basis_points <= MAX_BASIS_POINTS,
        MakeOfferErrorCode::InvalidFee
    );

    // Create the offer
    let mut offer = ctx.accounts.offer.load_init()?;
    offer.token_in_mint = ctx.accounts.token_in_mint.key();
    offer.token_out_mint = ctx.accounts.token_out_mint.key();
    offer.fee_basis_points = fee_basis_points;

    msg!("Offer created at: {}", ctx.accounts.offer.key());

    emit!(OfferMadeEvent {
        offer_pda: ctx.accounts.offer.key(),
        fee_basis_points,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}

/// Error codes for offer creation operations.
#[error_code]
pub enum MakeOfferErrorCode {
    /// Triggered when the OfferAccount is full.
    #[msg("Offer account is full, cannot create more offers")]
    AccountFull,
    /// Triggered when fee_basis_points is greater than 10000.
    #[msg("Invalid fee: fee_basis_points must be <= 10000")]
    InvalidFee,
    #[msg("Invalid token program")]
    InvalidTokenProgram,
}
