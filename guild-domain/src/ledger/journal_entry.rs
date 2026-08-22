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
//! # Corrections are entries too
//!
//! A mistaken entry is never edited. It is undone by
//! [`NormalEntry::reverse`], which mirrors every posting into the opposite
//! direction, so the correction balances for the same reason the original did
//! — the two sides are the original's, swapped whole. Nothing re-checks it,
//! and nothing needs to.
//!
//! The two kinds of entry are separate types rather than a flag, because the
//! interesting rule is about what you may *do* with one. `reverse` lives on
//! [`NormalEntry`], so "a reversal cannot itself be reversed" is not a runtime
//! check the ledger has to remember — it is a method that a [`ReversalEntry`]
//! simply does not have. Making impossible states unrepresentable, applied to
//! an operation rather than to data.
//! <https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/>
//!
//! [`JournalEntry`] is the sum of the two, so a journal can hold both in one
//! sequence, and matching on it is how a caller finds out which it has.
//!
//! A reversal records the [`EntryId`] it undoes, which is what lets the ledger
//! answer "has this been corrected already?" — see [`JournalEntry::reverses`]
//! and [`Ledger::reversal_of`](super::journal::Ledger::reversal_of). Its
//! narrative is marked with that id as well, so the same fact reads correctly
//! in a journal dump; both come from the one argument, so they cannot drift.
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
use crate::identifiers::EntryId;
use crate::money::{Coin, MoneyError};

/// An entry recording something that happened, rather than undoing it.
///
/// What [`JournalEntry::new`] builds, and the only kind that can be reversed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalEntry {
    postings: Vec<Posting>,
    narrative: Narrative,
}

/// An entry that undoes another, and says which.
///
/// Built only by [`NormalEntry::reverse`], so it cannot exist without naming
/// the entry it corrects, and cannot be built from a reversal. It has no
/// `reverse` of its own: undoing an undoing is written by posting the
/// original again under its own narrative, which reads honestly, rather than
/// by cancelling a correction the books have already recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReversalEntry {
    original: EntryId,
    postings: Vec<Posting>,
    narrative: Narrative,
}

impl NormalEntry {
    /// Mirrors this entry into one that undoes it.
    ///
    /// Every posting keeps its account and its amount and changes only its
    /// direction, so the reversal's debits are this entry's credits and the
    /// other way about. It therefore balances by construction — which is why
    /// this is infallible where [`JournalEntry::new`] is not: there is no
    /// arrangement of a balanced entry that mirrors into an unbalanced one.
    ///
    /// `original` is the id the ledger minted for this entry. It is recorded
    /// on the reversal and prefixed to `narrative`, so the correction can be
    /// tied to what it corrects by a query and by a reader.
    ///
    /// `narrative` is the caller's reason for correcting — a quest abandoned,
    /// an amount keyed wrong — not the original's. That reason is the part a
    /// reader cannot reconstruct from the postings.
    #[must_use]
    pub fn reverse(&self, original: EntryId, narrative: Narrative) -> ReversalEntry {
        let postings = self
            .postings
            .iter()
            .map(|posting| posting.reverse())
            .collect();
        let narrative = narrative.reversal_of(original.clone());
        ReversalEntry {
            original,
            postings,
            narrative,
        }
    }
}

/// A balanced set of postings, and what they were for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JournalEntry {
    /// An entry recording something that happened.
    Normal(NormalEntry),
    /// An entry undoing one that came before it.
    Reverse(ReversalEntry),
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

        Ok(NormalEntry {
            postings,
            narrative,
        }
        .into())
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
        match self {
            Self::Normal(normal) => &normal.postings,
            Self::Reverse(reversed) => &reversed.postings,
        }
    }

    /// The entry this one undoes, or `None` if it undoes nothing.
    #[must_use]
    pub fn reverses(&self) -> Option<&EntryId> {
        match self {
            Self::Normal(_) => None,
            Self::Reverse(reversal) => Some(&reversal.original),
        }
    }

    /// What this entry says it was for.
    #[must_use]
    pub fn narrative(&self) -> &Narrative {
        match self {
            Self::Normal(normal) => &normal.narrative,
            Self::Reverse(reversed) => &reversed.narrative,
        }
    }
}

