use anchor_lang::prelude::*;

use crate::state::Offer;

/// High-precision fixed-point natural logarithm using arctanh series
/// More accurate and faster converging than basic Taylor series
fn ln_fixed(x: u128) -> Result<i128> {
    const SCALE: u128 = 1_000_000_000_000;
    
    if x == 0 {
        return Err(ProgramError::InvalidArgument.into());
    }
    
    // Range reduction: normalize x to [SCALE/2, 2*SCALE]
    let mut k: i128 = 0;
    let mut x_adj = x;

    while x_adj > 2 * SCALE {
        x_adj /= 2;
        k += 1;
    }
    while x_adj < SCALE / 2 {
        x_adj = x_adj.checked_mul(2).ok_or(ProgramError::ArithmeticOverflow)?;
        k -= 1;
    }

    // Use arctanh identity: ln(x) = 2 * atanh((x-1)/(x+1))
    // t = (x - SCALE) / (x + SCALE)
    let num = (x_adj as i128) - (SCALE as i128);
    let den = (x_adj as i128) + (SCALE as i128);
    let t = num.checked_mul(SCALE as i128)
        .and_then(|x| Some(x / den))
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Series: atanh(t) = t + t³/3 + t⁵/5 + t⁷/7 + ...
    let t2 = t.checked_mul(t)
        .and_then(|x| Some(x / SCALE as i128))
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    let mut term = t;
    let mut sum = term;
    
    for i in (3..30).step_by(2) {
        term = term.checked_mul(t2)
            .and_then(|x| Some(x / SCALE as i128))
            .ok_or(ProgramError::ArithmeticOverflow)?;
        sum = sum.checked_add(term / i)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        
        if term.abs() < 1000 {
            break; // Precision cutoff
        }
    }
    
    let ln_core = 2 * sum; // atanh * 2 = ln

    // Add k * ln(2)
    const LN2: i128 = 693_147_180_559; // ln(2) * SCALE
    ln_core.checked_add(k.checked_mul(LN2).ok_or(ProgramError::ArithmeticOverflow)?)
        .ok_or(ProgramError::ArithmeticOverflow.into())
}

/// High-precision fixed-point exponential using range reduction
fn exp_fixed(y: i128) -> Result<u128> {
    const SCALE: u128 = 1_000_000_000_000;
    const LN2: i128 = 693_147_180_559; // ln(2) * SCALE
    
    // Range reduction: y = m * ln(2) + f
    let m = y / LN2;
    let f = y - m * LN2;

    // e^f via Taylor series around 0
    let mut term = SCALE as i128;
    let mut sum = term;
    
    for i in 1..20 {
        term = term.checked_mul(f)
            .and_then(|x| Some(x / SCALE as i128))
            .and_then(|x| Some(x / i as i128))
            .ok_or(ProgramError::ArithmeticOverflow)?;
        sum = sum.checked_add(term).ok_or(ProgramError::ArithmeticOverflow)?;
        
        if term.abs() < 1000 {
            break;
        }
    }

    // Scale by 2^m using bit shifts for exact powers of 2
    let result = if m >= 0 && m < 64 {
        (sum as u128).checked_shl(m as u32).unwrap_or(u128::MAX)
    } else if m < 0 && m > -64 {
        (sum as u128) >> (-m as u32)
    } else if m >= 64 {
        u128::MAX // Very large number
    } else {
        0 // Very small number
    };
    
    Ok(result)
}

#[derive(Accounts)]
pub struct GetApy<'info> {
    /// The offer to read NAV data from
    pub offer: Account<'info, Offer>,
}

