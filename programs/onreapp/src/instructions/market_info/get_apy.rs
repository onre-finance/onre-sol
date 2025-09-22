use crate::constants::seeds;
use crate::instructions::offer::offer_utils::{find_active_vector_at, find_offer};
use crate::instructions::OfferAccount;
use anchor_lang::prelude::*;
use anchor_lang::Accounts;

const EXT_SCALE: u128 = 1_000_000; // external/public scale
const INT_SCALE: u128 = 1_000_000_000_000_000_000; // 1e18 internal scale

const N: u128 = 365; // number of compounding periods per year

#[error_code]
pub enum GetAPYErrorCode {
    #[msg("Math overflow")]
    Overflow,
    #[msg("Division by zero")]
    DivByZero,
}

/// Event emitted when get_APY is called
#[event]
pub struct GetAPYEvent {
    /// The ID of the offer
    pub offer_id: u64,
    /// Current APY for the offer (scaled by 1_000_000)
    pub apy: u64,
    /// APR used for calculation (scaled by 1_000_000)
    pub apr: u64,
    /// Unix timestamp when the APY was calculated
    pub timestamp: u64,
}

/// Accounts required for getting APY information
#[derive(Accounts)]
pub struct GetAPY<'info> {
    /// The offer account containing all active offers
    #[account(seeds = [seeds::OFFERS], bump)]
    pub offer_account: AccountLoader<'info, OfferAccount>,
}

/// Calculates APY from APR using daily compounding
///
/// Formula: APY = (1 + APR/365)^365 - 1
/// Uses fixed-point arithmetic to maintain precision
///
/// # Arguments
/// * `apr` - Annual Percentage Rate scaled by 1_000_000 (e.g., 3.65% = 36_500)
///
/// # Returns
/// APY scaled by 1_000_000 (same scaling as input APR)
pub fn calculate_apy_from_apr(apr_scaled: u64) -> Result<u64> {
    let apr = apr_scaled as u128;

    // incr = INT_SCALE * (apr / EXT_SCALE) / N
    let num = INT_SCALE
        .checked_mul(apr)
        .ok_or_else(|| error!(GetAPYErrorCode::Overflow))?;
    let den = EXT_SCALE
        .checked_mul(N as u128)
        .ok_or_else(|| error!(GetAPYErrorCode::Overflow))?;
    let incr = num
        .checked_add(den / 2)
        .ok_or_else(|| error!(GetAPYErrorCode::Overflow))?
        .checked_div(den)
        .ok_or_else(|| error!(GetAPYErrorCode::DivByZero))?;

    let base = INT_SCALE
        .checked_add(incr)
        .ok_or_else(|| error!(GetAPYErrorCode::Overflow))?;

    // (1 + r/n)^n at 1e18 precision
    let pow = pow_fixed(base, N as u32, INT_SCALE)?;

    // APY_int = pow - 1.0
    let apy_int = pow
        .checked_sub(INT_SCALE)
        .ok_or_else(|| error!(GetAPYErrorCode::Overflow))?;

    // Convert back to 1e6 scale with rounding: apy_scaled = round(apy_int * EXT_SCALE / INT_SCALE)
    let apy_scaled_u128 = mul_div_round(apy_int, EXT_SCALE, INT_SCALE)?;

    if apy_scaled_u128 > u64::MAX as u128 {
        return Err(error!(GetAPYErrorCode::Overflow));
    }

    Ok(apy_scaled_u128 as u64)
}

#[inline]
fn mul_div_round(a: u128, b: u128, denom: u128) -> Result<u128> {
    // (a*b + denom/2) / denom  (round half-up)
    let prod = a
        .checked_mul(b)
        .ok_or_else(|| error!(GetAPYErrorCode::Overflow))?;
    let adj = prod
        .checked_add(denom / 2)
        .ok_or_else(|| error!(GetAPYErrorCode::Overflow))?;
    Ok(adj
        .checked_div(denom)
        .ok_or_else(|| error!(GetAPYErrorCode::DivByZero))?)
}

/// Fixed-point exponentiation by squaring:
/// returns (base^exp) in the same `scale`.
fn pow_fixed(mut base: u128, mut exp: u32, scale: u128) -> Result<u128> {
    let mut acc = scale; // 1.0
    while exp > 0 {
        if (exp & 1) == 1 {
            acc = mul_div_round(acc, base, scale)?;
        }
        exp >>= 1;
        if exp > 0 {
            base = mul_div_round(base, base, scale)?;
        }
    }
    Ok(acc)
}

/// Gets the current APY (Annual Percentage Yield) for a specific offer
///
/// This instruction allows anyone to query the current APY for an offer
/// without making any state modifications. The APY is calculated by converting
/// the stored APR using daily compounding formula.
///
/// # Arguments
///
/// * `ctx` - The instruction context containing required accounts
/// * `offer_id` - The unique ID of the offer to get the APY for
///
/// # Returns
///
/// * `Ok(apy)` - If the APY was successfully calculated
/// * `Err(_)` - If the offer doesn't exist or APY calculation fails
///
/// # Emits
///
/// * `GetAPYEvent` - Contains offer_id, apy, apr, and timestamp
pub fn get_apy(ctx: Context<GetAPY>, offer_id: u64) -> Result<u64> {
    let offer_account = ctx.accounts.offer_account.load()?;
    let current_time = Clock::get()?.unix_timestamp as u64;

    // Find the offer
    let offer = find_offer(&offer_account, offer_id)?;

    // Find the currently active pricing vector
    let active_vector = find_active_vector_at(&offer, current_time)?;

    // Calculate APY from the vector's APR
    let apy = calculate_apy_from_apr(active_vector.apr)?;

    msg!(
        "APY Info - Offer ID: {}, APR: {}, APY: {}, Timestamp: {}",
        offer_id,
        active_vector.apr,
        apy,
        current_time
    );

    emit!(GetAPYEvent {
        offer_id,
        apy,
        apr: active_vector.apr,
        timestamp: current_time,
    });

    Ok(apy)
}
