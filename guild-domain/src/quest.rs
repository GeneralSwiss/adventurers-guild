//! Quests: what the Guild is hired to do, and what it holds while doing it.
//!
//! | Module          | Holds                                                  |
//! |-----------------|--------------------------------------------------------|
//! | [`hazard_tier`] | [`HazardTier`], how dangerous a quest is               |
//! | [`escrow`]      | [`Escrow`], the bounty from payment to payout          |
//! | [`lifecycle`]   | [`Quest`], where it stands and what it may do next     |
//!
//! Types are re-exported here, so `quest::HazardTier` is the path to prefer
//! over `quest::hazard_tier::HazardTier`.
//!
//! `Stage` is the one exception. An escrow has one and so does a quest, and two
//! different things cannot share a name at this level — so the quest's
//! [`Stage`] is re-exported and the escrow's stays `escrow::Stage`.

pub mod client;
pub mod escrow;
pub mod hazard_tier;
pub mod lifecycle;

pub use escrow::{Escrow, EscrowError, Settlement};
pub use hazard_tier::HazardTier;
pub use lifecycle::{Quest, QuestError, Stage};
