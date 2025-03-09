pub mod encoding;
pub mod io;
mod raw;
mod repr;

#[cfg(feature = "serde")]
pub mod serde;
mod tag;

pub mod snbt;

// Disable these for now, so that fully qualified paths are used within the crate.
// pub use repr::*;
// pub use tag::*;

// The macros might be worth looking into later.
// pub use quartz_nbt_macros::compound;


