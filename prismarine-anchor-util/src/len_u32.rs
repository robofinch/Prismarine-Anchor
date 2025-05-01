// We use this here instead of `thiserror`, since it seems cleaner for this util crate
// to have 0 dependencies, and there isn't much boilerplate here for `thiserror` to reduce.
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};


/// Casts a `usize` to `u32`, saturating to `u32::MAX` if the `usize` is too large.
#[inline]
pub fn saturating_len_u32(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}

/// Casts a `usize` to `u32`, returning an error if the `usize` is too large.
#[inline]
pub fn lossless_len_u32(len: usize) -> Result<u32, ExcessiveLengthError> {
    u32::try_from(len).map_err(|_| ExcessiveLengthError)
}

/// Given a `usize` length, attempts to cast it to a `u32`, optionally saturating to `u32::MAX`
/// instead of returning an error.
///
/// If `len` is small enough to fit in a `u32`, then `u32` and `usize`
/// values that each have the numeric value of `len` are returned.
///
/// Otherwise, if `saturate_to_u32_max` is `false`, then an error is returned,
/// and if `true`, then  `u32` and `usize` values that each have the numeric value of
/// `u32::MAX` are returned.
#[inline]
pub fn len_u32(
    len:                 usize,
    saturate_to_u32_max: bool,
) -> Result<(u32, usize), ExcessiveLengthError> {
    if size_of::<usize>() >= size_of::<u32>() {
        let len = match u32::try_from(len) {
            Ok(len) => len,
            Err(_) => {
                if saturate_to_u32_max {
                    u32::MAX
                } else {
                    return Err(ExcessiveLengthError);
                }
            }
        };

        // This cast from u32 to usize won't overflow
        Ok((len, len as usize))
    } else {
        // This cast from usize to u32 won't overflow
        Ok((len as u32, len))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExcessiveLengthError;

impl Display for ExcessiveLengthError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "a usize length value was expected to fit in a u32, but could not")
    }
}

impl Error for ExcessiveLengthError {}
