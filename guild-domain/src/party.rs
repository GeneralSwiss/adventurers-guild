//! The band of adventurers that takes a quest on.
//!
//! A placeholder until M3, which gives it membership over time.

/// The adventurers working a quest, as yet without members.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Party;

impl Party {
    /// Musters an empty party.
    #[must_use]
    pub fn new() -> Self {
        Party
    }
}

impl std::default::Default for Party {
    fn default() -> Self {
        Party::new()
    }
}
