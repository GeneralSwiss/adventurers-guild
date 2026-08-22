//! When something happened, reckoned from the Founding of the Guild.
//!
//! [`WorldInstant`] is a moment; [`Duration`] is the distance between two of
//! them. Party membership, escrow deadlines, and the ledger's bitemporal
//! stamps are all built out of the pair.
//!
//! ```
//! use guild_domain::time::{Duration, WorldInstant};
//!
//! let muster = WorldInstant::from_seconds_since_founding(12 * Duration::SECONDS_PER_HOUR);
//! let march = (muster + Duration::from_seconds(3 * Duration::SECONDS_PER_DAY))?;
//!
//! assert_eq!((march - muster)?, Duration::from_seconds(3 * Duration::SECONDS_PER_DAY));
//! # Ok::<(), guild_domain::time::TimeError>(())
//! ```
//!
//! # The epoch is the Founding
//!
//! Zero is the moment the Guild's charter was sealed, and every moment the
//! domain can name is a count of seconds after it. The Guild keeps its books
//! in its own reckoning, which is what a founding date is *for*.
//!
//! Choosing an epoch inside the domain rather than borrowing one from a
//! calendar is what makes the type total: there is no timezone, no leap
//! second, and no locale in a `u64`, so two moments can be compared and
//! subtracted with no context beyond the two values.
//!
//! It also fixes the direction of the type's only real constraint. Nothing in
//! this domain happened before the Guild existed, so the reckoning has a
//! floor, and running off it is
//! [`TimeError::BeforeTheFounding`] rather than a negative number.
//!
//! # No calendar here
//!
//! Turning a [`WorldInstant`] into a date a person would recognise means
//! choosing a real founding date, a calendar, and a timezone — three decisions
//! that belong to whoever is doing the displaying, not to the domain. That
//! conversion is a `From` impl living in an adapter, alongside the `chrono` or
//! `time` dependency this crate does not have.
//!
//! What the domain does have is [`Display`], which renders a moment in the
//! Guild's own terms: `3d 6s after the Founding`. Enough for a ledger dump or
//! a failing assertion to be read by a human, and not a claim to be a date.
//!
//! # Moments and spans are different kinds of thing
//!
//! The two types form an affine space: subtracting moments gives a span,
//! moving a moment by a span gives a moment, and adding two moments is
//! meaningless and does not compile. The [`WorldInstant`] documentation
//! carries the example that holds the compiler to it.
//!
//! The operators exist, but they report rather than panic. `std::time::Instant`
//! implements `Sub` by panicking when the moments arrive in the wrong order;
//! here the same expression yields a `Result`, so the operator impls are thin
//! delegations to [`duration_since`](WorldInstant::duration_since),
//! [`checked_add`](WorldInstant::checked_add), and
//! [`checked_sub`](WorldInstant::checked_sub), and `?` at the call site is what
//! reversal costs.

use std::fmt::{self, Display, Formatter};
use std::ops::{Add, Sub};

use super::Duration;

/// A moment in world time, as an offset from the Founding.
///
/// Moments and spans are different kinds of thing, and the type system is
/// what keeps them apart. A moment minus a moment is a [`Duration`]; a moment
/// plus a `Duration` is another moment; and a moment plus a moment is nothing
/// at all, so it does not compile:
///
/// ```compile_fail
/// use guild_domain::time::WorldInstant;
///
/// let muster = WorldInstant::from_seconds_since_founding(12);
/// let march = WorldInstant::from_seconds_since_founding(42);
///
/// let _ = muster + march; // error[E0369]: cannot add `WorldInstant` to `WorldInstant`
/// ```
///
/// The same expression with a span on the right compiles, which is what keeps
/// the example above honest — were the two types to become interchangeable,
/// the failing example would start compiling and `cargo test` would report it:
///
/// ```
/// use guild_domain::time::{Duration, WorldInstant};
///
/// let muster = WorldInstant::from_seconds_since_founding(12);
///
/// let march = (muster + Duration::from_seconds(30))?;
///
/// assert_eq!((march - muster)?, Duration::from_seconds(30));
/// # Ok::<(), guild_domain::time::TimeError>(())
/// ```
///
/// Cheap to copy — the whole value is a `u64` — so moments pass by value
/// throughout the domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct WorldInstant(u64);

impl WorldInstant {
    /// The moment the Guild's charter was sealed, and the origin of the
    /// reckoning.
    pub const FOUNDING: Self = Self(0);

    /// Builds the moment `seconds` after the Founding.
    #[must_use]
    pub const fn from_seconds_since_founding(seconds: u64) -> Self {
        Self(seconds)
    }

    /// How far this moment falls after the Founding, counted in seconds.
    #[must_use]
    pub const fn as_seconds_since_founding(self) -> u64 {
        self.0
    }

    /// Measures the span from `earlier` up to this moment.
    ///
    /// # Errors
    ///
    /// [`TimeError::RunsBackwards`] if `earlier` is in fact the later of the
    /// two. A [`Duration`] has no direction to record that in, so the
    /// reversal is reported rather than folded into the answer.
    pub fn duration_since(self, earlier: Self) -> Result<Duration, TimeError> {
        self.0
            .checked_sub(earlier.0)
            .map(Duration::from_seconds)
            .ok_or(TimeError::RunsBackwards {
                from: earlier,
                to: self,
            })
    }

