use super::state::MAX_REDEMPTION_OFFERS;
use crate::instructions::SingleRedemptionOfferAccount;
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

/// Event emitted when a buy offer is created.
#[event]
pub struct SingleRedemptionOfferMadeEvent {
    pub offer_id: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub price: u64,
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
pub struct MakeSingleRedemptionOffer<'info> {
    /// The buy offer account within the BuyOfferAccount, rent paid by `boss`. Already initialized in initialize.
    #[account(mut, seeds = [b"single_redemption_offers"], bump)]
    pub single_redemption_offer_account: AccountLoader<'info, SingleRedemptionOfferAccount>,

    /// Mint of the token_in for the offer.
    pub token_in_mint: Box<Account<'info, Mint>>,

    /// Mint of the token_out for the offer.
    pub token_out_mint: Box<Account<'info, Mint>>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// The signer funding and authorizing the offer creation.
    #[account(mut)]
    pub boss: Signer<'info>,

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
/// - `offer_id`: Unique identifier for the offer.
/// - `token_in_amount`: Total amount of the token_in to be offered.
/// - `start_price`: Price at the beginning of the offer.
/// - `end_price`: Price at the end of the offer.
/// - `start_time`: Unix timestamp for when the offer becomes active.
/// - `end_time`: Unix timestamp for when the offer expires.
/// - `price_fix_duration`: Duration in seconds for each price interval.
///
/// # Errors
/// - [`MakeBuyOfferErrorCode::InsufficientBalance`] if the boss lacks sufficient `token_in_amount`.
/// - [`MakeBuyOfferErrorCode::InvalidAmount`] if amounts are zero or invalid.
/// - [`MakeBuyOfferErrorCode::InvalidOfferTime`] if times are invalid.
/// - [`MakeBuyOfferErrorCode::InvalidPriceFixDuration`] if duration is invalid.
/// - [`MakeBuyOfferErrorCode::AccountFull`] if the BuyOfferAccount is full.
pub fn make_single_redemption_offer(
    ctx: Context<MakeSingleRedemptionOffer>,
    start_time: u64,
    end_time: u64,
    price: u64,
) -> Result<()> {
    let single_redemption_offer_account =
        &mut ctx.accounts.single_redemption_offer_account.load_mut()?;

    let slot_index = single_redemption_offer_account
        .offers
        .iter()
        .position(|offer| offer.offer_id == 0)
        .ok_or(SingleRedemptionOfferErrorCode::AccountFull)?;

    let offer_id = single_redemption_offer_account.counter.saturating_add(1);
    single_redemption_offer_account.counter = offer_id;

    let redemption_offer = &mut single_redemption_offer_account.offers[slot_index];
    redemption_offer.offer_id = offer_id;
    redemption_offer.token_in_mint = ctx.accounts.token_in_mint.key();
    redemption_offer.token_out_mint = ctx.accounts.token_out_mint.key();
    redemption_offer.price = price;
    redemption_offer.start_time = start_time;
    redemption_offer.end_time = end_time;

    msg!(
        "Redemption offer created with ID: {}, price: {}, startTime: {}, endTime: {}",
        offer_id,
        price,
        start_time,
        end_time
    );

    emit!(SingleRedemptionOfferMadeEvent{
        offer_id,
        start_time,
        end_time,
        price,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}

/// Error codes for buy offer creation operations.
#[error_code]
pub enum SingleRedemptionOfferErrorCode {
    /// Triggered when the BuyOfferAccount is full.
    #[msg("Buy offer account is full, cannot create more offers")]
    AccountFull,
}
