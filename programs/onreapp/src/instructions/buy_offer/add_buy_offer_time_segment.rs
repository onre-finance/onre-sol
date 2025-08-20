use crate::state::State;
use super::state::{BuyOfferAccount, BuyOfferTimeSegment, BuyOffer, MAX_BUY_OFFERS};
use anchor_lang::prelude::*;

/// Event emitted when a time segment is added to a buy offer.
#[event]
pub struct BuyOfferTimeSegmentAddedEvent {
    pub offer_id: u64,
    pub segment_id: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub start_price: u64,
    pub end_price: u64,
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
/// - `end_time`: Unix timestamp when the segment expires.
/// - `start_price`: Price at the beginning of the segment.
/// - `end_price`: Price at the end of the segment.
/// - `price_fix_duration`: Duration in seconds for each price interval.
///
/// # Errors
/// - [`AddBuyOfferTimeSegmentErrorCode::OfferNotFound`] if offer_id doesn't exist.
/// - [`AddBuyOfferTimeSegmentErrorCode::InvalidTimeRange`] if start_time >= end_time.
/// - [`AddBuyOfferTimeSegmentErrorCode::InvalidPriceRange`] if start_price >= end_price.
/// - [`AddBuyOfferTimeSegmentErrorCode::ZeroValue`] if any value is zero.
/// - [`AddBuyOfferTimeSegmentErrorCode::OverlappingSegment`] if the segment overlaps with existing ones.
/// - [`AddBuyOfferTimeSegmentErrorCode::TooManySegments`] if the offer already has MAX_SEGMENTS.
pub fn add_buy_offer_time_segment(
    ctx: Context<AddBuyOfferTimeSegment>,
    offer_id: u64,
    start_time: u64,
    end_time: u64,
    start_price: u64,
    end_price: u64,
    price_fix_duration: u64,
) -> Result<()> {
    let buy_offer_account = &mut ctx.accounts.buy_offer_account.load_mut()?;

    // Validate input parameters
    require!(offer_id > 0, AddBuyOfferTimeSegmentErrorCode::ZeroValue);
    require!(start_time > 0, AddBuyOfferTimeSegmentErrorCode::ZeroValue);
    require!(end_time > 0, AddBuyOfferTimeSegmentErrorCode::ZeroValue);
    require!(start_price > 0, AddBuyOfferTimeSegmentErrorCode::ZeroValue);
    require!(end_price > 0, AddBuyOfferTimeSegmentErrorCode::ZeroValue);
    require!(price_fix_duration > 0, AddBuyOfferTimeSegmentErrorCode::ZeroValue);

    // Validate time range
    require!(
        start_time < end_time,
        AddBuyOfferTimeSegmentErrorCode::InvalidTimeRange
    );

    // Validate price range
    require!(
        start_price < end_price,
        AddBuyOfferTimeSegmentErrorCode::InvalidPriceRange
    );

    // Find the offer by offer_id
    let offer = find_buy_offer_by_id(&mut buy_offer_account.offers, offer_id)?;

    // Calculate the next segment_id
    let next_segment_id = calculate_next_segment_id(&offer.time_segments);

    // Validate no overlapping segments
    validate_no_overlap(&offer.time_segments, start_time, end_time)?;

    // Find an empty slot in time_segments array
    let empty_slot_index = find_empty_segment_slot(&offer.time_segments)?;

    // Create the new time segment
    let new_segment = BuyOfferTimeSegment {
        segment_id: next_segment_id,
        start_time,
        end_time,
        start_price,
        end_price,
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
        end_time,
        start_price,
        end_price,
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

/// Validates that the new time segment doesn't overlap with existing segments.
fn validate_no_overlap(
    segments: &[BuyOfferTimeSegment; 10],
    start_time: u64,
    end_time: u64,
) -> Result<()> {
    for segment in segments.iter() {
        // Skip empty segments
        if segment.segment_id == 0 {
            continue;
        }

        // Check for overlap: new segment starts before existing ends and new segment ends after existing starts
        if start_time < segment.end_time && end_time > segment.start_time {
            return Err(AddBuyOfferTimeSegmentErrorCode::OverlappingSegment.into());
        }
    }
    Ok(())
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

    /// Triggered when start_time >= end_time.
    #[msg("Invalid time range: start_time must be less than end_time")]
    InvalidTimeRange,

    /// Triggered when start_price >= end_price.
    #[msg("Invalid price range: start_price must be less than end_price")]
    InvalidPriceRange,

    /// Triggered when any required value is zero.
    #[msg("Invalid input: values cannot be zero")]
    ZeroValue,

    /// Triggered when the new segment overlaps with existing segments.
    #[msg("Time segment overlaps with existing segments")]
    OverlappingSegment,

    /// Triggered when the offer already has the maximum number of segments.
    #[msg("Cannot add more segments: maximum limit reached")]
    TooManySegments,
}