mod tag;
mod repr; // Used by tag module
// Note - tag module contains Debug and Display implementations for NBT -> SNBT
pub mod snbt;

pub mod io;
mod raw;

pub mod encoding;
mod limits;

#[cfg(feature = "serde")]
pub mod serde;


pub use repr::*;
pub use tag::*;
pub use limits::*;


// The macros might be worth looking into later.
// pub use quartz_nbt_macros::compound;
