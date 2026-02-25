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
