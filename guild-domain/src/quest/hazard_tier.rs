//! How dangerous a quest is, as a rank rather than a number.
//!
//! ```
//! use guild_domain::quest::HazardTier;
//!
//! assert!(HazardTier::Deadly > HazardTier::Venture);
//!
//! let mut board = vec![HazardTier::Mythic, HazardTier::Errand, HazardTier::Perilous];
//! board.sort();
//! assert_eq!(board.first(), Some(&HazardTier::Errand));
//! ```
//!
//! # Tier is a fact; the fee is a policy
//!
//! The Guild charges more for a Deadly quest than a Venture, and pays a
//! hazard bonus on top. Neither percentage lives here.
//!
//! A tier is a claim about the world: this quest will probably get someone
//! killed. What the Guild charges for that is a decision it makes, revises,
//! and will eventually vary by patron — and a future insurance context will
//! rate premiums off the same tier without caring what the Guild's fee
//! schedule says. Baking a number in would tie all of those together, so that
//! changing a fee meant editing a fact.
//!
//! So the percentages belong to the `FeeSchedule` and `SplitPolicy` of M4,
//! which take a tier and answer a question about it. This type only has to be
//! right about the ordering.
//!
//! # Why an ordered enum and not a number
//!
//! Ranks compare, and that is the whole of the behaviour: is this quest worse
//! than that one, and which is the worst on the board. `Ord` gives exactly
//! that, and gives nothing else — which is the point.
//!
//! A `u8` tier code would compare too, and would also add, average, and
//! multiply. `tier * 2` is meaningless and compiles; so does `tier + 1`,
//! which quietly invents a sixth tier. Ranks are ordinal, not cardinal, and
//! the type should only permit what the concept supports.
//!
//! Numeric codes also leak. Once a `3` is in the API it appears in a match
//! arm, then in a config file, then in a table somewhere, and renumbering the
//! tiers becomes a migration. There is no code here to leak: the variants are
//! the vocabulary.
//!
//! # The order is the declaration order
//!
//! `Ord` is derived, so [`Errand`](HazardTier::Errand) is least and
//! [`Mythic`](HazardTier::Mythic) greatest purely because that is the order
//! they are written in. Rearranging the variants would silently reverse
//! comparisons that M4 keys its fee and hazard bonus off.
//!
//! That is a real hazard of derived ordering, so it is pinned by a test —
//! `should_order_tiers_from_errand_up_to_mythic` sorts a shuffled list and
//! asserts the whole sequence — rather than left to a comment nobody reads.
//!
//! # Adding a tier is a compile error everywhere it matters
//!
//! No `_` arm on this enum, anywhere in the crate. A sixth tier should break
//! every place that decides something from a tier, because every one of them
//! needs a new answer:
//!
//! ```
//! use guild_domain::quest::HazardTier;
//!
//! // A fee schedule in M4 will look like this. No catch-all arm, so adding a
//! // variant stops the build here rather than defaulting the new tier to
//! // whatever the old one charged.
//! fn warning_for(tier: HazardTier) -> &'static str {
//!     match tier {
//!         HazardTier::Errand => "Wear boots.",
//!         HazardTier::Venture => "Bring a healer.",
//!         HazardTier::Perilous => "Bring two.",
//!         HazardTier::Deadly => "Write your will.",
//!         HazardTier::Mythic => "Write your ballad.",
//!     }
//! }
//!
//! assert_eq!(warning_for(HazardTier::Deadly), "Write your will.");
//! ```

/// How dangerous a quest is, from an afternoon's errand to the kind nobody
/// comes back from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HazardTier {
    /// Fetch a ledger from the next village. Danger is mostly the weather.
    Errand,
    /// Real work with real risk, and the tier most of the board sits at.
    Venture,
    /// People get hurt on these, and the Guild expects to pay for it.
    Perilous,
    /// People die on these. The party is told so before it signs.
    Deadly,
    /// The kind that gets a ballad whether or not anyone returns.
    Mythic,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_rank_a_deadlier_quest_above_a_safer_one() {
        // Names the direction the order runs in. `Ord` is derived from the
        // declaration order, so this is the reading that would silently
        // invert if someone rearranged the variants — and the fee and hazard
        // bonus in M4 both key off which way round it is.
        assert!(HazardTier::Deadly > HazardTier::Venture);
        assert!(HazardTier::Errand < HazardTier::Mythic);
    }

    #[test]
    fn should_order_tiers_from_errand_up_to_mythic() {
        let mut tiers = vec![
            HazardTier::Mythic,
            HazardTier::Errand,
            HazardTier::Deadly,
            HazardTier::Venture,
            HazardTier::Perilous,
        ];

        tiers.sort();

        assert_eq!(
            tiers,
            vec![
                HazardTier::Errand,
                HazardTier::Venture,
                HazardTier::Perilous,
                HazardTier::Deadly,
                HazardTier::Mythic,
            ]
        );
    }

    #[test]
    fn should_find_the_worst_tier_a_party_has_signed_for() {
        // The shape M4 asks in: given the quests a party is committed to,
        // which is the dangerous one. Worth pinning because `max` reads the
        // same order from the other end, and an order that is wrong in the
        // middle can still sort into a plausible-looking list.
        let signed = [
            HazardTier::Venture,
            HazardTier::Perilous,
            HazardTier::Errand,
        ];

        assert_eq!(signed.iter().max(), Some(&HazardTier::Perilous));
    }
}
