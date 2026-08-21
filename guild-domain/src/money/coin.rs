//! The Guild's money: one unit of currency, and the arithmetic that may be
//! performed on it.
//!
//! The base currency is the copper, and [`Coin`] is a count of coppers. Silver
//! and gold are rendering, not representation — a purse is one integer, and the
//! denominations exist only where a human reads them. Every bounty, fee, and
//! payout in this domain flows through this type.
//!
//! ```
//! use guild_domain::money::Coin;
//!
//! let bounty = Coin::from_coppers(31_207);
//! assert_eq!(bounty.to_string(), "3g 12s 7c");
//! ```
//!
//! # Integers, never floats
//!
//! `0.1 + 0.2 != 0.3` in binary floating point, and a settlement engine that
//! loses a copper per transaction loses the Guild's trust. Money is the one
//! place a float is unambiguously a bug, so the inner type is an integer count
//! of minor units — Fowler's Money pattern, whose `allocate` operation lands in
//! this module next: <https://martinfowler.com/eaaCatalog/money.html>
//!
//! # Why unsigned
//!
//! The inner type is `u64`, and this is settled here rather than in M1.
//!
//! Unsigned makes "a purse cannot owe" free: there is no negative `Coin` to
//! construct, so [`checked_sub`](Coin::checked_sub) returning
//! [`MoneyError::InsufficientFunds`] is the only way past zero, and no caller
//! can route around it. The cost lands on the ledger, which needs deltas in two
//! directions and cannot get them from the sign of the amount. It gets them
//! from a `Direction` enum on `Posting` instead — which is where that
//! distinction belongs anyway, since `Debit` and `Credit` are domain words and
//! a minus sign is not. Boolean blindness and sign blindness are the same
//! mistake.
//!
//! `u64` rather than `usize` because `usize` is the width of a pointer, which
//! is a property of the machine and not of the money. A purse that overflows on
//! a 32-bit target and not on a 64-bit one is a bug that only reproduces on the
//! hardware you do not have.
//!
//! # Why the constructor cannot fail
//!
//! Unlike [`identifiers`](crate::identifiers), where `TryFrom` is the sole
//! constructor because most strings are not valid identifiers, *every* `u64` is
//! a valid count of coppers. There is no invariant to enforce at construction,
//! so [`from_coppers`](Coin::from_coppers) is total and returns `Self`. A
//! `TryFrom` whose error can never be produced would be false advertising: it
//! forces every call site to handle a case that does not exist, and teaches
//! readers to ignore the `Result` — which is exactly the habit "parse, don't
//! validate" is trying to build. Fallibility is a claim about the domain, and
//! this type has none to make.
//!
//! The invariants here are on the *operations*, not on construction, which is
//! why [`checked_add`](Coin::checked_add) and [`checked_sub`](Coin::checked_sub)
//! are the fallible ones.

use std::fmt::{self, Display, Formatter};

/// A purse, counted in coppers.
///
/// Holds a count of the base denomination. Silver and gold appear only in
/// [`Display`]; internally there is one integer, so no conversion can round and
/// no arithmetic can drift.
///
/// Cheap to copy — the whole value is a `u64` — so unlike the identifiers this
/// type is [`Copy`], and money passes by value throughout the domain.
///
/// ```
/// use guild_domain::money::Coin;
///
/// let purse = Coin::from_coppers(42);
/// let spent = purse.checked_sub(Coin::from_coppers(12))?;
///
/// assert_eq!(spent, Coin::from_coppers(30));
/// # Ok::<(), guild_domain::money::MoneyError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Coin(u64);

impl Coin {
    /// An empty purse.
    pub const ZERO: Self = Self(0);

    /// Coppers to the silver.
    pub const COPPERS_PER_SILVER: u64 = 100;

    /// Silver to the gold.
    pub const SILVER_PER_GOLD: u64 = 100;

    /// Coppers to the gold, derived so the two rates above stay the only place
    /// the Guild's exchange rate is written down.
    pub const COPPERS_PER_GOLD: u64 = Self::COPPERS_PER_SILVER * Self::SILVER_PER_GOLD;

