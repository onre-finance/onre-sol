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
