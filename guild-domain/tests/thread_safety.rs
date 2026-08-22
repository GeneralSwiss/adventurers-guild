//! Every published type crosses a thread boundary freely.
//!
//! Nothing in this crate is concurrent, and nothing in it should be. The claim
//! here is the weaker, more useful one: the domain does not *obstruct* the
//! concurrency happening around it. An application layer can hold a [`Coin`]
//! in shared state, hand an [`Account`] to a worker, or park a [`Shares`] in a
//! request future without the domain having an opinion about it.
//!
//! # Why this is a test and not a comment
//!
//! `Send` and `Sync` are auto traits: they are inferred from a type's fields,
//! never written down. That makes them invisible, and invisible guarantees are
//! the ones that get revoked by accident. One `Rc` reached for in a moment of
//! convenience — the obvious way to let a `Party` and a `Quest` name the same
//! adventurer — silently strips `Send` from that type and from every type that
//! goes on to contain it.
//!
//! Being auto traits also makes them part of the public API. Losing one is a
//! breaking change under semver, and the breakage does not surface here: it
//! surfaces in a downstream crate, at a `thread::spawn` or an `.await`, in an
//! error message that names none of the code that caused it. This file moves
//! that discovery to the commit that causes it.
//!
//! # Why `'static` as well
//!
//! `std::thread::spawn` demands `Send + 'static`, so `'static` is half of the
//! property being claimed rather than a separate one. It also pins something
//! true of this crate today and worth keeping: the domain holds owned values,
//! not borrows. A lifetime parameter appearing on a published type would be a
//! design change, and this is the place to notice it.
//!
//! The error types earn the bound twice over. `Box<dyn Error + Send + Sync>`
//! and `anyhow::Error` both require it, so an application that propagates a
//! [`MoneyError`] out of a handler with `?` needs it to hold — the domain
//! would otherwise dictate its callers' error strategy from a distance.
//!
//! # Why the list is written out longhand
//!
//! A `macro_rules!` would spare the repetition, and was passed over for the
//! same reason the identifiers were, set out in that module's docs: the list
//! is read far more often than it is edited, it greps, and a failure points at
//! a real line rather than into an expansion. The cost is that a new type goes
//! uncovered until someone adds it here, and nothing but review catches the
//! omission — the same trade, and the same exposure, accepted knowingly.

use guild_domain::identifiers::{
    AdventurerId, ContractId, EntryId, InvalidAdventurerId, InvalidContractId, InvalidEntryId,
    InvalidPartyId, InvalidQuestId, PartyId, QuestId,
};
use guild_domain::ledger::{
    Account, AccountKind, Direction, InvalidNarrative, JournalEntry, LedgerError, Narrative,
    Posting, PostingError,
};
use guild_domain::money::{Coin, InvalidShare, MoneyError, Share, Shares};
use guild_domain::time::{Duration, TimeError, WorldInstant};

/// Compiles only for a `T` that can be moved to another thread, shared with
/// one by reference, and outlive the scope it was created in.
///
/// The body is empty on purpose. Every assertion in this file is discharged by
/// the compiler at the call site; there is nothing left to check at runtime,
/// and a passing run proves only that the file built.
fn assert_send_sync<T: Send + Sync + 'static>() {}

#[test]
fn every_identifier_crosses_thread_boundaries() {
    assert_send_sync::<AdventurerId>();
    assert_send_sync::<ContractId>();
    assert_send_sync::<EntryId>();
    assert_send_sync::<PartyId>();
    assert_send_sync::<QuestId>();

    assert_send_sync::<InvalidAdventurerId>();
    assert_send_sync::<InvalidContractId>();
    assert_send_sync::<InvalidEntryId>();
    assert_send_sync::<InvalidPartyId>();
    assert_send_sync::<InvalidQuestId>();
}

#[test]
fn every_money_type_crosses_thread_boundaries() {
    assert_send_sync::<Coin>();
    assert_send_sync::<Share>();
    assert_send_sync::<Shares>();

    assert_send_sync::<MoneyError>();
    assert_send_sync::<InvalidShare>();
}

#[test]
fn every_time_type_crosses_thread_boundaries() {
    assert_send_sync::<Duration>();
    assert_send_sync::<WorldInstant>();

    assert_send_sync::<TimeError>();
}

#[test]
fn every_ledger_type_crosses_thread_boundaries() {
    assert_send_sync::<Account>();
    assert_send_sync::<AccountKind>();
    assert_send_sync::<Direction>();
    assert_send_sync::<Posting>();
    assert_send_sync::<Narrative>();
    assert_send_sync::<JournalEntry>();

    assert_send_sync::<PostingError>();
    assert_send_sync::<InvalidNarrative>();
    assert_send_sync::<LedgerError>();
}
