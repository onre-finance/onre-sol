use crate::instructions::{Offer, OfferVector};
use crate::state::State;
use crate::utils::approver::approver_utils;
use crate::utils::{
    calculate_fees, calculate_token_out_amount, mul_div_round_u128, pow_fixed,
    program_controls_mint, ApprovalMessage,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

const INT_SCALE: u128 = 1_000_000_000_000_000_000;
const SECONDS_IN_DAY: u64 = 86_400;
const APR_SCALE: u128 = 1_000_000;

/// Result structure containing offer processing calculations
pub struct OfferProcessResult {
    /// Current price with scale=9 (1_000_000_000 = 1.0) at the time of processing
    pub current_price: u64,
    /// Amount of token_in after fee deduction
    pub token_in_net_amount: u64,
    /// Fee amount deducted from the original token_in amount
    pub token_in_fee_amount: u64,
    /// Calculated amount of token_out to be provided to the user
    pub token_out_amount: u64,
}

/// Verifies approval requirements for offer operations
///
/// Checks if the offer requires approval and validates the provided approval message
/// using cryptographic signature verification against one of the two trusted authorities.
///
/// # Arguments
/// * `offer` - The offer to check for approval requirement
/// * `approval_message` - Optional approval message from the user
/// * `program_id` - The program ID for verification context
/// * `user_pubkey` - The user's public key
/// * `approver1` - The first trusted authority's public key for verification
/// * `approver2` - The second trusted authority's public key for verification
/// * `instructions_sysvar` - The instructions sysvar account for signature verification
///
/// # Returns
/// * `Ok(())` - If approval is not needed or verification succeeds with either approver
/// * `Err(crate::OnreError::ApprovalRequired)` - If approval is required but not provided
/// * `Err(_)` - If approval verification fails with both approvers
pub fn verify_offer_approval(
    offer: &Offer,
    approval_message: &Option<ApprovalMessage>,
    program_id: &Pubkey,
    user_pubkey: &Pubkey,
    approver1: &Pubkey,
    approver2: &Pubkey,
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
                    approver1,
                    approver2,
                    instructions_sysvar,
                    msg,
                )?;
            }
            None => return Err(error!(crate::OnreError::ApprovalRequired)),
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

    process_offer_core_at(
        offer,
        token_in_amount,
        token_in_mint.key(),
        token_in_mint.decimals,
        token_out_mint.key(),
        token_out_mint.decimals,
        current_time,
    )
}

fn process_offer_core_at(
    offer: &Offer,
    token_in_amount: u64,
    token_in_mint_key: Pubkey,
    token_in_decimals: u8,
    token_out_mint_key: Pubkey,
    token_out_decimals: u8,
    current_time: u64,
) -> Result<OfferProcessResult> {
    require!(token_in_amount > 0, crate::OnreError::InvalidAmount);

    require!(
        offer.token_in_mint == token_in_mint_key,
        crate::OnreError::InvalidTokenInMint
    );
    require!(
        offer.token_out_mint == token_out_mint_key,
        crate::OnreError::InvalidTokenOutMint
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
        fee_amounts.token_in_net_amount,
        current_price,
        token_in_decimals,
        token_out_decimals,
    )?;

    Ok(OfferProcessResult {
        current_price,
        token_in_net_amount: fee_amounts.token_in_net_amount,
        token_out_amount,
        token_in_fee_amount: fee_amounts.token_in_fee_amount,
    })
}

pub fn is_onyc_token_out_mint<'info>(
    state: &Account<'info, State>,
    token_out_mint: &InterfaceAccount<'info, Mint>,
) -> bool {
    token_out_mint.key() == state.onyc_mint
}

