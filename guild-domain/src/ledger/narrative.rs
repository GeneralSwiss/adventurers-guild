//! What an entry says it was for.
//!
//! A ledger nobody can read is a ledger nobody trusts. Every journal entry
//! carries one of these, and the type exists so that "carries a narrative"
//! cannot quietly mean "carries an empty string".
//!
//! ```
//! use guild_domain::ledger::Narrative;
//!
//! let narrative: Narrative = "settlement of quest-1".parse()?;
//! assert_eq!(narrative.as_str(), "settlement of quest-1");
//!
//! assert!("   ".parse::<Narrative>().is_err());
//! # Ok::<(), guild_domain::ledger::InvalidNarrative>(())
//! ```
//!
//! # Why a type rather than a `String` field
//!
//! `JournalEntry::new(postings, "")` would compile, and the entry it produced
//! would satisfy every other rule the ledger has while telling a reader
//! nothing. Making blankness unrepresentable is cheaper than remembering to
//! check for it at each of the places entries get built — which by M4 is
//! settlement, reversal, and whatever a TUI does.
//!
//! Same shape as the [identifiers](crate::identifiers): a private field,
//! `TryFrom<String>` as the sole constructor, and [`FromStr`] delegating to it
//! so `"...".parse()` works. Holding a [`Narrative`] *is* the proof it says
//! something.
//!
//! # Blank, not empty
//!
//! The rule rejects anything that is only whitespace, not merely the empty
//! string — `"   "` reads exactly as well as `""`. What survives is stored
//! trimmed, so two narratives differing only in padding are the same
//! narrative and a ledger dump lines up. Trimming is at the ends only: a
//! narrative is prose, and its interior spaces are the words.
//!
//! Unlike an identifier, a narrative may hold whitespace and punctuation.
//! That is the whole point of it.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

/// The human-readable reason an entry exists.
///
/// Built only by parsing, so holding one is proof that it says something:
/// non-blank, and stored with its padding removed.
///
/// ```
/// use guild_domain::ledger::Narrative;
///
/// let narrative: Narrative = "  refund of quest-2  ".parse()?;
/// assert_eq!(narrative.to_string(), "refund of quest-2");
/// # Ok::<(), guild_domain::ledger::InvalidNarrative>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Narrative(String);

impl Narrative {
    /// Borrows the narrative as a string slice. Never blank.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Attaches the `original` entry's identifier to the narrative, so a reversal can be read as "this is the reversal of that entry".
    #[must_use]
    pub fn reversal_of(self, original: crate::identifiers::EntryId) -> Self {
        let reversed = format!("reversal of {}: {}", original, self.0);
        Self(reversed)
    }
}

impl Display for Narrative {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl TryFrom<String> for Narrative {
    type Error = InvalidNarrative;

    /// The sole constructor, and so the only place the non-blank rule can be
    /// enforced or bypassed. [`FromStr`] delegates here rather than the
    /// reverse, for the reason set out in the
    /// [identifiers](crate::identifiers) module docs: a `&str` must be copied
    /// before it can be owned, so the borrowing impl is the one that can be
    /// written in terms of the owning impl without waste.
    ///
    /// # Errors
    ///
    /// [`InvalidNarrative::Blank`] if the candidate is empty or holds nothing
    /// but whitespace.
    fn try_from(candidate: String) -> Result<Self, Self::Error> {
        let trimmed = candidate.trim();
        if trimmed.is_empty() {
            return Err(InvalidNarrative::Blank { candidate });
        }
        // Reuses the caller's buffer when there was no padding to remove, and
        // copies only the words when there was.
        if trimmed.len() == candidate.len() {
            Ok(Self(candidate))
        } else {
            Ok(Self(trimmed.to_owned()))
        }
    }
}

impl FromStr for Narrative {
    type Err = InvalidNarrative;

    /// Copies the candidate and defers to [`TryFrom<String>`] for the rule.
    ///
    /// # Errors
    ///
    /// [`InvalidNarrative::Blank`] if the candidate is empty or holds nothing
    /// but whitespace.
    fn from_str(candidate: &str) -> Result<Self, Self::Err> {
        Self::try_from(candidate.to_owned())
    }
}

/// The ways a candidate can fail to be a [`Narrative`].
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum InvalidNarrative {
    /// The candidate said nothing.
    #[error("an entry must say what it was for, and {candidate:?} says nothing")]
    Blank {
        /// The rejected candidate, echoed back to locate the offending input.
        candidate: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identifiers::EntryId;

    #[test]
    fn should_hold_the_words_it_was_given() {
        let narrative: Narrative = "settlement of quest-1".parse().expect("a narrative");

        assert_eq!(narrative.as_str(), "settlement of quest-1");
    }

    #[test]
    fn should_reject_a_narrative_with_no_words_in_it() {
        assert_eq!(
            Narrative::from_str(""),
            Err(InvalidNarrative::Blank {
                candidate: String::new()
            })
        );
    }

    #[test]
    fn should_reject_a_narrative_that_is_only_whitespace() {
        // Blank is not the same as empty. A ledger nobody can read is a ledger
        // nobody trusts, and "   " reads exactly as well as "".
        assert_eq!(
            Narrative::from_str("   \t "),
            Err(InvalidNarrative::Blank {
                candidate: "   \t ".to_owned()
            })
        );
    }

    #[test]
    fn should_trim_the_padding_off_a_narrative() {
        // Stored trimmed so that two narratives differing only in padding are
        // the same narrative, and so a ledger dump lines up.
        let narrative = Narrative::from_str("  refund of quest-2  ").expect("a narrative");

        assert_eq!(narrative.as_str(), "refund of quest-2");
    }

    #[test]
    fn should_keep_the_spaces_inside_a_narrative() {
        // Trimming is at the ends only. A narrative is prose, not an
        // identifier — the interior spaces are the words.
        let narrative = Narrative::from_str("estate payout: alder quill").expect("a narrative");

        assert_eq!(narrative.as_str(), "estate payout: alder quill");
    }

    #[test]
    fn should_name_the_entry_a_reversal_undoes() {
        // A reversal that does not say what it reverses is just a second
        // entry, and nobody reading the journal can tie the two together.
        let narrative = Narrative::from_str("settlement of quest-1").expect("a narrative");

        let reversal = narrative.reversal_of(EntryId::sequential(7));

        assert_eq!(
            reversal.as_str(),
            "reversal of entry-7: settlement of quest-1"
        );
    }

    #[test]
    fn should_still_read_as_a_narrative_after_being_marked_a_reversal() {
        // `reversal_of` builds its text rather than parsing it, so it is the
        // one door into a Narrative where nothing re-checks the rule. This
        // pins that what comes out would still be accepted going in.
        let reversal = Narrative::from_str("refund of quest-2")
            .expect("a narrative")
            .reversal_of(EntryId::sequential(1));

        assert_eq!(Narrative::from_str(reversal.as_str()), Ok(reversal));
    }

    #[test]
    fn should_keep_both_entries_readable_when_a_reversal_is_marked_twice() {
        // Nothing in this type forbids it — the rule that a reversal cannot be
        // reversed lives in the ledger. What is pinned here is that the text
        // nests rather than losing the inner entry's name.
        let twice = Narrative::from_str("settlement of quest-1")
            .expect("a narrative")
            .reversal_of(EntryId::sequential(3))
            .reversal_of(EntryId::sequential(4));

        assert_eq!(
            twice.as_str(),
            "reversal of entry-4: reversal of entry-3: settlement of quest-1"
        );
    }
}
