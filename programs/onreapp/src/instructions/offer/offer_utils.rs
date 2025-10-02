use crate::instructions::{Offer, OfferVector};
use crate::utils::approver::approver_utils;
use crate::utils::{calculate_fees, calculate_token_out_amount, ApprovalMessage};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

const SECONDS_IN_YEAR: u128 = 31_536_000;
const APR_SCALE: u128 = 1_000_000;

/// Common error codes for offer processing operations
#[error_code]
pub enum OfferCoreError {
    /// The specified offer was not found or is invalid
    #[msg("Offer not found")]
    OfferNotFound,
    /// No pricing vector is currently active for the given time
    #[msg("No active vector")]
    NoActiveVector,
    /// Arithmetic overflow occurred during calculations
    #[msg("Overflow error")]
    OverflowError,
    /// The provided token_in mint does not match the offer's expected mint
    #[msg("Invalid token in mint")]
    InvalidTokenInMint,
    /// The provided token_out mint does not match the offer's expected mint
    #[msg("Invalid token out mint")]
    InvalidTokenOutMint,
    /// The offer requires approval but none was provided or verification failed
    #[msg("Approval required for this offer")]
    ApprovalRequired,
}

/// Result structure containing offer processing calculations
pub struct OfferProcessResult {
    /// Current price with scale=9 (1_000_000_000 = 1.0) at the time of processing
    pub current_price: u64,
    /// Amount of token_in after fee deduction
    pub token_in_amount: u64,
    /// Calculated amount of token_out to be provided to the user
    pub token_out_amount: u64,
    /// Fee amount deducted from the original token_in amount
    pub fee_amount: u64,
}

/// Verifies approval requirements for offer operations
///
/// Checks if the offer requires approval and validates the provided approval message
/// using cryptographic signature verification against a trusted authority.
///
/// # Arguments
/// * `offer` - The offer to check for approval requirement
/// * `approval_message` - Optional approval message from the user
/// * `program_id` - The program ID for verification context
/// * `user_pubkey` - The user's public key
/// * `trusted_pubkey` - The trusted authority's public key for verification
/// * `instructions_sysvar` - The instructions sysvar account for signature verification
///
/// # Returns
/// * `Ok(())` - If approval is not needed or verification succeeds
/// * `Err(OfferCoreError::ApprovalRequired)` - If approval is required but not provided
/// * `Err(_)` - If approval verification fails
pub fn verify_offer_approval(
    offer: &Offer,
    approval_message: &Option<ApprovalMessage>,
    program_id: &Pubkey,
    user_pubkey: &Pubkey,
    trusted_pubkey: &Pubkey,
    instructions_sysvar: &UncheckedAccount,
) -> Result<()> {
    if offer.needs_approval() {
        match approval_message {
            Some(msg) => {
                msg!(
                    "Offer requires approval, verifying message {}",
                    msg.expiry_unix
                );
                approver_utils::verify_approval_message_generic(
                    program_id,
                    user_pubkey,
                    trusted_pubkey,
                    instructions_sysvar,
                    msg,
                )?;
            }
            None => return Err(error!(OfferCoreError::ApprovalRequired)),
        }
    }
    Ok(())
}

/// Core processing logic for offer execution calculations
///
/// Performs comprehensive validation and calculation for offer processing including
/// active vector identification, price calculation with APR-based growth, fee
/// calculation, and token amount conversions with decimal adjustments.
///
/// # Arguments
/// * `offer` - The loaded offer containing pricing vectors and configuration
/// * `token_in_amount` - Amount of token_in being provided by the user
/// * `token_in_mint` - The token_in mint for decimal and validation information
/// * `token_out_mint` - The token_out mint for decimal and validation information
///
/// # Returns
/// * `Ok(OfferProcessResult)` - Containing current price, token amounts, and fees
/// * `Err(_)` - If validation fails or no active vector exists
pub fn process_offer_core(
    offer: &Offer,
    token_in_amount: u64,
    token_in_mint: &InterfaceAccount<Mint>,
    token_out_mint: &InterfaceAccount<Mint>,
) -> Result<OfferProcessResult> {
    let current_time = Clock::get()?.unix_timestamp as u64;

    require!(
        offer.token_in_mint == token_in_mint.key(),
        OfferCoreError::InvalidTokenInMint
    );
    require!(
        offer.token_out_mint == token_out_mint.key(),
        OfferCoreError::InvalidTokenOutMint
    );

    // Find the currently active pricing vector
    let active_vector = find_active_vector_at(offer, current_time)?;

    // Calculate current price with 9 decimals
    let current_price = calculate_current_step_price(
        active_vector.apr,
        active_vector.base_price,
        active_vector.base_time,
        active_vector.price_fix_duration,
    )?;

    let fee_amounts = calculate_fees(token_in_amount, offer.fee_basis_points)?;

    // Calculate how many token_out to give for the provided token_in_amount
    let token_out_amount = calculate_token_out_amount(
        fee_amounts.remaining_token_in_amount,
        current_price,
        token_in_mint.decimals,
        token_out_mint.decimals,
    )?;

    Ok(OfferProcessResult {
        current_price,
        token_in_amount: fee_amounts.remaining_token_in_amount,
        token_out_amount,
        fee_amount: fee_amounts.fee_amount,
    })
}

/// Finds the currently active pricing vector at a specific time
///
/// Searches through the offer's pricing vectors to find the one that should be
/// active at the given time. Returns the vector with the latest start_time that
/// is still before or equal to the specified time.
///
/// # Arguments
/// * `offer` - The offer containing pricing vectors to search
/// * `time` - Unix timestamp to check for active vector
///
/// # Returns
/// * `Ok(OfferVector)` - The active pricing vector at the specified time
/// * `Err(OfferCoreError::NoActiveVector)` - If no vector is active at that time
pub fn find_active_vector_at(offer: &Offer, time: u64) -> Result<OfferVector> {
    let active_vector = offer
        .vectors
        .iter()
        .filter(|vector| vector.start_time != 0 && vector.start_time <= time) // Only consider non-empty vectors
        .max_by_key(|vector| vector.start_time) // Find latest start_time in the past
        .ok_or(OfferCoreError::NoActiveVector)?;

    Ok(*active_vector)
}

