//! One account, moved by one amount, in one direction.
//!
//! [`Posting`] is the atom of double-entry: the smallest thing that can be
//! said about money moving. Everything above it is composition — a journal
//! entry is a set of postings that balance, a ledger is a sequence of entries,
//! and a balance is a fold over the postings naming one account.
//!
//! ```
//! use guild_domain::ledger::{Account, Posting};
//! use guild_domain::money::Coin;
//!
//! let patron = "lord-bramble".parse()?;
//!
//! // The Guild takes 40g into escrow: its coin goes up, and so does what it owes.
//! let into_the_vault = Posting::debit(Account::GuildVault, Coin::from_coppers(400_000))?;
//! let owed_to_patron = Posting::credit(Account::ClientEscrow(patron), Coin::from_coppers(400_000))?;
//!
//! assert_eq!(into_the_vault.amount(), owed_to_patron.amount());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Those two balance, but nothing here knows that. A posting on its own is a
//! half-truth; the rule that debits must equal credits belongs to the journal
//! entry, which is where it can actually be enforced.
//!
//! # Why a direction rather than a sign
//!
//! [`Coin`] is unsigned, so the amount cannot say which way money moved. That
//! is deliberate, and it is the trade the [`money`](crate::money) module made
//! when it chose `u64`: a purse cannot owe, and in exchange the ledger carries
//! direction in a [`Direction`] where the domain words `Debit` and `Credit`
//! live. See [`direction`](super::direction) for the argument.
//!
//! # Why zero is refused
//!
//! A posting of nothing moves no money and leaves every balance it takes part
//! in unchanged. It is not wrong so much as empty — and an empty posting in a
//! journal is a line every future reader has to look at and dismiss.
//!
//! Refusing it at construction means no ledger anywhere has to filter them,
//! and `amount()` can promise a non-zero purse to everything downstream. That
//! is "parse, don't validate" applied to a posting: holding one *is* the proof
//! that it moves something.
//! <https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/>
//!
//! It also has teeth in M4. A member who served no time earns a zero share,
//! and settlement that tries to post it gets an error naming the account
//! rather than quietly writing a line that says nothing.
//!
//! # Why the fields are private
//!
//! A posting is a fact about something that already happened, so there is no
//! honest reason to edit one. The fields are private and there is no `&mut`
//! accessor, which is the same discipline the ledger applies at a larger
//! scale: history is corrected by posting a reversal, never by editing what is
//! already written.

use super::account::Account;
use super::direction::Direction;
use crate::money::Coin;

/// One account moved by one amount, one way. The atom of double-entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Posting {
    account: Account,
    amount: Coin,
    direction: Direction,
}

impl Posting {
    /// Moves `amount` onto the debit side of `account`.
    ///
    /// # Errors
    ///
    /// [`PostingError::MovesNothing`] if `amount` is zero.
    ///
    /// ```
    /// use guild_domain::ledger::{Account, Direction, Posting};
    /// use guild_domain::money::Coin;
    ///
    /// let funding = Posting::debit(Account::GuildVault, Coin::from_coppers(400_000))?;
    ///
    /// assert_eq!(funding.direction(), Direction::Debit);
    /// # Ok::<(), guild_domain::ledger::PostingError>(())
    /// ```
    pub fn debit(account: Account, amount: Coin) -> Result<Self, PostingError> {
        Self::new(account, amount, Direction::Debit)
    }

    /// Moves `amount` onto the credit side of `account`.
    ///
    /// # Errors
    ///
    /// [`PostingError::MovesNothing`] if `amount` is zero.
    ///
    /// ```
    /// use guild_domain::ledger::{Account, Direction, Posting};
    /// use guild_domain::money::Coin;
    ///
    /// let fee = Posting::credit(Account::GuildFeeIncome, Coin::from_coppers(60_000))?;
    ///
    /// assert_eq!(fee.direction(), Direction::Credit);
    /// # Ok::<(), guild_domain::ledger::PostingError>(())
    /// ```
    pub fn credit(account: Account, amount: Coin) -> Result<Self, PostingError> {
        Self::new(account, amount, Direction::Credit)
    }

