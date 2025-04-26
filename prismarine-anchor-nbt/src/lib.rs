mod tag;
mod repr; // Used by tag module
// Note - tag module contains Debug and Display implementations for NBT -> SNBT
pub mod snbt;

pub mod io;

#[expect(unreachable_pub, reason = "I know that nothing here is publicly reachable")]
mod raw;

pub mod settings;

#[cfg(feature = "serde")]
pub mod serde;


pub use self::repr::*;
pub use self::tag::*;

// The macros might be worth looking into later.
// pub use quartz_nbt_macros::compound;
