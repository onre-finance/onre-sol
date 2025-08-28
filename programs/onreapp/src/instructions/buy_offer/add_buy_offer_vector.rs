use super::state::{BuyOffer, BuyOfferAccount, BuyOfferVector};
use crate::instructions::{find_active_vector_at, find_offer_mut};
use crate::state::State;
use anchor_lang::prelude::*;

/// Event emitted when a time vector is added to a buy offer.
#[event]
pub struct BuyOfferVectorAddedEvent {
    pub offer_id: u64,
    pub vector_id: u64,
    pub start_time: u64,
    pub valid_from: u64,
    pub start_price: u64,
    pub price_yield: u64,
    pub price_fix_duration: u64,
}

/// Account structure for adding a time vector to a buy offer.
///
/// This struct defines the accounts required to add a time vector to an existing buy offer.
/// Only the boss can add time vectors to offers.
#[derive(Accounts)]
pub struct AddBuyOfferVector<'info> {
    /// The buy offer account containing all buy offers
    #[account(mut, seeds = [b"buy_offers"], bump)]
    pub buy_offer_account: AccountLoader<'info, BuyOfferAccount>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// The signer authorizing the time vector addition (must be boss).
    #[account(mut)]
    pub boss: Signer<'info>,
}

/// Adds a time vector to an existing buy offer.
///
/// Creates a new time vector with auto-generated vector_id for the specified buy offer.
/// The vector_id is calculated as the highest existing vector_id + 1.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the operation.
/// - `offer_id`: ID of the buy offer to add the vector to.
/// - `start_time`: Unix timestamp when the vector becomes active.
/// - `start_price`: Price at the beginning of the vector.
/// - `price_yield`: Price yield percentage * 10000 (with 4 decimal places).
/// - `price_fix_duration`: Duration in seconds for each price interval.
///
/// # Errors
/// - [`AddBuyOfferVectorErrorCode::OfferNotFound`] if offer_id doesn't exist.
/// - [`AddBuyOfferVectorErrorCode::InvalidTimeRange`] if start_time is before latest existing vector.
/// - [`AddBuyOfferVectorErrorCode::ZeroValue`] if any value is zero.
/// - [`AddBuyOfferVectorErrorCode::TooManyVectors`] if the offer already has MAX_SEGMENTS.
pub fn add_buy_offer_vector(
    ctx: Context<AddBuyOfferVector>,
    offer_id: u64,
    start_time: u64,
    start_price: u64,
    price_yield: u64,
    price_fix_duration: u64,
) -> Result<()> {
    let buy_offer_account = &mut ctx.accounts.buy_offer_account.load_mut()?;

    let current_time = Clock::get()?.unix_timestamp as u64;

    validate_inputs(
        offer_id,
        start_time,
        start_price,
        price_yield,
        price_fix_duration,
    )?;

    // Find the offer by offer_id
    let offer = find_offer_mut(buy_offer_account, offer_id)?;

    // Validate start_time is not before the latest existing vector's start_time
    let latest_start_time = offer
        .vectors
        .iter()
        .filter(|vector| vector.vector_id != 0)
        .map(|vector| vector.start_time)
        .max()
        .unwrap_or(0);

    require!(
        start_time > latest_start_time,
        AddBuyOfferVectorErrorCode::InvalidTimeRange
    );

    let next_vector_id = offer.counter + 1;
    offer.counter = next_vector_id;

    // Calculate valid_from: max(start_time, current_time)
    let valid_from = if start_time > current_time {
        start_time
    } else {
        current_time
    };

    // Find an empty slot in time_vectors array
    let empty_slot_index = find_empty_vector_slot(&offer.vectors)?;

    // Create the new time vector
    let new_vector = BuyOfferVector {
        vector_id: next_vector_id,
        valid_from,
        start_time,
        start_price,
        price_yield,
        price_fix_duration,
    };

    // Add the vector to the offer
    offer.vectors[empty_slot_index] = new_vector;

    // Clean up old vectors before emitting success message
    clean_old_vectors(offer, current_time)?;

    msg!(
        "Time vector added to buy offer ID: {}, vector ID: {}",
        offer_id,
        next_vector_id
    );

    emit!(BuyOfferVectorAddedEvent {
        offer_id,
        vector_id: next_vector_id,
        start_time,
        valid_from,
        start_price,
        price_yield,
        price_fix_duration,
    });

    Ok(())
}

fn validate_inputs(
    offer_id: u64,
    start_time: u64,
    start_price: u64,
    price_yield: u64,
    price_fix_duration: u64,
) -> Result<()> {
    // Validate input parameters
    require!(offer_id > 0, AddBuyOfferVectorErrorCode::ZeroValue);
    require!(start_time > 0, AddBuyOfferVectorErrorCode::ZeroValue);
    require!(start_price > 0, AddBuyOfferVectorErrorCode::ZeroValue);
    require!(price_yield > 0, AddBuyOfferVectorErrorCode::ZeroValue);
    require!(
        price_fix_duration > 0,
        AddBuyOfferVectorErrorCode::ZeroValue
    );

    Ok(())
}

/// Finds the first empty slot in the time_vectors array.
fn find_empty_vector_slot(vectors: &[BuyOfferVector; 10]) -> Result<usize> {
    vectors
        .iter()
        .position(|vector| vector.vector_id == 0)
        .ok_or(AddBuyOfferVectorErrorCode::TooManyVectors.into())
}

/// Cleans old inactive vectors from the offer, keeping only the currently active vector
/// and the last previously active vector.
///
/// # Arguments
/// - `offer`: Mutable reference to the buy offer containing vectors to clean
/// - `current_time`: Current unix timestamp for determining active vector
///
/// # Behavior
/// - Finds the currently active vector (most recent valid_from <= current_time)
/// - Finds the previously active vector (closest smaller vector_id to active vector)
/// - Deletes all other vectors by setting them to default (vector_id = 0)
fn clean_old_vectors(offer: &mut BuyOffer, current_time: u64) -> Result<()> {
    // Find currently active vector
    let active_vector = find_active_vector_at(offer, current_time);

    let active_vector_id = match active_vector {
        Ok(vector) => vector.vector_id,
        Err(_) => return Ok(()), // No active vector found, nothing to clean
    };

    // Find previously active vector (closest smaller vector_id)
    let prev_vector = find_active_vector_at(offer, active_vector?.valid_from - 1);

    let prev_vector_id = match prev_vector {
        Ok(vector) => vector.vector_id,
        Err(_) => 0, // If no previous vector exists, use 0
    };

    // Clear all vectors except active and previous
    for vector in offer.vectors.iter_mut() {
        if vector.vector_id != 0 // Don't touch already empty slots
            && vector.vector_id != active_vector_id // Keep active vector
            && vector.vector_id != prev_vector_id // Keep previous vector
            && vector.vector_id < active_vector_id
        // Keep all future vectors
        {
            *vector = BuyOfferVector::default(); // Clear the vector
        }
    }

    Ok(())
}

/// Error codes for add buy offer vector operations.
#[error_code]
pub enum AddBuyOfferVectorErrorCode {
    /// Triggered when start_time is before the latest existing vector.
    #[msg("Invalid time range: start_time must be after the latest existing vector")]
    InvalidTimeRange,

    /// Triggered when any required value is zero.
    #[msg("Invalid input: values cannot be zero")]
    ZeroValue,

    /// Triggered when the offer already has the maximum number of vectors.
    #[msg("Cannot add more vectors: maximum limit reached")]
    TooManyVectors,
}
