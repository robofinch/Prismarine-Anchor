//! Small utilities without functionality specific to Prismarine Anchor,
//! for small Rust-specific tasks.

mod char_conversion;
// Exports a small macro
mod declare_and_pub_use;
mod inspect_none;
mod lock_or_panic;
mod u64_equals_usize;
#[cfg(feature = "print_debug")]
mod print_debug;


pub use self::{
    inspect_none::InspectNone,
    lock_or_panic::LockOrPanic,
    u64_equals_usize::u64_equals_usize,
};
pub use self::char_conversion::{chars_to_u16, chars_to_u32, chars_to_u8, pair_to_u32};

#[cfg(feature = "print_debug")]
pub use self::print_debug::print_debug;
