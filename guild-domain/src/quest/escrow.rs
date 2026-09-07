//! Funds the Guild holds against a quest, from the patron's payment to the
//! party's payout.
//!
//! An escrow is custody, not history. It answers one question — *what is the
//! Guild holding right now, and has it let go of it yet?* — and refuses every
//! operation that would let the same bounty leave twice. Where the money came
//! from and where it went is the ledger's business (M1), not this type's.
//!
//! # The public surface these tests specify
//!
//! ```text
//! pub struct Escrow;                                    // state is private
//!
//! Escrow::unfunded() -> Escrow
//! Escrow::open_escrow_with_funds(Coin) -> Escrow
//! Escrow::balance(&self) -> Option<Coin>
//! Escrow::settlement(&self) -> Option<Settlement>
//! Escrow::fund(&mut self, Coin) -> Result<(), EscrowError>
//! Escrow::disperse(&mut self, Coin) -> Result<Coin, EscrowError>
//! Escrow::refund(&mut self) -> Result<Coin, EscrowError>
//!
//! pub enum Settlement { Dispersed, Refunded }
//!
//! pub enum EscrowError {
//!     AlreadyFunded { held: Coin },
//!     NotFunded,
//!     AlreadyRefunded,
//!     AlreadyDispersed,
//!     DispursementError(MoneyError),
//! }
//! ```
//!
//! # Why the states are private
//!
//! The obvious alternative is typestate — `Escrow<Unfunded>` becoming
//! `Escrow<Funded>` — which turns every illegal transition into a compile
//! error. It is the right tool for a protocol that begins and ends inside one
//! stack frame.
//!
//! An escrow is not that. It is posted on one day, funded on another, and
//! settled forty days later, which means it round-trips through storage in
//! between. A repository cannot return `Escrow<Funded>` — it does not know
//! which state the row holds until it reads it. So the caller would have to
//! match to recover the type, and that match is this enum with extra steps.
//! The compile-time guarantee evaporates at the storage boundary, which is
//! precisely where escrow lives.
//!
//! Typestate also publishes the state machine. The state names land in every
//! caller's signature, `Quest` has to become generic over its escrow's state,
//! and that genericity spreads outward. Adding a `Disputed` state later would
//! break every one of them.
//!
//! So the state is a private enum inside a public struct — not a public enum,
//! whose variants Rust makes as visible as the enum itself. Callers see
//! behaviour (`fund`, `disperse`, `refund`, `balance`) and nothing else: they
//! cannot match on the state, cannot build a funded escrow holding no purse,
//! and cannot depend on how many states there are. A sixth state is an
//! internal change.
//!
//! What that costs is real and is stated here rather than hidden: dispersing
//! twice compiles, and fails at runtime with
//! [`EscrowError::AlreadyDispersed`]. The tests below are what hold that line.
//!
//! # Why `balance` returns an `Option`
//!
//! An unfunded escrow does not hold zero coppers — it holds *nothing*. There
//! is no purse yet, and `Coin::ZERO` would be an answer invented to fill a
//! hole in the type. `None` says the true thing.
//!
//! # Why dispersal both mutates and returns
//!
//! [`disperse`](Escrow::disperse) hands back the `Coin` and closes the escrow
//! in one step, which is a deliberate exception to command-query separation.
//! Splitting it into a query for the amount and a command to close would open
//! exactly the window this type exists to shut: between the two calls, the
//! bounty could be read repeatedly and paid out more than once. The atomicity
//! is the invariant.

use core::fmt::Formatter;
use std::fmt::Display;

use crate::money::{Coin, MoneyError};

/// Funds the Guild holds against a quest, from the patron's payment to the party's payout.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Escrow {
    state: State,
}

/// The escrow's whole lifecycle, private so no caller can name or match a state.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum State {
    Unfunded,
    Funded(Coin),
    Closed(Settlement),
}

/// Which state an escrow is in, without what that state holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    /// Holding no purse yet.
    Unfunded,
    /// Holding a bounty.
    Funded,
    /// Has let its bounty go.
    Closed,
}

impl std::fmt::Display for Stage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let variant = match self {
            Stage::Unfunded => "unfunded",
            Stage::Funded => "funded",
            Stage::Closed => "closed",
        };

        f.write_str(variant)
    }
}

impl State {
    /// Which stage this state belongs to.
    fn stage(&self) -> Stage {
        match self {
            State::Unfunded => Stage::Unfunded,
            State::Funded(_) => Stage::Funded,
            State::Closed(_) => Stage::Closed,
        }
    }
}

impl Escrow {
    /// Which stage the escrow is in.
    #[must_use]
    pub fn stage(&self) -> Stage {
        self.state.stage()
    }

