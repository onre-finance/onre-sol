/// Integer ceil division for `u128`.
///
/// Returns `None` on division by zero or overflow.
pub fn ceil_div_u128(numerator: u128, denominator: u128) -> Option<u128> {
    if denominator == 0 {
        return None;
    }

    numerator
        .checked_add(denominator.checked_sub(1)?)
        .and_then(|adjusted| adjusted.checked_div(denominator))
}

/// Multiply a `u64` amount by basis points using floor division.
///
/// Returns `None` on overflow.
pub fn mul_basis_points_floor(amount: u64, basis_points: u16) -> Option<u64> {
    let amount_u128 = (amount as u128)
        .checked_mul(basis_points as u128)?
        .checked_div(10_000)?;

    u64::try_from(amount_u128).ok()
}

/// Fixed-point exponentiation by squaring with half-up rounding at each multiply.
///
/// `base` and the returned value use the same fixed-point `scale`.
/// Returns `None` on overflow.
pub fn pow_fixed(mut base: u128, mut exp: u64, scale: u128) -> Option<u128> {
    let mut acc = scale;
    while exp > 0 {
        if (exp & 1) == 1 {
            acc = mul_div_round_u128(acc, base, scale)?;
        }
        exp >>= 1;
        if exp > 0 {
            base = mul_div_round_u128(base, base, scale)?;
        }
    }
    Some(acc)
}

fn mul_div_round_u128(a: u128, b: u128, denom: u128) -> Option<u128> {
    let prod = a.checked_mul(b)?;
    let adj = prod.checked_add(denom / 2)?;
    adj.checked_div(denom)
}