pub fn should_accrue_onyc_mint<'info>(
    state: &Account<'info, State>,
    token_out_mint: &InterfaceAccount<'info, Mint>,
    buffer_is_initialized: bool,
    mint_authority: &AccountInfo<'info>,
) -> bool {
    is_onyc_token_out_mint(state, token_out_mint)
        && buffer_is_initialized
        && program_controls_mint(token_out_mint, mint_authority)
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
/// * `Err(crate::OnreError::NoActiveVector)` - If no vector is active at that time
pub fn find_active_vector_at(offer: &Offer, time: u64) -> Result<OfferVector> {
    let active_vector = offer
        .vectors
        .iter()
        .filter(|vector| vector.start_time != 0 && vector.start_time <= time) // Only consider non-empty vectors
        .max_by_key(|vector| vector.start_time) // Find latest start_time in the past
        .ok_or(crate::OnreError::NoActiveVector)?;

    Ok(*active_vector)
}

/// Calculates price growth using daily compounding with linear intra-day interpolation
///
/// The annual percentage rate is compounded daily, matching the APY semantics
/// used by market info. For elapsed times that are not an exact number of days,
/// the remaining fractional day is linearly interpolated between the current
/// daily-compounded price and the next daily-compounded price.
///
/// Formula:
///   daily_factor = 1 + apr / (APR_SCALE * 365)
///   full_day_price = base_price * daily_factor^(elapsed_seconds / 86400)
///   price = full_day_price + daily_delta * (elapsed_seconds % 86400) / 86400
///
/// # Arguments
/// * `apr` - Annual Percentage Rate with scale=6 (10_000 = 1%, 1_000_000 = 100%)
/// * `base_price` - Starting price with scale=9
/// * `elapsed_time` - Time elapsed since base_time in seconds
///
/// # Returns
/// * `Ok(u64)` - Calculated price with same scale as base_price
/// * `Err(crate::OnreError::OverflowError)` - If arithmetic overflow occurs
pub fn calculate_vector_price(apr: u64, base_price: u64, elapsed_time: u64) -> Result<u64> {
    if apr == 0 || elapsed_time == 0 {
        return Ok(base_price);
    }

    let daily_increment = INT_SCALE
        .checked_mul(apr as u128)
        .ok_or(crate::OnreError::OverflowError)?
        .checked_add((APR_SCALE * 365) / 2)
        .ok_or(crate::OnreError::OverflowError)?
        .checked_div(APR_SCALE * 365)
        .ok_or(crate::OnreError::OverflowError)?;

    let daily_factor = INT_SCALE
        .checked_add(daily_increment)
        .ok_or(crate::OnreError::OverflowError)?;

    let full_days = elapsed_time / SECONDS_IN_DAY;
    let remaining_seconds = elapsed_time % SECONDS_IN_DAY;

    let full_day_factor =
        pow_fixed(daily_factor, full_days, INT_SCALE).ok_or(crate::OnreError::OverflowError)?;
    let full_day_price = mul_div_round_u128(base_price as u128, full_day_factor, INT_SCALE)
        .ok_or(crate::OnreError::OverflowError)?;

    if remaining_seconds == 0 {
        if full_day_price > u64::MAX as u128 {
            return Err(error!(crate::OnreError::OverflowError));
        }

        return Ok(full_day_price as u64);
    }

    let next_day_price = mul_div_round_u128(full_day_price, daily_factor, INT_SCALE)
        .ok_or(crate::OnreError::OverflowError)?;
    let daily_delta = next_day_price
        .checked_sub(full_day_price)
        .ok_or(crate::OnreError::OverflowError)?;
    let partial_day_delta = mul_div_round_u128(
        daily_delta,
        remaining_seconds as u128,
        SECONDS_IN_DAY as u128,
    )
    .ok_or(crate::OnreError::OverflowError)?;
    let price = full_day_price
        .checked_add(partial_day_delta)
        .ok_or(crate::OnreError::OverflowError)?;

    if price > u64::MAX as u128 {
        return Err(error!(crate::OnreError::OverflowError));
    }

    Ok(price as u64)
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
    require!(base_time <= time, crate::OnreError::NoActiveVector);

    let elapsed_since_start = time.saturating_sub(base_time);

    // Calculate which price interval we're in (discrete intervals)
    let current_step = elapsed_since_start / price_fix_duration;

    // elapsed_effective = (k + 1) * D  (end-of-current-interval snap)
    let step_end_time = current_step
        .checked_add(1)
        .ok_or(crate::OnreError::OverflowError)?
        .checked_mul(price_fix_duration)
        .ok_or(crate::OnreError::OverflowError)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::MAX_BASIS_POINTS;

    fn offer_for_test(
        token_in_mint: Pubkey,
        token_out_mint: Pubkey,
        fee_basis_points: u16,
        vector: OfferVector,
    ) -> Offer {
        let mut offer: Offer = unsafe { std::mem::zeroed() };
        offer.token_in_mint = token_in_mint;
        offer.token_out_mint = token_out_mint;
        offer.fee_basis_points = fee_basis_points;
        offer.vectors[0] = vector;
        offer
    }

    fn active_vector_for_test() -> OfferVector {
        OfferVector {
            start_time: 1,
            base_time: 1,
            base_price: 1_000_000_000,
            apr: 0,
            price_fix_duration: SECONDS_IN_DAY,
        }
    }

    #[test]
    fn process_offer_core_rejects_invalid_token_in_mint() {
        let token_in_mint = Pubkey::new_unique();
        let token_out_mint = Pubkey::new_unique();
        let offer = offer_for_test(token_in_mint, token_out_mint, 0, active_vector_for_test());

        let result = process_offer_core_at(
            &offer,
            1_000_000,
            Pubkey::new_unique(),
            6,
            token_out_mint,
            6,
            1,
        );

        assert!(result.is_err());
    }

    #[test]
    fn process_offer_core_rejects_invalid_token_out_mint() {
        let token_in_mint = Pubkey::new_unique();
        let token_out_mint = Pubkey::new_unique();
        let offer = offer_for_test(token_in_mint, token_out_mint, 0, active_vector_for_test());

        let result = process_offer_core_at(
            &offer,
            1_000_000,
            token_in_mint,
            6,
            Pubkey::new_unique(),
            6,
            1,
        );

        assert!(result.is_err());
    }

    #[test]
    fn process_offer_core_rejects_invalid_fee_basis_points() {
        let token_in_mint = Pubkey::new_unique();
        let token_out_mint = Pubkey::new_unique();
        let offer = offer_for_test(
            token_in_mint,
            token_out_mint,
            MAX_BASIS_POINTS + 1,
            active_vector_for_test(),
        );

        let result =
            process_offer_core_at(&offer, 1_000_000, token_in_mint, 6, token_out_mint, 6, 1);

        assert!(result.is_err());
    }

    #[test]
    fn process_offer_core_rejects_missing_active_vector() {
        let token_in_mint = Pubkey::new_unique();
        let token_out_mint = Pubkey::new_unique();
        let offer = offer_for_test(
            token_in_mint,
            token_out_mint,
            0,
            OfferVector {
                start_time: 2,
                ..active_vector_for_test()
            },
        );

        let result =
            process_offer_core_at(&offer, 1_000_000, token_in_mint, 6, token_out_mint, 6, 1);

        assert!(result.is_err());
    }

    #[test]
    fn vector_price_matches_daily_compounding_on_day_boundaries() {
        let price = calculate_vector_price(97_600, 1_000_000_000, SECONDS_IN_DAY).unwrap();
        assert_eq!(price, 1_000_267_397);

        let price = calculate_vector_price(97_600, 1_000_000_000, SECONDS_IN_DAY * 2).unwrap();
        assert_eq!(price, 1_000_534_866);
    }

    #[test]
    fn vector_price_grows_with_subday_elapsed_time() {
        let one_hour = calculate_vector_price(97_600, 1_000_000_000, 3_600).unwrap();
        let six_hours = calculate_vector_price(97_600, 1_000_000_000, 21_600).unwrap();
        let one_day = calculate_vector_price(97_600, 1_000_000_000, SECONDS_IN_DAY).unwrap();

        assert_eq!(one_hour, 1_000_011_142);
        assert_eq!(six_hours, 1_000_066_849);
        assert_eq!(one_day, 1_000_267_397);
    }

    #[test]
    fn vector_price_matches_multi_day_compounding() {
        let three_days = calculate_vector_price(97_600, 1_000_000_000, SECONDS_IN_DAY * 3).unwrap();
        let three_days_six_hours =
            calculate_vector_price(97_600, 1_000_000_000, SECONDS_IN_DAY * 3 + 21_600).unwrap();

        assert_eq!(three_days, 1_000_802_406);
        assert_eq!(three_days_six_hours, 1_000_869_309);
    }

    #[test]
    fn vector_price_rejects_price_overflow() {
        let result = calculate_vector_price(u64::MAX, u64::MAX, SECONDS_IN_DAY);

        assert!(result.is_err());
    }

    #[test]
    fn step_price_rejects_time_before_base_time() {
        let result = calculate_step_price_at(0, 1_000_000_000, 2, SECONDS_IN_DAY, 1);

        assert!(result.is_err());
    }

    #[test]
    fn step_price_rejects_step_end_overflow() {
        let result = calculate_step_price_at(0, 1_000_000_000, 0, 1, u64::MAX);

        assert!(result.is_err());
    }
}
