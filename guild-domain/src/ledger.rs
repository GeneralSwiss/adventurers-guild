//! Double-entry bookkeeping: the accounts money is posted against, and the
//! postings that move it.
//!
//! | Module        | Holds                                                     |
//! |---------------|-----------------------------------------------------------|
//! | [`account`]   | [`Account`], the chart of accounts, and [`AccountKind`]   |
//! | [`direction`] | [`Direction`], which side of an entry a posting falls on  |
//!
//! Quest, Party, and Escrow are the *story* — who agreed to what, who served
//! which hours, who died. This module is the *record*: where every copper is,
//! and how it got there. The two do not know about each other, and `settle()`
//! is the only bridge between them.
//!
//! # The shape of the thing
//!
//! A posting moves one [`Account`] by one [`Coin`](crate::money::Coin) in one
//! [`Direction`]. A journal entry is a set of postings whose debits equal its
//! credits. A ledger is an append-only sequence of entries, and a balance is a
//! fold over the postings naming a given account.
//!
//! Only the accounts exist so far. The posting, the entry, and the ledger
//! arrive with M1's remaining issues, and the invariant they carry — that
//! debits equal credits, always — is what the rest of the crate leans on.
//!
//! Every type is re-exported here, so `ledger::Account` is the path to prefer
//! over `ledger::account::Account`.

pub mod account;
pub mod direction;

pub use account::{Account, AccountKind};
pub use direction::Direction;
