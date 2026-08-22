//! The Guild's chart of accounts: the buckets money sits in, and how it is
//! classified.
//!
//! This module is the noun list of the [record](super). [`Account`] names
//! every bucket the Guild's books recognise, and the enum *is* the chart of
//! accounts — an account nobody has enumerated here is a compile error rather
//! than a typo that silently opens a new one.
//!
//! # An account holds no money
//!
//! This is the design decision worth understanding before reading the rest.
//!
//! [`Account`] carries no balance and no entries. It is an inert key. The
//! ledger holds every posting, and a balance is a *fold* over the postings
//! that name a given account — derived on demand, never stored.
//!
//! Three things break if an account holds its own balance:
//!
//! - **Two sources of truth.** The entries and the cached balance can
//!   disagree, which in a ledger is the specific bug that destroys the reason
//!   to keep one.
//! - **Corrections stop being free.** The Guild corrects by reversal, never by
//!   edit: post the mirror entry and refold. If accounts carried balances, a
//!   reversal would have to reach in and mutate one — which is exactly the
//!   "fix the number" move that reversal exists to forbid.
//! - **Bitemporal history becomes unrepresentable.** There is no single
//!   balance for an account. There is a two-dimensional surface of them, one
//!   per (`occurred_at`, `recorded_at`) pair. A `balance` field can only ever
//!   answer "now, as best we currently know", which is the one question this
//!   crate is built to see past.
//!
//! There is a Rust-shaped reason too. `Account::AdventurerPayable(alice)` is a
//! *value*, constructed fresh at each call site; there is nowhere to hang
//! mutable state. Giving accounts balances would mean a registry keeping one
//! live account object per adventurer, at which point this enum would have
//! stopped being the chart of accounts and become a lookup key into the real
//! one.
//!
//! # Then where do "debit" and "credit" live?
//!
//! On [`Posting`](super::Posting), as constructors of immutable facts rather
//! than as mutators:
//!
//! ```
//! use guild_domain::ledger::{Account, Posting};
//! use guild_domain::money::Coin;
//!
//! let thorne = "bramblewick-thorne".parse()?;
//!
//! let fee = Posting::credit(Account::GuildFeeIncome, Coin::from_coppers(60_000))?;
//! let share = Posting::credit(Account::AdventurerPayable(thorne), Coin::from_coppers(340_000))?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Gathering postings like those into an entry that refuses to exist unless
//! they balance is the journal entry's job, not the account's.
//!
//! The money-tracking behaviour lives on the ledger because **the invariant is
//! global**. "Debits equal credits", and "the trial balance is zero", span
//! every account at once. No single account can enforce a rule that crosses
//! its own boundary, so the aggregate root has to be the thing that contains
//! them all — the same reason an order owns its total rather than each line
//! knowing it.
//!
//! That leaves [`Account`] with classification behaviour and nothing else,
//! which is genuinely all the behaviour a chart-of-accounts entry has.
//!
//! # Where this departs from Fowler
//!
//! Fowler's [Accounting Patterns] models `Account` as an object *holding* its
//! collection of entries, with `balance(dateRange)` on it. That is a good
//! model in mutable OO, and this module deliberately does not follow it.
//!
//! The reason is the journal entry. A settlement touching six accounts is one
//! fact. Under Fowler's shape it is stored in six places and has to be
//! reassembled before it can be reversed or audited; here it is stored once,
//! whole, on the ledger — which is what keeps reversal and bitemporal queries
//! tractable.
//!
//! [Accounting Patterns]: https://martinfowler.com/apsupp/accounting.pdf
//!
//! # Open question: what identifies an escrow?
//!
//! [`Account::ClientEscrow`] is currently keyed by an
//! [`AdventurerId`](crate::identifiers::AdventurerId), which quietly claims
//! that clients and adventurers are one population — that any client could be
//! paid a share, and any adventurer could post a bounty. That may be the
//! intended world; it has not been decided.
//!
//! The alternatives, when it is:
//!
//! - A sixth identifier, `ClientId`, if a patron is not an adventurer. Note
//!   that the [`identifiers`](crate::identifiers) module docs say the
//!   hand-written-longhand argument "does not survive being multiplied", so
//!   this reopens that decision too.
//! - [`ContractId`](crate::identifiers::ContractId), on the grounds that
//!   escrow is held per contract rather than per client — one patron with
//!   three open quests has three escrows.
//!
//! Whatever it becomes, it should stay a *single* key. The whole variant is
//! the account's identity, so a composite `ClientEscrow { client, contract }`
//! would make `{ client: A, contract: X }` and `{ client: B, contract: X }`
//! two different accounts holding one escrow's money, with nothing in the type
//! stopping a posting from naming the wrong pair.