/// Calculates continuous price growth using APR-based compound interest
///
/// Implements linear price growth formula for continuous pricing without discrete
/// intervals. Uses fixed-point arithmetic to maintain precision in calculations.
///
/// Formula: P(t) = P0 * (1 + apr * elapsed_time / SECONDS_IN_YEAR)
/// where SECONDS_IN_YEAR = 31,536,000 and apr is scaled by 1,000,000.
///
/// # Arguments
/// * `apr` - Annual Percentage Rate scaled by 1_000_000 (1_000_000 = 1% APR)
/// * `base_price` - Starting price with scale=9
/// * `elapsed_time` - Time elapsed since base_time in seconds
///
/// # Returns
/// * `Ok(u64)` - Calculated price with same scale as base_price
/// * `Err(OfferCoreError::OverflowError)` - If arithmetic overflow occurs
pub fn calculate_vector_price(apr: u64, base_price: u64, elapsed_time: u64) -> Result<u64> {
    // Compute: price = P0 * (1 + y * elapsed_time / SECONDS_IN_YEAR)
    // With fixed-point:
    //   factor_num = SCALE*SECONDS_IN_YEAR + APR*elapsed_time
    //   factor_den = SCALE*SECONDS_IN_YEAR
    //   price = base_price * (factor_num / factor_den)
    let factor_den = APR_SCALE
        .checked_mul(SECONDS_IN_YEAR)
        .expect("SCALE*S overflow (should not happen)");
    let y_part = (apr as u128)
        .checked_mul(elapsed_time as u128)
        .ok_or(OfferCoreError::OverflowError)?;
    let factor_num = factor_den
        .checked_add(y_part)
        .ok_or(OfferCoreError::OverflowError)?;

    // price growth applied to base_price
    let price_u128 = (base_price as u128)
        .checked_mul(factor_num)
        .ok_or(OfferCoreError::OverflowError)?
        .checked_div(factor_den)
        .ok_or(OfferCoreError::OverflowError)?;

    if price_u128 > u64::MAX as u128 {
        return Err(error!(OfferCoreError::OverflowError));
    }

    Ok(price_u128 as u64)
}

/// Calculates discrete interval pricing with fixed price windows
///
/// Implements discrete interval pricing where prices are fixed within specific
/// time windows and snap to the end of the current interval. This creates
/// step-function price behavior rather than continuous growth.
///
/// Formula:
///   interval = floor((current_time - base_time) / price_fix_duration)
///   effective_time = (interval + 1) * price_fix_duration
///   price = calculate_vector_price(apr, base_price, effective_time)
///
/// # Arguments
/// * `apr` - Annual Percentage Rate scaled by 1_000_000
/// * `base_price` - Starting price with scale=9
/// * `base_time` - Unix timestamp when pricing vector starts
/// * `price_fix_duration` - Duration of each discrete price interval in seconds
///
/// # Returns
/// * `Ok(u64)` - Current price at the discrete interval
/// * `Err(_)` - If calculation fails or time is before base_time
pub fn calculate_current_step_price(
    apr: u64,
    base_price: u64,
    base_time: u64,
    price_fix_duration: u64,
) -> Result<u64> {
    let current_time = Clock::get()?.unix_timestamp as u64;

    calculate_step_price_at(apr, base_price, base_time, price_fix_duration, current_time)
}

/// Calculates discrete step price at a specific time
///
/// Internal helper function that calculates the step price at any given time
/// using the discrete interval pricing model.
///
/// # Arguments
/// * `apr` - Annual Percentage Rate scaled by 1_000_000
/// * `base_price` - Starting price with scale=9
/// * `base_time` - Unix timestamp when pricing vector starts
/// * `price_fix_duration` - Duration of each discrete price interval in seconds
/// * `time` - Specific time to calculate price for
///
/// # Returns
/// * `Ok(u64)` - Price at the specified time
/// * `Err(_)` - If calculation fails or time is invalid
pub fn calculate_step_price_at(
    apr: u64,
    base_price: u64,
    base_time: u64,
    price_fix_duration: u64,
    time: u64,
) -> Result<u64> {
    require!(base_time <= time, OfferCoreError::NoActiveVector);

    let elapsed_since_start = time.saturating_sub(base_time);

    // Calculate which price interval we're in (discrete intervals)
    let current_step = elapsed_since_start / price_fix_duration;

    // elapsed_effective = (k + 1) * D  (end-of-current-interval snap)
    let step_end_time = current_step
        .checked_add(1)
        .unwrap()
        .checked_mul(price_fix_duration)
        .ok_or(OfferCoreError::OverflowError)?;

    // Use the vector price calculation with the effective elapsed time
    calculate_vector_price(apr, base_price, step_end_time)
}


/// Finds the array index of a pricing vector by its start time
///
/// Searches through the offer's pricing vector array to find the index
/// of the vector with the specified start_time.
///
/// # Arguments
/// * `offer` - The offer containing pricing vectors to search
/// * `start_time` - The start_time to search for
///
/// # Returns
/// * `Some(usize)` - Array index of the matching vector
/// * `None` - If no vector with that start_time exists
pub fn find_vector_index_by_start_time(offer: &Offer, start_time: u64) -> Option<usize> {
    offer
        .vectors
        .iter()
        .position(|vector| vector.start_time == start_time)
}