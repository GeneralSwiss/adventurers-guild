//! The Adventurers' Guild contract, escrow, and settlement domain.
//!
//! This crate is the pure domain core: business rules only, no I/O, no
//! persistence, no framework. Everything here must be constructible and
//! testable without a runtime, a database, or a clock.
//!
//! # House rules
//!
//! - Expected failures return [`Result`]; `panic!` is for unreachable invariants.
//! - Money is integer minor units. There is no `f64` anywhere in this crate.
//! - Domain concepts get newtypes. A quest identifier is not a `String`.
//! - Variants are enums, matched exhaustively — no `_` arms on domain enums,
//!   so adding a variant becomes a compile error rather than a silent bug.
//! - Time is a domain type, not `chrono`. See the `time` module when it lands.
//!
//! # Modules to come
//!
//! Built in dependency order, one milestone at a time — see `backlog/BACKLOG.md`
//! at the repository root:
//!
//! | Module        | Milestone | Holds                                          |
//! |---------------|-----------|------------------------------------------------|
//! | `identifiers` | M0        | Typed identifiers                              |
//! | `money`       | M0        | `Coin`, `Share`, allocation without loss       |
//! | `time`        | M0        | `WorldInstant`, `Duration`                     |
//! | `ledger`      | M1        | Double-entry postings that must balance        |
//! | `quest`       | M2        | Bounty escrow and the quest lifecycle          |
//! | `party`       | M3        | Membership as intervals over time              |
//! | `settlement`  | M4        | Fee schedules, split policies, payout          |
//! | `event`       | —         | Domain events emitted by the aggregates        |

pub mod allocation;
pub mod coin;
pub mod identifiers;
