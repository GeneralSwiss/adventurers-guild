//! Where a quest stands, and the moves that are legal from there.
//!
//! A quest is drafted, posted to the board, taken by a party, worked, and then
//! either settled or given up on. Each of those is a state, and the value of
//! writing them down is that most pairs of them are *not* connected: a quest
//! nobody has accepted cannot be resolved, and a quest already paid out cannot
//! be abandoned to get the bounty back.
//!
//! ```
//! use guild_domain::quest::Quest;
//!
//! let mut quest = Quest::new();
//! quest.post()?;
//! quest.accept()?;
//! quest.begin()?;
//! quest.resolve()?;
//! quest.settle()?;
//!
//! assert_eq!(quest.state().to_string(), "settled");
//! # Ok::<(), guild_domain::quest::QuestError>(())
//! ```
//!
//! # The legal moves
//!
//! | From          | Move        | To            |
//! |---------------|-------------|---------------|
//! | `Draft`       | [`post`]    | `Posted`      |
//! | `Posted`      | [`accept`]  | `Accepted`    |
//! | `Accepted`    | [`begin`]   | `InProgress`  |
//! | `InProgress`  | [`resolve`] | `Resolved`    |
//! | `Resolved`    | [`settle`]  | `Settled`     |
//! | `Accepted`, `InProgress` | [`abandon`] | `Abandoned` |
//!
//! [`post`]: Quest::post
//! [`accept`]: Quest::accept
//! [`begin`]: Quest::begin
//! [`resolve`]: Quest::resolve
//! [`settle`]: Quest::settle
//! [`abandon`]: Quest::abandon
//!
//! `Settled` and `Abandoned` are terminal — every move out of them is refused.
//! Anything not in that table is refused too, and the error names both the
//! state the quest was in and the one it was asked to become.
//!
//! # Three judgement calls
//!
//! **Abandonment starts at `Accepted`.** There is something to give up only
//! once a party has taken the quest on. A `Draft` has nobody waiting on it, and
//! a `Posted` quest has no party to walk away — pulling that one off the board
//! is a different act, and it is not modelled here yet.
//!
//! **A posting cannot be withdrawn.** There is no `Posted -> Draft`. Once the
//! board says a quest is available, retracting it is abandonment — visible,
//! recorded, and paired with a refund — not an edit that leaves no trace.
//!
//! **`Resolved` is not `Settled`.** Resolution is a fact about the world (the
//! party came back, alive or otherwise); settlement is a fact about the books
//! (the bounty was split and paid). They happen at different moments and can
//! disagree in between, which is exactly the gap a settlement run has to close.
//!
//! # Why every arm is written out
//!
//! No `_` arm appears in this module. Adding an eighth state should break every
//! transition that has to decide something about it, because every one of them
//! needs a new answer — the same rule [`HazardTier`](super::hazard_tier) is
//! held to. A catch-all would compile instead, and silently refuse the new
//! state everywhere.

use std::fmt::{self, Display, Formatter};

/// A quest the Guild is hired to do, tracked through its lifecycle.
///
/// The state is private: callers move a quest with [`post`](Quest::post),
/// [`accept`](Quest::accept) and the rest, and read it with
/// [`state`](Quest::state). There is no way to set it directly, so a quest can
/// only ever have arrived where it is by a legal route.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Quest {
    state: State,
}

/// Where a quest stands.
///
/// Public because it is a question the Guild asks constantly — the board shows
/// what is `Posted`, settlement looks for what is `Resolved` — unlike the
/// escrow's states, which callers only ever need through
/// [`balance`](super::escrow::Escrow::balance).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Written up, but not yet on the board.
    #[default]
    Draft,
    /// On the board, waiting for a party to take it.
    Posted,
    /// Taken by a party that has not set out yet.
    Accepted,
    /// Being worked.
    InProgress,
    /// Come to an end in the world, for good or ill; the books do not know yet.
    Resolved,
    /// Given up before resolution.
    Abandoned,
    /// Resolved, and the bounty accounted for.
    Settled,
}

impl Display for State {
    /// Renders the state as a Guild clerk would say it, so an error message
    /// reads `... quest that is in progress` rather than `... InProgress`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            State::Draft => "a draft",
            State::Posted => "posted",
            State::Accepted => "accepted",
            State::InProgress => "in progress",
            State::Resolved => "resolved",
            State::Abandoned => "abandoned",
            State::Settled => "settled",
        })
    }
}

