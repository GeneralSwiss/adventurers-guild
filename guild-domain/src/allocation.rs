//! Splitting a purse into parts that still sum to the whole.

use crate::coin::Coin;

/// Distributes `purse` across `weights` by the largest-remainder method.
///
/// # Errors
///
/// [`AllocationError::WeightsSumToZero`] if there is nobody to allocate to.
pub fn allocate(purse: Coin, weights: &[u32]) -> Result<Vec<Coin>, AllocationError> {
    let total: u128 = weights.iter().map(|&weight| u128::from(weight)).sum();
    if total == 0 {
        return Err(AllocationError::WeightsSumToZero);
    }

    let held = u128::from(purse.as_coppers());

    // Multiply before dividing. Flooring first would discard the fraction this
    // algorithm exists to redistribute, and the product reaches 2^96, which is
    // why the intermediate is a u128 while nothing observable ever is.
    let (mut parts, remainders): (Vec<u64>, Vec<u128>) = weights
        .iter()
        .map(|&weight| {
            let exact = held * u128::from(weight);
            // `total >= weight`, so `exact / total <= held`: the cast is exact.
            ((exact / total) as u64, exact % total)
        })
        .unzip();

    // Every floor discarded strictly less than one copper, so across n weights
    // the shortfall is strictly less than n. One spare copper each covers it,
    // and there is never a second pass.
    let handed_out: u64 = parts.iter().sum();
    let spare = purse.as_coppers() - handed_out;

    let mut order: Vec<usize> = (0..parts.len()).collect();
    order.sort_unstable_by(|&left, &right| {
        remainders[right]
            .cmp(&remainders[left])
            .then_with(|| left.cmp(&right))
    });

    for &index in order.iter().take(spare as usize) {
        parts[index] += 1;
    }

    Ok(parts.into_iter().map(Coin::from_coppers).collect())
}

