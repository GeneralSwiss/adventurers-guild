//! How much world time passed: a span, with a length but no direction.
//!
//! [`Duration`] is a count of seconds. It is what you get by measuring between
//! two [`WorldInstant`](super::WorldInstant)s, and what you add to one to reach
//! another.
//!
//! ```
//! use guild_domain::time::Duration;
//!
//! let watch = Duration::from_seconds(4 * Duration::SECONDS_PER_HOUR + 30 * Duration::SECONDS_PER_MINUTE);
//! assert_eq!(watch.to_string(), "4h 30m");
//! ```
//!
//! # Why not `std::time::Duration`
//!
//! It is in the standard library, so it costs no dependency and the issue's
//! ban on `chrono` would not have caught it. It is still the wrong type here.
//!
//! `std::time::Duration` exists to measure *real* time, and the types it pairs
//! with — `Instant`, `SystemTime` — are readings from a machine clock. This
//! crate has no clock, and a domain type that invites `SystemTime::now()` into
//! a settlement calculation has given away the property the whole module is
//! for. It also carries nanosecond precision the Guild has no use for, and its
//! `Sub` panics on underflow, which is exactly the failure this crate reports
//! rather than raises.
//!
//! Owning the type costs an afternoon. Conversion to `std::time::Duration` for
//! a caller that genuinely wants one is a `From` impl, and it belongs outside
//! this crate along with the calendar.
//!
//! # Why seconds
//!
//! The unit has to be fine enough for the ledger's bitemporal stamps to
//! distinguish two entries made in the same audit, and coarse enough that no
//! quest lasts longer than the type can count. A second does both: `u64`
//! seconds runs to some 584 billion years, and the Guild will not outlast it.
//!
//! Minutes, hours, and days are rendering, not representation — the same
//! arrangement [`Coin`](crate::money::Coin) makes with silver and gold. A span
//! is one integer, so no conversion can round and no arithmetic can drift.
//!
//! # Why a span has no direction
//!
//! The inner type is `u64`, so there is no negative `Duration` to construct.
//! A span is a *length*: "three days" is a length, and whether it runs forwards
//! or backwards is a property of the operation you hand it to, not of the span
//! itself.
//!
//! This is the same call [`Coin`](crate::money::Coin) makes, for the same
//! reason and at the same cost. A purse cannot owe; a span cannot run
//! backwards. The direction that a signed type would have carried lives on
//! [`WorldInstant`](super::WorldInstant)'s operations instead — where
//! [`checked_add`](super::WorldInstant::checked_add) and
//! [`checked_sub`](super::WorldInstant::checked_sub) name it in words, and the
//! one measurement that could come out backwards,
//! [`duration_since`](super::WorldInstant::duration_since), reports it rather
//! than returning a number with a minus sign on it.

use std::fmt::{self, Display, Formatter};

/// A span of world time, counted in seconds.
///
/// Has a length but no direction — see the [module documentation](self) for
/// why, and for why this is not `std::time::Duration`.
///
/// Cheap to copy — the whole value is a `u64` — so spans pass by value
/// throughout the domain.
///
/// ```
/// use guild_domain::time::Duration;
///
/// let a_day = Duration::from_seconds(Duration::SECONDS_PER_DAY);
///
/// assert_eq!(a_day.to_string(), "1d");
/// assert!(a_day > Duration::ZERO);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Duration(u64);

impl Duration {
    /// No time at all.
    pub const ZERO: Self = Self(0);

    /// Seconds to the minute.
    pub const SECONDS_PER_MINUTE: u64 = 60;

    /// Minutes to the hour.
    pub const MINUTES_PER_HOUR: u64 = 60;

    /// Hours to the day.
    pub const HOURS_PER_DAY: u64 = 24;

    /// Seconds to the hour, derived so the ladder above stays the only place
    /// the world's units are written down.
    pub const SECONDS_PER_HOUR: u64 = Self::SECONDS_PER_MINUTE * Self::MINUTES_PER_HOUR;

    /// Seconds to the day, derived on the same ground.
    pub const SECONDS_PER_DAY: u64 = Self::SECONDS_PER_HOUR * Self::HOURS_PER_DAY;

