//! Quests: what the Guild is hired to do, and what it holds while doing it.
//!
//! | Module          | Holds                                              |
//! |-----------------|-----------------------------------------------------|
//! | [`hazard_tier`] | [`HazardTier`], how dangerous a quest is            |
//!
//! The aggregate itself, its bounty escrow, and its state machine arrive with
//! the rest of M2 — see `backlog/BACKLOG.md`. This module starts with the one
//! piece the others and M4's settlement all read: the tier.
//!
//! Every type is re-exported here, so `quest::HazardTier` is the path to
//! prefer over `quest::hazard_tier::HazardTier`.

pub mod hazard_tier;

pub use hazard_tier::HazardTier;
