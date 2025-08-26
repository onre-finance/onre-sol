use crate::constants::seeds;
use crate::instructions::SingleRedemptionOfferAccount;
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

/// Event emitted when a single redemption offer is created.
#[event]
pub struct SingleRedemptionOfferMadeEvent {
    pub offer_id: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub price: u64,
    pub boss: Pubkey,
}

/// Account structure for creating a single redemption offer.
///
/// This struct defines the accounts required to initialize a redemption offer where users can
/// exchange token_in for token_out at a fixed price during the offer's duration.
///
/// # Preconditions
/// - All Associated Token Accounts (ATAs) must be initialized prior to execution.
#[derive(Accounts)]
pub struct MakeSingleRedemptionOffer<'info> {
    /// The single redemption offer account within the SingleRedemptionOfferAccount, rent paid by `boss`. Already initialized in initialize.
    #[account(mut, seeds = [seeds::SINGLE_REDEMPTION_OFFERS], bump)]
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

/// Creates a single redemption offer.
///
/// Initializes a redemption offer where users can exchange token_in for token_out at a fixed price.
/// The price remains constant throughout the offer's duration.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the redemption offer.
/// - `start_time`: Unix timestamp for when the offer becomes active.
/// - `end_time`: Unix timestamp for when the offer expires.
/// - `price`: How much token_in needed for 1 token_out, with 9 decimal precision (e.g., 1.5 = 1500000000).
///
/// # Errors
/// - [`SingleRedemptionOfferErrorCode::AccountFull`] if the SingleRedemptionOfferAccount is full.
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

/// Error codes for single redemption offer creation operations.
#[error_code]
pub enum SingleRedemptionOfferErrorCode {
    /// Triggered when the SingleRedemptionOfferAccount is full.
    #[msg("Single redemption offer account is full, cannot create more offers")]
    AccountFull,
}
