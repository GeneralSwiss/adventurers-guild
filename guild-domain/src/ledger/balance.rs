//! What an account is worth, and which way round.
//!
//! ```
//! use guild_domain::ledger::{Account, Balance, Direction};
//! use guild_domain::money::Coin;
//!
//! let balance = Balance::netting(Coin::from_coppers(400_000), Coin::from_coppers(60_000));
//!
//! assert_eq!(balance, Balance::Debit(Coin::from_coppers(340_000)));
//! assert_eq!(balance.side(), Some(Direction::Debit));
//! ```
//!
//! # Why not a signed number
//!
//! A balance needs to point two ways and [`Coin`] only counts upward, so
//! something has to carry the direction. A signed integer would do it, and
//! would mean the books' most-read number is one whose meaning depends on
//! remembering that negative means credit — sign blindness, the same mistake
//! as boolean blindness. The variant says which side it is on in the domain's
//! own words.
//!
//! It also keeps the subtraction honest: [`Balance::netting`] always takes the
//! smaller side from the larger, so no intermediate ever needs to represent a
//! debt, and the function has no failure to report.
//!
//! # Why nil is its own variant
//!
//! A balance of nothing does not fall on a side. Modelling it as `Coin::ZERO`
//! plus a [`Direction`] would invent a fact the books do not have, and every
//! reader would have to know that the direction on a zero is meaningless.
//! [`Balance::Nil`] makes that unrepresentable instead.
//!
//! # Abnormal balances
//!
//! [`Balance::is_abnormal_for`] is where [`Account::normal_side`] finally pays
//! off. An account sitting on the side it does not grow on is nearly always a
//! bug rather than a fact: a payable in debit means the Guild handed out more
//! than was earned, a vault in credit means it paid out coin it never held.
//! The ledger does not refuse those — they are real states a broken
//! calculation can reach — but it can say so.

use std::cmp::Ordering;

use super::account::Account;
use super::direction::Direction;
use crate::money::Coin;

/// What an account is worth once its postings are netted off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Balance {
    /// Nothing landed here, or what landed cancelled out exactly.
    Nil,
    /// More was debited than credited, by this much.
    Debit(Coin),
    /// More was credited than debited, by this much.
    Credit(Coin),
}

impl Balance {
    /// Nets `debits` against `credits`.
    ///
    /// Cannot fail: the larger side is always subtracted from, so the
    /// difference never runs below zero and [`Coin`] never has to represent a
    /// debt. Which side was larger is carried by the variant instead, which is
    /// the same trade the [`money`](crate::money) module made when it chose an
    /// unsigned purse.
    #[must_use]
    pub fn netting(debits: Coin, credits: Coin) -> Self {
        match debits.cmp(&credits) {
            Ordering::Equal => Self::Nil,
            Ordering::Greater => Self::Debit(Coin::from_coppers(
                debits.as_coppers() - credits.as_coppers(),
            )),
            Ordering::Less => Self::Credit(Coin::from_coppers(
                credits.as_coppers() - debits.as_coppers(),
            )),
        }
    }

    /// Which side the balance falls on, or `None` when it is [`Nil`](Balance::Nil).
    #[must_use]
    pub fn side(self) -> Option<Direction> {
        match self {
            Self::Nil => None,
            Self::Debit(_) => Some(Direction::Debit),
            Self::Credit(_) => Some(Direction::Credit),
        }
    }

    /// How far from nil the balance is.
    #[must_use]
    pub fn amount(self) -> Coin {
        match self {
            Self::Nil => Coin::ZERO,
            Self::Debit(amount) | Self::Credit(amount) => amount,
        }
    }

    /// Whether this balance sits on the side its account does not grow on.
    ///
    /// An abnormal balance is nearly always a bug rather than a fact: a
    /// payable in debit means the Guild handed out more than was earned, and a
    /// vault in credit means it paid out coin it never held. A [`Nil`](Balance::Nil)
    /// balance is never abnormal — an account nobody has posted to is not in
    /// trouble.
    #[must_use]
    pub fn is_abnormal_for(self, account: &Account) -> bool {
        match self.side() {
            None => false,
            Some(side) => side != account.normal_side(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identifiers::AdventurerId;

    fn adventurer(name: &str) -> AdventurerId {
        name.parse().expect("a well-formed id")
    }

    fn coppers(amount: u64) -> Coin {
        Coin::from_coppers(amount)
    }

    #[test]
    fn should_fall_on_the_debit_side_when_more_was_debited() {
        assert_eq!(
            Balance::netting(coppers(400_000), coppers(60_000)),
            Balance::Debit(coppers(340_000))
        );
    }

    #[test]
    fn should_fall_on_the_credit_side_when_more_was_credited() {
        assert_eq!(
            Balance::netting(coppers(60_000), coppers(400_000)),
            Balance::Credit(coppers(340_000))
        );
    }

    #[test]
    fn should_be_nil_when_the_two_sides_cancel() {
        // Nil is its own variant rather than a zero on some side, because a
        // balance of nothing does not fall on a side. Making that a `Coin::ZERO`
        // with a `Direction` attached would invent a fact the books do not have.
        assert_eq!(
            Balance::netting(coppers(400_000), coppers(400_000)),
            Balance::Nil
        );
    }

    #[test]
    fn should_be_nil_when_nothing_was_posted_at_all() {
        assert_eq!(Balance::netting(Coin::ZERO, Coin::ZERO), Balance::Nil);
    }

    #[test]
    fn should_report_no_side_for_a_nil_balance() {
        assert_eq!(Balance::Nil.side(), None);
        assert_eq!(Balance::Debit(coppers(1)).side(), Some(Direction::Debit));
        assert_eq!(Balance::Credit(coppers(1)).side(), Some(Direction::Credit));
    }

    #[test]
    fn should_report_nothing_as_the_amount_of_a_nil_balance() {
        assert_eq!(Balance::Nil.amount(), Coin::ZERO);
        assert_eq!(Balance::Debit(coppers(7)).amount(), coppers(7));
    }

    #[test]
    fn should_call_a_payable_in_debit_abnormal() {
        // The payoff of Account::normal_side. A payable grows on the credit
        // side, so a debit balance there means the Guild handed out more than
        // the adventurer earned — nearly always a bug rather than a fact.
        let thorne = Account::AdventurerPayable(adventurer("bramblewick-thorne"));

        assert!(Balance::Debit(coppers(1)).is_abnormal_for(&thorne));
        assert!(!Balance::Credit(coppers(1)).is_abnormal_for(&thorne));
    }

    #[test]
    fn should_call_a_vault_in_credit_abnormal() {
        // The mirror case, on the one debit-normal account in the chart. A
        // vault in credit means the Guild paid out coin it never held.
        assert!(Balance::Credit(coppers(1)).is_abnormal_for(&Account::GuildVault));
        assert!(!Balance::Debit(coppers(1)).is_abnormal_for(&Account::GuildVault));
    }

    #[test]
    fn should_not_call_a_nil_balance_abnormal() {
        // An account nobody has posted to is not in trouble, whichever way it
        // would grow if somebody did.
        assert!(!Balance::Nil.is_abnormal_for(&Account::GuildVault));
        assert!(!Balance::Nil.is_abnormal_for(&Account::GuildFeeIncome));
    }
}
