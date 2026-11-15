//! Shared rule for boss-only `enabled` switches of modular components.
use crate::OnreError;
use anchor_lang::prelude::*;

/// Sets `flag` to `enabled`, rejecting a no-op with `NoChange`; returns the
/// current timestamp for the caller's event.
pub(super) fn set_enabled_flag(flag: &mut bool, enabled: bool) -> Result<i64> {
    require!(*flag != enabled, OnreError::NoChange);
    *flag = enabled;
    Ok(Clock::get()?.unix_timestamp)
}
