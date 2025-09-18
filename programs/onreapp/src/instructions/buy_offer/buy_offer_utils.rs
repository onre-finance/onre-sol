use crate::instructions::{BuyOffer, BuyOfferAccount, BuyOfferVector};
use crate::utils::{calculate_fees, calculate_token_out_amount};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

const SECONDS_IN_YEAR: u128 = 31_536_000;
const APR_SCALE: u128 = 1_000_000;

/// Common error codes that can be used by both take_buy_offer instructions
#[error_code]
pub enum BuyOfferCoreError {
    #[msg("Offer not found")]
    OfferNotFound,
    #[msg("No active vector")]
    NoActiveVector,
    #[msg("Overflow error")]
    OverflowError,
    #[msg("Invalid token in mint")]
    InvalidTokenInMint,
    #[msg("Invalid token out mint")]
    InvalidTokenOutMint,
}

/// Result structure for the core buy offer processing
pub struct BuyOfferProcessResult {
    pub current_price: u64,
    pub token_in_amount: u64,
    pub token_out_amount: u64,
    pub fee_amount: u64,
}

/// Core processing logic shared between both take_buy_offer instructions
///
/// This function handles all the common validation and calculation logic:
/// 1. Find and validate the offer exists
/// 2. Find the currently active pricing vector
/// 3. Calculate current price based on time and APR parameters
/// 4. Calculate token_out_amount based on price and decimals
///
/// # Arguments
/// * `offer_account` - The loaded buy offer account
/// * `offer_id` - The ID of the offer to process
/// * `token_in_amount` - Amount of token_in being provided
/// * `token_in_mint` - The token_in mint for decimal information
/// * `token_out_mint` - The token_out mint for decimal information
///
/// # Returns
/// A `BuyOfferProcessResult` containing the calculated price and token_out_amount
pub fn process_buy_offer_core(
    offer_account: &BuyOfferAccount,
    offer_id: u64,
    token_in_amount: u64,
    token_in_mint: &InterfaceAccount<Mint>,
    token_out_mint: &InterfaceAccount<Mint>,
) -> Result<BuyOfferProcessResult> {
    let current_time = Clock::get()?.unix_timestamp as u64;

    // Find the offer
    let offer = find_offer(offer_account, offer_id)?;

    require!(
        offer.token_in_mint == token_in_mint.key(),
        BuyOfferCoreError::InvalidTokenInMint
    );
    require!(
        offer.token_out_mint == token_out_mint.key(),
        BuyOfferCoreError::InvalidTokenOutMint
    );

    // Find the currently active pricing vector
    let active_vector = find_active_vector_at(&offer, current_time)?;

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

    Ok(BuyOfferProcessResult {
        current_price,
        token_in_amount: fee_amounts.remaining_token_in_amount,
        token_out_amount,
        fee_amount: fee_amounts.fee_amount,
    })
}

/// Finds a buy offer by ID in the offer account
///
/// # Arguments
/// * `offer_account` - The buy offer account containing all offers
/// * `offer_id` - The ID of the offer to find
///
/// # Returns
/// The found `BuyOffer` or an error if not found
pub fn find_offer(offer_account: &BuyOfferAccount, offer_id: u64) -> Result<BuyOffer> {
    if offer_id == 0 {
        return Err(error!(BuyOfferCoreError::OfferNotFound));
    }

    let offer = offer_account
        .offers
        .iter()
        .find(|offer| offer.offer_id == offer_id)
        .ok_or(BuyOfferCoreError::OfferNotFound)?;

    Ok(*offer)
}

/// Finds a mutable reference to buy offer by ID in the offer account
///
/// # Arguments
/// * `offer_account` - The buy offer account containing all offers
/// * `offer_id` - The ID of the offer to find
///
/// # Returns
/// The found `BuyOffer` or an error if not found
pub fn find_offer_mut(offer_account: &mut BuyOfferAccount, offer_id: u64) -> Result<&mut BuyOffer> {
    if offer_id == 0 {
        return Err(error!(BuyOfferCoreError::OfferNotFound));
    }

    let offer = offer_account
        .offers
        .iter_mut()
        .find(|offer| offer.offer_id == offer_id)
        .ok_or(BuyOfferCoreError::OfferNotFound)?;

    Ok(offer)
}

