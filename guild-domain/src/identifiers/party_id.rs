//! The identifier of a party mustered to attempt a quest.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

/// Identifies a party mustered to attempt a quest.
///
/// Constructed only by parsing, so holding one is proof that the text inside
/// is well formed: non-empty, and free of whitespace and control characters.
///
/// Not interchangeable with any other identifier in this module — see the
/// [module documentation](crate::identifiers) for the compile-time proof.
///
/// ```
/// use guild_domain::identifiers::PartyId;
///
/// let id: PartyId = "party-1".parse()?;
/// assert_eq!(id.to_string(), "party-1");
/// # Ok::<(), guild_domain::identifiers::InvalidPartyId>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartyId(String);

impl PartyId {
    /// Borrows the identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for PartyId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl TryFrom<String> for PartyId {
    type Error = InvalidPartyId;

    /// Parses an owned candidate, moving its buffer into the result: into the
    /// [`PartyId`] when the candidate is well formed, into the error when it is
    /// not. Neither path copies.
    ///
    /// This is the only place a [`PartyId`] is constructed, so it is the
    /// only place the rule can be enforced or bypassed. [`FromStr`] delegates
    /// here rather than the reverse, because a `&str` must be copied before it
    /// can be owned — the borrowing impl is the one that can be written in
    /// terms of the owning impl without waste.
    ///
    /// `should_reuse_the_callers_buffer` holds the no-copy claim to account.
    fn try_from(candidate: String) -> Result<Self, Self::Error> {
        if candidate.is_empty() {
            return Err(InvalidPartyId::Empty);
        }
        match candidate
            .chars()
            .find(|c| c.is_whitespace() || c.is_control())
        {
            Some(found) => Err(InvalidPartyId::IllegalCharacter { candidate, found }),
            None => Ok(Self(candidate)),
        }
    }
}

impl FromStr for PartyId {
    type Err = InvalidPartyId;

    /// Takes the copy a borrowed candidate requires, then defers to
    /// [`TryFrom<String>`] for the rule.
    fn from_str(candidate: &str) -> Result<Self, Self::Err> {
        Self::try_from(candidate.to_owned())
    }
}

/// The ways a candidate can fail to be a [`PartyId`].
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum InvalidPartyId {
    /// The candidate held no characters at all.
    #[error("a party id must not be empty")]
    Empty,
    /// The candidate held a character an identifier may not carry.
    #[error(
        "a party id must not contain whitespace or control characters, but {candidate:?} contains {found:?}"
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
        let id = PartyId::from_str("party-1").expect("a well-formed id");

        assert_eq!(id.to_string(), "party-1");
    }

    #[test]
    fn should_borrow_the_candidate_as_a_string_slice() {
        let id = PartyId::from_str("party-1").expect("a well-formed id");

        assert_eq!(id.as_str(), "party-1");
    }

    #[test]
    fn should_reject_an_empty_candidate() {
        assert_eq!(PartyId::from_str(""), Err(InvalidPartyId::Empty));
    }

    #[test]
    fn should_reject_a_candidate_containing_whitespace() {
        assert_eq!(
            PartyId::from_str("two words"),
            Err(InvalidPartyId::IllegalCharacter {
                candidate: "two words".to_owned(),
                found: ' ',
            })
        );
    }

    #[test]
    fn should_reject_a_candidate_containing_a_control_character() {
        assert_eq!(
            PartyId::from_str("line\nbreak"),
            Err(InvalidPartyId::IllegalCharacter {
                candidate: "line\nbreak".to_owned(),
                found: '\n',
            })
        );
    }

    #[test]
    fn should_parse_an_owned_candidate() {
        let id = PartyId::try_from("party-1".to_owned()).expect("a well-formed id");

        assert_eq!(id.as_str(), "party-1");
    }

    #[test]
    fn should_reuse_the_callers_buffer() {
        // Asserts on the heap address on purpose. Not copying is the only
        // reason `TryFrom<String>` earns its place beside `from_str`, so a
        // later delegation to `from_str` has to fail here rather than pass
        // quietly: `from_str` would allocate a fresh buffer for the copy.
        let owned = String::from("party-1");
        let buffer = owned.as_ptr();

        let id = PartyId::try_from(owned).expect("a well-formed id");

        assert_eq!(id.as_str().as_ptr(), buffer);
    }

    #[test]
    fn should_treat_identical_text_as_the_same_identifier() {
        let first = PartyId::from_str("party-1").expect("a well-formed id");
        let second = PartyId::from_str("party-1").expect("a well-formed id");
        let mut seen: HashSet<PartyId> = HashSet::new();

        assert!(seen.insert(first));
        assert!(!seen.insert(second));
    }

    #[test]
    fn should_order_identifiers_lexicographically() {
        let first = PartyId::from_str("party-1").expect("a well-formed id");
        let second = PartyId::from_str("party-2").expect("a well-formed id");

        assert!(first < second);
    }
}
