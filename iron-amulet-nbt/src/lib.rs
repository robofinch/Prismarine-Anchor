pub mod encoding;
pub mod io;
mod raw;

#[cfg(feature = "serde")]
pub mod serde;

mod repr; // Used by tag module
mod tag;
// Module with SNBT -> NBT parser;
// tag module contains Debug and Display implementations for NBT -> SNBT
pub mod snbt;


pub use repr::*;
pub use tag::*;


// The macros might be worth looking into later.
// pub use quartz_nbt_macros::compound;
