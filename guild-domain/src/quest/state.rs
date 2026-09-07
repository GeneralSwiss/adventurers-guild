//! Documentation for the state of a quest.

pub struct Quest {
    state: State,
}

enum State {
    Draft,
    Posted,
    Accepted,
    InProgress,
    Resolved,
    Abandoned,
    Settled,
}
