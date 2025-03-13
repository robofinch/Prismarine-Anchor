// This module could be expanded to add one or two more limits on parsing and writing data,
// in particular in regards to length. It wouldn't be too hard to give functions here data
// that results in too much memory allocation and ultimately a crash. Is that truly important
// to fix? Compared to functional tasks, no, so it's left as an idea that would be
// time-consuming to implement across the crate.


/// The recursive NBT tags (Compounds and Lists)
/// can be nested up to (and including) 512 levels deep in the standard specification.
/// The limit may be increased here, but note that this crate uses recursive functions
/// to read and write NBT data; if the limit is too high and unreasonably nested data is received,
/// a crash could occur from the nested function calls exceeding the maximum stack size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DepthLimit(pub u32);

impl Default for DepthLimit {
    /// The maximum depth that NBT compounds and tags can be nested in the standard Minecraft specification.
    fn default() -> Self {
        Self(512)
    }
}
