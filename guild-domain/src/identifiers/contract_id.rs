//! The identifier of a contract binding a party to a quest and its bounty.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

/// Identifies a contract binding a party to a quest and its bounty.
///
/// Constructed only by parsing, so holding one is proof that the text inside
/// is well formed: non-empty, and free of whitespace and control characters.
///
/// Not interchangeable with any other identifier in this module — see the
/// [module documentation](crate::identifiers) for the compile-time proof.
///
/// ```
/// use guild_domain::identifiers::ContractId;
///
/// let id: ContractId = "contract-1".parse()?;
/// assert_eq!(id.to_string(), "contract-1");
/// # Ok::<(), guild_domain::identifiers::InvalidContractId>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContractId(String);

impl ContractId {
    /// Borrows the identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Rejects candidates that could not name a contract.
    ///
    /// Kept private and shared by [`FromStr`] and [`TryFrom<String>`] so the
    /// rule lives in exactly one place, and so no caller can construct a
    /// [`ContractId`] without passing through it.
    fn validate(candidate: &str) -> Result<(), InvalidContractId> {
        if candidate.is_empty() {
            return Err(InvalidContractId::Empty);
        }
        match candidate
            .chars()
            .find(|c| c.is_whitespace() || c.is_control())
        {
            Some(found) => Err(InvalidContractId::IllegalCharacter {
                candidate: candidate.to_owned(),
                found,
            }),
            None => Ok(()),
        }
    }
}

impl Display for ContractId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl FromStr for ContractId {
    type Err = InvalidContractId;

    fn from_str(candidate: &str) -> Result<Self, Self::Err> {
        Self::validate(candidate)?;
        Ok(Self(candidate.to_owned()))
    }
}

impl TryFrom<String> for ContractId {
    type Error = InvalidContractId;

    /// Moves the caller's buffer in rather than copying it.
    ///
    /// Deliberately does not delegate to [`FromStr`] or `candidate.parse()`.
    /// `from_str` takes a `&str` and so must `to_owned` it; routing through it
    /// would allocate a second buffer and drop the one the caller already
    /// owns. Skipping that copy is the *only* thing this impl offers over
    /// `candidate.parse()` — were the copy acceptable, the right move would be
    /// to delete this impl, not to delegate from it.
    ///
    /// `should_reuse_the_callers_buffer` holds that claim to account.
    fn try_from(candidate: String) -> Result<Self, Self::Error> {
        Self::validate(&candidate)?;
        Ok(Self(candidate))
    }
}

/// The ways a candidate can fail to be a [`ContractId`].
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum InvalidContractId {
    /// The candidate held no characters at all.
    #[error("a contract id must not be empty")]
    Empty,
    /// The candidate held a character an identifier may not carry.
    #[error(
        "a contract id must not contain whitespace or control characters, but {candidate:?} contains {found:?}"
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
        let id = ContractId::from_str("contract-1").expect("a well-formed id");

        assert_eq!(id.to_string(), "contract-1");
    }

    #[test]
    fn should_borrow_the_candidate_as_a_string_slice() {
        let id = ContractId::from_str("contract-1").expect("a well-formed id");

        assert_eq!(id.as_str(), "contract-1");
    }

    #[test]
    fn should_reject_an_empty_candidate() {
        assert_eq!(ContractId::from_str(""), Err(InvalidContractId::Empty));
    }

    #[test]
    fn should_reject_a_candidate_containing_whitespace() {
        assert_eq!(
            ContractId::from_str("two words"),
            Err(InvalidContractId::IllegalCharacter {
                candidate: "two words".to_owned(),
                found: ' ',
            })
        );
    }

    #[test]
    fn should_reject_a_candidate_containing_a_control_character() {
        assert_eq!(
            ContractId::from_str("line\nbreak"),
            Err(InvalidContractId::IllegalCharacter {
                candidate: "line\nbreak".to_owned(),
                found: '\n',
            })
        );
    }

    #[test]
    fn should_parse_an_owned_candidate() {
        let id = ContractId::try_from("contract-1".to_owned()).expect("a well-formed id");

        assert_eq!(id.as_str(), "contract-1");
    }

    #[test]
    fn should_reuse_the_callers_buffer() {
        // Asserts on the heap address on purpose. Not copying is the only
        // reason `TryFrom<String>` earns its place beside `from_str`, so a
        // later delegation to `from_str` has to fail here rather than pass
        // quietly: `from_str` would allocate a fresh buffer for the copy.
        let owned = String::from("contract-1");
        let buffer = owned.as_ptr();

        let id = ContractId::try_from(owned).expect("a well-formed id");

        assert_eq!(id.as_str().as_ptr(), buffer);
    }

    #[test]
    fn should_treat_identical_text_as_the_same_identifier() {
        let first = ContractId::from_str("contract-1").expect("a well-formed id");
        let second = ContractId::from_str("contract-1").expect("a well-formed id");
        let mut seen: HashSet<ContractId> = HashSet::new();

        assert!(seen.insert(first));
        assert!(!seen.insert(second));
    }

    #[test]
    fn should_order_identifiers_lexicographically() {
        let first = ContractId::from_str("contract-1").expect("a well-formed id");
        let second = ContractId::from_str("contract-2").expect("a well-formed id");

        assert!(first < second);
    }
}
