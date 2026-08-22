//! When things happened, and how long they took.
//!
//! Two types, and the distinction between them is the point:
//!
//! | Module                | Holds                                            |
//! |-----------------------|--------------------------------------------------|
//! | [`world_instant`]     | [`WorldInstant`], a moment reckoned from the Founding |
//! | [`duration`]          | [`Duration`], a span with a length but no direction |
//!
//! A moment minus a moment is a span; a moment plus a span is a moment; a
//! moment plus a moment does not compile. The compiler carries that rule, so
//! nothing downstream has to remember it.
//!
//! # Why the domain owns its own time
//!
//! Nothing here reads a clock, and there is no `chrono` or `time` in the
//! crate's dependencies. That is not frugality: a settlement calculation that
//! can reach `now()` is a calculation whose result depends on when it ran, and
//! the point of this crate is that everything in it is constructible and
//! testable in memory.
//!
//! Time enters the domain the same way money does — as a value someone hands
//! in at the boundary. Turning a real date into a [`WorldInstant`], or back,
//! is an adapter's job. See [`world_instant`] for what that adapter would have
//! to decide.
//!
//! Every type is re-exported here, so `time::WorldInstant` is the path to
//! prefer over `time::world_instant::WorldInstant`.

pub mod duration;
pub mod world_instant;

pub use duration::Duration;
pub use world_instant::{TimeError, WorldInstant};
