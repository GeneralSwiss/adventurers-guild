//! Which side of an entry a posting falls on.

/// Which side of an entry a posting falls on.
///
/// An enum rather than a sign, because `Debit` and `Credit` are domain words
/// and a minus sign is not. This is the same call the [`money`](crate::money)
/// module made when it chose an unsigned [`Coin`](crate::money::Coin): a purse
/// cannot owe, so direction cannot be carried in the amount and has to be
/// modelled where it belongs.
///
/// Boolean blindness and sign blindness are the same mistake. A
/// `Posting { negative: true }` compiles just as happily when it means the
/// opposite of what its writer intended; [`Direction::Credit`] does not.
///
/// The type does double duty. On a [`Posting`](super::Posting) it is the side
/// that posting falls on; on an [`Account`](super::Account) it is the side
/// that account *grows* on — its normal balance. Those are the same question
/// asked of two different things, which is why they share one type rather than
/// two identically-shaped ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// The left side. Increases assets and expenses.
    Debit,
    /// The right side. Increases liabilities, equity, and income.
    Credit,
}
