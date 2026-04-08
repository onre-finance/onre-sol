use crate::constants::seeds;
use crate::instructions::offer::offer_utils::find_active_vector_at;
use crate::instructions::Offer;
use crate::utils::{mul_div_round_u128, pow_fixed};
use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use anchor_spl::token_interface::Mint;

/// External scale factor used for APR/APY representation (scale=6)
/// 1_000_000 represents 100%, so 10_000 = 1%, 100_000 = 10%
const EXT_SCALE: u128 = 1_000_000;

/// Internal scale factor for high-precision fixed-point arithmetic (scale=18)
/// Used during intermediate calculations to maintain precision
const INT_SCALE: u128 = 1_000_000_000_000_000_000;

/// Number of compounding periods per year for daily compounding
/// Standard financial calculation uses 365 days per year
const N: u128 = 365;

// Error codes for APY calculation operations

/// Event emitted when APY calculation is successfully completed
///
/// This event provides transparency for off-chain applications to track
/// APY queries and monitor yield calculation results for specific offers.
#[event]
pub struct GetAPYEvent {
    /// The PDA address of the offer for which APY was calculated
    pub offer_pda: Pubkey,
    /// Calculated Annual Percentage Yield with scale=6 (1_000_000 = 100%)
    pub apy: u64,
    /// Source Annual Percentage Rate with scale=6 used for calculation
    pub apr: u64,
    /// Unix timestamp when the APY calculation was performed
    pub timestamp: u64,
}

/// Account structure for querying APY information
///
/// This struct defines the accounts required to calculate the Annual Percentage Yield
/// for a specific offer. The calculation is read-only and does not modify any state.
/// All accounts are validated to ensure they belong to the same offer.
#[derive(Accounts)]
pub struct GetAPY<'info> {
    /// The offer account containing the pricing vectors and APR data
    ///
    /// This account is validated as a PDA derived from the "offer" seed combined
    /// with both token mint addresses. Contains the time-based pricing vectors
    /// that include the APR values used for APY calculation.
    #[account(
        seeds = [
            seeds::OFFER,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref()
        ],
        bump = offer.load()?.bump
    )]
    pub offer: AccountLoader<'info, Offer>,

    /// The input token mint account for offer validation
    ///
    /// Must match the token_in_mint stored in the offer account to ensure
    /// the correct offer is being queried. This validation prevents
    /// accidental queries against incorrect token pairs.
    #[account(
        constraint =
            token_in_mint.key() == offer.load()?.token_in_mint
            @ crate::OnreError::InvalidTokenInMint
    )]
    pub token_in_mint: InterfaceAccount<'info, Mint>,

    /// The output token mint account for offer validation
    ///
    /// Must match the token_out_mint stored in the offer account to ensure
    /// the correct offer is being queried. This validation prevents
    /// accidental queries against incorrect token pairs.
    #[account(
        constraint =
            token_out_mint.key() == offer.load()?.token_out_mint
            @ crate::OnreError::InvalidTokenOutMint
    )]
    pub token_out_mint: InterfaceAccount<'info, Mint>,
}