    /// The moment `span` after this one.
    ///
    /// # Errors
    ///
    /// [`TimeError::Overflow`] if the moment would fall past the end of the
    /// reckoning, [`u64::MAX`] seconds after the Founding.
    pub fn checked_add(self, span: Duration) -> Result<Self, TimeError> {
        self.0
            .checked_add(span.as_seconds())
            .map(Self)
            .ok_or(TimeError::Overflow {
                instant: self,
                span,
            })
    }

    /// The moment `span` before this one.
    ///
    /// # Errors
    ///
    /// [`TimeError::BeforeTheFounding`] if the moment would fall before the
    /// Founding. The Guild's reckoning begins at its charter; there is no
    /// earlier moment for the domain to name.
    pub fn checked_sub(self, span: Duration) -> Result<Self, TimeError> {
        self.0
            .checked_sub(span.as_seconds())
            .map(Self)
            .ok_or(TimeError::BeforeTheFounding {
                instant: self,
                span,
            })
    }
}

impl Sub for WorldInstant {
    type Output = Result<Duration, TimeError>;

    /// Measures the span between two moments — see
    /// [`duration_since`](WorldInstant::duration_since), which this delegates
    /// to and which documents the failure.
    fn sub(self, earlier: Self) -> Self::Output {
        self.duration_since(earlier)
    }
}

impl Add<Duration> for WorldInstant {
    type Output = Result<Self, TimeError>;

    /// Moves forward by a span — see
    /// [`checked_add`](WorldInstant::checked_add), which this delegates to and
    /// which documents the failure.
    fn add(self, span: Duration) -> Self::Output {
        self.checked_add(span)
    }
}

impl Sub<Duration> for WorldInstant {
    type Output = Result<Self, TimeError>;

    /// Moves back by a span — see
    /// [`checked_sub`](WorldInstant::checked_sub), which this delegates to and
    /// which documents the failure.
    fn sub(self, span: Duration) -> Self::Output {
        self.checked_sub(span)
    }
}

impl Display for WorldInstant {
    /// Renders the moment as the span that has run since the Founding:
    /// `3d 6s after the Founding`.
    ///
    /// The Founding itself is named rather than rendered as an empty span,
    /// because `0s after the Founding` reads as a measurement that happened to
    /// come out zero, and this is the origin of the reckoning.
    ///
    /// This is the domain's own rendering, not a date. Putting a calendar to
    /// it belongs in an adapter — see the [module documentation](self).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.0 == 0 {
            return f.write_str("the Founding");
        }
        write!(f, "{} after the Founding", Duration::from_seconds(self.0))
    }
}