pub fn get_apy(ctx: Context<GetApy>) -> Result<u64> {
    let offer = &ctx.accounts.offer;
    // Ensure buy token amount is not zero
    if offer.buy_token_1.amount == 0 {
        return Err(ProgramError::InvalidArgument.into());
    }

    // Calculate NAV values with high precision
    // Use 1e12 scaling (1 trillion) for much higher precision than basis points
    // This allows us to detect small NAV changes with large token amounts
    const NAV_SCALE: u128 = 1_000_000_000_000; // 1 trillion for high precision
    
    let start_nav_scaled = (offer.sell_token_start_amount as u128)
        .checked_mul(NAV_SCALE)
        .and_then(|x| x.checked_div(offer.buy_token_1.amount as u128))
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    let end_nav_scaled = (offer.sell_token_end_amount as u128)
        .checked_mul(NAV_SCALE)
        .and_then(|x| x.checked_div(offer.buy_token_1.amount as u128))
        .ok_or(ProgramError::ArithmeticOverflow)?;

    msg!("Start NAV (scaled): {}", start_nav_scaled);
    msg!("End NAV (scaled): {}", end_nav_scaled);

    let start_time = offer.offer_start_time;
    let end_time = offer.offer_end_time;

    // Ensure valid time range
    if end_time <= start_time || start_nav_scaled == 0 {
        msg!("Error: Invalid time range or zero start NAV");
        return Err(ProgramError::InvalidArgument.into());
    }

    // Constants
    const SECONDS_PER_YEAR: u64 = 365 * 86400; // 31,536,000 seconds in a year

    // Calculate time duration
    let time_duration = end_time - start_time;
    msg!("Time duration (seconds): {}", time_duration);
    msg!("Seconds per year: {}", SECONDS_PER_YEAR);
    
    // For APY calculation, use simpler approximation to avoid overflow
    // APY ≈ (end_nav / start_nav - 1) * (SECONDS_PER_YEAR / duration)
    
    if end_nav_scaled <= start_nav_scaled {
        msg!("No growth detected, returning 0% APY");
        return Ok(0);
    }

    // Calculate growth ratio = end_nav / start_nav
    let growth_ratio_scaled = end_nav_scaled
        .checked_mul(NAV_SCALE)
        .and_then(|x| x.checked_div(start_nav_scaled))
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    msg!("Growth ratio (scaled): {}", growth_ratio_scaled);
    
    // APY formula: ((end_nav/start_nav)^(SECONDS_PER_YEAR/time_duration)) - 1
    // Using logarithmic identity: (1 + x)^n = exp(n * ln(1 + x))
    // So: APY = exp(n * ln(growth_ratio)) - 1
    
    let annualization_factor_scaled = (SECONDS_PER_YEAR as u128)
        .checked_mul(NAV_SCALE)
        .and_then(|x| x.checked_div(time_duration as u128))
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    msg!("Annualization factor (scaled): {}", annualization_factor_scaled);
    
    // Calculate ln(growth_ratio) using high-precision fixed-point natural logarithm
    let ln_growth_ratio = ln_fixed(growth_ratio_scaled)?;
    msg!("ln(growth_ratio): {}", ln_growth_ratio);
    
    // Calculate n * ln(growth_ratio)
    let exponent = (annualization_factor_scaled as i128)
        .checked_mul(ln_growth_ratio)
        .and_then(|x| Some(x / NAV_SCALE as i128))
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    msg!("Exponent (n * ln(ratio)): {}", exponent);
    
    // Calculate exp(n * ln(growth_ratio))
    let result_scaled = exp_fixed(exponent)?;
    msg!("exp(exponent): {}", result_scaled);
    
    // APY = result - 1 (subtract the NAV_SCALE which represents 1)
    let apy_scaled = if result_scaled > NAV_SCALE {
        result_scaled - NAV_SCALE
    } else {
        0
    };
    
    msg!("APY (scaled): {}", apy_scaled);
    
    // Convert to basis points: multiply by 10000 and divide by our scale
    let apy_bp_u128 = apy_scaled
        .checked_mul(10000)
        .and_then(|x| x.checked_div(NAV_SCALE))
        .unwrap_or(0);
    
    // Convert back to u64, capping if too large
    // Cap at 10,000,000 basis points (100,000% APY) to handle extreme scenarios
    let apy_bp = u64::try_from(apy_bp_u128.min(10_000_000)).unwrap_or(10_000_000);
    
    msg!("APY calculation result: {}", apy_bp);

    msg!("Calculated APY (basis points): {}", apy_bp);

    // Already capped during u128 to u64 conversion
    let final_apy = apy_bp;
    msg!("Final APY (basis points, capped): {}", final_apy);
    msg!("=== APY Calculation End ===");

    Ok(final_apy)
}

