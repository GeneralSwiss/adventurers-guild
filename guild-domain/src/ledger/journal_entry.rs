//! An entry that cannot exist unless its two sides agree.
//!
//! This is the headline invariant of the crate. [`JournalEntry::new`] is the
//! only way to build one and it refuses anything that does not balance, so an
//! unbalanced entry is not a bug to be found — it is a value that cannot be
//! constructed. Nothing downstream can produce a ledger that does not balance,
//! because nothing downstream can produce the entry that would do it.
//!
//! ```
//! use guild_domain::ledger::{Account, JournalEntry, LedgerError, Posting};
//! use guild_domain::money::Coin;
//!
//! let patron = "lord-bramble".parse()?;
//!
//! let refused = JournalEntry::new(
//!     vec![
//!         Posting::debit(Account::GuildVault, Coin::from_coppers(400_000))?,
//!         Posting::credit(Account::ClientEscrow(patron), Coin::from_coppers(399_999))?,
//!     ],
//!     "quest-1 funded".parse()?,
//! );
//!
//! assert!(matches!(refused, Err(LedgerError::Unbalanced { .. })));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! One copper out is still out. This is a domain where "close enough" is the
//! bug.
//!
//! # Why there is no `new_unchecked`
//!
//! A test-only escape hatch would make the arrange blocks in this file
//! shorter, and it would cost the type its entire reason to exist. The moment
//! `new_unchecked` exists, something calls it — first a test, then a helper
//! that a test uses, then production code that borrowed the helper. The
//! guarantee here is only worth having while it is unconditional.
//!
//! # Two postings, minimum
//!
//! An entry moves money *between* accounts, so one posting cannot be an entry
//! and zero certainly cannot.
//!
//! This is a rule in its own right, not something the balance check happens to
//! cover. Zero debits equal zero credits, so an empty entry balances
//! perfectly — without a count rule it would be accepted.
//!
//! The count is checked first so that the error names the real problem. A lone
//! posting is unbalanced too, but reporting [`LedgerError::Unbalanced`] for it
//! would send a reader hunting for a wrong amount when what is missing is the
//! other half of the entry.
//!
//! # What balances is the totals, not the shape
//!
//! Settlement discharges one escrow across a fee and one payable per member —
//! one debit against four credits. Nothing requires the two sides to have the
//! same number of postings, only the same sum.
//!
//! # Where the money can run out
//!
//! [`Coin`] is a `u64` count of coppers, so summing a side can in principle
//! run past the end of one. That is reported as
//! [`LedgerError::SideOverflowed`] rather than wrapped: a ledger that silently
//! rolls over is worse than one that stops.

use super::direction::Direction;
use super::narrative::Narrative;
use super::posting::Posting;
use crate::money::{Coin, MoneyError};

/// A balanced set of postings, and what they were for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalEntry {
    postings: Vec<Posting>,
    narrative: Narrative,
}

impl JournalEntry {
    /// Gathers `postings` into an entry, or refuses.
    ///
    /// The only constructor, and deliberately the only one. There is no
    /// `new_unchecked`: the moment one exists something calls it, and the
    /// guarantee this type sells is that nothing can.
    ///
    /// # Errors
    ///
    /// - [`LedgerError::TooFewPostings`] if fewer than two postings were
    ///   offered. Checked first, so a lone posting is reported as the half an
    ///   entry it is rather than as an imbalance.
    /// - [`LedgerError::Unbalanced`] unless the two sides are exactly equal.
    /// - [`LedgerError::SideOverflowed`] if either side totals past
    ///   `u64::MAX` coppers.
    ///
    /// ```
    /// use guild_domain::ledger::{Account, JournalEntry, Posting};
    /// use guild_domain::money::Coin;
    ///
    /// let patron = "lord-bramble".parse()?;
    ///
    /// let entry = JournalEntry::new(
    ///     vec![
    ///         Posting::debit(Account::GuildVault, Coin::from_coppers(400_000))?,
    ///         Posting::credit(Account::ClientEscrow(patron), Coin::from_coppers(400_000))?,
    ///     ],
    ///     "quest-1 funded".parse()?,
    /// )?;
    ///
    /// assert_eq!(entry.postings().len(), 2);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(postings: Vec<Posting>, narrative: Narrative) -> Result<Self, LedgerError> {
        if postings.len() < 2 {
            return Err(LedgerError::TooFewPostings {
                found: postings.len(),
            });
        }

