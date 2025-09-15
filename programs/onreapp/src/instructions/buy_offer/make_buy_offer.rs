use super::buy_offer_state::BuyOfferAccount;
use crate::constants::seeds;
use crate::state::State;
use crate::utils::MAX_BASIS_POINTS;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};

/// Event emitted when a buy offer is created.
#[event]
pub struct BuyOfferMadeEvent {
    pub offer_id: u64,
    pub fee_basis_points: u64,
    pub boss: Pubkey,
}

/// Account structure for creating a buy offer.
///
/// This struct defines the accounts required to initialize a buy offer where the boss provides
/// token_in in exchange for token_out. The price can change dynamically over the offer's duration.
///
/// # Preconditions
/// - All Associated Token Accounts (ATAs) must be initialized prior to execution.
#[derive(Accounts)]
pub struct MakeBuyOffer<'info> {
    /// The buy offer account within the BuyOfferAccount, rent paid by `boss`. Already initialized in initialize.
    #[account(mut, seeds = [seeds::BUY_OFFERS], bump)]
    pub buy_offer_account: AccountLoader<'info, BuyOfferAccount>,

    /// The buy offer vault authority PDA that controls vault token accounts
    #[account(seeds = [seeds::BUY_OFFER_VAULT_AUTHORITY], bump)]
    /// CHECK: This is safe as it's a PDA used for signing
    pub vault_authority: UncheckedAccount<'info>,

    /// Mint of the token_in for the offer.
    pub token_in_mint: Box<Account<'info, Mint>>,

    /// Vault token_in account, used to transfer tokens to a program owned account for burning
    /// when program has mint authority.
    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = token_in_mint,
        associated_token::authority = vault_authority
    )]
    pub vault_token_in_account: Account<'info, TokenAccount>,

    /// Mint of the token_out for the offer.
    pub token_out_mint: Box<Account<'info, Mint>>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// The signer funding and authorizing the offer creation.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// SPL Token program for token transfers
    pub token_program: Program<'info, Token>,

    /// Associated Token Program for automatic token account creation
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

/// Creates a buy offer.
///
/// Initializes a buy offer where the boss provides token_in in exchange for token_out.
/// The price of the token_out can change dynamically over the offer's duration.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the offer.
/// - `fee_basis_points`: Fee in basis points (e.g., 500 = 5%) charged when taking the offer.
///
/// # Errors
/// - [`MakeBuyOfferErrorCode::AccountFull`] if the BuyOfferAccount is full.
/// - [`MakeBuyOfferErrorCode::InvalidFee`] if fee_basis_points > 10000.
pub fn make_buy_offer(ctx: Context<MakeBuyOffer>, fee_basis_points: u64) -> Result<()> {
    // Validate fee is within valid range (0-10000 basis points = 0-100%)
    require!(
        fee_basis_points <= MAX_BASIS_POINTS,
        MakeBuyOfferErrorCode::InvalidFee
    );

    let buy_offer_account = &mut ctx.accounts.buy_offer_account.load_mut()?;

    // Find the next available slot
    let slot_index = buy_offer_account
        .offers
        .iter()
        .position(|offer| offer.offer_id == 0)
        .ok_or(MakeBuyOfferErrorCode::AccountFull)?;

    // Get the next offer ID and update counter
    let offer_id = buy_offer_account.counter.saturating_add(1);
    buy_offer_account.counter = offer_id;

    // Create the buy offer
    let buy_offer = &mut buy_offer_account.offers[slot_index];
    buy_offer.offer_id = offer_id;
    buy_offer.token_in_mint = ctx.accounts.token_in_mint.key();
    buy_offer.token_out_mint = ctx.accounts.token_out_mint.key();
    buy_offer.fee_basis_points = fee_basis_points;

    msg!("Buy offer created with ID: {}", offer_id);

    emit!(BuyOfferMadeEvent {
        offer_id,
        fee_basis_points,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}

/// Error codes for buy offer creation operations.
#[error_code]
pub enum MakeBuyOfferErrorCode {
    /// Triggered when the BuyOfferAccount is full.
    #[msg("Buy offer account is full, cannot create more offers")]
    AccountFull,
    /// Triggered when fee_basis_points is greater than 10000.
    #[msg("Invalid fee: fee_basis_points must be <= 10000")]
    InvalidFee,
}