    /// Builds a purse holding `coppers`.
    ///
    /// Total: every `u64` is a valid purse. See the [module
    /// documentation](self) for why this does not return a `Result`.
    #[must_use]
    pub const fn from_coppers(coppers: u64) -> Self {
        Self(coppers)
    }

    /// The whole purse, counted in coppers.
    ///
    /// The only way out of the type, and deliberately named for its unit: a
    /// bare `u64` at a call site says nothing about which denomination it
    /// counts, and this is the one place that ambiguity could enter.
    #[must_use]
    pub const fn as_coppers(self) -> u64 {
        self.0
    }

    /// Adds `other` to this purse.
    ///
    /// # Errors
    ///
    /// [`MoneyError::Overflow`] if the total exceeds [`u64::MAX`] coppers.
    /// Wrapping would turn the Guild's treasury into pocket change, so the
    /// overflow is reported rather than absorbed.
    pub fn checked_add(self, other: Self) -> Result<Self, MoneyError> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .ok_or(MoneyError::Overflow {
                augend: self,
                addend: other,
            })
    }

    /// Takes `withdrawal` out of this purse.
    ///
    /// # Errors
    ///
    /// [`MoneyError::InsufficientFunds`] if `withdrawal` exceeds what the purse
    /// holds. A purse cannot owe — debt is the ledger's concern, not a
    /// negative amount of money.
    pub fn checked_sub(self, withdrawal: Self) -> Result<Self, MoneyError> {
        self.0
            .checked_sub(withdrawal.0)
            .map(Self)
            .ok_or(MoneyError::InsufficientFunds {
                held: self,
                withdrawal,
            })
    }
}

impl Display for Coin {
    /// Renders the purse in the denominations a Guild clerk would speak,
    /// largest first, omitting any the purse has none of: `3g 12s 7c`, `1g 7c`,
    /// `7c`.
    ///
    /// An empty purse renders `0c` rather than nothing at all, so a zero
    /// balance in a ledger dump is visibly zero and not a missing line.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.0 == 0 {
            return f.write_str("0c");
        }

        let gold = self.0 / Self::COPPERS_PER_GOLD;
        let silver = (self.0 % Self::COPPERS_PER_GOLD) / Self::COPPERS_PER_SILVER;
        let coppers = self.0 % Self::COPPERS_PER_SILVER;

        let mut separator = "";
        for (amount, suffix) in [(gold, 'g'), (silver, 's'), (coppers, 'c')] {
            if amount > 0 {
                write!(f, "{separator}{amount}{suffix}")?;
                separator = " ";
            }
        }
        Ok(())
    }
}

