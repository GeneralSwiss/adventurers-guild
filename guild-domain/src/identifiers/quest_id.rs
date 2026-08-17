//! The identifier of a quest posted on the Guild board.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

/// Identifies a quest posted on the Guild board.
///
/// Constructed only by parsing, so holding one is proof that the text inside
/// is well formed: non-empty, and free of whitespace and control characters.
///
/// Not interchangeable with any other identifier in this module — see the
/// [module documentation](crate::identifiers) for the compile-time proof.
///
/// ```
/// use guild_domain::identifiers::QuestId;
///
/// let id: QuestId = "quest-1".parse()?;
/// assert_eq!(id.to_string(), "quest-1");
/// # Ok::<(), guild_domain::identifiers::InvalidQuestId>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QuestId(String);

impl QuestId {
    /// Borrows the identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for QuestId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl TryFrom<String> for QuestId {
    type Error = InvalidQuestId;

    /// Parses an owned candidate, moving its buffer into the result: into the
    /// [`QuestId`] when the candidate is well formed, into the error when it is
    /// not. Neither path copies.
    ///
    /// This is the only place a [`QuestId`] is constructed, so it is the
    /// only place the rule can be enforced or bypassed. [`FromStr`] delegates
    /// here rather than the reverse, because a `&str` must be copied before it
    /// can be owned — the borrowing impl is the one that can be written in
    /// terms of the owning impl without waste.
    ///
    /// `should_reuse_the_callers_buffer` holds the no-copy claim to account.
    fn try_from(candidate: String) -> Result<Self, Self::Error> {
        if candidate.is_empty() {
            return Err(InvalidQuestId::Empty);
        }
        match candidate
            .chars()
            .find(|c| c.is_whitespace() || c.is_control())
        {
            Some(found) => Err(InvalidQuestId::IllegalCharacter { candidate, found }),
            None => Ok(Self(candidate)),
        }
    }
}

impl FromStr for QuestId {
    type Err = InvalidQuestId;

    /// Takes the copy a borrowed candidate requires, then defers to
    /// [`TryFrom<String>`] for the rule.
    fn from_str(candidate: &str) -> Result<Self, Self::Err> {
        Self::try_from(candidate.to_owned())
    }
}

/// The ways a candidate can fail to be a [`QuestId`].
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum InvalidQuestId {
    /// The candidate held no characters at all.
    #[error("a quest id must not be empty")]
    Empty,
    /// The candidate held a character an identifier may not carry.
    #[error(
        "a quest id must not contain whitespace or control characters, but {candidate:?} contains {found:?}"
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
        let id = QuestId::from_str("quest-1").expect("a well-formed id");

        assert_eq!(id.to_string(), "quest-1");
    }

    #[test]
    fn should_borrow_the_candidate_as_a_string_slice() {
        let id = QuestId::from_str("quest-1").expect("a well-formed id");

        assert_eq!(id.as_str(), "quest-1");
    }

    #[test]
    fn should_reject_an_empty_candidate() {
        assert_eq!(QuestId::from_str(""), Err(InvalidQuestId::Empty));
    }

    #[test]
    fn should_reject_a_candidate_containing_whitespace() {
        assert_eq!(
            QuestId::from_str("two words"),
            Err(InvalidQuestId::IllegalCharacter {
                candidate: "two words".to_owned(),
                found: ' ',
            })
        );
    }

    #[test]
    fn should_reject_a_candidate_containing_a_control_character() {
        assert_eq!(
            QuestId::from_str("line\nbreak"),
            Err(InvalidQuestId::IllegalCharacter {
                candidate: "line\nbreak".to_owned(),
                found: '\n',
            })
        );
    }

    #[test]
    fn should_parse_an_owned_candidate() {
        let id = QuestId::try_from("quest-1".to_owned()).expect("a well-formed id");

        assert_eq!(id.as_str(), "quest-1");
    }

    #[test]
    fn should_reuse_the_callers_buffer() {
        // Asserts on the heap address on purpose. Not copying is the only
        // reason `TryFrom<String>` earns its place beside `from_str`, so a
        // later delegation to `from_str` has to fail here rather than pass
        // quietly: `from_str` would allocate a fresh buffer for the copy.
        let owned = String::from("quest-1");
        let buffer = owned.as_ptr();

        let id = QuestId::try_from(owned).expect("a well-formed id");

        assert_eq!(id.as_str().as_ptr(), buffer);
    }

    #[test]
    fn should_treat_identical_text_as_the_same_identifier() {
        let first = QuestId::from_str("quest-1").expect("a well-formed id");
        let second = QuestId::from_str("quest-1").expect("a well-formed id");
        let mut seen: HashSet<QuestId> = HashSet::new();

        assert!(seen.insert(first));
        assert!(!seen.insert(second));
    }

    #[test]
    fn should_order_identifiers_lexicographically() {
        let first = QuestId::from_str("quest-1").expect("a well-formed id");
        let second = QuestId::from_str("quest-2").expect("a well-formed id");

        assert!(first < second);
    }
}
