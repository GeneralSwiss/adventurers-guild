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

    /// Rejects candidates that could not name a party.
    ///
    /// Kept private and shared by [`FromStr`] and [`TryFrom<String>`] so the
    /// rule lives in exactly one place, and so no caller can construct a
    /// [`PartyId`] without passing through it.
    fn validate(candidate: &str) -> Result<(), InvalidPartyId> {
        if candidate.is_empty() {
            return Err(InvalidPartyId::Empty);
        }
        match candidate
            .chars()
            .find(|c| c.is_whitespace() || c.is_control())
        {
            Some(found) => Err(InvalidPartyId::IllegalCharacter {
                candidate: candidate.to_owned(),
                found,
            }),
            None => Ok(()),
        }
    }
}

impl Display for PartyId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl FromStr for PartyId {
    type Err = InvalidPartyId;

    fn from_str(candidate: &str) -> Result<Self, Self::Err> {
        Self::validate(candidate)?;
        Ok(Self(candidate.to_owned()))
    }
}

impl TryFrom<String> for PartyId {
    type Error = InvalidPartyId;

    /// Takes ownership of an already-owned candidate rather than copying it,
    /// which [`FromStr`] cannot do.
    fn try_from(candidate: String) -> Result<Self, Self::Error> {
        Self::validate(&candidate)?;
        Ok(Self(candidate))
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
    fn should_take_ownership_of_a_valid_owned_candidate() {
        let id = PartyId::try_from("party-1".to_owned()).expect("a well-formed id");

        assert_eq!(id.as_str(), "party-1");
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
