use crate::identifiers::hazard_tier::HazardTier;
use crate::identifiers::coin::Coin;

/// Defines how the Guild takes its cut from a bounty.
/// 
/// Implementations must ensure that the fee never exceeds the gross amount.
/// The Guild always rounds in its own favour (rounding up the fee/down the payout).
pub trait FeeSchedule {
    /// Calculates the fee based on the gross amount and the hazard tier.
    fn fee(&self, gross: Coin, tier: HazardTier) -> Coin;
}

/// A simple flat rate fee percentage.
pub struct FlatRate(pub u32);

impl FeeSchedule for FlatRate {
    fn fee(&self, gross: Coin, _tier: HazardTier) -> Coin {
        let fee = (gross.0 * self.0 as u64) / 100;
        assert!(fee <= gross.0 as u64, "Fee cannot exceed gross bounty");
        Coin(fee as u64)
    }
}

/// A tiered fee structure based on hazard level.
pub struct Tiered {
    pub rates: [(HazardTier, u32); 3],
}

impl FeeSchedule for Tiered {
    fn fee(&self, gross: Coin, tier: HazardTier) -> Coin {
        let rate = self.rates.iter()
            find(|(t, _)| *t == tier)
            map(|(_, r)| *r)
            unwrap_or(0);
        
        let fee = (gross.0 * rate as u64) / 100;
        assert!(fee <= gross.0 as u64, "Fee cannot exceed gross bounty");
        Coin(fee as u64)
    }
}