use super::direction::Direction;
use crate::identifiers::AdventurerId;

/// What kind of thing an account is, in the accounting equation.
///
/// ```text
/// Assets = Liabilities + Equity (+ Income − Expenses)
/// ```
///
/// A debit increases the left-hand side and a credit the right; every journal
/// entry moves both by equal amounts, so the equation survives every
/// transaction. That is the invariant the whole ledger is built to make
/// unbreakable, and this enum is the classification it rests on.
///
/// Kind is the thing accounting has a theory about;
/// [`normal_side`](Account::normal_side) is a consequence of it. Keeping them
/// separate means a new account gets classified once, and its normal balance
/// follows for free.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountKind {
    /// What the Guild has. The vault, and anything owed *to* the Guild.
    Asset,
    /// What the Guild owes onward — escrowed bounties and unpaid shares.
    Liability,
    /// The Guild's own stake in itself. Nothing posts here yet.
    Equity,
    /// What the Guild has earned — its cut of a resolved bounty.
    Income,
    /// What running the Guild costs. Nothing posts here yet.
    Expense,
}

/// One bucket in the Guild's chart of accounts.
///
/// An inert key, not a container — see the [module docs](self) for why an
/// account holds no balance. Postings name an `Account`; the ledger folds
/// them to answer what that account is worth, as of whenever you ask.
///
/// Enumerating the chart means the compiler owns it. When M4 adds a
/// hazard-bonus account, every match that must reconsider becomes a build
/// failure — which only works because no match on this type carries a `_` arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Account {
    /// A bounty the Guild is holding on a client's behalf.
    ///
    /// A liability: the money is in the vault but the Guild does not own it,
    /// and owes it either onward to the party or back to the client. Credited
    /// when the quest is funded, debited when it settles or refunds.
    ///
    /// See the [module docs](self) on what should identify this account — the
    /// current key is not settled.
    ClientEscrow(AdventurerId),
    /// A share an adventurer has earned but not yet been handed.
    ///
    /// Credited at settlement, debited when the coins actually change hands.
    /// Settlement records a debt rather than paying it, which is why this is a
    /// *payable*: it is what lets the ledger answer who the Guild is still
    /// holding money for.
    AdventurerPayable(AdventurerId),
    /// A share earned by an adventurer who died and stayed dead.
    ///
    /// The work was done and the share is owed; it simply does not go to them.
    /// Split out from [`AdventurerPayable`](Account::AdventurerPayable)
    /// because the money leaves the Guild by a different route and to a
    /// different claimant — not because the amount differs. Someone who died
    /// and was raised before the quest resolved is paid normally; they are
    /// standing there.
    EstatePayable(AdventurerId),
    /// The Guild's cut of a resolved bounty.
    ///
    /// Credited at settlement for whatever the fee schedule says the Guild has
    /// earned. The only account here that the Guild keeps rather than owes.
    GuildFeeIncome,
    /// The Guild's actual coin.
    ///
    /// Debited when a client funds a quest, credited when a payable is
    /// settled in coin. Without an asset account every other variant here is
    /// credit-balanced, and funding an escrow would have no debit side to
    /// post against.
    GuildVault,
}

