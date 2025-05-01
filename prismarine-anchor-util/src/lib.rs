//! Utilities without functionality specific to Prismarine Anchor, which don't particularly
//! fit in other crates in this project.

mod char_conversion;
// Note that this enum_map module exports three macros
mod enum_map;
mod lock_or_panic;
mod len_u32;


pub use self::lock_or_panic::LockOrPanic;
pub use self::{
    char_conversion::{chars_to_u16, chars_to_u32, chars_to_u8, pair_to_u32},
    len_u32::{ExcessiveLengthError, saturating_len_u32, len_u32, lossless_len_u32},
};
