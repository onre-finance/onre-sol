use super::offer_state::{Offer, OfferVector};
use crate::constants::seeds;
use crate::instructions::{find_active_vector_at, find_vector_index_by_start_time};
use crate::state::State;
use crate::OfferCoreError;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

/// Event emitted when a time vector is added to an offer.
#[event]
pub struct OfferVectorAddedEvent {
    pub offer_pda: Pubkey,
    pub start_time: u64,
    pub base_time: u64,
    pub base_price: u64,
    pub apr: u64,
    pub price_fix_duration: u64,
}

/// Account structure for adding a time vector to an offer.
///
/// This struct defines the accounts required to add a time vector to an existing offer.
/// Only the boss can add time vectors to offers.
#[derive(Accounts)]
pub struct AddOfferVector<'info> {
    /// The offer account containing all offers
    #[account(
        mut,
        seeds = [
            seeds::OFFER,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref()
        ],
        bump = offer.load()?.bump
    )]
    pub offer: AccountLoader<'info, Offer>,

    #[account(
        constraint =
            token_in_mint.key() == offer.load()?.token_in_mint
            @ OfferCoreError::InvalidTokenInMint
    )]
    pub token_in_mint: InterfaceAccount<'info, Mint>,

    #[account(
        constraint =
            token_out_mint.key() == offer.load()?.token_out_mint
            @ OfferCoreError::InvalidTokenOutMint
    )]
    pub token_out_mint: InterfaceAccount<'info, Mint>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Account<'info, State>,

    /// The signer authorizing the time vector addition (must be boss).
    #[account(mut)]
    pub boss: Signer<'info>,
}

/// Adds a time vector to an existing offer.
///
/// Creates a new time vector with auto-generated vector_id for the specified offer.
/// The vector_id is calculated as the highest existing vector_id + 1.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the operation.
/// - `base_time`: Unix timestamp when the vector becomes active.
/// - `base_price`: Price at the beginning of the vector.
/// - `apr`: Annual Percentage Rate (APR) in basis points (see OfferVector::apr for details).
/// - `price_fix_duration`: Duration in seconds for each price interval.
///
/// # Errors
/// - [`AddOfferVectorErrorCode::InvalidTimeRange`] if base_time is before latest existing vector.
/// - [`AddOfferVectorErrorCode::ZeroValue`] if any value is zero.
pub fn add_offer_vector(
    ctx: Context<AddOfferVector>,
    base_time: u64,
    base_price: u64,
    apr: u64,
    price_fix_duration: u64,
) -> Result<()> {
    let offer = &mut ctx.accounts.offer.load_mut()?;

    let current_time = Clock::get()?.unix_timestamp as u64;

    validate_inputs(base_time, base_price, price_fix_duration)?;

    // Calculate start_time: max(base_time, current_time)
    let start_time = if base_time > current_time {
        base_time
    } else {
        current_time
    };

    // Validate start_time is not duplicated
    let existing_start_times: Vec<u64> = offer
        .vectors
        .iter()
        .filter(|vector| vector.start_time != 0)
        .map(|vector| vector.start_time)
        .collect();

    require!(
        !existing_start_times.contains(&start_time),
        AddOfferVectorErrorCode::DuplicateStartTime
    );

    if let Some(latest_start_time) = existing_start_times.iter().max() {
        require!(
          &base_time > latest_start_time,
          AddOfferVectorErrorCode::InvalidTimeRange
      );
    }

    // Find an empty slot in time_vectors array
    let empty_slot_index = find_vector_index_by_start_time(&offer, 0)
        .ok_or_else(|| error!(AddOfferVectorErrorCode::TooManyVectors))?;

    // Create the new time vector
    let new_vector = OfferVector {
        start_time,
        base_time,
        base_price,
        apr,
        price_fix_duration,
    };

    // Add the vector to the offer
    offer.vectors[empty_slot_index] = new_vector;

    // Clean up old vectors before emitting success message
    clean_old_vectors(offer, current_time)?;

    msg!(
        "Time vector added to offer: {}, vector start_time: {}",
        ctx.accounts.offer.key(),
        start_time
    );

    emit!(OfferVectorAddedEvent {
        offer_pda: ctx.accounts.offer.key(),
        start_time,
        base_time,
        base_price,
        apr,
        price_fix_duration,
    });

    Ok(())
}

fn validate_inputs(base_time: u64, base_price: u64, price_fix_duration: u64) -> Result<()> {
    // Validate input parameters
    require!(base_time > 0, AddOfferVectorErrorCode::ZeroValue);
    require!(base_price > 0, AddOfferVectorErrorCode::ZeroValue);
    require!(price_fix_duration > 0, AddOfferVectorErrorCode::ZeroValue);

    Ok(())
}

/// Cleans old inactive vectors from the offer, keeping only the currently active vector
/// and the last previously active vector.
///
/// # Arguments
/// - `offer`: Mutable reference to the offer containing vectors to clean
/// - `current_time`: Current unix timestamp for determining active vector
///
/// # Behavior
/// - Finds the currently active vector (most recent start_time <= current_time)
/// - Finds the previously active vector (closest smaller vector_id to active vector)
/// - Deletes all other vectors by setting them to default (vector_id = 0)
fn clean_old_vectors(offer: &mut Offer, current_time: u64) -> Result<()> {
    // Find currently active vector
    let active_vector = find_active_vector_at(offer, current_time);

    let active_vector_start_time = match active_vector {
        Ok(vector) => vector.start_time,
        Err(_) => return Ok(()), // No active vector found, nothing to clean
    };

    // Find previously active vector (closest smaller vector_id)
    let prev_vector = find_active_vector_at(offer, active_vector?.start_time - 1);

    let prev_vector_start_time = match prev_vector {
        Ok(vector) => vector.start_time,
        Err(_) => 0, // If no previous vector exists, use 0
    };

    // Clear all vectors except active and previous
    for vector in offer.vectors.iter_mut() {
        if vector.start_time != 0 // Don't touch already empty slots
            && vector.start_time != active_vector_start_time // Keep active vector
            && vector.start_time != prev_vector_start_time // Keep previous vector
            && vector.start_time < active_vector_start_time
        // Keep all future vectors
        {
            *vector = OfferVector::default(); // Clear the vector
        }
    }

    Ok(())
}

/// Error codes for add offer vector operations.
#[error_code]
pub enum AddOfferVectorErrorCode {
    /// Triggered when base_time is before the latest existing vector.
    #[msg("Invalid time range: base_time must be after the latest existing vector")]
    InvalidTimeRange,

    /// Triggered when any required value is zero.
    #[msg("Invalid input: values cannot be zero")]
    ZeroValue,

    /// Triggered when a vector with the same start_time already exists.
    #[msg("A vector with this start_time already exists")]
    DuplicateStartTime,

    /// Triggered when the offer already has the maximum number of vectors.
    #[msg("Offer already has the maximum number of vectors")]
    TooManyVectors,
}