impl Account {
    /// Classifies the account within the accounting equation.
    ///
    /// The exhaustive match is the point: this is the one place a new account
    /// must be considered, and the absence of a `_` arm is what guarantees the
    /// compiler will say so.
    ///
    /// ```
    /// use guild_domain::ledger::{Account, AccountKind};
    ///
    /// assert!(matches!(Account::GuildVault.kind(), AccountKind::Asset));
    /// assert!(matches!(Account::GuildFeeIncome.kind(), AccountKind::Income));
    /// ```
    pub fn kind(&self) -> AccountKind {
        match self {
            Account::ClientEscrow { .. } => AccountKind::Liability,
            Account::AdventurerPayable { .. } => AccountKind::Liability,
            Account::EstatePayable { .. } => AccountKind::Liability,
            Account::GuildFeeIncome => AccountKind::Income,
            Account::GuildVault => AccountKind::Asset,
        }
    }

    /// The side this account grows on — its normal balance.
    ///
    /// Derived from [`kind`](Account::kind) rather than declared per variant,
    /// so the two cannot drift apart.
    ///
    /// It earns its keep twice: rendering a balance the right way round, and
    /// exposing an **abnormal balance** — an account sitting on the side it
    /// does not grow on. A payable in debit means the Guild paid someone more
    /// than they earned, and that is nearly always a bug rather than a fact.
    ///
    /// ```
    /// use guild_domain::ledger::{Account, Direction};
    ///
    /// assert!(matches!(Account::GuildVault.normal_side(), Direction::Debit));
    /// assert!(matches!(Account::GuildFeeIncome.normal_side(), Direction::Credit));
    /// ```
    pub fn normal_side(&self) -> Direction {
        match self.kind() {
            AccountKind::Asset | AccountKind::Expense => Direction::Debit,
            AccountKind::Liability | AccountKind::Equity | AccountKind::Income => Direction::Credit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adventurer(name: &str) -> AdventurerId {
        name.parse().expect("a well-formed id")
    }

    #[test]
    fn should_classify_what_the_guild_owes_onward_as_liabilities() {
        // The three accounts holding money the Guild does not own. An escrowed
        // bounty is the client's until it settles; a share is the adventurer's
        // the moment it is earned, whoever ends up collecting it.
        let alder = adventurer("alder-quill");

        assert_eq!(
            Account::ClientEscrow(alder.clone()).kind(),
            AccountKind::Liability
        );
        assert_eq!(
            Account::AdventurerPayable(alder.clone()).kind(),
            AccountKind::Liability
        );
        assert_eq!(Account::EstatePayable(alder).kind(), AccountKind::Liability);
    }

    #[test]
    fn should_classify_the_guilds_cut_as_income() {
        assert_eq!(Account::GuildFeeIncome.kind(), AccountKind::Income);
    }

    #[test]
    fn should_classify_the_vault_as_an_asset() {
        assert_eq!(Account::GuildVault.kind(), AccountKind::Asset);
    }

    #[test]
    fn should_grow_what_the_guild_owes_and_earns_on_the_credit_side() {
        assert_eq!(
            Account::AdventurerPayable(adventurer("mirren-vale")).normal_side(),
            Direction::Credit
        );
        assert_eq!(Account::GuildFeeIncome.normal_side(), Direction::Credit);
    }

    #[test]
    fn should_offer_a_debit_side_to_fund_an_escrow_against() {
        // The rule that earns the vault its place in the chart. Every other
        // account here is credit-normal, so without an asset the funding entry
        // — debit the vault, credit the escrow — has nothing to debit, and the
        // escrow spends its life in an abnormal balance.
        assert_eq!(Account::GuildVault.normal_side(), Direction::Debit);
    }

    #[test]
    fn should_hold_a_separate_account_per_adventurer() {
        // What makes a per-adventurer balance meaningful. Were these to compare
        // equal, every share in the party would fold into one payable and the
        // ledger could not say who is owed what.
        let mirren = adventurer("mirren-vale");
        let osric = adventurer("osric-penn");

        assert_ne!(
            Account::AdventurerPayable(mirren),
            Account::AdventurerPayable(osric)
        );
    }

    #[test]
    fn should_keep_an_adventurers_payable_apart_from_their_estates() {
        // The distinction estate payouts rest on: Alder's share is owed either
        // to Alder or to Alder's estate, and which one is a fact about the
        // account, not about the amount.
        let alder = adventurer("alder-quill");

        assert_ne!(
            Account::AdventurerPayable(alder.clone()),
            Account::EstatePayable(alder)
        );
    }
}
