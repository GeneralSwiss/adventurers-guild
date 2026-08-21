//! What a purse holds, how a claim on it is weighed, and how it divides.
//!
//! Three pieces, each with one job:
//!
//! | Module                    | Holds                                          |
//! |---------------------------|------------------------------------------------|
//! | [`coin`]                  | [`Coin`], a purse counted in indivisible units |
//! | [`share`]                 | [`Share`] and [`Shares`], weights as exact ratios |
//! | [`allocation`]            | [`Allocate`], dividing a purse without loss    |
//!
//! The invariants are split the same way. A [`Coin`] cannot go negative, a
//! [`Shares`] cannot fail to claim the whole purse, and because both hold,
//! [`Allocate::allocate`] has nothing left to report and returns no `Result`.
//!
//! Every type is re-exported here, so `money::Coin` is the path to prefer over
//! `money::coin::Coin`.

pub mod allocation;
pub mod coin;
pub mod share;

pub use allocation::Allocate;
pub use coin::{Coin, MoneyError};
pub use share::{InvalidShare, Share, Shares};
