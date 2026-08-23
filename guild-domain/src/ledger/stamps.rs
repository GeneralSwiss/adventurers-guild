//! Two clocks on every entry: when it happened, and when we found out.
//!
//! A party walks back through the gate on day 21 and reports a death that
//! happened on day 12. The books have to answer two different questions about
//! that, and they are not the same question:
//!
//! - *What was true?* — the death happened on day 12, and always will have.
//! - *What did we know, and when?* — on day 20 the Guild believed that
//!   adventurer alive, and any statement it made about the payroll that day
//!   was correct **given what it knew**.
//!
//! [`Stamps`] carries both, so a balance can be asked for as of a moment in
//! either reckoning. See [`Ledger::balance_as_of`](super::journal::Ledger::balance_as_of).
//!
//! ```
//! use guild_domain::ledger::Stamps;
//! use guild_domain::time::{Duration, WorldInstant};
//!
//! let day = |n: u64| WorldInstant::from_seconds_since_founding(n * Duration::SECONDS_PER_DAY);
//!
//! // Learned on day 21 about a death on day 12.
//! let backdated = Stamps::new(day(12), day(21));
//! assert!(backdated.occurred_at() < backdated.recorded_at());
//!
//! // A payout the Guild commits to now, effective at the muster next week.
//! let postdated = Stamps::new(day(28), day(21));
//! assert!(postdated.recorded_at() < postdated.occurred_at());
//! ```
//!
//! # Why the two stamps constrain each other in neither direction
//!
//! Backdating is the case the Guild lives on: news travels at the speed of a
//! walking party, so almost everything is learned after the fact. Postdating
//! is rarer but just as real — a bounty agreed today and effective at the
//! next muster is known before it happens.
//!
//! Requiring one order or the other would make one of those unrepresentable,
//! and the workaround would be a lie in the stamps. So [`Stamps::new`] takes
//! any pair and cannot fail.
//!
//! The one rule that does exist lives on the ledger rather than here, because
//! it is about a journal rather than about a pair: `recorded_at` may not run
//! backwards from one entry to the next. Knowledge only accumulates. See
//! [`Ledger::post`](super::journal::Ledger::post).
//!
//! # Further reading
//!
//! Martin Fowler, [Bitemporal History](https://martinfowler.com/articles/bitemporal-history.html),
//! which names the two axes and works through why a single timestamp cannot
//! answer both questions.

use crate::time::WorldInstant;

/// When something happened, and when the Guild found out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Stamps {
    occurred_at: WorldInstant,
    recorded_at: WorldInstant,
}

impl Stamps {
    /// The pair for something that happened at `occurred_at` and was written
    /// down at `recorded_at`.
    #[must_use]
    pub const fn new(occurred_at: WorldInstant, recorded_at: WorldInstant) -> Self {
        Self {
            occurred_at,
            recorded_at,
        }
    }

    /// When the thing this entry records actually happened.
    #[must_use]
    pub const fn occurred_at(self) -> WorldInstant {
        self.occurred_at
    }

    /// When the Guild learned of it and wrote it down.
    #[must_use]
    pub const fn recorded_at(self) -> WorldInstant {
        self.recorded_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::Duration;

    fn day(count: u64) -> WorldInstant {
        WorldInstant::from_seconds_since_founding(count * Duration::SECONDS_PER_DAY)
    }

    #[test]
    fn should_hold_when_it_happened_and_when_the_guild_learned_of_it() {
        let stamps = Stamps::new(day(12), day(21));

        assert_eq!(stamps.occurred_at(), day(12));
        assert_eq!(stamps.recorded_at(), day(21));
    }
}
