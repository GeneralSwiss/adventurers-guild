//! Double-entry bookkeeping: the accounts money is posted against, and the
//! postings that move it.
//!
//! | Module            | Holds                                                    |
//! |-------------------|----------------------------------------------------------|
//! | [`account`]       | [`Account`], the chart of accounts, and [`AccountKind`]  |
//! | [`direction`]     | [`Direction`], which side of an entry a posting falls on |
//! | [`posting`]       | [`Posting`], one account moved by one amount, one way    |
//! | [`narrative`]     | [`Narrative`], what an entry says it was for             |
//! | [`journal_entry`] | [`JournalEntry`], a set of postings that must balance    |
//! | [`balance`]       | [`Balance`], what an account is worth and which way      |
//! | [`journal`]       | [`Ledger`], the append-only record of every entry        |
//!
//! Quest, Party, and Escrow are the *story* — who agreed to what, who served
//! which hours, who died. This module is the *record*: where every copper is,
//! and how it got there. The two do not know about each other, and `settle()`
//! is the only bridge between them.
//!
//! # The shape of the thing
//!
//! A [`Posting`] moves one [`Account`] by one [`Coin`](crate::money::Coin) in
//! one [`Direction`]. A [`JournalEntry`] is a set of postings whose debits
//! equal its credits. A ledger is an append-only sequence of entries, and a
//! balance is a fold over the postings naming a given account.
//!
//! The [`Ledger`] inherits the invariant rather than enforcing it: because
//! [`JournalEntry::new`] is the only way to build an entry and it refuses
//! anything unbalanced, no sequence of entries can add up to books that do not
//! balance.
//!
//! # Nothing is ever unwritten
//!
//! The journal only grows. A mistake is corrected by
//! [`Ledger::reverse`](journal::Ledger::reverse), which appends an entry
//! mirroring the mistaken one — so the accounts return to where they were,
//! and both entries stay readable forever. That is what makes the bitemporal
//! questions in M1 answerable at all: "what did the books say last Tuesday"
//! has no answer over a store that can be edited.
//!
//! It is also why entries come in two kinds. A [`ReversalEntry`] has no
//! `reverse` method, so undoing an undoing is not a rule anyone has to
//! enforce.
//!
//! Every type is re-exported here, so `ledger::Account` is the path to prefer
//! over `ledger::account::Account`.

pub mod account;
pub mod balance;
pub mod direction;
pub mod journal;
pub mod journal_entry;
pub mod narrative;
pub mod posting;

pub use account::{Account, AccountKind};
pub use balance::Balance;
pub use direction::Direction;
pub use journal::Ledger;
pub use journal_entry::{JournalEntry, LedgerError, NormalEntry, ReversalEntry};
pub use narrative::{InvalidNarrative, Narrative};
pub use posting::{Posting, PostingError};
