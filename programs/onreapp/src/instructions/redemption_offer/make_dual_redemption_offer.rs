use crate::constants::seeds;
use crate::instructions::DualRedemptionOfferAccount;
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

/// Event emitted when a dual redemption offer is created.
#[event]
pub struct DualRedemptionOfferMadeEvent {
    pub offer_id: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub price_1: u64,
    pub price_2: u64,
    pub ratio_basis_points: u64,
    pub fee_basis_points: u64,
    pub boss: Pubkey,
}

/// Account structure for creating a dual redemption offer.
///
/// This struct defines the accounts required to initialize a dual redemption offer where users can
/// exchange token_in for two different token_out at fixed prices with a specified ratio during the offer's duration.
///
/// # Preconditions
/// - All Associated Token Accounts (ATAs) must be initialized prior to execution.
#[derive(Accounts)]
pub struct MakeDualRedemptionOffer<'info> {
    /// The dual redemption offer account within the DualRedemptionOfferAccount, rent paid by `boss`. Already initialized in initialize.
    #[account(mut, seeds = [seeds::DUAL_REDEMPTION_OFFERS], bump)]
    pub dual_redemption_offer_account: AccountLoader<'info, DualRedemptionOfferAccount>,

    /// Mint of the token_in for the offer.
    pub token_in_mint: Box<Account<'info, Mint>>,

    /// Mint of the first token_out for the offer.
    pub token_out_mint_1: Box<Account<'info, Mint>>,

    /// Mint of the second token_out for the offer.
    pub token_out_mint_2: Box<Account<'info, Mint>>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// The signer funding and authorizing the offer creation.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

/// Creates a dual redemption offer.
///
/// Initializes a dual redemption offer where users can exchange token_in for two different token_out at fixed prices.
/// The ratio_basis_points determines the split: 8000 = 80% goes to token_out_1, 20% goes to token_out_2.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the dual redemption offer.
/// - `start_time`: Unix timestamp for when the offer becomes active.
/// - `end_time`: Unix timestamp for when the offer expires.
/// - `price_1`: Fixed price for token_out_1 with 9 decimal precision (e.g., 1.5 = 1500000000).
/// - `price_2`: Fixed price for token_out_2 with 9 decimal precision (e.g., 2.0 = 2000000000).
/// - `ratio_basis_points`: Ratio in basis points for token_out_1 (e.g., 8000 = 80% for token_out_1, 20% for token_out_2).
/// - `fee_basis_points`: Fee in basis points (e.g., 500 = 5%) charged when taking the offer.
///
/// # Errors
/// - [`DualRedemptionOfferErrorCode::AccountFull`] if the DualRedemptionOfferAccount is full.
/// - [`DualRedemptionOfferErrorCode::InvalidRatio`] if ratio_basis_points > 10000.
/// - [`DualRedemptionOfferErrorCode::InvalidFee`] if fee_basis_points > 10000.
pub fn make_dual_redemption_offer(
    ctx: Context<MakeDualRedemptionOffer>,
    start_time: u64,
    end_time: u64,
    price_1: u64,
    price_2: u64,
    ratio_basis_points: u64,
    fee_basis_points: u64,
) -> Result<()> {
    // Validate ratio is within valid range (0-10000 basis points = 0-100%)
    if ratio_basis_points > 10000 {
        return Err(error!(DualRedemptionOfferErrorCode::InvalidRatio));
    }

    // Validate fee is within valid range (0-10000 basis points = 0-100%)
    if fee_basis_points > 10000 {
        return Err(error!(DualRedemptionOfferErrorCode::InvalidFee));
    }

    let dual_redemption_offer_account =
        &mut ctx.accounts.dual_redemption_offer_account.load_mut()?;

    let slot_index = dual_redemption_offer_account
        .offers
        .iter()
        .position(|offer| offer.offer_id == 0)
        .ok_or(DualRedemptionOfferErrorCode::AccountFull)?;

    let offer_id = dual_redemption_offer_account.counter.saturating_add(1);
    dual_redemption_offer_account.counter = offer_id;

    let dual_redemption_offer = &mut dual_redemption_offer_account.offers[slot_index];
    dual_redemption_offer.offer_id = offer_id;
    dual_redemption_offer.token_in_mint = ctx.accounts.token_in_mint.key();
    dual_redemption_offer.token_out_mint_1 = ctx.accounts.token_out_mint_1.key();
    dual_redemption_offer.token_out_mint_2 = ctx.accounts.token_out_mint_2.key();
    dual_redemption_offer.price_1 = price_1;
    dual_redemption_offer.price_2 = price_2;
    dual_redemption_offer.ratio_basis_points = ratio_basis_points;
    dual_redemption_offer.fee_basis_points = fee_basis_points;
    dual_redemption_offer.start_time = start_time;
    dual_redemption_offer.end_time = end_time;

    msg!(
        "Dual redemption offer created with ID: {}, price_1: {}, price_2: {}, ratio: {}bp, startTime: {}, endTime: {}",
        offer_id,
        price_1,
        price_2,
        ratio_basis_points,
        start_time,
        end_time
    );

    emit!(DualRedemptionOfferMadeEvent{
        offer_id,
        start_time,
        end_time,
        price_1,
        price_2,
        ratio_basis_points,
        fee_basis_points,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}

/// Error codes for dual redemption offer creation operations.
#[error_code]
pub enum DualRedemptionOfferErrorCode {
    /// Triggered when the DualRedemptionOfferAccount is full.
    #[msg("Dual redemption offer account is full, cannot create more offers")]
    AccountFull,
    /// Triggered when ratio_basis_points is greater than 10000.
    #[msg("Invalid ratio: ratio_basis_points must be <= 10000")]
    InvalidRatio,
    /// Triggered when fee_basis_points is greater than 10000.
    #[msg("Invalid fee: fee_basis_points must be <= 10000")]
    InvalidFee,
}