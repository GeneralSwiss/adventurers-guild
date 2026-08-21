//! Splitting a purse into parts that still sum to the whole.
//!
//! Allocation is where naive money code loses a copper. An exact third of 100c
//! is 33.33..., and rounding each part on its own either invents a copper or
//! destroys one. [`Allocate`] rounds the parts *against the total* instead, by
//! the largest-remainder method, so what goes in comes out.
//!
//! The weights come in as [`Shares`], which has already proved that they claim
//! the whole purse — see [that module](super::share) for why the invariant
//! lives in the type rather than in a check here.

use super::coin::Coin;
use super::share::Shares;

/// Divides a whole into parts that still sum to the whole.
///
/// Implemented for anything counted in indivisible units, where handing out
/// the parts must not create or destroy any of them.
pub trait Allocate: Sized {
    /// Splits `self` across `shares`, one part per share, in order.
    fn allocate(self, shares: Shares) -> Vec<Self>;
}

impl Allocate for Coin {
    /// Distributes the purse across `shares` by the largest-remainder method.
    ///
    /// Each share takes the floor of its exact portion; the coppers those
    /// floors discarded are then handed out one apiece, largest fraction
    /// first, ties going to the earlier position. The parts sum to the
    /// original purse exactly.
    ///
    /// # Why this cannot fail
    ///
    /// The one thing that could go wrong — weights that do not claim the whole
    /// purse — was ruled out by [`Shares::new`], so there is nothing left to
    /// report. A `Result` whose error can never be produced would force every
    /// call site to handle a case that does not exist; the [`coin`
    /// module](super::coin) makes the same argument about construction.
    fn allocate(self, shares: Shares) -> Vec<Coin> {
        let held = u128::from(self.as_coppers());

        // Multiply before dividing. Flooring first would discard the fraction this
        // algorithm exists to redistribute, and the product reaches 2^96, which is
        // why the intermediate is a u128 while nothing observable ever is.
        // Each share carries its own denominator, so the remainder is kept
        // beside it: on its own the integer is meaningless, because a
        // remainder of 1 is half a copper under 1/2 and a quarter under 1/4.
        let (mut parts, remainders): (Vec<u64>, Vec<(u128, u128)>) = shares
            .iter()
            .map(|share| {
                let denominator = u128::from(share.denominator());
                let exact = held * u128::from(share.numerator());
                // A share never exceeds one, so `exact / denominator <= held`
                // and the cast is exact.
                (
                    (exact / denominator) as u64,
                    (exact % denominator, denominator),
                )
            })
            .unzip();

        // Every floor discarded strictly less than one copper, so across n shares
        // the shortfall is strictly less than n. One spare copper each covers it,
        // and there is never a second pass.
        let handed_out: u64 = parts.iter().sum();
        let spare = self.as_coppers() - handed_out;

        let mut order: Vec<usize> = (0..parts.len()).collect();
        order.sort_unstable_by(|&left, &right| {
            // Cross-multiply to put the two fractions over a common
            // denominator. Comparing the bare remainders would rank 1/4 above
            // 1/2 whenever the coarser share happened to carry a smaller one.
            // Both products stay under 2^64, so the u128 cannot overflow.
            let (remainder_left, denominator_left) = remainders[left];
            let (remainder_right, denominator_right) = remainders[right];
            (remainder_right * denominator_left)
                .cmp(&(remainder_left * denominator_right))
                .then_with(|| left.cmp(&right))
        });

        for &index in order.iter().take(spare as usize) {
            parts[index] += 1;
        }

        parts.into_iter().map(Coin::from_coppers).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::share::Share;
    use proptest::prelude::*;

    fn coppers(parts: &[Coin]) -> Vec<u64> {
        parts.iter().map(|part| part.as_coppers()).collect()
    }

    /// Turns relative weights into shares of the whole, which is the shape the
    /// old tests were written in. Every weight becomes `weight / total`, so the
    /// set sums to exactly one by construction.
    fn shares_of(weights: &[u32]) -> Shares {
        let total: u32 = weights.iter().sum();
        let shares: Vec<Share> = weights
            .iter()
            .map(|&weight| Share::new(weight, total).expect("a total above zero"))
            .collect();
        Shares::new(&shares).expect("weights over their own total sum to one")
    }

    #[test]
    fn should_split_a_purse_that_divides_evenly() {
        let parts = Coin::from_coppers(100).allocate(shares_of(&[1, 1, 1, 1]));

        assert_eq!(coppers(&parts), vec![25, 25, 25, 25]);
    }

    #[test]
    fn should_hand_each_spare_copper_to_the_largest_remainder() {
        // 10c at 1:2:4 — exact shares 1.428, 2.857, 5.714. The floors hand out
        // 8, so 2 coppers are spare, and they go to the two largest fractions.
        let parts = Coin::from_coppers(10).allocate(shares_of(&[1, 2, 4]));

        assert_eq!(coppers(&parts), vec![1, 3, 6]);
    }

    #[test]
    fn should_break_a_tie_by_position() {
        // Three equal shares leave three equal remainders. Position decides,
        // so the result is stable rather than merely correct.
        let parts = Coin::from_coppers(5).allocate(shares_of(&[1, 1, 1]));

        assert_eq!(coppers(&parts), vec![2, 2, 1]);
    }

    #[test]
    fn should_allocate_the_purse_from_the_draft() {
        let parts = Coin::from_coppers(5).allocate(shares_of(&[1, 3, 5]));

        assert_eq!(coppers(&parts), vec![0, 2, 3]);
    }

    #[test]
    fn should_rank_remainders_by_fraction_not_by_numerator() {
        // The share of 1/2 and the shares of 1/4 all leave a bare remainder of
        // 1, but half a copper outranks a quarter. Ranking the integers alone
        // would see a three-way tie and pay position 0 instead.
        let shares = [
            Share::new(1, 4).expect("a non-zero denominator"),
            Share::new(1, 4).expect("a non-zero denominator"),
            Share::new(1, 2).expect("a non-zero denominator"),
        ];
        let parts = Coin::from_coppers(1)
            .allocate(Shares::new(&shares).expect("quarters and a half sum to one"));

        assert_eq!(coppers(&parts), vec![0, 0, 1]);
    }

    #[test]
    fn should_let_position_change_who_is_paid() {
        // Deliberate, and worth pinning down: allocation is *not* invariant
        // under reordering. Both splits below are exact, but the party holding
        // a sixth is paid a copper in the first and nothing in the second.
        // Someone has to receive the favourable rounding, and making it
        // positional puts that choice in the caller's hands.
        let purse = Coin::from_coppers(3);

        let first = purse.allocate(shares_of(&[1, 3, 2]));
        let reordered = purse.allocate(shares_of(&[3, 1, 2]));

        assert_eq!(coppers(&first), vec![1, 1, 1]);
        assert_eq!(coppers(&reordered), vec![2, 0, 1]);
    }

    #[test]
    fn should_give_nothing_away_from_an_empty_purse() {
        let parts = Coin::ZERO.allocate(shares_of(&[1, 1]));

        assert_eq!(coppers(&parts), vec![0, 0]);
    }

    #[test]
    fn should_not_overflow_on_a_purse_near_the_maximum() {
        let parts = Coin::from_coppers(u64::MAX).allocate(shares_of(&[1, 1]));

        assert_eq!(coppers(&parts).iter().sum::<u64>(), u64::MAX);
    }

    #[test]
    fn should_not_overflow_on_a_share_with_a_large_denominator() {
        // The product reaches 2^96 here, which is the whole reason the
        // intermediate is a u128.
        let shares = [
            Share::new(1, u32::MAX).expect("a non-zero denominator"),
            Share::new(u32::MAX - 1, u32::MAX).expect("a non-zero denominator"),
        ];
        let parts = Coin::from_coppers(u64::MAX)
            .allocate(Shares::new(&shares).expect("the pair sums to one"));

        assert_eq!(coppers(&parts).iter().sum::<u64>(), u64::MAX);
    }

    proptest! {
        /// The whole point: allocation moves money, it never creates or
        /// destroys it. This is the property that catches a lost copper.
        #[test]
        fn should_hand_out_every_copper_and_no_more(
            coppers_held in 0..=u64::MAX,
            weights in prop::collection::vec(1u32..=1000, 1..=20),
        ) {
            let parts = Coin::from_coppers(coppers_held)
                .allocate(shares_of(&weights))
                ;

            prop_assert_eq!(
                parts.iter().map(|part| part.as_coppers()).sum::<u64>(),
                coppers_held
            );
        }

        /// One part per share, always — a caller indexes these against the
        /// parties that supplied the shares.
        #[test]
        fn should_return_one_part_per_share(
            coppers_held in 0..=u64::MAX,
            weights in prop::collection::vec(1u32..=1000, 1..=20),
        ) {
            let parts = Coin::from_coppers(coppers_held)
                .allocate(shares_of(&weights))
                ;

            prop_assert_eq!(parts.len(), weights.len());
        }

        /// Same input, same output. Largest-remainder without a stable
        /// tie-break would pass the sum property and still fail this one.
        #[test]
        fn should_allocate_the_same_way_every_time(
            coppers_held in 0..=u64::MAX,
            weights in prop::collection::vec(1u32..=1000, 1..=20),
        ) {
            let purse = Coin::from_coppers(coppers_held);

            let first = purse.allocate(shares_of(&weights));
            let second = purse.allocate(shares_of(&weights));

            prop_assert_eq!(first, second);
        }

        /// A larger share is never paid less than a smaller one.
        ///
        /// Strictly larger. Equal shares are settled by the tie-break instead,
        /// and there the *later* one is paid less — see
        /// `should_break_a_tie_by_position`, where equal shares pay 2, 2, 1.
        #[test]
        fn should_never_pay_a_larger_share_less_than_a_smaller_one(
            coppers_held in 0..=u64::MAX,
            weights in prop::collection::vec(1u32..=1000, 1..=20),
        ) {
            let parts = Coin::from_coppers(coppers_held)
                .allocate(shares_of(&weights))
                ;

            for (i, &left) in weights.iter().enumerate() {
                for (j, &right) in weights.iter().enumerate() {
                    if left > right {
                        prop_assert!(parts[i].as_coppers() >= parts[j].as_coppers());
                    }
                }
            }
        }

        /// Shares are a ratio, so scaling every weight changes nothing. 1:2 and
        /// 2:4 must pay out identically, down to who receives the spare copper.
        #[test]
        fn should_allocate_by_ratio_not_by_magnitude(
            coppers_held in 0..=u64::MAX,
            weights in prop::collection::vec(1u32..=1000, 1..=20),
            scale in 1u32..=1000,
        ) {
            let purse = Coin::from_coppers(coppers_held);
            let scaled: Vec<u32> = weights.iter().map(|weight| weight * scale).collect();

            prop_assert_eq!(
                purse.allocate(shares_of(&weights)),
                purse.allocate(shares_of(&scaled))
            );
        }

        /// A sole claimant takes the purse entire — no rounding, no residue.
        #[test]
        fn should_give_a_lone_share_the_whole_purse(coppers_held in 0..=u64::MAX) {
            let whole = [Share::new(1, 1).expect("a non-zero denominator")];

            let parts = Coin::from_coppers(coppers_held)
                .allocate(Shares::new(&whole).expect("one whole share sums to one"))
                ;

            prop_assert_eq!(parts, vec![Coin::from_coppers(coppers_held)]);
        }

        /// No part may exceed its exact share by a whole copper — that is what
        /// makes largest-remainder *fair* rather than merely exact.
        #[test]
        fn should_keep_every_part_within_one_copper_of_its_exact_share(
            coppers_held in 0..=u64::MAX,
            weights in prop::collection::vec(1u32..=1000, 1..=20),
        ) {
            let total: u128 = weights.iter().map(|&w| u128::from(w)).sum();
            let parts = Coin::from_coppers(coppers_held)
                .allocate(shares_of(&weights))
                ;

            for (part, &weight) in parts.iter().zip(weights.iter()) {
                let floor = u128::from(coppers_held) * u128::from(weight) / total;
                let given = u128::from(part.as_coppers());
                prop_assert!(given == floor || given == floor + 1);
            }
        }
    }
}
