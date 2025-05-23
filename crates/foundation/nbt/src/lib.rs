mod tag;
mod repr; // Used by tag module
// Note - tag module contains Debug and Display implementations for NBT -> SNBT
pub mod snbt;

pub mod io;

#[expect(
    unreachable_pub,
    reason = "I know that nothing here is publicly reachable, no need for pub(crate) everywhere",
)]
mod raw;

mod settings;

#[cfg(feature = "serde")]
pub mod serde;


pub use self::repr::*;
pub use self::tag::*;
pub use self::settings::*;

// The macros might be worth looking into later.
// pub use quartz_nbt_macros::compound;
