use super::offer_state::{Offer, OfferVector};
use crate::constants::seeds;
use crate::instructions::{find_active_vector_at, find_vector_index_by_start_time};
use crate::state::State;
use crate::OfferCoreError;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;
use std::cmp::max;

/// Event emitted when a pricing vector is successfully added to an offer
///
/// Provides transparency for tracking pricing vector additions and configurations.
#[event]
pub struct OfferVectorAddedEvent {
    /// The PDA address of the offer to which the vector was added
    pub offer_pda: Pubkey,
    /// Calculated start time when the vector becomes active (max(base_time, current_time))
    pub start_time: u64,
    /// Original base time specified for the vector
    pub base_time: u64,
    /// Base price with 9 decimal precision at the vector start
    pub base_price: u64,
    /// Annual Percentage Rate scaled by 1,000,000 (1_000_000 = 1% APR)
    pub apr: u64,
    /// Duration in seconds for each discrete pricing step
    pub price_fix_duration: u64,
}

/// Account structure for adding a pricing vector to an offer
///
/// This struct defines the accounts required to add a time-based pricing vector
/// to an existing offer. Only the boss can add pricing vectors to control offer dynamics.
#[derive(Accounts)]
pub struct AddOfferVector<'info> {
    /// The offer account to which the pricing vector will be added
    ///
    /// This account is validated as a PDA derived from token mint addresses
    /// and contains the array of pricing vectors for the offer.
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

    /// The input token mint account for offer validation
    #[account(
        constraint =
            token_in_mint.key() == offer.load()?.token_in_mint
            @ OfferCoreError::InvalidTokenInMint
    )]
    pub token_in_mint: InterfaceAccount<'info, Mint>,

    /// The output token mint account for offer validation
    #[account(
        constraint =
            token_out_mint.key() == offer.load()?.token_out_mint
            @ OfferCoreError::InvalidTokenOutMint
    )]
    pub token_out_mint: InterfaceAccount<'info, Mint>,

    /// Program state account containing boss authorization
    #[account(seeds = [seeds::STATE], bump = state.bump, has_one = boss)]
    pub state: Account<'info, State>,

    /// The boss account authorized to add pricing vectors to offers
    #[account(mut)]
    pub boss: Signer<'info>,
}

