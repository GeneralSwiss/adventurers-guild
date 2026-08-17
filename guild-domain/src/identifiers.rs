//! Typed identifiers for the entities the domain names.
//!
//! Every identifier is its own type. There is no shared `Id` type and no
//! type alias, so the compiler refuses the swap that a `String` identifier
//! would wave through:
//!
//! ```compile_fail
//! use guild_domain::identifiers::{PartyId, QuestId};
//!
//! fn muster(_: PartyId) {}
//!
//! let quest: QuestId = "quest-1".parse().unwrap();
//! muster(quest); // error[E0308]: expected `PartyId`, found `QuestId`
//! ```
//!
//! The same call with the right type compiles, which is what keeps the
//! example above honest — were the two types to become interchangeable, the
//! failing example would start compiling and `cargo test` would report it:
//!
//! ```
//! use guild_domain::identifiers::PartyId;
//!
//! fn muster(_: PartyId) {}
//!
//! let party: PartyId = "party-1".parse().unwrap();
//! muster(party);
//! ```
//!
//! # Shape
//!
//! Each identifier wraps a `String` behind a private field, and is built by
//! `TryFrom<String>` — the sole constructor, which rejects the empty candidate
//! and any candidate carrying whitespace or control characters. Parsing at the
//! boundary means the rest of the domain never re-checks: holding an
//! [`AdventurerId`] *is* the proof that it is well formed.
//!
//! [`FromStr`](std::str::FromStr) delegates to it, so `"quest-1".parse()` works
//! too. The delegation runs that way around because a `&str` has to be copied
//! before it can be owned: the borrowing impl can be written in terms of the
//! owning one for the price of the copy it already owed, whereas the reverse
//! would force an owned candidate through a needless second allocation. One
//! constructor, and neither path copies more than it must.
//!
//! They are deliberately not `Copy`. A `String` cannot be, and the readable
//! identifier was judged worth the `.clone()` at call sites — `quest-1` in a
//! ledger dump or a failing assertion carries meaning that an opaque integer
//! does not. Should the clones become a burden, the private field means the
//! representation can change without touching a single call site.
//!
//! # Why these are hand-written
//!
//! Issue #2 weighed a `macro_rules! id_type` and a generic
//! `Id<T>(u128, PhantomData<T>)` against each other. Both were rejected in
//! favour of writing all five out longhand.
//!
//! The generic loses on ergonomics: `PhantomData` leaks into every signature
//! that names an identifier, and `derive` would impose a spurious `T: Clone`
//! bound that has to be undone with hand-written impls anyway.
//!
//! The macro is the closer call, and it wins on DRY — the shape lives once,
//! and these five files repeat it. It was passed over because identifiers are
//! the most-read types in the crate and the ones every other module builds on
//! top of: `cargo doc` renders a written-out impl and hides a generated one,
//! errors point at real lines, and a reader can see the whole of `QuestId`
//! without expanding a macro in their head. The cost is real and is accepted
//! knowingly — a change to the shape is a five-file edit, and nothing but
//! review stops the five from drifting apart.
//!
//! If a sixth and seventh identifier arrive, revisit this: the argument is
//! finely balanced at five and does not survive being multiplied.

mod adventurer_id;
mod contract_id;
mod entry_id;
mod party_id;
mod quest_id;

pub use adventurer_id::{AdventurerId, InvalidAdventurerId};
pub use contract_id::{ContractId, InvalidContractId};
pub use entry_id::{EntryId, InvalidEntryId};
pub use party_id::{InvalidPartyId, PartyId};
pub use quest_id::{InvalidQuestId, QuestId};

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn every_identifier_round_trips_through_display() {
        let adventurer = AdventurerId::from_str("adventurer-1").expect("a well-formed id");
        let quest = QuestId::from_str("quest-1").expect("a well-formed id");
        let party = PartyId::from_str("party-1").expect("a well-formed id");
        let contract = ContractId::from_str("contract-1").expect("a well-formed id");
        let entry = EntryId::from_str("entry-1").expect("a well-formed id");

        assert_eq!(adventurer.to_string(), "adventurer-1");
        assert_eq!(quest.to_string(), "quest-1");
        assert_eq!(party.to_string(), "party-1");
        assert_eq!(contract.to_string(), "contract-1");
        assert_eq!(entry.to_string(), "entry-1");
    }

    #[test]
    fn identical_text_under_different_identifiers_are_distinct_values() {
        // Nothing stops the same text naming both a quest and a party. The
        // types are what keep the two apart, so this is a type-level claim
        // the test can only gesture at — the compile_fail example in the
        // module docs is what actually enforces it.
        let quest = QuestId::from_str("shared-text").expect("a well-formed id");
        let party = PartyId::from_str("shared-text").expect("a well-formed id");

        assert_eq!(quest.as_str(), party.as_str());
    }
}
