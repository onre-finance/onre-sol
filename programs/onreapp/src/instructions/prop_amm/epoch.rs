//! Fixed-boundary volume epochs shared by the legacy and modular Prop AMM trackers.
//!
//! Epochs of duration `D` stay aligned to the stored `epoch_start`. Within an epoch,
//! current net volume counts at full weight. At the scheduled end of the epoch its net
//! becomes previous-epoch pressure and decays linearly to zero over the next `D`, even
//! when no trade rolled the stored state. Trades advance `epoch_start` by whole
//! multiples of `D`, never to the trade's own timestamp, so late trades cannot extend
//! or restart decay. After two or more elapsed epochs all history has expired.
use crate::OnreError;
use anchor_lang::prelude::*;

/// Position of `now` relative to the stored epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Epoch {
    /// No stored epoch, or a clock before it: no history applies.
    Unstarted,
    /// Inside the stored epoch, `elapsed` seconds after its start.
    Current { elapsed: u64 },
    /// The stored epoch ended; its net decays from the scheduled boundary.
    Previous { elapsed: u64 },
    /// Two or more epochs elapsed; all stored history has expired.
    Expired { elapsed: u64 },
}

/// State changes a recorded trade applies before accumulating its own volume.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rollover {
    /// New aligned epoch start.
    pub start: i64,
    /// True when the stored current net becomes the previous net; otherwise it is cleared.
    pub carry_current_net: bool,
}

impl Epoch {
    pub(crate) fn at(epoch_start: i64, duration: u64, now: i64) -> Result<Self> {
        require!(duration > 0, OnreError::InvalidAmount);
        require!(now >= 0 && epoch_start >= 0, OnreError::InvalidTimestamp);
        if epoch_start == 0 || now < epoch_start {
            return Ok(Self::Unstarted);
        }
        let elapsed = (now - epoch_start) as u64;
        Ok(if elapsed as u128 >= duration as u128 * 2 {
            Self::Expired { elapsed }
        } else if elapsed >= duration {
            Self::Previous { elapsed }
        } else {
            Self::Current { elapsed }
        })
    }

    /// Cadence counts and current volume apply only inside the stored epoch.
    pub(crate) fn is_current(self) -> bool {
        matches!(self, Self::Current { .. })
    }

    /// Effective directional volume: decayed previous net, plus the current net while
    /// the stored epoch is still current, plus the pending trade at full weight.
    pub(crate) fn effective_volume(
        self,
        duration: u64,
        previous_net: u128,
        current_net: u128,
        pending: u128,
    ) -> Result<u128> {
        let duration = duration as u128;
        let (previous, current, offset) = match self {
            Self::Unstarted | Self::Expired { .. } => (0, 0, 0),
            Self::Previous { elapsed } => (current_net, 0, elapsed as u128 - duration),
            Self::Current { elapsed } => (previous_net, current_net, elapsed as u128),
        };
        // Split the product to retain the full u128 accumulator range during decay.
        let remaining = duration - offset;
        let decayed = (previous / duration)
            .checked_mul(remaining)
            .and_then(|v| v.checked_add((previous % duration) * remaining / duration))
            .ok_or(OnreError::MathOverflow)?;
        decayed
            .checked_add(current)
            .and_then(|v| v.checked_add(pending))
            .ok_or_else(|| OnreError::MathOverflow.into())
    }

    /// Rollover a trade at `now` must apply first; `None` inside the current epoch.
    pub(crate) fn rollover(self, duration: u64, now: i64) -> Option<Rollover> {
        let aligned = |elapsed: u64| now - (elapsed % duration) as i64;
        match self {
            Self::Current { .. } => None,
            Self::Unstarted => Some(Rollover {
                start: now,
                carry_current_net: false,
            }),
            Self::Previous { elapsed } => Some(Rollover {
                start: aligned(elapsed),
                carry_current_net: true,
            }),
            Self::Expired { elapsed } => Some(Rollover {
                start: aligned(elapsed),
                carry_current_net: false,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: u64 = 100;

    #[test]
    fn previous_net_decays_from_the_scheduled_boundary() {
        // Net sells of 100 in [100, 200) contribute 100, 50, 1 and 0 at 200, 250, 299 and 300.
        for (now, expected) in [(200, 100), (250, 50), (299, 1), (300, 0)] {
            let epoch = Epoch::at(100, D, now).unwrap();
            assert_eq!(epoch.effective_volume(D, 0, 100, 0).unwrap(), expected);
        }
        // A pending 20 at 299 counts at full weight on top of the decayed history.
        let epoch = Epoch::at(100, D, 299).unwrap();
        assert_eq!(epoch.effective_volume(D, 0, 100, 20).unwrap(), 21);
    }

    #[test]
    fn current_epoch_adds_decayed_previous_and_current_net() {
        let epoch = Epoch::at(100, D, 150).unwrap();
        assert!(epoch.is_current());
        assert_eq!(epoch.effective_volume(D, 80, 30, 5).unwrap(), 40 + 30 + 5);
        assert_eq!(epoch.rollover(D, 150), None);
    }

    #[test]
    fn rollover_keeps_boundaries_aligned_to_the_stored_start() {
        let late = Epoch::at(100, D, 299).unwrap();
        assert!(!late.is_current());
        assert_eq!(
            late.rollover(D, 299),
            Some(Rollover {
                start: 200,
                carry_current_net: true
            })
        );
        let expired = Epoch::at(100, D, 1_234).unwrap();
        assert_eq!(expired, Epoch::Expired { elapsed: 1_134 });
        assert_eq!(expired.effective_volume(D, 9, 9, 7).unwrap(), 7);
        assert_eq!(
            expired.rollover(D, 1_234),
            Some(Rollover {
                start: 1_200,
                carry_current_net: false
            })
        );
    }

    #[test]
    fn unstarted_epoch_has_no_history_and_starts_at_now() {
        for (start, now) in [(0, 500), (600, 500)] {
            let epoch = Epoch::at(start, D, now).unwrap();
            assert_eq!(epoch, Epoch::Unstarted);
            assert_eq!(epoch.effective_volume(D, 9, 9, 3).unwrap(), 3);
            assert_eq!(
                epoch.rollover(D, now),
                Some(Rollover {
                    start: now,
                    carry_current_net: false
                })
            );
        }
    }

    #[test]
    fn invalid_inputs_are_rejected() {
        assert!(Epoch::at(100, 0, 150).is_err());
        assert!(Epoch::at(100, D, -1).is_err());
        assert!(Epoch::at(-1, D, 150).is_err());
    }

    #[test]
    fn decay_keeps_full_u128_range() {
        let epoch = Epoch::at(100, D, 250).unwrap();
        let huge = u128::MAX / 2;
        assert_eq!(epoch.effective_volume(D, 0, huge, 0).unwrap(), huge / 2);
    }
}
