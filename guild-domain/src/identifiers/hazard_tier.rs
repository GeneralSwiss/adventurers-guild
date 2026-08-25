#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HazardTier {
    Trivial,
    Dangerous,
    Lethal,
}