        let debits = Self::total(&postings, Direction::Debit)?;
        let credits = Self::total(&postings, Direction::Credit)?;
        if debits != credits {
            return Err(LedgerError::Unbalanced { debits, credits });
        }

        Ok(Self {
            postings,
            narrative,
        })
    }

    /// Sums one side of the entry.
    fn total(postings: &[Posting], side: Direction) -> Result<Coin, LedgerError> {
        postings
            .iter()
            .filter(|posting| posting.direction() == side)
            .try_fold(Coin::ZERO, |sum, posting| sum.checked_add(posting.amount()))
            .map_err(|source| LedgerError::SideOverflowed { side, source })
    }

    /// The postings this entry is made of. Always at least two, and always
    /// balanced.
    #[must_use]
    pub fn postings(&self) -> &[Posting] {
        &self.postings
    }

    /// What this entry says it was for.
    #[must_use]
    pub fn narrative(&self) -> &Narrative {
        &self.narrative
    }
}

/// The ways the ledger can refuse.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LedgerError {
    /// The two sides of the entry did not agree.
    #[error("an entry must balance: {debits} was debited against {credits} credited")]
    Unbalanced {
        /// What the debit side totalled.
        debits: Coin,
        /// What the credit side totalled.
        credits: Coin,
    },
    /// There was nothing for the entry to balance against.
    #[error(
        "an entry moves money between accounts, so it needs at least two postings, not {found}"
    )]
    TooFewPostings {
        /// How many postings were offered.
        found: usize,
    },
    /// One side totalled more than a purse can hold.
    #[error("an entry's {side:?} side overflowed a purse: {source}")]
    SideOverflowed {
        /// The side that ran past `u64::MAX` coppers.
        side: Direction,
        /// The arithmetic failure underneath.
        source: MoneyError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identifiers::AdventurerId;
    use crate::ledger::Account;
    use proptest::prelude::*;

    fn adventurer(name: &str) -> AdventurerId {
        name.parse().expect("a well-formed id")
    }

    fn narrative() -> Narrative {
        "settlement of quest-1".parse().expect("a narrative")
    }

    fn debit(account: Account, coppers: u64) -> Posting {
        Posting::debit(account, Coin::from_coppers(coppers)).expect("a posting of something")
    }

    fn credit(account: Account, coppers: u64) -> Posting {
        Posting::credit(account, Coin::from_coppers(coppers)).expect("a posting of something")
    }

    #[test]
    fn should_reject_an_entry_whose_debits_do_not_equal_its_credits() {
        // The headline invariant. A single copper out is still out — this is a
        // domain where "close enough" is the bug.
        let patron = adventurer("lord-bramble");

        let refused = JournalEntry::new(
            vec![
                debit(Account::GuildVault, 400_000),
                credit(Account::ClientEscrow(patron), 399_999),
            ],
            narrative(),
        );

        assert_eq!(
            refused,
            Err(LedgerError::Unbalanced {
                debits: Coin::from_coppers(400_000),
                credits: Coin::from_coppers(399_999),
            })
        );
    }

    #[test]
    fn should_hold_the_postings_and_narrative_it_was_given() {
        let patron = adventurer("lord-bramble");
        let postings = vec![
            debit(Account::GuildVault, 400_000),
            credit(Account::ClientEscrow(patron), 400_000),
        ];

        let entry =
            JournalEntry::new(postings.clone(), narrative()).expect("a balanced pair of postings");

        assert_eq!(entry.postings(), postings.as_slice());
        assert_eq!(entry.narrative(), &narrative());
    }

    #[test]
    fn should_balance_one_debit_against_many_credits() {
        // The settlement shape: escrow is discharged in one posting and lands
        // as a fee plus one payable per member. Balance is about the totals,
        // not about the count on each side.
        let thorne = adventurer("bramblewick-thorne");
        let alder = adventurer("alder-quill");

        let entry = JournalEntry::new(
            vec![
                debit(Account::ClientEscrow(adventurer("lord-bramble")), 400_000),
                credit(Account::GuildFeeIncome, 60_000),
                credit(Account::AdventurerPayable(thorne), 298_367),
                credit(Account::EstatePayable(alder), 41_633),
            ],
            narrative(),
        )
        .expect("400,000 debited against 400,000 credited");

        assert_eq!(entry.postings().len(), 4);
    }

    #[test]
    fn should_reject_an_entry_with_only_one_posting() {
        let refused = JournalEntry::new(vec![debit(Account::GuildVault, 400_000)], narrative());

        assert_eq!(refused, Err(LedgerError::TooFewPostings { found: 1 }));
    }

    #[test]
    fn should_reject_an_empty_entry_even_though_nothing_balances_nothing() {
        // Zero debits equal zero credits, so an empty entry balances
        // perfectly. Without a count rule of its own it would be accepted,
        // which is what this pins — not the order the two rules run in.
        let refused = JournalEntry::new(vec![], narrative());

        assert_eq!(refused, Err(LedgerError::TooFewPostings { found: 0 }));
    }

    #[test]
    fn should_refuse_a_side_that_totals_more_than_a_purse_can_hold() {
        // Coin is a u64 count of coppers, so a side can in principle be summed
        // past the end of one. Reported rather than wrapped: a ledger that
        // silently rolls over is worse than one that stops.
        let refused = JournalEntry::new(
            vec![
                debit(Account::GuildVault, u64::MAX),
                debit(Account::GuildVault, 1),
                credit(Account::GuildFeeIncome, 1),
            ],
            narrative(),
        )
        .expect_err("a debit side past u64::MAX");

        assert!(matches!(
            refused,
            LedgerError::SideOverflowed {
                side: Direction::Debit,
                ..
            }
        ));
    }

    proptest! {
        /// Whatever shape a balanced entry arrives in, it is accepted.
        ///
        /// The example tests pin two shapes — a pair, and one debit against
        /// three credits. This ranges over the rest: any number of credits,
        /// any amounts, discharged by a single debit built to match. Stated as
        /// a property because the failure mode is a filter or a fold that is
        /// subtly wrong for a count the examples happen not to use.
        #[test]
        fn should_accept_any_entry_whose_debit_matches_the_credits_it_discharges(
            credits in proptest::collection::vec(1_u64..1_000_000, 1..12)
        ) {
            let total: u64 = credits.iter().sum();

            let mut postings = vec![debit(Account::GuildVault, total)];
            postings.extend(
                credits
                    .iter()
                    .map(|amount| credit(Account::GuildFeeIncome, *amount)),
            );

            prop_assert!(JournalEntry::new(postings, narrative()).is_ok());
        }

        /// One copper in either direction is refused.
        ///
        /// The mirror of the property above, and the one that would catch a
        /// balance check written as `>=` or against the wrong side. The
        /// perturbation is deliberately the smallest one the domain can
        /// express, because that is the one a sloppy comparison lets through.
        #[test]
        fn should_reject_an_entry_that_is_off_by_a_single_copper(
            amount in 1_u64..1_000_000,
            over in proptest::bool::ANY,
        ) {
            let credited = if over { amount + 1 } else { amount - 1 };
            prop_assume!(credited > 0);

            let refused = JournalEntry::new(
                vec![
                    debit(Account::GuildVault, amount),
                    credit(Account::GuildFeeIncome, credited),
                ],
                narrative(),
            );

            prop_assert_eq!(
                refused,
                Err(LedgerError::Unbalanced {
                    debits: Coin::from_coppers(amount),
                    credits: Coin::from_coppers(credited),
                })
            );
        }
    }
}