/// Adds a time-based pricing vector to an existing offer
///
/// This instruction creates a new pricing vector that defines price evolution over time
/// using APR-based growth. The vector becomes active at the start time and
/// implements discrete pricing steps based on the specified duration.
///
/// The start time cannot be in the past. After adding the vector, old inactive vectors are
/// automatically cleaned up to maintain storage efficiency.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `start_time` - Optional Unix timestamp when the vector becomes active. If not provided,
/// max(base_time, current_time) is used.
/// * `base_time` - Unix timestamp when the vector should become active
/// * `base_price` - Initial price with scale=9 (1_000_000_000 = 1.0)
/// * `apr` - Annual Percentage Rate scaled by 1,000,000 (1_000_000 = 1% APR)
/// * `price_fix_duration` - Duration in seconds for each discrete pricing step
///
/// # Returns
/// * `Ok(())` - If the vector is successfully added
/// * `Err(AddOfferVectorErrorCode::InvalidTimeRange)` - If start_time is before latest existing vector
/// * `Err(AddOfferVectorErrorCode::ZeroValue)` - If any required value is zero
/// * `Err(AddOfferVectorErrorCode::DuplicateStartTime)` - If start_time already exists
/// * `Err(AddOfferVectorErrorCode::TooManyVectors)` - If offer has maximum vectors
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss account must match the one stored in program state
///
/// # Events
/// * `OfferVectorAddedEvent` - Emitted on successful vector addition with parameters
pub fn add_offer_vector(
    ctx: Context<AddOfferVector>,
    start_time_opt: Option<u64>,
    base_time: u64,
    base_price: u64,
    apr: u64,
    price_fix_duration: u64,
) -> Result<()> {
    let offer = &mut ctx.accounts.offer.load_mut()?;
    let current_time = Clock::get()?.unix_timestamp as u64;
    let start_time = start_time_opt.unwrap_or_else(|| max(current_time, base_time));

    validate_inputs(
        start_time,
        base_time,
        base_price,
        price_fix_duration,
        current_time,
        &offer,
    )?;

    // Create the new time vector
    let new_vector = OfferVector {
        start_time,
        base_time,
        base_price,
        apr,
        price_fix_duration,
    };

    // Clean up old vectors before emitting success message
    clean_old_vectors(offer, &new_vector, current_time)?;

    // Find an empty slot in time_vectors array
    let empty_slot_index = find_vector_index_by_start_time(&offer, 0)
        .ok_or_else(|| error!(AddOfferVectorErrorCode::TooManyVectors))?;

    // Add the vector to the offer
    offer.vectors[empty_slot_index] = new_vector;

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

/// Validates input parameters for pricing vector creation
///
/// Ensures all required parameters are non-zero values to prevent
/// invalid pricing vector configurations.
///
/// # Arguments
/// * `base_time` - Unix timestamp for vector activation
/// * `base_price` - Initial price value
/// * `price_fix_duration` - Duration for pricing steps
///
/// # Returns
/// * `Ok(())` - If all parameters are valid
/// * `Err(AddOfferVectorErrorCode::ZeroValue)` - If any parameter is zero
fn validate_inputs(
    start_time: u64,
    base_time: u64,
    base_price: u64,
    price_fix_duration: u64,
    current_time: u64,
    offer: &Offer,
) -> Result<()> {
    require!(
        start_time >= current_time,
        AddOfferVectorErrorCode::StartTimeInPast
    );
    require!(base_time > 0, AddOfferVectorErrorCode::ZeroValue);
    require!(base_price > 0, AddOfferVectorErrorCode::ZeroValue);
    require!(price_fix_duration > 0, AddOfferVectorErrorCode::ZeroValue);

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

    // Validate start_time is after latest existing vector
    if let Some(latest_start_time) = existing_start_times.iter().max() {
        require!(
            &start_time > latest_start_time,
            AddOfferVectorErrorCode::InvalidTimeRange
        );
    }

    Ok(())
}

/// Removes old inactive pricing vectors to maintain storage efficiency
///
/// This function preserves the currently active vector and the most recent
/// previously active vector while clearing older historical vectors that
/// are no longer needed for pricing calculations.
///
/// # Arguments
/// * `offer` - Mutable reference to the offer containing vectors to clean
/// * `current_time` - Current unix timestamp for determining active vector
///
/// # Returns
/// * `Ok(())` - If cleanup completes successfully or no active vector exists
fn clean_old_vectors(offer: &mut Offer, new_vector: &OfferVector, current_time: u64) -> Result<()> {
    // Find currently active vector
    let active_vector = if new_vector.start_time == current_time {
        Ok(*new_vector)
    } else {
        find_active_vector_at(offer, current_time)
    };

    let active_vector_start_time = match active_vector {
        Ok(vector) => vector.start_time,
        Err(_) => return Ok(()), // No active vector found, nothing to clean
    };

    // Find previously active vector (closest smaller vector_start_timestamp)
    let prev_vector = find_active_vector_at(offer, active_vector?.start_time - 1);

    let prev_vector_start_time = match prev_vector {
        Ok(vector) => vector.start_time,
        Err(_) => 0, // If no previous vector exists, use 0
    };

    // Clear all vectors except active and previous
    for vector in offer.vectors.iter_mut() {
        if vector.start_time != 0 // Don't touch already empty slots
            // Keep active vector
            && vector.start_time != active_vector_start_time
            // Keep previous vector
            && vector.start_time != prev_vector_start_time
            // Keep all future vectors
            && vector.start_time < active_vector_start_time
        {
            *vector = OfferVector::default(); // Clear the vector
        }
    }

    Ok(())
}

/// Error codes for add offer vector operations
#[error_code]
pub enum AddOfferVectorErrorCode {
    /// The base_time is before the latest existing vector's base_time
    #[msg("Invalid time range: base_time must be after the latest existing vector")]
    InvalidTimeRange,

    /// One or more required parameters (base_time, base_price, price_fix_duration) is zero
    #[msg("Invalid input: values cannot be zero")]
    ZeroValue,

    /// The start_time is in the past
    #[msg("Invalid input: start_time cannot be in the past")]
    StartTimeInPast,

    /// A vector with the calculated start_time already exists in the offer
    #[msg("A vector with this start_time already exists")]
    DuplicateStartTime,

    /// The offer has reached the maximum number of pricing vectors allowed
    #[msg("Offer already has the maximum number of vectors")]
    TooManyVectors,
}
