//! Weights that are exact ratios, and sets of them that claim a whole purse.
//!
//! [`Share`] is an exact ratio — a numerator over a denominator, never a float,
//! because a weight of `0.1` reintroduces the rounding that
//! [allocation](super::allocation) exists to remove. [`Shares`] is a set of
//! them proved, once, to sum to exactly one.
//!
//! That is parse, don't validate applied to weights: holding a [`Shares`] *is*
//! the proof that the set is whole, so
//! [`allocate`](super::allocation::Allocate::allocate) never re-checks it and
//! has no failure to report.
//! <https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/>

use num_rational::Ratio;

/// One party's exact claim on a purse, as a ratio.
///
/// Built only through [`Share::new`], so a share can never carry the zero
/// denominator that would make it undefined. Reduced on construction, so
/// `2/4` and `1/2` are the same share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Share(num_rational::Ratio<u32>);

impl Share {
    /// Builds a share of `numerator` parts in `denominator`.
    ///
    /// # Errors
    ///
    /// [`InvalidShare::ZeroDenominator`] if `denominator` is zero, which would
    /// name no ratio at all.
    pub fn new(numerator: u32, denominator: u32) -> Result<Self, InvalidShare> {
        if denominator == 0 {
            return Err(InvalidShare::ZeroDenominator { numerator });
        }
        Ok(Self(num_rational::Ratio::new(numerator, denominator)))
    }

    /// The numerator, after reduction.
    pub fn numerator(&self) -> u32 {
        *self.0.numer()
    }

    /// The denominator, after reduction. Never zero.
    pub fn denominator(&self) -> u32 {
        *self.0.denom()
    }
}

/// A set of shares that between them claim the whole purse.
///
/// The sum-to-one rule is checked once, in [`Shares::new`], and holds for the
/// life of the value. That is what lets
/// [`allocate`](super::allocation::Allocate::allocate) be infallible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shares(Vec<Share>);

impl Shares {
    /// Gathers `shares` into a set, proving they claim the whole purse.
    ///
    /// # Errors
    ///
    /// [`InvalidShare::SharesMustTotalOne`] if the set sums to anything other
    /// than one, reporting what it actually summed to. An empty set sums to
    /// zero and is rejected on the same ground.
    pub fn new(shares: &[Share]) -> Result<Self, InvalidShare> {
        let total: Ratio<u32> = shares.iter().map(|share| share.0).sum();
        if total != Ratio::ONE {
            return Err(InvalidShare::SharesMustTotalOne { total });
        }
        Ok(Self(shares.to_vec()))
    }

    /// Borrows the shares in the order they were supplied.
    pub fn iter(&self) -> std::slice::Iter<'_, Share> {
        self.0.iter()
    }
}

impl IntoIterator for Shares {
    type Item = Share;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Error types for working with Share types
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidShare {
    /// Ratio is undefined
    #[error("Cannot have an undefined ratio: {numerator}/0")]
    ZeroDenominator {
        /// The numerator used with the undefined ratio
        numerator: u32,
    },
    /// Shares must sum to ONE and if they don't that's a problem.
    #[error("Shares must sum to ONE: {total}")]
    SharesMustTotalOne {
        /// The total of all shares found at the site of the error.
        total: Ratio<u32>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_hand_out_its_shares_in_the_order_they_were_given() {
        // `allocate` borrows through `iter`, so consuming the set is public
        // API nothing in the crate exercises yet. Order is the part that
        // matters: a caller indexes the parts it gets back against the
        // parties that supplied the shares.
        let supplied = [
            Share::new(1, 4).expect("a non-zero denominator"),
            Share::new(3, 4).expect("a non-zero denominator"),
        ];
        let shares = Shares::new(&supplied).expect("a quarter and three quarters sum to one");

        let consumed: Vec<Share> = shares.into_iter().collect();

        assert_eq!(consumed, supplied.to_vec());
    }

    #[test]
    fn should_reject_a_zero_denominator() {
        assert_eq!(
            Share::new(3, 0),
            Err(InvalidShare::ZeroDenominator { numerator: 3 })
        );
    }

    #[test]
    fn should_reject_shares_that_do_not_sum_to_one() {
        let thirds = [
            Share::new(1, 3).expect("a non-zero denominator"),
            Share::new(1, 3).expect("a non-zero denominator"),
        ];

        assert_eq!(
            Shares::new(&thirds),
            Err(InvalidShare::SharesMustTotalOne {
                total: Ratio::new(2, 3)
            })
        );
    }

    #[test]
    fn should_reject_an_empty_set_of_shares() {
        assert_eq!(
            Shares::new(&[]),
            Err(InvalidShare::SharesMustTotalOne {
                total: Ratio::from_integer(0)
            })
        );
    }
}
