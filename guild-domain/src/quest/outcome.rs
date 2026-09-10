//! How a quest ended, once it is no longer being worked.

/// The outcome of a quest, did the party succeed for fail?
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Outcome {
    /// A succesful quest outcome
    Successful,
    /// A failed quest outcome
    Failure,
}
