use crate::state::State;
use super::state::{BuyOfferAccount, BuyOfferTimeSegment, MAX_BUY_OFFERS};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

/// Event emitted when a buy offer is created.
#[event]
pub struct BuyOfferMadeEvent {
    pub offer_id: u64,
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
#[instruction(offer_id: u64)]
pub struct MakeBuyOffer<'info> {

    /// The buy offer account within the BuyOfferAccount, rent paid by `boss`. Already initialized in initialize.
    #[account(mut, seeds = [b"buy_offers"], bump)]
    pub buy_offer_account: AccountLoader<'info, BuyOfferAccount>,

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

    /// SPL Token program for token operations.
    pub token_program: Program<'info, Token>,

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
pub fn make_buy_offer(
    ctx: Context<MakeBuyOffer>,
    offer_id: u64
) -> Result<()> {
    let buy_offer_account = &mut ctx.accounts.buy_offer_account.load_mut()?;

    // Check if account is full
    require!(
        buy_offer_account.count < MAX_BUY_OFFERS as u64,
        MakeBuyOfferErrorCode::AccountFull
    );

    // Find the next available slot
    let slot_index = buy_offer_account.count as usize;
    
    // Create the buy offer
    let buy_offer = &mut buy_offer_account.offers[slot_index];
    buy_offer.offer_id = offer_id;
    buy_offer.token_in_mint = ctx.accounts.token_in_mint.key();
    buy_offer.token_out_mint = ctx.accounts.token_out_mint.key();
    
    // Initialize time segments to default
    for i in 0..buy_offer.time_segments.len() {
        buy_offer.time_segments[i] = BuyOfferTimeSegment::default();
    }

    buy_offer_account.count += 1;

    msg!("Buy offer created with ID: {}", offer_id);

    emit!(BuyOfferMadeEvent {
        offer_id,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}

/// Transfers tokens from a source to a destination account.
// fn transfer_token(
//     ctx: &Context<MakeBuyOffer>,
//     amount: u64,
// ) -> Result<()> {
//     let cpi_ctx = CpiContext::new(
//         ctx.accounts.token_program.to_account_info(),
//         Transfer {
//             from: ctx.accounts.boss_token_in_account.to_account_info(),
//             to: ctx.accounts.offer_token_in_account.to_account_info(),
//             authority: ctx.accounts.boss.to_account_info(),
//         },
//     );
//     token::transfer(cpi_ctx, amount)?;
//     Ok(())
// }

fn validate_non_zero_amounts(amounts: &[u64]) -> Result<()> {
    require!(
        amounts.iter().all(|&x| x > 0),
        MakeBuyOfferErrorCode::InvalidAmount
    );
    Ok(())
}

fn validate_time_params(start_time: u64, end_time: u64, price_fix_duration: u64) -> Result<()> {
    require!(
        start_time < end_time,
        MakeBuyOfferErrorCode::InvalidOfferTime
    );
    require!(
        price_fix_duration > 0,
        MakeBuyOfferErrorCode::InvalidPriceFixDuration
    );
    require!(
        (end_time - start_time) >= price_fix_duration,
        MakeBuyOfferErrorCode::InvalidPriceFixDuration
    );
    Ok(())
}

/// Error codes for buy offer creation operations.
#[error_code]
pub enum MakeBuyOfferErrorCode {
    /// Triggered when the boss's token account doesn't have sufficient balance.
    #[msg("Insufficient token balance in boss account")]
    InsufficientBalance,

    /// Triggered when amounts are zero or invalid.
    #[msg("Token amounts must be greater than zero")]
    InvalidAmount,

    /// Triggered when offer times are invalid.
    #[msg("Offer end time must be greater than start time")]
    InvalidOfferTime,

    /// Triggered when the price fix duration is invalid.
    #[msg("Price fix duration must be greater than zero and less than or equal to the total offer duration")]
    InvalidPriceFixDuration,

    /// Triggered when the BuyOfferAccount is full.
    #[msg("Buy offer account is full, cannot create more offers")]
    AccountFull,
}