    /// The one place a posting is built, and so the only place the rule that a
    /// posting must move something can be enforced or bypassed.
    ///
    /// Private, because [`debit`](Self::debit) and [`credit`](Self::credit)
    /// read as the accounting they are, while a `Direction` argument at a call
    /// site reads as a parameter to be looked up.
    fn new(account: Account, amount: Coin, direction: Direction) -> Result<Self, PostingError> {
        if amount == Coin::ZERO {
            return Err(PostingError::MovesNothing { account });
        }
        Ok(Self {
            account,
            amount,
            direction,
        })
    }

    /// The account this posting names.
    #[must_use]
    pub fn account(&self) -> &Account {
        &self.account
    }

    /// How much this posting moves. Never zero.
    #[must_use]
    pub fn amount(&self) -> Coin {
        self.amount
    }

    /// Which side of the entry this posting falls on.
    #[must_use]
    pub fn direction(&self) -> Direction {
        self.direction
    }
}

/// The ways a posting can fail to be constructed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PostingError {
    /// The posting moved nothing.
    #[error("a posting must move something: {account:?} was posted zero coppers")]
    MovesNothing {
        /// The account the empty posting named.
        account: Account,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adventurer(name: &str) -> crate::identifiers::AdventurerId {
        name.parse().expect("a well-formed id")
    }

    #[test]
    fn should_hold_the_account_amount_and_direction_it_was_given() {
        let thorne = adventurer("bramblewick-thorne");

        let posting = Posting::debit(
            Account::AdventurerPayable(thorne.clone()),
            Coin::from_coppers(166_530),
        )
        .expect("a posting of more than nothing");

        assert_eq!(posting.account(), &Account::AdventurerPayable(thorne));
        assert_eq!(posting.amount(), Coin::from_coppers(166_530));
        assert_eq!(posting.direction(), Direction::Debit);
    }

    #[test]
    fn should_place_a_credit_on_the_credit_side() {
        let posting = Posting::credit(Account::GuildFeeIncome, Coin::from_coppers(60_000))
            .expect("a posting of more than nothing");

        assert_eq!(posting.direction(), Direction::Credit);
    }

    #[test]
    fn should_reject_a_debit_of_nothing() {
        // A zero posting carries no information and clutters the ledger: it
        // moves no money, and every balance it takes part in is unchanged by
        // it. Rejecting it at construction keeps the noise out of the journal
        // rather than teaching every reader to filter it.
        assert_eq!(
            Posting::debit(Account::GuildVault, Coin::from_coppers(0)),
            Err(PostingError::MovesNothing {
                account: Account::GuildVault
            })
        );
    }

    #[test]
    fn should_reject_a_credit_of_nothing_too() {
        // Triangulates the rule onto both constructors. An implementation that
        // checks only the path it was first written against passes the test
        // above and fails this one.
        let alder = adventurer("alder-quill");

        assert_eq!(
            Posting::credit(Account::EstatePayable(alder.clone()), Coin::from_coppers(0)),
            Err(PostingError::MovesNothing {
                account: Account::EstatePayable(alder)
            })
        );
    }

    #[test]
    fn should_name_the_account_that_was_posted_nothing() {
        // The error is read in settlement, where many postings are built at
        // once and "one of them was zero" is not enough to act on.
        let mirren = adventurer("mirren-vale");

        let refused = Posting::credit(
            Account::AdventurerPayable(mirren.clone()),
            Coin::from_coppers(0),
        )
        .expect_err("zero is refused");

        assert_eq!(
            refused,
            PostingError::MovesNothing {
                account: Account::AdventurerPayable(mirren)
            }
        );
    }
}