    /// Opens an escrow that holds nothing yet.
    #[must_use]
    pub fn unfunded() -> Self {
        Escrow {
            state: State::Unfunded,
        }
    }

    /// Opens an escrow already holding `coin`, for a patron who pays on posting.
    #[must_use]
    pub fn open_escrow_with_funds(coin: Coin) -> Escrow {
        Escrow {
            state: State::Funded(coin),
        }
    }

    /// Places `coin` in an escrow that holds nothing yet.
    ///
    /// # Errors
    ///
    /// [`EscrowError::AlreadyFunded`] if a bounty is already held, since funding
    /// again would overwrite it, and [`EscrowError::AlreadyDispersed`] or
    /// [`EscrowError::AlreadyRefunded`] if the escrow has already closed.
    pub fn fund(&mut self, coin: Coin) -> Result<(), EscrowError> {
        match self.state {
            State::Unfunded => {
                self.state = State::Funded(coin);
                Ok(())
            }
            State::Funded(original_amount) => Err(EscrowError::AlreadyFunded {
                held: original_amount,
            }),
            State::Closed(settlement) => match settlement {
                Settlement::Dispersed => Err(EscrowError::AlreadyDispersed),
                Settlement::Refunded => Err(EscrowError::AlreadyRefunded),
            },
        }
    }

    /// How the escrow was closed, or `None` while it is still open.
    #[must_use]
    pub fn settlement(&self) -> Option<Settlement> {
        match self.state {
            State::Unfunded | State::Funded(_) => None,
            State::Closed(settlement) => Some(settlement),
        }
    }

    /// What the escrow holds, or `None` when there is no purse in it at all.
    #[must_use]
    pub fn balance(&self) -> Option<Coin> {
        match self.state {
            State::Funded(coin) => Some(coin),
            State::Unfunded | State::Closed(_) => None,
        }
    }

    /// Pays `coin` out of the escrow to the party.
    ///
    /// # Errors
    ///
    /// [`EscrowError::NotFunded`] if there is no bounty to pay from,
    /// [`EscrowError::DispursementError`] if `coin` is more than the escrow
    /// holds, and [`EscrowError::AlreadyDispersed`] or
    /// [`EscrowError::AlreadyRefunded`] if the escrow has already closed.
    pub fn disperse(&mut self, coin: Coin) -> Result<Coin, EscrowError> {
        match self.state {
            State::Unfunded => Err(EscrowError::NotFunded),
            State::Funded(starting_amount) => {
                if starting_amount == coin {
                    self.state = State::Closed(Settlement::Dispersed);
                } else {
                    self.state = State::Funded(starting_amount.checked_sub(coin)?);
                }
                Ok(coin)
            }
            State::Closed(settlement) => match settlement {
                Settlement::Dispersed => Err(EscrowError::AlreadyDispersed),
                Settlement::Refunded => Err(EscrowError::AlreadyRefunded),
            },
        }
    }

    /// Returns the whole bounty to the patron and closes the escrow.
    ///
    /// # Errors
    ///
    /// [`EscrowError::NotFunded`] if there is no bounty to return, and
    /// [`EscrowError::AlreadyDispersed`] or [`EscrowError::AlreadyRefunded`] if
    /// the escrow has already closed.
    pub fn refund(&mut self) -> Result<Coin, EscrowError> {
        match self.state {
            State::Unfunded => Err(EscrowError::NotFunded),
            State::Funded(coin) => {
                self.state = State::Closed(Settlement::Refunded);
                Ok(coin)
            }
            State::Closed(settlement) => match settlement {
                Settlement::Dispersed => Err(EscrowError::AlreadyDispersed),
                Settlement::Refunded => Err(EscrowError::AlreadyRefunded),
            },
        }
    }
}

impl Display for Escrow {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.state {
            State::Unfunded => write!(f, "unfunded escrow"),
            State::Funded(coin) => write!(f, "funded escrow with balance {coin}"),
            State::Closed(settlement) => write!(f, "{settlement} escrow"),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
/// How an escrow was closed, which is the one thing a closed escrow still knows.
pub enum Settlement {
    /// The bounty went out to the party.
    Dispersed,
    /// The bounty went back to the patron.
    Refunded,
}

impl Display for Settlement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Settlement::Dispersed => write!(f, "Dispersed"),
            Settlement::Refunded => write!(f, "Refunded"),
        }
    }
}