impl Quest {
    /// Writes up a new quest, not yet on the board.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: State::Draft,
        }
    }

    /// Where the quest stands.
    #[must_use]
    pub fn state(&self) -> State {
        self.state
    }

    /// Puts the quest on the board, where a party can take it.
    ///
    /// # Errors
    ///
    /// [`QuestError::IllegalTransition`] from any state but `Draft` — a quest
    /// is posted once, and re-posting one already taken would offer work the
    /// Guild has promised to somebody else.
    pub fn post(&mut self) -> Result<(), QuestError> {
        match self.state {
            State::Draft => {
                self.state = State::Posted;
                Ok(())
            }
            State::Posted
            | State::Accepted
            | State::InProgress
            | State::Resolved
            | State::Abandoned
            | State::Settled => Err(QuestError::IllegalTransition {
                from: self.state,
                to: State::Posted,
            }),
        }
    }

    /// Hands the quest to the party that has taken it.
    ///
    /// # Errors
    ///
    /// [`QuestError::IllegalTransition`] from any state but `Posted` — only
    /// a quest on the board is on offer, and one already accepted belongs to
    /// the party that got there first.
    pub fn accept(&mut self) -> Result<(), QuestError> {
        match self.state {
            State::Posted => {
                self.state = State::Accepted;
                Ok(())
            }
            State::Draft
            | State::Accepted
            | State::InProgress
            | State::Resolved
            | State::Abandoned
            | State::Settled => Err(QuestError::IllegalTransition {
                from: self.state,
                to: State::Accepted,
            }),
        }
    }

    /// Marks the party as having set out.
    ///
    /// # Errors
    ///
    /// [`QuestError::IllegalTransition`] from any state but `Accepted` —
    /// there is no party to set out until one has taken the quest.
    pub fn begin(&mut self) -> Result<(), QuestError> {
        match self.state {
            State::Accepted => {
                self.state = State::InProgress;
                Ok(())
            }
            State::Draft
            | State::Posted
            | State::InProgress
            | State::Resolved
            | State::Abandoned
            | State::Settled => Err(QuestError::IllegalTransition {
                from: self.state,
                to: State::InProgress,
            }),
        }
    }

    /// Records that the quest has come to an end in the world.
    ///
    /// # Errors
    ///
    /// [`QuestError::IllegalTransition`] from any state but `InProgress` —
    /// work that never started cannot have finished.
    pub fn resolve(&mut self) -> Result<(), QuestError> {
        match self.state {
            State::InProgress => {
                self.state = State::Resolved;
                Ok(())
            }
            State::Draft
            | State::Posted
            | State::Accepted
            | State::Resolved
            | State::Abandoned
            | State::Settled => Err(QuestError::IllegalTransition {
                from: self.state,
                to: State::Resolved,
            }),
        }
    }

    /// Closes the books on a resolved quest.
    ///
    /// # Errors
    ///
    /// [`QuestError::IllegalTransition`] from any state but `Resolved` —
    /// settling twice would pay the bounty twice, and settling early would pay
    /// for work that has not ended.
    pub fn settle(&mut self) -> Result<(), QuestError> {
        match self.state {
            State::Resolved => {
                self.state = State::Settled;
                Ok(())
            }
            State::Draft
            | State::Posted
            | State::Accepted
            | State::InProgress
            | State::Abandoned
            | State::Settled => Err(QuestError::IllegalTransition {
                from: self.state,
                to: State::Settled,
            }),
        }
    }

    /// Gives the quest up, before anyone resolved it.
    ///
    /// # Errors
    ///
    /// [`QuestError::IllegalTransition`] from `Draft` and `Posted`, where no
    /// party has taken the quest on and so none can walk away from it, and
    /// from `Resolved`, `Settled` or `Abandoned`, where it has already reached
    /// its end. See the [module docs](self) for why the window opens at
    /// `Accepted`.
    pub fn abandon(&mut self) -> Result<(), QuestError> {
        match self.state {
            State::Accepted | State::InProgress => {
                self.state = State::Abandoned;
                Ok(())
            }
            State::Posted | State::Draft | State::Resolved | State::Abandoned | State::Settled => {
                Err(QuestError::IllegalTransition {
                    from: self.state,
                    to: State::Abandoned,
                })
            }
        }
    }
}

