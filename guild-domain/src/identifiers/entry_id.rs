//! The identifier of a single entry in the Guild's journal.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

/// Identifies a single entry in the Guild's journal.
///
/// Holding one is proof that the text inside is well formed: non-empty, and
/// free of whitespace and control characters. Text from outside gets there by
/// parsing; the ledger mints its own with
/// [`sequential`](EntryId::sequential).
///
/// Not interchangeable with any other identifier in this module — see the
/// [module documentation](crate::identifiers) for the compile-time proof.
///
/// ```
/// use guild_domain::identifiers::EntryId;
///
/// let id: EntryId = "entry-1".parse()?;
/// assert_eq!(id.to_string(), "entry-1");
/// # Ok::<(), guild_domain::identifiers::InvalidEntryId>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryId(String);

impl EntryId {
    /// The identifier of the `ordinal`-th entry in a journal.
    ///
    /// The ledger mints these as it appends, so the id of an entry is its
    /// position in the journal — which is what makes a journal readable back
    /// in the order it was written.
    ///
    /// Infallible, unlike [`TryFrom<String>`], and deliberately so. That impl
    /// is fallible because most strings are not valid identifiers; the text
    /// this one builds is well formed by construction, and there is no failure
    /// to report. A `Result` whose error can never be produced forces every
    /// call site to handle a case that does not exist — the same argument the
    /// [`money`](crate::money) module makes about `Coin::from_coppers`.
    ///
    /// `TryFrom` remains the only door *untrusted text* comes through.
    ///
    /// ```
    /// use guild_domain::identifiers::EntryId;
    ///
    /// assert_eq!(EntryId::sequential(1).to_string(), "entry-1");
    /// ```
    #[must_use]
    pub fn sequential(ordinal: u64) -> Self {
        Self(format!("entry-{ordinal}"))
    }

    /// Borrows the identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for EntryId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl TryFrom<String> for EntryId {
    type Error = InvalidEntryId;

    /// Parses an owned candidate, moving its buffer into the result: into the
    /// [`EntryId`] when the candidate is well formed, into the error when it is
    /// not. Neither path copies.
    ///
    /// This is the only place *text from outside* becomes an [`EntryId`], so
    /// it is the only place the rule can be enforced or bypassed.
    /// [`sequential`](EntryId::sequential) is the other constructor, and it
    /// needs no rule because it builds its own text. [`FromStr`] delegates
    /// here rather than the reverse, because a `&str` must be copied before it
    /// can be owned — the borrowing impl is the one that can be written in
    /// terms of the owning impl without waste.
    ///
    /// `should_reuse_the_callers_buffer` holds the no-copy claim to account.
    fn try_from(candidate: String) -> Result<Self, Self::Error> {
        if candidate.is_empty() {
            return Err(InvalidEntryId::Empty);
        }
        match candidate
            .chars()
            .find(|c| c.is_whitespace() || c.is_control())
        {
            Some(found) => Err(InvalidEntryId::IllegalCharacter { candidate, found }),
            None => Ok(Self(candidate)),
        }
    }
}

impl FromStr for EntryId {
    type Err = InvalidEntryId;

    /// Takes the copy a borrowed candidate requires, then defers to
    /// [`TryFrom<String>`] for the rule.
    fn from_str(candidate: &str) -> Result<Self, Self::Err> {
        Self::try_from(candidate.to_owned())
    }
}

/// The ways a candidate can fail to be an [`EntryId`].
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum InvalidEntryId {
    /// The candidate held no characters at all.
    #[error("an entry id must not be empty")]
    Empty,
    /// The candidate held a character an identifier may not carry.
    #[error(
        "an entry id must not contain whitespace or control characters, but {candidate:?} contains {found:?}"
    )]
    IllegalCharacter {
        /// The rejected candidate, echoed back to locate the offending input.
        candidate: String,
        /// The first character that failed the rule.
        found: char,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn should_display_a_well_formed_candidate_unchanged() {
        let id = EntryId::from_str("entry-1").expect("a well-formed id");

        assert_eq!(id.to_string(), "entry-1");
    }

    #[test]
    fn should_borrow_the_candidate_as_a_string_slice() {
        let id = EntryId::from_str("entry-1").expect("a well-formed id");

        assert_eq!(id.as_str(), "entry-1");
    }

    #[test]
    fn should_reject_an_empty_candidate() {
        assert_eq!(EntryId::from_str(""), Err(InvalidEntryId::Empty));
    }

    #[test]
    fn should_reject_a_candidate_containing_whitespace() {
        assert_eq!(
            EntryId::from_str("two words"),
            Err(InvalidEntryId::IllegalCharacter {
                candidate: "two words".to_owned(),
                found: ' ',
            })
        );
    }

    #[test]
    fn should_reject_a_candidate_containing_a_control_character() {
        assert_eq!(
            EntryId::from_str("line\nbreak"),
            Err(InvalidEntryId::IllegalCharacter {
                candidate: "line\nbreak".to_owned(),
                found: '\n',
            })
        );
    }

    #[test]
    fn should_parse_an_owned_candidate() {
        let id = EntryId::try_from("entry-1".to_owned()).expect("a well-formed id");

        assert_eq!(id.as_str(), "entry-1");
    }

    #[test]
    fn should_reuse_the_callers_buffer() {
        // Asserts on the heap address on purpose. Not copying is the only
        // reason `TryFrom<String>` earns its place beside `from_str`, so a
        // later delegation to `from_str` has to fail here rather than pass
        // quietly: `from_str` would allocate a fresh buffer for the copy.
        let owned = String::from("entry-1");
        let buffer = owned.as_ptr();

        let id = EntryId::try_from(owned).expect("a well-formed id");

        assert_eq!(id.as_str().as_ptr(), buffer);
    }

    #[test]
    fn should_mint_a_sequential_id_from_its_position() {
        assert_eq!(EntryId::sequential(7).as_str(), "entry-7");
    }

    #[test]
    fn should_mint_distinct_ids_for_distinct_positions() {
        assert_ne!(EntryId::sequential(1), EntryId::sequential(2));
    }

    #[test]
    fn should_mint_ids_a_parser_would_also_accept() {
        // The point of the infallible constructor is that it cannot produce
        // text the fallible one would reject. If that ever stops holding, an
        // id minted by the ledger would fail to round-trip through storage.
        let minted = EntryId::sequential(42);

        assert_eq!(EntryId::from_str(minted.as_str()), Ok(minted));
    }

    #[test]
    fn should_treat_identical_text_as_the_same_identifier() {
        let first = EntryId::from_str("entry-1").expect("a well-formed id");
        let second = EntryId::from_str("entry-1").expect("a well-formed id");
        let mut seen: HashSet<EntryId> = HashSet::new();

        assert!(seen.insert(first));
        assert!(!seen.insert(second));
    }

    #[test]
    fn should_order_identifiers_lexicographically() {
        let first = EntryId::from_str("entry-1").expect("a well-formed id");
        let second = EntryId::from_str("entry-2").expect("a well-formed id");

        assert!(first < second);
    }
}