/// The ways an allocation can fail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AllocationError {
    /// No weight to allocate against: an empty slice, or every weight zero.
    #[error("a purse cannot be allocated across weights that sum to zero")]
    WeightsSumToZero,
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn coppers(parts: &[Coin]) -> Vec<u64> {
        parts.iter().map(|part| part.as_coppers()).collect()
    }

    #[test]
    fn should_split_a_purse_that_divides_evenly() {
        let parts =
            allocate(Coin::from_coppers(100), &[1, 1, 1, 1]).expect("weights sum above zero");

        assert_eq!(coppers(&parts), vec![25, 25, 25, 25]);
    }

    #[test]
    fn should_hand_each_spare_copper_to_the_largest_remainder() {
        // 10c at 1:2:4 — exact shares 1.428, 2.857, 5.714. The floors hand out
        // 8, so 2 coppers are spare, and they go to the two largest fractions.
        let parts = allocate(Coin::from_coppers(10), &[1, 2, 4]).expect("weights sum above zero");

        assert_eq!(coppers(&parts), vec![1, 3, 6]);
    }

    #[test]
    fn should_break_a_tie_by_position() {
        // Three equal weights leave three equal remainders. Position decides,
        // so the result is stable rather than merely correct.
        let parts = allocate(Coin::from_coppers(5), &[1, 1, 1]).expect("weights sum above zero");

        assert_eq!(coppers(&parts), vec![2, 2, 1]);
    }

    #[test]
    fn should_allocate_the_purse_from_the_draft() {
        let parts = allocate(Coin::from_coppers(5), &[1, 3, 5]).expect("weights sum above zero");

        assert_eq!(coppers(&parts), vec![0, 2, 3]);
    }

    #[test]
    fn should_let_position_change_who_is_paid() {
        // Deliberate, and worth pinning down: allocation is *not* invariant
        // under reordering the weights. Both splits below are exact, but the
        // party weighted 1 is paid a copper in the first and nothing in the
        // second. This is Fowler's point — someone has to receive the
        // favourable rounding, and making it positional puts that choice in
        // the caller's hands rather than hiding it.
        let purse = Coin::from_coppers(3);

        let first = allocate(purse, &[1, 3, 2]).expect("weights sum above zero");
        let reordered = allocate(purse, &[3, 1, 2]).expect("weights sum above zero");

        assert_eq!(coppers(&first), vec![1, 1, 1]);
        assert_eq!(coppers(&reordered), vec![2, 0, 1]);
    }

    #[test]
    fn should_give_nothing_away_from_an_empty_purse() {
        let parts = allocate(Coin::ZERO, &[1, 1]).expect("weights sum above zero");

        assert_eq!(coppers(&parts), vec![0, 0]);
    }

    #[test]
    fn should_reject_weights_that_are_all_zero() {
        assert_eq!(
            allocate(Coin::from_coppers(10), &[0, 0]),
            Err(AllocationError::WeightsSumToZero)
        );
    }

    #[test]
    fn should_reject_an_empty_slice_of_weights() {
        assert_eq!(
            allocate(Coin::from_coppers(10), &[]),
            Err(AllocationError::WeightsSumToZero)
        );
    }

    #[test]
    fn should_not_overflow_on_a_purse_near_the_maximum() {
        let parts = allocate(Coin::from_coppers(u64::MAX), &[u32::MAX, u32::MAX])
            .expect("weights sum above zero");

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
            let purse = Coin::from_coppers(coppers_held);

            let parts = allocate(purse, &weights).expect("weights sum above zero");

            prop_assert_eq!(
                parts.iter().map(|part| part.as_coppers()).sum::<u64>(),
                coppers_held
            );
        }

        /// One part per weight, always — a caller indexes these against the
        /// parties that supplied the weights.
        #[test]
        fn should_return_one_part_per_weight(
            coppers_held in 0..=u64::MAX,
            weights in prop::collection::vec(1u32..=1000, 1..=20),
        ) {
            let parts = allocate(Coin::from_coppers(coppers_held), &weights)
                .expect("weights sum above zero");

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

            let first = allocate(purse, &weights).expect("weights sum above zero");
            let second = allocate(purse, &weights).expect("weights sum above zero");

            prop_assert_eq!(first, second);
        }

        /// A heavier weight is never paid less than a lighter one.
        ///
        /// Strictly heavier. Equal weights are settled by the tie-break
        /// instead, and there the *later* one is paid less — see
        /// `should_break_a_tie_by_position`, where equal weights pay 2, 2, 1.
        ///
        /// For a strict inequality the argument holds: a heavier weight has
        /// the larger exact share, so it either lands on a higher floor, or on
        /// the same floor with a larger remainder — which sorts it ahead in the
        /// queue for spare coppers. It can never come out behind.
        #[test]
        fn should_never_pay_a_heavier_weight_less_than_a_lighter_one(
            coppers_held in 0..=u64::MAX,
            weights in prop::collection::vec(1u32..=1000, 1..=20),
        ) {
            let parts = allocate(Coin::from_coppers(coppers_held), &weights)
                .expect("weights sum above zero");

            for (i, &left) in weights.iter().enumerate() {
                for (j, &right) in weights.iter().enumerate() {
                    if left > right {
                        prop_assert!(parts[i].as_coppers() >= parts[j].as_coppers());
                    }
                }
            }
        }

        /// Weights are a ratio, so scaling them all changes nothing. 1:2 and
        /// 2:4 must pay out identically, down to which party gets the spare
        /// copper.
        #[test]
        fn should_allocate_by_ratio_not_by_magnitude(
            coppers_held in 0..=u64::MAX,
            weights in prop::collection::vec(1u32..=1000, 1..=20),
            scale in 1u32..=1000,
        ) {
            let purse = Coin::from_coppers(coppers_held);
            let scaled: Vec<u32> = weights.iter().map(|w| w * scale).collect();

            prop_assert_eq!(
                allocate(purse, &weights).expect("weights sum above zero"),
                allocate(purse, &scaled).expect("weights sum above zero")
            );
        }

        /// A sole claimant takes the purse entire — no rounding, no residue.
        #[test]
        fn should_give_a_lone_weight_the_whole_purse(
            coppers_held in 0..=u64::MAX,
            weight in 1u32..=u32::MAX,
        ) {
            let parts = allocate(Coin::from_coppers(coppers_held), &[weight])
                .expect("weights sum above zero");

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
            let parts = allocate(Coin::from_coppers(coppers_held), &weights)
                .expect("weights sum above zero");

            for (part, &weight) in parts.iter().zip(weights.iter()) {
                let floor = u128::from(coppers_held) * u128::from(weight) / total;
                let given = u128::from(part.as_coppers());
                prop_assert!(given == floor || given == floor + 1);
            }
        }
    }
}
