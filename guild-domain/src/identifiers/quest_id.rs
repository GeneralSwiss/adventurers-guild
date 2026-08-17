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

    /// Rejects candidates that could not name a quest.
    ///
    /// Kept private and shared by [`FromStr`] and [`TryFrom<String>`] so the
    /// rule lives in exactly one place, and so no caller can construct a
    /// [`QuestId`] without passing through it.
    fn validate(candidate: &str) -> Result<(), InvalidQuestId> {
        if candidate.is_empty() {
            return Err(InvalidQuestId::Empty);
        }
        match candidate
            .chars()
            .find(|c| c.is_whitespace() || c.is_control())
        {
            Some(found) => Err(InvalidQuestId::IllegalCharacter {
                candidate: candidate.to_owned(),
                found,
            }),
            None => Ok(()),
        }
    }
}

impl Display for QuestId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl FromStr for QuestId {
    type Err = InvalidQuestId;

    fn from_str(candidate: &str) -> Result<Self, Self::Err> {
        Self::validate(candidate)?;
        Ok(Self(candidate.to_owned()))
    }
}

impl TryFrom<String> for QuestId {
    type Error = InvalidQuestId;

    /// Takes ownership of an already-owned candidate rather than copying it,
    /// which [`FromStr`] cannot do.
    fn try_from(candidate: String) -> Result<Self, Self::Error> {
        Self::validate(&candidate)?;
        Ok(Self(candidate))
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
    fn should_take_ownership_of_a_valid_owned_candidate() {
        let id = QuestId::try_from("quest-1".to_owned()).expect("a well-formed id");

        assert_eq!(id.as_str(), "quest-1");
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