    /// Builds a span of `seconds`.
    ///
    /// Total: every `u64` is a valid span. Longer units are reached by
    /// multiplying through the constants above —
    /// `Duration::from_seconds(3 * Duration::SECONDS_PER_DAY)` — rather than
    /// by a `from_days` that would have to report the overflow that
    /// multiplication can produce.
    #[must_use]
    pub const fn from_seconds(seconds: u64) -> Self {
        Self(seconds)
    }

    /// The whole span, counted in seconds.
    ///
    /// The only way out of the type, and deliberately named for its unit: a
    /// bare `u64` at a call site says nothing about which unit it counts.
    #[must_use]
    pub const fn as_seconds(self) -> u64 {
        self.0
    }
}

impl Display for Duration {
    /// Renders the span largest unit first, omitting any it spans none of:
    /// `3d 4h 5m 6s`, `1d 6s`, `6s`.
    ///
    /// An empty span renders `0s` rather than nothing at all, so a zero-length
    /// membership in a settlement dump is visibly zero and not a missing line.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.0 == 0 {
            return f.write_str("0s");
        }

        let days = self.0 / Self::SECONDS_PER_DAY;
        let hours = (self.0 % Self::SECONDS_PER_DAY) / Self::SECONDS_PER_HOUR;
        let minutes = (self.0 % Self::SECONDS_PER_HOUR) / Self::SECONDS_PER_MINUTE;
        let seconds = self.0 % Self::SECONDS_PER_MINUTE;

        let mut separator = "";
        for (amount, suffix) in [(days, 'd'), (hours, 'h'), (minutes, 'm'), (seconds, 's')] {
            if amount > 0 {
                write!(f, "{separator}{amount}{suffix}")?;
                separator = " ";
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn should_hold_the_seconds_it_was_given() {
        assert_eq!(Duration::from_seconds(7).as_seconds(), 7);
    }

    #[test]
    fn should_fix_the_ladder_from_seconds_up_to_days() {
        assert_eq!(Duration::SECONDS_PER_MINUTE, 60);
        assert_eq!(Duration::MINUTES_PER_HOUR, 60);
        assert_eq!(Duration::HOURS_PER_DAY, 24);
        assert_eq!(Duration::SECONDS_PER_HOUR, 3_600);
        assert_eq!(Duration::SECONDS_PER_DAY, 86_400);
    }

    #[test]
    fn should_render_every_unit_it_spans() {
        let span = Duration::from_seconds(
            3 * Duration::SECONDS_PER_DAY
                + 4 * Duration::SECONDS_PER_HOUR
                + 5 * Duration::SECONDS_PER_MINUTE
                + 6,
        );

        assert_eq!(span.to_string(), "3d 4h 5m 6s");
    }

    #[test]
    fn should_omit_a_unit_it_spans_none_of() {
        let span = Duration::from_seconds(Duration::SECONDS_PER_DAY + 6);

        assert_eq!(span.to_string(), "1d 6s");
    }

    #[test]
    fn should_render_an_empty_span_as_no_seconds() {
        assert_eq!(Duration::ZERO.to_string(), "0s");
    }

    #[test]
    fn should_render_a_span_of_exact_hours_as_hours_alone() {
        let span = Duration::from_seconds(2 * Duration::SECONDS_PER_HOUR);

        assert_eq!(span.to_string(), "2h");
    }

    #[test]
    fn should_order_spans_by_their_length() {
        assert!(Duration::from_seconds(7) < Duration::from_seconds(8));
    }

    proptest! {
        /// Rendering never invents or drops a unit: the parts read back off
        /// the rendered string re-count the span exactly.
        ///
        /// Stated as a property because the four units carry four chances to
        /// drop one at a boundary, and enumerating those by hand is how a
        /// boundary gets missed.
        #[test]
        fn should_render_parts_that_sum_to_the_whole_span(seconds in 0..=u64::MAX) {
            let span = Duration::from_seconds(seconds);
            let rendered = span.to_string();

            let counted: u64 = rendered
                .split(' ')
                .map(|part| {
                    let (amount, suffix) = part.split_at(part.len() - 1);
                    let amount: u64 = amount.parse().unwrap_or_default();
                    match suffix {
                        "d" => amount * Duration::SECONDS_PER_DAY,
                        "h" => amount * Duration::SECONDS_PER_HOUR,
                        "m" => amount * Duration::SECONDS_PER_MINUTE,
                        _ => amount,
                    }
                })
                .sum();

            prop_assert_eq!(counted, seconds);
        }
    }
}