/// The ways a quest refuses to move.
///
/// One variant, carrying both ends of the move that was refused, so a caller
/// can tell "someone else already took it" from "this is already paid out"
/// without matching a verb baked into a variant name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum QuestError {
    /// The move is not one the lifecycle allows from where the quest stands.
    #[error("a quest that is {from} cannot become {to}")]
    IllegalTransition {
        /// Where the quest stood.
        from: State,
        /// What it was asked to become.
        to: State,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn posted() -> Quest {
        let mut quest = Quest::new();
        quest.post().expect("a draft can be posted");
        quest
    }

    fn accepted() -> Quest {
        let mut quest = posted();
        quest.accept().expect("a posted quest can be accepted");
        quest
    }

    fn in_progress() -> Quest {
        let mut quest = accepted();
        quest.begin().expect("an accepted quest can begin");
        quest
    }

    fn resolved() -> Quest {
        let mut quest = in_progress();
        quest
            .resolve()
            .expect("a quest in progress can be resolved");
        quest
    }

    fn settled() -> Quest {
        let mut quest = resolved();
        quest.settle().expect("a resolved quest can be settled");
        quest
    }

    fn abandoned() -> Quest {
        let mut quest = accepted();
        quest.abandon().expect("an accepted quest can be abandoned");
        quest
    }

    fn refused(from: State, to: State) -> Result<(), QuestError> {
        Err(QuestError::IllegalTransition { from, to })
    }

    // Where a quest starts.

    #[test]
    fn should_begin_life_as_a_draft() {
        assert_eq!(Quest::new().state(), State::Draft);
    }

    #[test]
    fn should_default_to_the_same_quest_new_builds() {
        assert_eq!(Quest::default(), Quest::new());
    }

    // Posting.

    #[test]
    fn should_post_a_draft() {
        let mut quest = Quest::new();

        let moved = quest.post();

        assert_eq!(moved, Ok(()));
        assert_eq!(quest.state(), State::Posted);
    }

    #[test]
    fn should_refuse_to_post_a_quest_that_is_already_posted() {
        let mut quest = posted();

        let moved = quest.post();

        assert_eq!(moved, refused(State::Posted, State::Posted));
    }

    #[test]
    fn should_refuse_to_post_a_settled_quest() {
        let mut quest = settled();

        let moved = quest.post();

        assert_eq!(moved, refused(State::Settled, State::Posted));
    }

    // Accepting.

    #[test]
    fn should_accept_a_posted_quest() {
        let mut quest = posted();

        let moved = quest.accept();

        assert_eq!(moved, Ok(()));
        assert_eq!(quest.state(), State::Accepted);
    }

    #[test]
    fn should_refuse_to_accept_a_draft_that_was_never_posted() {
        let mut quest = Quest::new();

        let moved = quest.accept();

        assert_eq!(moved, refused(State::Draft, State::Accepted));
    }

    #[test]
    fn should_refuse_to_accept_a_quest_a_party_already_took() {
        let mut quest = accepted();

        let moved = quest.accept();

        assert_eq!(moved, refused(State::Accepted, State::Accepted));
    }

    // Setting out.

    #[test]
    fn should_begin_an_accepted_quest() {
        let mut quest = accepted();

        let moved = quest.begin();

        assert_eq!(moved, Ok(()));
        assert_eq!(quest.state(), State::InProgress);
    }

    #[test]
    fn should_refuse_to_begin_a_quest_no_party_has_taken() {
        let mut quest = posted();

        let moved = quest.begin();

        assert_eq!(moved, refused(State::Posted, State::InProgress));
    }

    // Resolving.

    #[test]
    fn should_resolve_a_quest_in_progress() {
        let mut quest = in_progress();

        let moved = quest.resolve();

        assert_eq!(moved, Ok(()));
        assert_eq!(quest.state(), State::Resolved);
    }

    #[test]
    fn should_refuse_to_resolve_work_that_never_started() {
        let mut quest = accepted();

        let moved = quest.resolve();

        assert_eq!(moved, refused(State::Accepted, State::Resolved));
    }

    // Settling.

    #[test]
    fn should_settle_a_resolved_quest() {
        let mut quest = resolved();

        let moved = quest.settle();

        assert_eq!(moved, Ok(()));
        assert_eq!(quest.state(), State::Settled);
    }

    #[test]
    fn should_refuse_to_settle_a_quest_that_has_not_ended() {
        let mut quest = in_progress();

        let moved = quest.settle();

        assert_eq!(moved, refused(State::InProgress, State::Settled));
    }

    /// The one that stops the bounty being paid twice.
    #[test]
    fn should_refuse_to_settle_a_quest_twice() {
        let mut quest = settled();

        let moved = quest.settle();

        assert_eq!(moved, refused(State::Settled, State::Settled));
    }

    // Abandonment.

    #[test]
    fn should_abandon_an_accepted_quest() {
        let mut quest = accepted();

        let moved = quest.abandon();

        assert_eq!(moved, Ok(()));
        assert_eq!(quest.state(), State::Abandoned);
    }

    #[test]
    fn should_abandon_a_quest_in_progress() {
        let mut quest = in_progress();

        let moved = quest.abandon();

        assert_eq!(moved, Ok(()));
        assert_eq!(quest.state(), State::Abandoned);
    }

    #[test]
    fn should_refuse_to_abandon_a_draft_nobody_was_promised() {
        let mut quest = Quest::new();

        let moved = quest.abandon();

        assert_eq!(moved, refused(State::Draft, State::Abandoned));
    }

    /// Nobody has taken a posted quest on, so nobody can walk away from it.
    /// Pulling it off the board is a different act, not modelled here yet.
    #[test]
    fn should_refuse_to_abandon_a_quest_no_party_has_taken() {
        let mut quest = posted();

        let moved = quest.abandon();

        assert_eq!(moved, refused(State::Posted, State::Abandoned));
    }

    #[test]
    fn should_refuse_to_abandon_a_quest_that_already_ended() {
        let mut quest = resolved();

        let moved = quest.abandon();

        assert_eq!(moved, refused(State::Resolved, State::Abandoned));
    }

    #[test]
    fn should_refuse_to_abandon_a_quest_twice() {
        let mut quest = abandoned();

        let moved = quest.abandon();

        assert_eq!(moved, refused(State::Abandoned, State::Abandoned));
    }

    // A refused move must leave the quest exactly as it found it.

    #[test]
    fn should_leave_the_state_untouched_when_a_move_is_refused() {
        let mut quest = accepted();

        let _ = quest.post();
        let _ = quest.resolve();
        let _ = quest.settle();

        assert_eq!(quest.state(), State::Accepted);
    }

    // Terminal states let nothing out.

    #[test]
    fn should_refuse_every_move_out_of_a_settled_quest() {
        let mut quest = settled();

        assert!(quest.post().is_err());
        assert!(quest.accept().is_err());
        assert!(quest.begin().is_err());
        assert!(quest.resolve().is_err());
        assert!(quest.settle().is_err());
        assert!(quest.abandon().is_err());
        assert_eq!(quest.state(), State::Settled);
    }

    #[test]
    fn should_refuse_every_move_out_of_an_abandoned_quest() {
        let mut quest = abandoned();

        assert!(quest.post().is_err());
        assert!(quest.accept().is_err());
        assert!(quest.begin().is_err());
        assert!(quest.resolve().is_err());
        assert!(quest.settle().is_err());
        assert!(quest.abandon().is_err());
        assert_eq!(quest.state(), State::Abandoned);
    }

    // The whole road.

    #[test]
    fn should_walk_the_lifecycle_from_draft_to_settled() {
        let mut quest = Quest::new();

        quest.post().expect("a draft can be posted");
        quest.accept().expect("a posted quest can be accepted");
        quest.begin().expect("an accepted quest can begin");
        quest
            .resolve()
            .expect("a quest in progress can be resolved");
        quest.settle().expect("a resolved quest can be settled");

        assert_eq!(quest.state(), State::Settled);
    }

    // How they read.

    #[test]
    fn should_render_each_state_in_the_words_a_clerk_would_use() {
        assert_eq!(State::Draft.to_string(), "a draft");
        assert_eq!(State::Posted.to_string(), "posted");
        assert_eq!(State::Accepted.to_string(), "accepted");
        assert_eq!(State::InProgress.to_string(), "in progress");
        assert_eq!(State::Resolved.to_string(), "resolved");
        assert_eq!(State::Abandoned.to_string(), "abandoned");
        assert_eq!(State::Settled.to_string(), "settled");
    }

    #[test]
    fn should_name_both_ends_of_the_move_it_refused() {
        let refused = QuestError::IllegalTransition {
            from: State::InProgress,
            to: State::Settled,
        };

        assert_eq!(
            refused.to_string(),
            "a quest that is in progress cannot become settled"
        );
    }
}