/// Finds a buy offer index by ID in the offer account
///
/// # Arguments
/// * `offer_account` - The buy offer account containing all offers
/// * `offer_id` - The ID of the offer to find
///
/// # Returns
/// The found `BuyOffer` or an error if not found
pub fn find_offer_index(offer_account: &BuyOfferAccount, offer_id: u64) -> Result<usize> {
    if offer_id == 0 {
        return Err(error!(BuyOfferCoreError::OfferNotFound));
    }

    let offer_id = offer_account
        .offers
        .iter()
        .position(|offer| offer.offer_id == offer_id)
        .ok_or(BuyOfferCoreError::OfferNotFound)?;

    Ok(offer_id)
}

/// Finds the currently active vector for a buy offer
///
/// # Arguments
/// * `offer` - The buy offer to search for an active vector
///
/// # Returns
/// The active `BuyOfferVector` or an error if none is active
pub fn find_active_vector_at(offer: &BuyOffer, time: u64) -> Result<BuyOfferVector> {
    let active_vector = offer
        .vectors
        .iter()
        .filter(|vector| vector.vector_id != 0) // Only consider non-empty vectors
        .filter(|vector| vector.start_time <= time) // Only vectors that have started
        .max_by_key(|vector| vector.start_time) // Find latest start_time in the past
        .ok_or(BuyOfferCoreError::NoActiveVector)?;

    Ok(*active_vector)
}

/// Calculates the price for a pricing vector based on APR and elapsed time.
///
/// This function implements the core linear price growth formula without discrete intervals.
/// It computes a continuous price based on the APR (Annual Percentage Rate) and time elapsed.
///
/// Formula: P(t) = P0 * (1 + y * elapsed_time / S)
/// where S = 365*24*3600 (seconds per year), and y is yearly APR as a decimal.
/// We compute this in fixed-point arithmetic to avoid precision loss.
///
/// # Arguments
/// * `apr` - Annual Percentage Rate scaled by 1_000_000 (e.g., 10.12% => 101_200)
/// * `base_price` - Starting price
/// * `elapsed_time` - Time elapsed since base_time in seconds
///
/// # Returns
/// The calculated price for the given elapsed time
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
        .ok_or(BuyOfferCoreError::OverflowError)?;
    let factor_num = factor_den
        .checked_add(y_part)
        .ok_or(BuyOfferCoreError::OverflowError)?;

    // price growth applied to base_price
    let price_u128 = (base_price as u128)
        .checked_mul(factor_num)
        .ok_or(BuyOfferCoreError::OverflowError)?
        .checked_div(factor_den)
        .ok_or(BuyOfferCoreError::OverflowError)?;

    if price_u128 > u64::MAX as u128 {
        return Err(error!(BuyOfferCoreError::OverflowError));
    }

    Ok(price_u128 as u64)
}

/// Calculates the current price using discrete interval pricing with "price-fix" windows.
///
/// This function implements the discrete interval pricing model where prices are fixed
/// within specific time windows and snap to the END of the current interval.
/// It determines which interval we're currently in, then calculates the price at that interval.
///
/// - price_fix_duration: duration (seconds) of each price-fix window; price is constant within a window
///
/// Formula:
///   k = current interval = floor((current_time - base_time) / price_fix_duration)
///   elapsed_effective = (k + 1) * price_fix_duration  (snap to end of interval)
///   P(t) = calculate_vector_price(apr, base_price, elapsed_effective)
///
/// # Arguments
/// * `apr` - Annual Percentage Rate scaled by 1_000_000
/// * `base_price` - Starting price
/// * `base_time` - Unix timestamp when pricing starts
/// * `price_fix_duration` - Duration of each price interval in seconds
///
/// # Returns
/// The calculated current price at the discrete interval
pub fn calculate_current_step_price(
    apr: u64,
    base_price: u64,
    base_time: u64,
    price_fix_duration: u64,
) -> Result<u64> {
    let current_time = Clock::get()?.unix_timestamp as u64;

    require!(base_time <= current_time, BuyOfferCoreError::NoActiveVector);

    let elapsed_since_start = current_time.saturating_sub(base_time);

    // Calculate which price interval we're in (discrete intervals)
    let current_step = elapsed_since_start / price_fix_duration;

    // elapsed_effective = (k + 1) * D  (end-of-current-interval snap)
    let step_end_time = current_step
        .checked_add(1)
        .unwrap()
        .checked_mul(price_fix_duration)
        .ok_or(BuyOfferCoreError::OverflowError)?;

    // Use the vector price calculation with the effective elapsed time
    calculate_vector_price(apr, base_price, step_end_time)
}