impl From<ReversalEntry> for JournalEntry {
    fn from(reversal: ReversalEntry) -> Self {
        Self::Reverse(reversal)
    }
}

impl From<NormalEntry> for JournalEntry {
    fn from(normal: NormalEntry) -> Self {
        Self::Normal(normal)
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
    /// The entry `id` names was not found in the ledger.
    #[error("the entry {id} was not found in the ledger")]
    EntryNotFound {
        /// The entry that was looked for.
        id: EntryId,
    },
    /// The entry was already a reversal, so it cannot be reversed again.
    #[error("the entry {0} was already a reversal, so it cannot be reversed again")]
    UnableToReverse(EntryId),
    /// The entry has a reversal already, and a second would undo it twice.
    #[error("the entry {original} was already reversed by {reversal}")]
    AlreadyReversed {
        /// The entry the caller asked to reverse.
        original: EntryId,
        /// The reversal that already undoes it.
        reversal: EntryId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identifiers::AdventurerId;
    use crate::identifiers::EntryId;
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

    fn correction() -> Narrative {
        "correction of quest-1".parse().expect("a narrative")
    }

    /// A funding entry: coin into the vault, and the debt to show for it.
    fn funding() -> JournalEntry {
        JournalEntry::new(
            vec![
                debit(Account::GuildVault, 400_000),
                credit(Account::ClientEscrow(adventurer("lord-bramble")), 400_000),
            ],
            narrative(),
        )
        .expect("a balanced entry")
    }

    /// The `NormalEntry` inside an entry `new` just built.
    ///
    /// `reverse` lives on `NormalEntry` rather than on `JournalEntry`, which
    /// is what makes "a reversal cannot be reversed" a matter of which type
    /// you are holding rather than a runtime check.
    fn normal(entry: &JournalEntry) -> &NormalEntry {
        match entry {
            JournalEntry::Normal(normal) => normal,
            JournalEntry::Reverse(_) => panic!("`new` builds a normal entry"),
        }
    }

    /// The debit and credit totals of an entry, summed wide so that a
    /// deliberately huge entry cannot overflow the check itself.
    fn sides(entry: &JournalEntry) -> (u128, u128) {
        entry
            .postings()
            .iter()
            .fold((0, 0), |(debits, credits), posting| {
                let amount = u128::from(posting.amount().as_coppers());
                match posting.direction() {
                    Direction::Debit => (debits + amount, credits),
                    Direction::Credit => (debits, credits + amount),
                }
            })
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

    #[test]
    fn should_record_the_entry_a_reversal_undoes() {
        // The acceptance criterion that makes a reversal more than a second
        // entry that happens to cancel: the journal can be asked what this
        // one corrects, without parsing prose out of the narrative.
        let entry = funding();

        let reversal: JournalEntry = normal(&entry)
            .reverse(EntryId::sequential(1), correction())
            .into();

        assert_eq!(reversal.reverses(), Some(&EntryId::sequential(1)));
    }

    #[test]
    fn should_record_nothing_for_an_entry_that_undoes_nothing() {
        // A normal entry corrects nothing, and says so.
        assert_eq!(funding().reverses(), None);
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

    #[test]
    fn should_build_a_normal_entry_from_balanced_postings() {
        // `new` is the door normal entries come through, and the only one.
        // A reversal is built from an entry that already exists.
        assert!(matches!(funding(), JournalEntry::Normal(_)));
    }

    #[test]
    fn should_flip_every_posting_when_reversing_an_entry() {
        // Same accounts, same amounts, same order — every one the other way
        // round. Order is part of it: a journal is read, and a reversal that
        // shuffles its lines is harder to set against the entry it undoes.
        let entry = funding();

        let reversal: JournalEntry = normal(&entry)
            .reverse(EntryId::sequential(1), correction())
            .into();

        assert_eq!(
            reversal.postings(),
            [
                credit(Account::GuildVault, 400_000),
                debit(Account::ClientEscrow(adventurer("lord-bramble")), 400_000),
            ]
        );
    }

    #[test]
    fn should_reverse_an_entry_of_more_than_two_postings() {
        // The settlement shape. A `reverse` that handled only a pair — or that
        // flipped the first posting and copied the rest — passes the test
        // above and fails this one.
        let thorne = adventurer("bramblewick-thorne");
        let entry = JournalEntry::new(
            vec![
                debit(Account::ClientEscrow(adventurer("lord-bramble")), 400_000),
                credit(Account::GuildFeeIncome, 60_000),
                credit(Account::AdventurerPayable(thorne.clone()), 340_000),
            ],
            narrative(),
        )
        .expect("a balanced entry");

        let reversal: JournalEntry = normal(&entry)
            .reverse(EntryId::sequential(1), correction())
            .into();

        assert_eq!(
            reversal.postings(),
            [
                credit(Account::ClientEscrow(adventurer("lord-bramble")), 400_000),
                debit(Account::GuildFeeIncome, 60_000),
                debit(Account::AdventurerPayable(thorne), 340_000),
            ]
        );
    }

    #[test]
    fn should_mark_the_narrative_with_the_entry_the_reversal_undoes() {
        // The caller's reason is kept — a reversal is written for its own
        // reason, and that is what a reader needs — but it is prefixed with
        // the entry being undone, so a journal dump reads as prose without
        // anyone having to cross-reference ids by hand.
        let entry = funding();

        let reversal: JournalEntry = normal(&entry)
            .reverse(EntryId::sequential(1), correction())
            .into();

        assert_eq!(
            reversal.narrative().as_str(),
            "reversal of entry-1: correction of quest-1"
        );
    }

    #[test]
    fn should_read_a_reversal_as_a_reversal_rather_than_a_normal_entry() {
        // The distinction the enum exists for: it is what lets the ledger
        // refuse to reverse a reversal without keeping a flag of its own.
        let entry = funding();

        let reversal: JournalEntry = normal(&entry)
            .reverse(EntryId::sequential(1), correction())
            .into();

        assert!(matches!(reversal, JournalEntry::Reverse(_)));
    }

    #[test]
    fn should_leave_the_entry_it_reverses_untouched() {
        // `reverse` borrows. Corrections are written, never applied in place.
        let entry = funding();

        let _ = normal(&entry).reverse(EntryId::sequential(1), correction());

        assert_eq!(entry, funding());
    }

    #[test]
    fn should_read_postings_and_narrative_through_either_kind_of_entry() {
        // Both accessors match on the variant, so each has two arms that could
        // disagree — reaching into the wrong field, or handing back the
        // original's narrative for a reversal.
        let entry = funding();
        let reversal: JournalEntry = normal(&entry)
            .reverse(EntryId::sequential(1), correction())
            .into();

        assert_eq!(entry.postings().len(), 2);
        assert_eq!(entry.narrative(), &narrative());
        assert_eq!(reversal.postings().len(), 2);
        assert_eq!(
            reversal.narrative().as_str(),
            "reversal of entry-1: correction of quest-1"
        );
    }

    proptest! {
        /// A reversal balances, whatever entry it came from.
        ///
        /// Worth stating because a reversal does not go through
        /// [`JournalEntry::new`] and so is never checked: it is balanced only
        /// because flipping every posting swaps the two sides whole, and this
        /// is what would fail were `reverse` to drop, duplicate, or re-amount
        /// one of them.
        ///
        /// It asserts the sides *swap* rather than merely that they agree,
        /// which is the stronger claim — an entry built with equal sides
        /// balances either way round, and would not notice a reversal that
        /// quietly rebuilt it.
        #[test]
        fn should_swap_the_two_sides_of_whatever_entry_it_reverses(
            credits in proptest::collection::vec(1_u64..1_000_000, 1..12)
        ) {
            let total: u64 = credits.iter().sum();
            let mut postings = vec![debit(Account::GuildVault, total)];
            postings.extend(
                credits
                    .iter()
                    .map(|amount| credit(Account::GuildFeeIncome, *amount)),
            );
            let entry = JournalEntry::new(postings, narrative()).expect("a balanced entry");

            let reversal: JournalEntry = normal(&entry).reverse(EntryId::sequential(1), correction()).into();

            let (debits, credits) = sides(&entry);
            prop_assert_eq!(sides(&reversal), (credits, debits));
        }
    }
}