/// The ways arithmetic on world time can fail.
///
/// Every variant carries its operands, so a failure in settlement names the
/// moments that caused it rather than leaving them to be reconstructed from a
/// stack trace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TimeError {
    /// The two moments were handed over in the wrong order.
    #[error("time does not run backwards: {to} falls before {from}")]
    RunsBackwards {
        /// The moment measured from, which turned out to be the later.
        from: WorldInstant,
        /// The moment measured to, which turned out to be the earlier.
        to: WorldInstant,
    },
    /// The moment reached would fall before the Guild existed.
    #[error("the Guild's reckoning begins at the Founding: {span} before {instant} precedes it")]
    BeforeTheFounding {
        /// The moment counted back from.
        instant: WorldInstant,
        /// The span counted back.
        span: Duration,
    },
    /// The moment reached would fall past the end of the reckoning.
    #[error(
        "the reckoning ends {} seconds after the Founding: {span} after {instant} runs past it",
        u64::MAX
    )]
    Overflow {
        /// The moment counted forward from.
        instant: WorldInstant,
        /// The span counted forward.
        span: Duration,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn should_hold_the_seconds_since_the_founding_it_was_given() {
        let instant = WorldInstant::from_seconds_since_founding(7);

        assert_eq!(instant.as_seconds_since_founding(), 7);
    }

    #[test]
    fn should_place_the_founding_at_the_start_of_the_reckoning() {
        assert_eq!(WorldInstant::FOUNDING.as_seconds_since_founding(), 0);
    }

    #[test]
    fn should_order_moments_by_when_they_fall() {
        assert!(WorldInstant::FOUNDING < WorldInstant::from_seconds_since_founding(1));
    }

    #[test]
    fn should_subtract_one_moment_from_another_to_give_a_span() {
        let muster = WorldInstant::from_seconds_since_founding(12);
        let march = WorldInstant::from_seconds_since_founding(42);

        assert_eq!(march - muster, Ok(Duration::from_seconds(30)));
    }

    #[test]
    fn should_add_a_span_to_a_moment_to_give_a_moment() {
        let muster = WorldInstant::from_seconds_since_founding(12);

        assert_eq!(
            muster + Duration::from_seconds(30),
            Ok(WorldInstant::from_seconds_since_founding(42))
        );
    }

    #[test]
    fn should_subtract_a_span_from_a_moment_to_give_a_moment() {
        let march = WorldInstant::from_seconds_since_founding(42);

        assert_eq!(
            march - Duration::from_seconds(30),
            Ok(WorldInstant::from_seconds_since_founding(12))
        );
    }

    #[test]
    fn should_report_through_the_operators_what_the_methods_would_have_reported() {
        let dawn = WorldInstant::from_seconds_since_founding(7);
        let dusk = WorldInstant::from_seconds_since_founding(8);

        assert_eq!(dawn - dusk, dawn.duration_since(dusk));
        assert_eq!(
            dawn - Duration::from_seconds(8),
            dawn.checked_sub(Duration::from_seconds(8))
        );
    }

    #[test]
    fn should_render_a_moment_as_the_span_that_has_run_since_the_founding() {
        let instant = WorldInstant::from_seconds_since_founding(3 * Duration::SECONDS_PER_DAY + 6);

        assert_eq!(instant.to_string(), "3d 6s after the Founding");
    }

    #[test]
    fn should_render_the_founding_by_name_rather_than_as_an_empty_span() {
        assert_eq!(WorldInstant::FOUNDING.to_string(), "the Founding");
    }

    #[test]
    fn should_measure_the_span_from_an_earlier_moment() {
        let muster = WorldInstant::from_seconds_since_founding(12);
        let march = WorldInstant::from_seconds_since_founding(42);

        let served = march.duration_since(muster);

        assert_eq!(served, Ok(Duration::from_seconds(30)));
    }

    #[test]
    fn should_measure_no_span_at_all_from_the_same_moment() {
        let muster = WorldInstant::from_seconds_since_founding(12);

        assert_eq!(muster.duration_since(muster), Ok(Duration::ZERO));
    }

    #[test]
    fn should_refuse_to_measure_a_span_that_runs_backwards() {
        let muster = WorldInstant::from_seconds_since_founding(12);
        let march = WorldInstant::from_seconds_since_founding(42);

        let served = muster.duration_since(march);

        assert_eq!(
            served,
            Err(TimeError::RunsBackwards {
                from: march,
                to: muster,
            })
        );
    }

    #[test]
    fn should_reach_a_later_moment_by_adding_a_span() {
        let muster = WorldInstant::from_seconds_since_founding(12);

        let march = muster.checked_add(Duration::from_seconds(30));

        assert_eq!(march, Ok(WorldInstant::from_seconds_since_founding(42)));
    }

    #[test]
    fn should_refuse_an_addition_that_runs_past_the_end_of_the_reckoning() {
        let last = WorldInstant::from_seconds_since_founding(u64::MAX);

        let past_it = last.checked_add(Duration::from_seconds(1));

        assert_eq!(
            past_it,
            Err(TimeError::Overflow {
                instant: last,
                span: Duration::from_seconds(1),
            })
        );
    }

    #[test]
    fn should_reach_an_earlier_moment_by_subtracting_a_span() {
        let march = WorldInstant::from_seconds_since_founding(42);

        let muster = march.checked_sub(Duration::from_seconds(30));

        assert_eq!(muster, Ok(WorldInstant::from_seconds_since_founding(12)));
    }

    #[test]
    fn should_refuse_a_subtraction_that_reaches_before_the_founding() {
        let dawn = WorldInstant::from_seconds_since_founding(7);

        let before_it = dawn.checked_sub(Duration::from_seconds(8));

        assert_eq!(
            before_it,
            Err(TimeError::BeforeTheFounding {
                instant: dawn,
                span: Duration::from_seconds(8),
            })
        );
    }

    #[test]
    fn should_allow_a_subtraction_that_lands_exactly_on_the_founding() {
        let dawn = WorldInstant::from_seconds_since_founding(7);

        let back = dawn.checked_sub(Duration::from_seconds(7));

        assert_eq!(back, Ok(WorldInstant::FOUNDING));
    }

    proptest! {
        /// Measuring back the span you moved forward by returns it exactly.
        ///
        /// This is the invariant party membership leans on: a member who
        /// joined and then served a span has served exactly that span, with
        /// no residue at any boundary.
        #[test]
        fn should_measure_back_the_span_it_moved_forward_by(
            since_founding in 0..=u64::MAX,
            span in 0..=u64::MAX,
        ) {
            let moment = WorldInstant::from_seconds_since_founding(since_founding);
            let span = Duration::from_seconds(span);

            if let Ok(later) = moment.checked_add(span) {
                prop_assert_eq!(later.duration_since(moment), Ok(span));
            }
        }

        /// A span can be measured between two moments exactly when the one
        /// measured from does not fall later — which is to say, the ordering
        /// on moments and the success of the measurement are the same fact.
        #[test]
        fn should_measure_a_span_exactly_when_the_moments_are_in_order(
            earlier in 0..=u64::MAX,
            later in 0..=u64::MAX,
        ) {
            let earlier = WorldInstant::from_seconds_since_founding(earlier);
            let later = WorldInstant::from_seconds_since_founding(later);

            prop_assert_eq!(later.duration_since(earlier).is_ok(), earlier <= later);
        }
    }
}
