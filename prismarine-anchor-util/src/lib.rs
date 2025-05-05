//! Small utilities without functionality specific to Prismarine Anchor, which don't particularly
//! fit in other crates in this project.

mod char_conversion;
mod lock_or_panic;
mod len_u32;
mod inspect_none;


pub use self::{inspect_none::InspectNone, lock_or_panic::LockOrPanic};
pub use self::{
    char_conversion::{chars_to_u16, chars_to_u32, chars_to_u8, pair_to_u32},
    len_u32::{ExcessiveLengthError, saturating_len_u32, len_u32, lossless_len_u32},
};
