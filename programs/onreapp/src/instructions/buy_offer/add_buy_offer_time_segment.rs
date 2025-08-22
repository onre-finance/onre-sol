use crate::state::State;
use super::state::{BuyOfferAccount, BuyOfferTimeSegment, BuyOffer, MAX_BUY_OFFERS};
use anchor_lang::prelude::*;

/// Event emitted when a time segment is added to a buy offer.
#[event]
pub struct BuyOfferTimeSegmentAddedEvent {
    pub offer_id: u64,
    pub segment_id: u64,
    pub start_time: u64,
    pub valid_from: u64,
    pub start_price: u64,
    pub price_yield: u64,
    pub price_fix_duration: u64,
}

/// Account structure for adding a time segment to a buy offer.
///
/// This struct defines the accounts required to add a time segment to an existing buy offer.
/// Only the boss can add time segments to offers.
#[derive(Accounts)]
pub struct AddBuyOfferTimeSegment<'info> {
    /// The buy offer account containing all buy offers
    #[account(mut, seeds = [b"buy_offers"], bump)]
    pub buy_offer_account: AccountLoader<'info, BuyOfferAccount>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// The signer authorizing the time segment addition (must be boss).
    #[account(mut)]
    pub boss: Signer<'info>,
}

/// Adds a time segment to an existing buy offer.
///
/// Creates a new time segment with auto-generated segment_id for the specified buy offer.
/// The segment_id is calculated as the highest existing segment_id + 1.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the operation.
/// - `offer_id`: ID of the buy offer to add the segment to.
/// - `start_time`: Unix timestamp when the segment becomes active.
/// - `start_price`: Price at the beginning of the segment.
/// - `price_yield`: Price yield percentage * 10000 (with 4 decimal places).
/// - `price_fix_duration`: Duration in seconds for each price interval.
///
/// # Errors
/// - [`AddBuyOfferTimeSegmentErrorCode::OfferNotFound`] if offer_id doesn't exist.
/// - [`AddBuyOfferTimeSegmentErrorCode::InvalidTimeRange`] if start_time is before latest existing segment.
/// - [`AddBuyOfferTimeSegmentErrorCode::ZeroValue`] if any value is zero.
/// - [`AddBuyOfferTimeSegmentErrorCode::TooManySegments`] if the offer already has MAX_SEGMENTS.
pub fn add_buy_offer_time_segment(
    ctx: Context<AddBuyOfferTimeSegment>,
    offer_id: u64,
    start_time: u64,
    start_price: u64,
    price_yield: u64,
    price_fix_duration: u64,
) -> Result<()> {
    let buy_offer_account = &mut ctx.accounts.buy_offer_account.load_mut()?;

    let current_time = Clock::get()?.unix_timestamp as u64;

    // Validate input parameters
    require!(offer_id > 0, AddBuyOfferTimeSegmentErrorCode::ZeroValue);
    require!(start_time > 0, AddBuyOfferTimeSegmentErrorCode::ZeroValue);
    require!(start_price > 0, AddBuyOfferTimeSegmentErrorCode::ZeroValue);
    require!(price_yield > 0, AddBuyOfferTimeSegmentErrorCode::ZeroValue);
    require!(price_fix_duration > 0, AddBuyOfferTimeSegmentErrorCode::ZeroValue);

    // Find the offer by offer_id
    let offer = find_buy_offer_by_id(&mut buy_offer_account.offers, offer_id)?;

    // Validate start_time is not before the latest existing segment's start_time
    let latest_start_time = offer.time_segments
        .iter()
        .filter(|segment| segment.segment_id != 0)
        .map(|segment| segment.start_time)
        .max()
        .unwrap_or(0);
    
    require!(
        start_time > latest_start_time,
        AddBuyOfferTimeSegmentErrorCode::InvalidTimeRange
    );

    // Calculate the next segment_id
    let next_segment_id = calculate_next_segment_id(&offer.time_segments);

    // Calculate valid_from: max(start_time, current_time) 
    let valid_from = if start_time > current_time {
        start_time
    } else {
        current_time
    };

    // Find an empty slot in time_segments array
    let empty_slot_index = find_empty_segment_slot(&offer.time_segments)?;

    // Create the new time segment
    let new_segment = BuyOfferTimeSegment {
        segment_id: next_segment_id,
        valid_from,
        start_time,
        start_price,
        price_yield,
        price_fix_duration,
    };

    // Add the segment to the offer
    offer.time_segments[empty_slot_index] = new_segment;

    msg!(
        "Time segment added to buy offer ID: {}, segment ID: {}",
        offer_id,
        next_segment_id
    );

    emit!(BuyOfferTimeSegmentAddedEvent {
        offer_id,
        segment_id: next_segment_id,
        start_time,
        valid_from,
        start_price,
        price_yield,
        price_fix_duration,
    });

    Ok(())
}

/// Finds a buy offer by its ID in the offers array.
fn find_buy_offer_by_id(offers: &mut [BuyOffer; MAX_BUY_OFFERS], offer_id: u64) -> Result<&mut BuyOffer> {
    offers
        .iter_mut()
        .find(|offer| offer.offer_id == offer_id && offer.offer_id != 0)
        .ok_or(AddBuyOfferTimeSegmentErrorCode::OfferNotFound.into())
}

/// Calculates the next segment_id by finding the highest existing segment_id and adding 1.
fn calculate_next_segment_id(segments: &[BuyOfferTimeSegment; 10]) -> u64 {
    let max_segment_id = segments
        .iter()
        .filter(|segment| segment.segment_id != 0)
        .map(|segment| segment.segment_id)
        .max()
        .unwrap_or(0);
    
    max_segment_id + 1
}


/// Finds the first empty slot in the time_segments array.
fn find_empty_segment_slot(segments: &[BuyOfferTimeSegment; 10]) -> Result<usize> {
    segments
        .iter()
        .position(|segment| segment.segment_id == 0)
        .ok_or(AddBuyOfferTimeSegmentErrorCode::TooManySegments.into())
}

/// Error codes for add buy offer time segment operations.
#[error_code]
pub enum AddBuyOfferTimeSegmentErrorCode {
    /// Triggered when the specified offer_id is not found.
    #[msg("Buy offer with the specified ID was not found")]
    OfferNotFound,

    /// Triggered when start_time is before the latest existing segment.
    #[msg("Invalid time range: start_time must be after the latest existing segment")]
    InvalidTimeRange,

    /// Triggered when any required value is zero.
    #[msg("Invalid input: values cannot be zero")]
    ZeroValue,

    /// Triggered when the offer already has the maximum number of segments.
    #[msg("Cannot add more segments: maximum limit reached")]
    TooManySegments,
}