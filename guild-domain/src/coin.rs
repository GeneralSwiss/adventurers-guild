//! The guild's currency: three denominations related by a fixed exchange
//! rate, each its own type so an amount can't be added or spent under the
//! wrong denomination without the compiler noticing.
//!
//! ```text
//! 100 Copper = 1 Silver
//! 100 Silver = 1 Gold   (10,000 Copper)
//! ```
//!
//! Each type stores a `u64` count of coins in its own denomination — a
//! `Gold(3)` is three gold pieces, not three copper — and the [`From`] impls
//! convert a count from one denomination to another along the rate above.
//! Converting to a larger denomination (e.g. [`Silver`] from [`Copper`])
//! truncates any remainder that doesn't divide evenly, the same way
//! exchanging a purse for the fewest possible coins would leave loose change
//! behind.

use std::{
    iter::Sum,
    ops::{Add, AddAssign},
};

/// The smallest denomination: one copper piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Copper(u64);

/// The middle denomination. Worth 100 [`Copper`]; 100 of these make one [`Gold`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Silver(u64);

/// The largest denomination. Worth 100 [`Silver`], or 10,000 [`Copper`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Gold(u64);

impl Copper {
    /// Returns the value of the Copper coin in its smallest unit.
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl Silver {
    /// Returns the value of the Silver coin in its smallest unit.
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl Gold {
    /// Returns the value of the Gold coin in its smallest unit.
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for Copper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}c", self.0)
    }
}

impl std::fmt::Display for Silver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}s", self.0)
    }
}

impl std::fmt::Display for Gold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}g", self.0)
    }
}

impl From<Copper> for Silver {
    fn from(copper: Copper) -> Self {
        Silver(copper.0 / 100)
    }
}

impl From<Silver> for Gold {
    fn from(silver: Silver) -> Self {
        Gold(silver.0 / 100)
    }
}

impl From<Gold> for Copper {
    fn from(gold: Gold) -> Self {
        Copper(gold.0 * 10000)
    }
}

impl From<Copper> for Gold {
    fn from(value: Copper) -> Self {
        Gold(value.0 / 10000)
    }
}

impl From<Gold> for Silver {
    fn from(value: Gold) -> Self {
        Silver(value.0 * 100)
    }
}

impl From<Silver> for Copper {
    fn from(value: Silver) -> Self {
        Copper(value.0 * 100)
    }
}

impl From<u64> for Copper {
    fn from(value: u64) -> Self {
        Copper(value)
    }
}

impl From<u64> for Silver {
    fn from(value: u64) -> Self {
        Silver(value)
    }
}

impl From<u64> for Gold {
    fn from(value: u64) -> Self {
        Gold(value)
    }
}

impl Add for Copper {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Copper(self.0 + other.0)
    }
}

impl Add for Silver {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Silver(self.0 + other.0)
    }
}

impl Add for Gold {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Gold(self.0 + other.0)
    }
}

impl AddAssign for Copper {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl AddAssign for Silver {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl AddAssign for Gold {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl Sum for Copper {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Copper(0), |acc, x| acc + x)
    }
}

impl Sum for Silver {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Silver(0), |acc, x| acc + x)
    }
}

impl Sum for Gold {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Gold(0), |acc, x| acc + x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copper_from_silver() {
        let silver = Silver(2000);
        let copper = Copper::from(silver);
        assert_eq!(copper.0, 200000);
    }

    #[test]
    fn test_silver_from_gold() {
        let gold = Gold(50);
        let silver = Silver::from(gold);
        assert_eq!(silver.0, 5000);
    }

    #[test]
    fn test_gold_from_copper() {
        let copper = Copper(500000);
        let gold = Gold::from(copper);
        assert_eq!(gold.0, 50);
    }

    #[test]
    fn test_silver_from_copper() {
        let copper = Copper(2000);
        let silver = Silver::from(copper);
        assert_eq!(silver.0, 20);
    }

    #[test]
    fn test_gold_from_silver() {
        let silver = Silver(500);
        let gold = Gold::from(silver);
        assert_eq!(gold.0, 5);
    }

    #[test]
    fn test_copper_from_gold() {
        let gold = Gold(3);
        let copper = Copper::from(gold);
        assert_eq!(copper.0, 30000);
    }

    #[test]
    fn test_copper_display() {
        let copper = Copper(10);
        assert_eq!(format!("{}", copper), "10c");
    }

    #[test]
    fn test_silver_display() {
        let silver = Silver(20);
        assert_eq!(format!("{}", silver), "20s");
    }

    #[test]
    fn test_gold_display() {
        let gold = Gold(50);
        assert_eq!(format!("{}", gold), "50g");
    }

    #[test]
    fn test_copper_from_u64() {
        let copper: Copper = 10u64.into();
        assert_eq!(copper.0, 10);
    }

    #[test]
    fn test_silver_from_u64() {
        let silver: Silver = 20u64.into();
        assert_eq!(silver.0, 20);
    }

    #[test]
    fn test_gold_from_u64() {
        let gold: Gold = 50u64.into();
        assert_eq!(gold.0, 50);
    }

    #[test]
    fn test_copper_value() {
        let copper = Copper(10);
        assert_eq!(copper.value(), 10);
    }

    #[test]
    fn test_silver_value() {
        let silver = Silver(20);
        assert_eq!(silver.value(), 20);
    }

    #[test]
    fn test_gold_value() {
        let gold = Gold(50);
        assert_eq!(gold.value(), 50);
    }

    #[test]
    fn test_copper_default() {
        assert_eq!(Copper::default(), Copper(0));
    }

    #[test]
    fn test_silver_default() {
        assert_eq!(Silver::default(), Silver(0));
    }

    #[test]
    fn test_gold_default() {
        assert_eq!(Gold::default(), Gold(0));
    }

    #[test]
    fn test_copper_add() {
        assert_eq!(Copper(10) + Copper(5), Copper(15));
    }

    #[test]
    fn test_silver_add() {
        assert_eq!(Silver(10) + Silver(5), Silver(15));
    }

    #[test]
    fn test_gold_add() {
        assert_eq!(Gold(10) + Gold(5), Gold(15));
    }

    #[test]
    fn test_copper_add_assign() {
        let mut copper = Copper(10);
        copper += Copper(5);
        assert_eq!(copper, Copper(15));
    }

    #[test]
    fn test_silver_add_assign() {
        let mut silver = Silver(10);
        silver += Silver(5);
        assert_eq!(silver, Silver(15));
    }

    #[test]
    fn test_gold_add_assign() {
        let mut gold = Gold(10);
        gold += Gold(5);
        assert_eq!(gold, Gold(15));
    }

    #[test]
    fn test_copper_sum() {
        let total: Copper = vec![Copper(10), Copper(20), Copper(30)].into_iter().sum();
        assert_eq!(total, Copper(60));
    }

    #[test]
    fn test_silver_sum() {
        let total: Silver = vec![Silver(10), Silver(20), Silver(30)].into_iter().sum();
        assert_eq!(total, Silver(60));
    }

    #[test]
    fn test_gold_sum() {
        let total: Gold = vec![Gold(10), Gold(20), Gold(30)].into_iter().sum();
        assert_eq!(total, Gold(60));
    }

    #[test]
    fn test_copper_sum_empty() {
        let total: Copper = Vec::<Copper>::new().into_iter().sum();
        assert_eq!(total, Copper(0));
    }
}