#[derive(Debug, PartialEq, thiserror::Error)]
/// The ways an escrow refuses to move funds, each naming what stood in the way.
pub enum EscrowError {
    /// The escrow already holds a bounty, and funding it again would overwrite one.
    #[error("the escrow already holds a bounty of {held}")]
    AlreadyFunded {
        /// What the escrow holds.
        held: Coin,
    },
    /// The escrow holds no bounty, so there is nothing to settle.
    #[error("the escrow holds no bounty to settle")]
    NotFunded,
    /// The bounty has already gone back to the patron.
    #[error("the bounty has already been refunded to the patron")]
    AlreadyRefunded,
    /// The bounty has already gone out to the party.
    #[error("the bounty has already been dispersed to the party")]
    AlreadyDispersed,
    /// More was asked of the escrow than it holds.
    #[error("the escrow cannot pay out more than it holds: {0}")]
    DispursementError(#[from] MoneyError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::Coin;
    use proptest::prelude::*;

    fn bounty() -> Coin {
        Coin::from_coppers(31_207)
    }

    fn funded_escrow() -> Escrow {
        let mut escrow = Escrow::unfunded();
        let _ = escrow.fund(bounty());
        escrow
    }

    // An escrow holding nothing.

    #[test]
    fn should_hold_no_balance_when_unfunded() {
        let escrow = Escrow::unfunded();

        assert_eq!(escrow.balance(), None);
    }

    #[test]
    fn should_have_no_settlement_when_unfunded() {
        let escrow = Escrow::unfunded();

        assert_eq!(escrow.settlement(), None);
    }

    // Funding.

    #[test]
    fn should_hold_the_bounty_it_was_funded_with() {
        let mut escrow = Escrow::unfunded();

        let funded = escrow.fund(bounty());

        assert_eq!(funded, Ok(()));
        assert_eq!(escrow.balance(), Some(bounty()));
    }

    /// A quest may be posted with no reward — a favour, or a Guild errand.
    /// Whether that is *allowed* is quest policy, not custody policy, so
    /// escrow accepts an empty purse and leaves the judgement to M2's
    /// `Quest`.
    #[test]
    fn should_accept_a_bounty_of_nothing() {
        let mut escrow = Escrow::unfunded();

        let funded = escrow.fund(Coin::ZERO);

        assert_eq!(funded, Ok(()));
        assert_eq!(escrow.balance(), Some(Coin::ZERO));
    }

    #[test]
    fn should_refuse_to_fund_an_escrow_that_already_holds_a_bounty() {
        let mut escrow = funded_escrow();

        let refunded = escrow.fund(Coin::from_coppers(500));

        assert_eq!(refunded, Err(EscrowError::AlreadyFunded { held: bounty() }));
    }

    /// A refused funding must not corrupt what is held. Overwriting the purse
    /// and *then* reporting the error would lose the original bounty.
    #[test]
    fn should_keep_the_original_bounty_when_a_second_funding_is_refused() {
        let mut escrow = funded_escrow();

        let _ = escrow.fund(Coin::from_coppers(500));

        assert_eq!(escrow.balance(), Some(bounty()));
    }

    #[test]
    fn should_refuse_to_fund_an_escrow_that_has_already_been_settled() {
        let mut escrow = funded_escrow();
        let _ = escrow.disperse(bounty());

        let refunded = escrow.fund(bounty());

        assert_eq!(refunded, Err(EscrowError::AlreadyDispersed));
    }

    // Dispersal: the bounty leaves for the party.

    #[test]
    fn should_hand_over_the_whole_bounty_when_dispersed() {
        let mut escrow = funded_escrow();

        let paid = escrow.disperse(bounty());

        assert_eq!(paid, Ok(bounty()));
    }

    #[test]
    fn should_hold_no_balance_once_dispersed() {
        let mut escrow = funded_escrow();

        let _ = escrow.disperse(bounty());

        assert_eq!(escrow.balance(), None);
    }

    #[test]
    fn should_record_that_it_was_dispersed() {
        let mut escrow = funded_escrow();

        let _ = escrow.disperse(bounty());

        assert_eq!(escrow.settlement(), Some(Settlement::Dispersed));
    }

    #[test]
    fn should_refuse_to_disperse_an_unfunded_escrow() {
        let mut escrow = Escrow::unfunded();

        let paid = escrow.disperse(bounty());

        assert_eq!(paid, Err(EscrowError::NotFunded));
    }

    #[test]
    fn should_stay_unfunded_when_a_dispersal_is_refused() {
        let mut escrow = Escrow::unfunded();

        let _ = escrow.disperse(bounty());

        assert_eq!(escrow.settlement(), None);
        assert_eq!(escrow.balance(), None);
    }

    /// The double-spend test. Everything else in this file exists to make
    /// this one hold.
    #[test]
    fn should_refuse_to_disperse_a_bounty_twice() {
        let mut escrow = funded_escrow();
        let _ = escrow.disperse(bounty());

        let paid_again = escrow.disperse(bounty());

        assert_eq!(paid_again, Err(EscrowError::AlreadyDispersed));
    }

    #[test]
    fn should_refuse_to_disperse_a_bounty_that_was_refunded() {
        let mut escrow = funded_escrow();
        let _ = escrow.refund();

        let paid = escrow.disperse(bounty());

        assert_eq!(paid, Err(EscrowError::AlreadyRefunded));
    }

    // Refund: the bounty goes back to the patron.

    #[test]
    fn should_hand_back_the_whole_bounty_when_refunded() {
        let mut escrow = funded_escrow();

        let returned = escrow.refund();

        assert_eq!(returned, Ok(bounty()));
    }

    #[test]
    fn should_hold_no_balance_once_refunded() {
        let mut escrow = funded_escrow();

        let _ = escrow.refund();

        assert_eq!(escrow.balance(), None);
    }

    #[test]
    fn should_record_that_it_was_refunded() {
        let mut escrow = funded_escrow();

        let _ = escrow.refund();

        assert_eq!(escrow.settlement(), Some(Settlement::Refunded));
    }

    #[test]
    fn should_refuse_to_refund_an_unfunded_escrow() {
        let mut escrow = Escrow::unfunded();

        let returned = escrow.refund();

        assert_eq!(returned, Err(EscrowError::NotFunded));
    }

    #[test]
    fn should_refuse_to_refund_a_bounty_twice() {
        let mut escrow = funded_escrow();
        let _ = escrow.refund();

        let returned_again = escrow.refund();

        assert_eq!(returned_again, Err(EscrowError::AlreadyRefunded));
    }

    #[test]
    fn should_refuse_to_refund_a_bounty_that_was_dispersed() {
        let mut escrow = funded_escrow();
        let _ = escrow.disperse(bounty());

        let returned = escrow.refund();

        assert_eq!(returned, Err(EscrowError::AlreadyDispersed));
    }

    // Errors read the way a Guild clerk would say them.

    #[test]
    fn should_name_the_held_bounty_when_refusing_a_second_funding() {
        let error = EscrowError::AlreadyFunded { held: bounty() };

        assert_eq!(
            error.to_string(),
            "the escrow already holds a bounty of 3g 12s 7c"
        );
    }

    #[test]
    fn should_say_the_escrow_is_empty_when_there_is_nothing_to_settle() {
        assert_eq!(
            EscrowError::NotFunded.to_string(),
            "the escrow holds no bounty to settle"
        );
    }

    #[test]
    fn should_name_how_it_was_settled_when_refusing_a_second_dispersal() {
        let error = EscrowError::AlreadyDispersed;
        assert_eq!(
            error.to_string(),
            "the bounty has already been dispersed to the party"
        );
    }

    #[test]
    fn should_name_a_refund_when_refusing_to_settle_again() {
        let error = EscrowError::AlreadyRefunded;

        assert_eq!(
            error.to_string(),
            "the bounty has already been refunded to the patron"
        );
    }

    proptest! {
        /// Conservation, on the way out to the party: an escrow hands over
        /// exactly what went in, for every bounty the type can hold, and
        /// keeps nothing back.
        #[test]
        fn should_hand_over_exactly_what_was_funded(coppers in 0..=u64::MAX) {
            let mut escrow = Escrow::unfunded();
            let bounty = Coin::from_coppers(coppers);

            prop_assert_eq!(escrow.fund(bounty), Ok(()));
            prop_assert_eq!(escrow.disperse(bounty), Ok(bounty));
            prop_assert_eq!(escrow.balance(), None);
        }

        /// Conservation, on the way back to the patron. Stated separately
        /// from dispersal so a bug in one path cannot hide behind the other.
        #[test]
        fn should_hand_back_exactly_what_was_funded(coppers in 0..=u64::MAX) {
            let mut escrow = Escrow::unfunded();
            let bounty = Coin::from_coppers(coppers);

            prop_assert_eq!(escrow.fund(bounty), Ok(()));
            prop_assert_eq!(escrow.refund(), Ok(bounty));
            prop_assert_eq!(escrow.balance(), None);
        }

        /// A settled escrow is settled for good: no bounty, of any size, can
        /// be put back in to be paid out a second time.
        #[test]
        fn should_stay_closed_once_settled(
            first in 0..=u64::MAX,
            second in 0..=u64::MAX,
        ) {
            let mut escrow = Escrow::unfunded();
            prop_assert_eq!(escrow.fund(Coin::from_coppers(first)), Ok(()));
            prop_assert!(escrow.disperse(Coin::from_coppers(first)).is_ok());

            prop_assert_eq!(
                escrow.fund(Coin::from_coppers(second)),
                Err(EscrowError::AlreadyDispersed)
            );
            prop_assert_eq!(escrow.balance(), None);
        }
    }
}