/// The ways arithmetic on a purse can fail.
///
/// Both variants carry the operands, so a failure in settlement names the
/// amounts that caused it rather than leaving them to be reconstructed from
/// a stack trace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MoneyError {
    /// The withdrawal was larger than the purse.
    #[error("a purse cannot owe: it holds {held}, and {withdrawal} was withdrawn")]
    InsufficientFunds {
        /// What the purse held.
        held: Coin,
        /// What was asked of it.
        withdrawal: Coin,
    },
    /// The sum was larger than a purse can count.
    #[error(
        "a purse cannot hold more than {} coppers: {augend} plus {addend} overflows",
        u64::MAX
    )]
    Overflow {
        /// The purse added to.
        augend: Coin,
        /// The purse added.
        addend: Coin,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn should_hold_the_coppers_it_was_given() {
        assert_eq!(Coin::from_coppers(7).as_coppers(), 7);
    }

    #[test]
    fn should_fix_a_silver_at_a_hundred_coppers_and_a_gold_at_a_hundred_silver() {
        assert_eq!(Coin::COPPERS_PER_SILVER, 100);
        assert_eq!(Coin::SILVER_PER_GOLD, 100);
        assert_eq!(Coin::COPPERS_PER_GOLD, 10_000);
    }

    #[test]
    fn should_add_two_purses() {
        let purse = Coin::from_coppers(30);

        let total = purse.checked_add(Coin::from_coppers(12));

        assert_eq!(total, Ok(Coin::from_coppers(42)));
    }

    #[test]
    fn should_reject_an_addition_that_would_overflow() {
        let purse = Coin::from_coppers(u64::MAX);

        let total = purse.checked_add(Coin::from_coppers(1));

        assert_eq!(
            total,
            Err(MoneyError::Overflow {
                augend: Coin::from_coppers(u64::MAX),
                addend: Coin::from_coppers(1),
            })
        );
    }

    #[test]
    fn should_subtract_a_smaller_purse() {
        let purse = Coin::from_coppers(42);

        let left = purse.checked_sub(Coin::from_coppers(12));

        assert_eq!(left, Ok(Coin::from_coppers(30)));
    }

    #[test]
    fn should_reject_subtracting_more_than_the_purse_holds() {
        let purse = Coin::from_coppers(7);

        let left = purse.checked_sub(Coin::from_coppers(8));

        assert_eq!(
            left,
            Err(MoneyError::InsufficientFunds {
                held: Coin::from_coppers(7),
                withdrawal: Coin::from_coppers(8),
            })
        );
    }

    #[test]
    fn should_allow_a_withdrawal_that_empties_the_purse() {
        let purse = Coin::from_coppers(7);

        let left = purse.checked_sub(Coin::from_coppers(7));

        assert_eq!(left, Ok(Coin::ZERO));
    }

    #[test]
    fn should_render_every_denomination_it_holds() {
        let purse = Coin::from_coppers(31_207);

        assert_eq!(purse.to_string(), "3g 12s 7c");
    }

    #[test]
    fn should_omit_a_denomination_it_has_none_of() {
        let purse = Coin::from_coppers(10_007);

        assert_eq!(purse.to_string(), "1g 7c");
    }

    #[test]
    fn should_render_an_empty_purse_as_no_coppers() {
        assert_eq!(Coin::ZERO.to_string(), "0c");
    }

    #[test]
    fn should_render_a_purse_of_exact_silver_as_silver_alone() {
        assert_eq!(Coin::from_coppers(200).to_string(), "2s");
    }

    #[test]
    fn should_order_purses_by_what_they_hold() {
        assert!(Coin::from_coppers(7) < Coin::from_coppers(8));
    }

    proptest! {
        /// Subtraction undoes addition for every purse the type can hold.
        ///
        /// This is the invariant settlement leans on: money moved out of one
        /// purse and back leaves no residue. Stated as a property because the
        /// interesting cases are at the boundaries, and enumerating them by
        /// hand is how a boundary gets missed.
        #[test]
        fn should_undo_an_addition_by_subtracting_the_same_purse(
            held in 0..=u64::MAX,
            deposit in 0..=u64::MAX,
        ) {
            let purse = Coin::from_coppers(held);
            let deposit = Coin::from_coppers(deposit);

            if let Ok(total) = purse.checked_add(deposit) {
                prop_assert_eq!(total.checked_sub(deposit), Ok(purse));
            }
        }

        /// Rendering never invents or drops a denomination: the parts read back
        /// off the rendered string re-count the purse exactly.
        #[test]
        fn should_render_parts_that_sum_to_the_whole_purse(coppers in 0..=u64::MAX) {
            let purse = Coin::from_coppers(coppers);
            let rendered = purse.to_string();

            let counted: u64 = rendered
                .split(' ')
                .map(|part| {
                    let (amount, suffix) = part.split_at(part.len() - 1);
                    let amount: u64 = amount.parse().unwrap_or_default();
                    match suffix {
                        "g" => amount * Coin::COPPERS_PER_GOLD,
                        "s" => amount * Coin::COPPERS_PER_SILVER,
                        _ => amount,
                    }
                })
                .sum();

            prop_assert_eq!(counted, coppers);
        }
    }
}
