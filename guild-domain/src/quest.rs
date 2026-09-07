//! Quests: what the Guild is hired to do, and what it holds while doing it.
//!
//! | Module          | Holds                                                  |
//! |-----------------|--------------------------------------------------------|
//! | [`hazard_tier`] | [`HazardTier`], how dangerous a quest is               |
//! | [`escrow`]      | [`Escrow`], the bounty from payment to payout          |
//! | [`state`]       | [`Quest`], where a quest stands and what it may do next |
//!
//! The aggregate that binds a tier, an escrow and a state together arrives with
//! the rest of M2 — see `backlog/BACKLOG.md`.
//!
//! Every type is re-exported here, so `quest::HazardTier` is the path to
//! prefer over `quest::hazard_tier::HazardTier`.

pub mod escrow;
pub mod hazard_tier;
pub mod state;

pub use escrow::{Escrow, EscrowError, Settlement};
pub use hazard_tier::HazardTier;
pub use state::{Quest, QuestError, State};
