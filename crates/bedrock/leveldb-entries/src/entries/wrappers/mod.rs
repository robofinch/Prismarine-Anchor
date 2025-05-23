//! Trivial wrappers around data (usually integers) used to distinguish their meaning,
//! aside from wrappers that are directly LevelDB values themselves.

#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct HardcodedSpawnerTypeWrapper(pub u8);