/// Calculates and returns the current Annual Percentage Yield (APY) for a specific offer
///
/// This is a read-only instruction that queries the current APY for an offer by:
/// 1. Finding the currently active pricing vector based on the current timestamp
/// 2. Extracting the APR from the active vector
/// 3. Converting APR to APY using daily compounding mathematics
/// 4. Returning the calculated APY with the same scale as the input APR
///
/// The calculation uses the standard financial formula: APY = (1 + APR/365)^365 - 1
/// with daily compounding (365 periods per year). This provides a more accurate
/// representation of the actual annual yield compared to simple APR.
///
/// # Process Flow
/// 1. Load the offer account and get current timestamp
/// 2. Identify the active pricing vector for the current time
/// 3. Extract APR from the active vector (scale=6)
/// 4. Apply daily compounding formula to convert APR to APY
/// 5. Return APY with scale=6 (same as APR)
/// 6. Emit event with calculation details
///
/// # Access Control
/// - No authorization required (public read-only instruction)
/// - No state modifications performed
/// - Can be called by anyone at any time
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
///
/// # Returns
/// * `Ok(apy)` - The calculated APY with scale=6 (1_000_000 = 100%)
/// * `Err(crate::OnreError::NoActiveVector)` - If no pricing vector is currently active
/// * `Err(crate::OnreError::Overflow)` - If mathematical overflow occurs during calculation
/// * `Err(crate::OnreError::DivByZero)` - If division by zero occurs during calculation
///
/// # Events
/// * `GetAPYEvent` - Emitted on successful calculation containing offer PDA, APY, source APR, and timestamp
pub fn get_apy(ctx: Context<GetAPY>) -> Result<u64> {
    let offer = ctx.accounts.offer.load()?;
    let current_time = Clock::get()?.unix_timestamp as u64;

    // Find the currently active pricing vector
    let active_vector = find_active_vector_at(&offer, current_time)?;

    // Calculate APY from the vector's APR
    let apy = calculate_apy_from_apr(active_vector.apr)?;

    msg!(
        "APY Info - Offer PDA: {}, APR: {}, APY: {}, Timestamp: {}",
        ctx.accounts.offer.key(),
        active_vector.apr,
        apy,
        current_time
    );

    emit!(GetAPYEvent {
        offer_pda: ctx.accounts.offer.key(),
        apy,
        apr: active_vector.apr,
        timestamp: current_time,
    });

    Ok(apy)
}

/// Converts Annual Percentage Rate (APR) to Annual Percentage Yield (APY) using daily compounding
///
/// This function implements the standard financial formula for converting APR to APY
/// with daily compounding (365 periods per year). The calculation uses high-precision
/// fixed-point arithmetic to maintain accuracy across the full range of input values.
///
/// # Mathematical Formula
/// ```text
/// APY = (1 + APR/365)^365 - 1
/// ```
///
/// # Implementation Details
/// - Uses 1e18 internal precision for intermediate calculations
/// - Applies proper rounding for the final result
/// - Employs exponentiation by squaring for efficient power calculation
/// - All operations are checked for overflow protection
///
/// # Arguments
/// * `apr_scaled` - Annual Percentage Rate with scale=6 (1_000_000 = 100%)
///
/// # Returns
/// * `Ok(apy)` - Annual Percentage Yield with scale=6 (same scaling as input)
/// * `Err(crate::OnreError::Overflow)` - If mathematical overflow occurs
/// * `Err(crate::OnreError::DivByZero)` - If division by zero occurs
///
/// # Scale Information
/// Both input and output use scale=6, where 1_000_000 represents 100%:
/// - 10_000 = 1%
/// - 100_000 = 10%
/// - 1_000_000 = 100%
pub fn calculate_apy_from_apr(apr_scaled: u64) -> Result<u64> {
    let apr = apr_scaled as u128;

    // incr = INT_SCALE * (apr / EXT_SCALE) / N
    let num = INT_SCALE
        .checked_mul(apr)
        .ok_or_else(|| error!(crate::OnreError::Overflow))?;
    let den = EXT_SCALE
        .checked_mul(N as u128)
        .ok_or_else(|| error!(crate::OnreError::Overflow))?;
    let incr = num
        .checked_add(den / 2)
        .ok_or_else(|| error!(crate::OnreError::Overflow))?
        .checked_div(den)
        .ok_or_else(|| error!(crate::OnreError::DivByZero))?;

    let base = INT_SCALE
        .checked_add(incr)
        .ok_or_else(|| error!(crate::OnreError::Overflow))?;

    // (1 + r/n)^n at 1e18 precision
    let pow =
        pow_fixed(base, N as u64, INT_SCALE).ok_or_else(|| error!(crate::OnreError::Overflow))?;

    // APY_int = pow - 1.0
    let apy_int = pow
        .checked_sub(INT_SCALE)
        .ok_or_else(|| error!(crate::OnreError::Overflow))?;

    // Convert back to 1e6 scale with rounding: apy_scaled = round(apy_int * EXT_SCALE / INT_SCALE)
    let apy_scaled_u128 = mul_div_round_u128(apy_int, EXT_SCALE, INT_SCALE)
        .ok_or_else(|| error!(crate::OnreError::Overflow))?;

    if apy_scaled_u128 > u64::MAX as u128 {
        return Err(error!(crate::OnreError::Overflow));
    }

    Ok(apy_scaled_u128 as u64)
}
