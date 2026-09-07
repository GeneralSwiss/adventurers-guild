//! Client module.

use std::fmt;

/// The client that posts a quest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Client;

impl fmt::Display for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the client")
    }
}
