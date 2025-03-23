//! Specialized lexing functions for parsing numeric tokens.

use std::{str::Chars, iter::Peekable};

use thiserror::Error;

use super::super::SnbtError;
use super::{Lexer, Token, TokenData};


// This module contains:
// - Lexer functions
// - Three main numeric parsing functions
// - Functions for calculating tokens from parsed information
// - Helper functions
// - Numeric parse error

#[derive(Debug)]
enum IntSuffix {
    B,
    S,
    I,
    L,
    None,
}

type CharIter<'a> = Peekable<Chars<'a>>;

// ================================================================
//      Format Overviews
// ================================================================

// Overview of updated spec:

// Below, "digits" could include underscores, and radix is assumed.
// However, underscores can't be at the start or end of a sequence of digits.
// The notation [a | b] denotes that, optionally, an 'a' or 'b' (but not both)
// may be there. Any letter except 'x' in "0x" or 'b' in "0b" may be upper or lower case.
//
// Parts of an int: (no leading 0 in digits, unless it's the only digit)
// [+ | -][0x | 0b] digits [u | s][B | S | I | L]
//
// Floats are more complicated. If it has no digits after the decimal:
// (note that if neither the period, e, or suffix were there, it would be an int,
// so int parsing needs to happen first)
// [+ | -] digits [.] [e [+ | -] digits] [F | D]
//
// If it has digits after the decimal:
// [+ | -] [digits] . digits [e [+ | -] digits] [F | D]
//
// Then, after performing the above analysis, we must translate it into actual values.
// Fun!
//
// Note that the parsing can error early if, e.g., the integer parser notices the string is
// just [+ | -], or if a parse error occurs after a radix [0x | 0b] is specified
// as floats don't support different bases, making integers the only options.
//
// By the way, the above description allows for something like `-0xFFFsi` being a negative int.
// Combining a sign with hex notation (so that the hex doesn't match the 2's-complemented-bits)
// feels slightly cursed (and it's not the default), but the spec seems to support it.

// Overview of original spec:

// The original spec is simpler. Below, "digits" contains a nonempty string of
// decimal digits and [a | b] denotes an optional value of "a" or "b",
// and type suffixes may be upper or lower case.
//
// Floats have more parts that can be included / excluded. They're basically the same as
// updated floats, but without underscores or e.
//
// If it has no digits after the decimal:
// (note that if neither the period nor suffix were there, it would be an int,
// so int parsing needs to happen first)
// [+ | -] digits [.] [F | D]
//
// If it has digits after the decimal:
// [+ | -] [digits] . digits [F | D]
//
// Note that it isn't explicitly said on minecraft.wiki which parts of "digits.digits"
// can be excluded on the old format, but as best as I can tell, Minecraft parses such
// data leniently. The 1.21.5 spec seems to imply that it was invalid. It might be a similar
// deal to whitespace, where we trim whitespace here, but whitespace may technically be invalid.

// ================================================================
//      Lexer functions
// ================================================================

impl Lexer<'_> {
    /// Parses a numeric token, in the UpdatedJava version. See numeric module source for details.
    pub fn parse_updated_numeric(
        &self,
        index: usize,
        char_width: usize,
        num_string: &str,
    ) -> Result<TokenData, SnbtError> {

        let result = try_parse_updated_int(num_string)
            .unwrap_or_else(||
                parse_float(num_string, true, true, self.opts.replace_non_finite, true)
            );

        result
            .map(|token| TokenData::new(token, index, char_width))
            .map_err(|err| SnbtError::invalid_number(num_string, index, char_width, err))
    }

    /// Parses a numeric token, in the Original version. See numeric module source for details.
    pub fn parse_original_numeric(
        &self,
        index: usize,
        char_width: usize,
        num_string: &str,
    ) -> Result<TokenData, SnbtError> {

        let result = try_parse_original_int(num_string)
            .unwrap_or_else(||
                parse_float(num_string, false, false, self.opts.replace_non_finite, false)
            );

        result
            .map(|token| TokenData::new(token, index, char_width))
            .map_err(|err| SnbtError::invalid_number(num_string, index, char_width, err))
    }
}

// ================================================================
//      Central parsing functions: updated int, original int, float
// ================================================================

/// Tries to parse `num_string` into an integer token in the UpdatedJava version.
/// Note that this function is not entirely decoupled from float parsing,
/// as whether it returns `None` or `Some(Err(..))`
/// depends on whether the value can be confirmed invalid for only integers
/// or for both floats and integers. Integer parsing should occur before float parsing.
fn try_parse_updated_int(num_string: &str) -> Option<Result<Token, NumericParseError>> {

    let mut chars = num_string.chars().peekable();

    let positive_sign = match read_sign(&mut chars) {
        Some(s) => s,
        // Empty `num_string` is invalid
        None => return Some(Err(NumericParseError::EmptyString))
    };

    // Hilariously, the bulk of the `try_parse_int` function is actually just reading the radix
    // (though of course work is offloaded to helper functions)
    let radix = match chars.peek() {
        Some('0') => {
            // This could be a digit or the start of a radix indicator
            // (or an invalid leading zero; if it's a digit it needs to be the only digit.)
            chars.next();
            match chars.peek() {
                Some('x') => {
                    chars.next();
                    16
                }
                Some('b') => {
                    chars.next();
                    2
                }
                Some(_) => {
                    // Leading zeroes are prohibited, so this better be the end.
                    if let Some((_, suffix)) = finish_integer(chars, 10) {
                        return Some(Ok(
                            match suffix {
                                IntSuffix::B    => Token::Byte(0),
                                IntSuffix::S    => Token::Short(0),
                                IntSuffix::I    => Token::Int(0),
                                IntSuffix::L    => Token::Long(0),
                                IntSuffix::None => Token::UnsuffixedInt(0),
                            }
                        ));
                    } else {
                        // Could still be a valid float, so don't error.
                        return None;
                    };
                }
                None => {
                    // The input is [+ | -]0 with no other digits.
                    // This is indeed an integer, it's zero.
                    return Some(Ok(Token::UnsuffixedInt(0)))
                }
            }
        }
        Some(_) => 10,
        // The entire string is just [+ | -] (it's nonempty, but empty after
        // we read a plus or minux). That's not a valid number.
        None => return Some(Err(NumericParseError::NoDigits))
    };

    // Notice that if we get here, we've only consumed the [+ | -]
    // and the radix specifier. If there was a leading 0, we returned already.
    // Digits come next.

    let (digits, read_result) = read_digits(
        &mut chars, radix, true, false, |_| false
    );

    // A helper function is a machine that converts code duplication
    // into analyzing how to use the helper function.
    match read_result {
        // We reached the end, the string is [+ | -][0x | 0b],
        // which isn't a valid integer or float.
        ReadDigitsResult::FirstPeekedNone => return Some(Err(NumericParseError::NoDigits)),
        // Either we read a sign/suffix or a completely invalid character.
        // We haven't read any digits yet, and underscores may only come between digits.
        // This is an error, but it could be a valid float.
        ReadDigitsResult::FirstUnlistedChar(_) => return None,
        // The function reached its final loop and halted normally, either cause is fine.
        ReadDigitsResult::LoopPeekedNone => {},
        ReadDigitsResult::LoopNonDigit   => {},
        // An underscore happened at the start or end of a sequence of digits.
        // The digits ending right after an underscore are probably an issue for floats, too,
        // since the only edge case is probably hex or binary (e.g., `1_2` would be an issue in
        // radix 2, but not decimal), but floats can't have 'x' or 'b' in them.
        // It's easier to not think too hard, and let it try to parse as a float.
        // TODO: after thorough testing is set up, check if returning an error here is valid.
        ReadDigitsResult::InvalidUnderscore => return None,
    }

    let (unsigned, suffix) = finish_integer(chars, radix)?;

    Some(integral_value(positive_sign, radix, digits, unsigned, suffix))
}

/// Tries to parse `num_string` into an integer token in the Original version.
/// Note that this function is not entirely decoupled from float parsing,
/// as whether it returns `None` or `Some(Err(..))`
/// depends on whether the value can be confirmed invalid for only integers
/// or for both floats and integers. Integer parsing should occur before float parsing.
fn try_parse_original_int(num_string: &str) -> Option<Result<Token, NumericParseError>> {

    let mut chars = num_string.chars().peekable();

    let positive_sign = match read_sign(&mut chars) {
        Some(s) => s,
        // Empty `num_string` is invalid
        None => return Some(Err(NumericParseError::EmptyString))
    };

    // Read digits
    let (digits, read_result) = read_digits(
        &mut chars, 10, false, false, |_| false
    );

    match read_result {
        // We don't read a single character before reaching end of input;
        // the string is [+ | -] which is invalid for floats as well.
        ReadDigitsResult::FirstPeekedNone => return Some(Err(
            NumericParseError::NoDigits
        )),
        // We read some other character before a digit.
        // Could be a period, which floats could handle.
        ReadDigitsResult::FirstUnlistedChar(_) => return None,
        // We read at least one character, and then entered and exited the loop normally.
        // Either cause is fine.
        ReadDigitsResult::LoopPeekedNone => {},
        ReadDigitsResult::LoopNonDigit   => {},
        // The original SNBT version does not permit underscores at all
        ReadDigitsResult::InvalidUnderscore => return Some(Err(
            NumericParseError::InvalidUnderscore
        )),
    }

    // Read the suffix
    let suffix = match chars.next() {
        Some('b' | 'B') => IntSuffix::B,
        Some('s' | 'S') => IntSuffix::S,
        Some('i' | 'I') => IntSuffix::I,
        Some('l' | 'L') => IntSuffix::L,
        None            => IntSuffix::None,
        // Some sort of invalid character (for an integer, anyway).
        // Could be a period, which floats could handle.
        Some(_) => return None
    };

    Some(integral_value(positive_sign, 10, digits, false, suffix))
}

/// Tries to parse `num_string` into a floating-point token. See module source for details.
fn parse_float(
    num_string: &str,
    allow_underscores: bool,
    allow_exponent: bool,
    replace_non_finite: bool,
    require_finite: bool,
) -> Result<Token, NumericParseError> {

    let mut chars = num_string.chars().peekable();

    // Empty `num_string` is invalid.
    let positive_sign = read_sign(&mut chars).ok_or(NumericParseError::EmptyString)?;

    // Read the digits before the decimal point
    let (integral_digits, read_result) = read_digits(
        &mut chars, 10, allow_underscores, false, |ch| ch == '.'
    );

    match read_result {
        // We reached the end, the string is [+ | -], which isn't a valid float.
        ReadDigitsResult::FirstPeekedNone => return Err(NumericParseError::NoDigits),
        // Either we read 'e', a suffix, or a completely invalid character.
        // We hadn't read any digits yet, so this is invalid syntax.
        ReadDigitsResult::FirstUnlistedChar(_) => return Err(NumericParseError::NoDigits),
        // The function reached its final loop and halted normally, either cause is fine.
        ReadDigitsResult::LoopPeekedNone => {},
        ReadDigitsResult::LoopNonDigit   => {},
        // Underscores are not allowed to be the first or last character of a sequence
        // of digits and underscores, and might be disallowed altogether
        ReadDigitsResult::InvalidUnderscore => return Err(NumericParseError::InvalidUnderscore),
    }

    // We've read the sign (if any), and the integral digits (if any).
    // Next, consume the period (if there is one). It's fine if there isn't, since
    // we parsed integers first.
    if chars.peek() == Some(&'.') {
        chars.next();
    }

    // Read the digits after the decimal point (if any).
    let (fractional_digits, read_result) = read_digits(
        &mut chars,
        10,
        allow_underscores,
        // If `integral_digits` is nonempty, then reading `None` here and
        // getting an empty `fractional_digits` vector is fine.
        !integral_digits.is_empty(),
        // If we read an 'e' or suffix as the first character, then there are no fractional
        // digits, and that's fine... if integral_digits is nonempty.
        |ch| matches!(ch, 'e' | 'E' | 'f' | 'F' | 'd' | 'D')
    );

    match read_result {
        // `integral_digits` was empty, and we read `None`.
        ReadDigitsResult::FirstPeekedNone => return Err(NumericParseError::NoDigits),
        // We read some invalid character (not 'e', a suffix, an underscore, a digit)
        ReadDigitsResult::FirstUnlistedChar(ch) => return Err(
            NumericParseError::InvalidFloatCharacter(ch)
        ),
        // We read at least one character and halted in the loop as normal. That's fine.
        ReadDigitsResult::LoopPeekedNone => {},
        ReadDigitsResult::LoopNonDigit   => {},
        // Underscores are not allowed to be the first or last character of a sequence
        // of digits and underscores, and might be disallowed altogether
        ReadDigitsResult::InvalidUnderscore => return Err(NumericParseError::InvalidUnderscore),
    }

    // We might have read no digits by reading 'e' or a suffix as the first character
    if integral_digits.is_empty() && fractional_digits.is_empty() {
        return Err(NumericParseError::InvalidUnderscore)
    }

    // We've read [+ | -][digits][.][digits] and the digits aren't both empty.
    // Next, check for 'e'
    let exp = if matches!(chars.peek(), Some(&'e' | &'E')) {
        // Check if the exponent is allowed.
        if !allow_exponent {
            return Err(NumericParseError::ExponentProhibited)
        }

        // consume the 'e'. Begin parsing the exponent by checking its sign
        chars.next();

        // An empty exponent is invalid
        let exp_sign = read_sign(&mut chars).ok_or(NumericParseError::EmptyExponent)?;

        let (exp_digits, read_result) = read_digits(
            &mut chars,
            10,
            allow_underscores,
            false,
            |_| false
        );

        match read_result {
            // In either of these first two cases, `exp_digits` is empty, which isn't allowed
            ReadDigitsResult::FirstPeekedNone   => return Err(NumericParseError::NoExponentDigits),
            // We either read a suffix or an invalid character.
            ReadDigitsResult::FirstUnlistedChar(ch) => {
                return Err(match ch {
                    'f' | 'F' | 'd' | 'D' => NumericParseError::NoExponentDigits,
                    _ => NumericParseError::InvalidExponentCharacter(ch),
                })
            }
            // These last three are the same as usual
            ReadDigitsResult::LoopPeekedNone    => {},
            ReadDigitsResult::LoopNonDigit      => {},
            ReadDigitsResult::InvalidUnderscore => return Err(NumericParseError::InvalidUnderscore),
        }

        Some((exp_sign, exp_digits))
    } else {
        None
    };

    // Lastly, read the suffix.
    let is_double = match chars.next() {
        Some('d' | 'D') | None => true,
        Some('f' | 'F') => false,
        Some(ch) => return Err(NumericParseError::InvalidFloatSuffix(ch)),
    };

    // If there's any characters left, that's an error.
    if chars.next().is_some() {
        return Err(NumericParseError::AdditionalCharacters(1 + chars.count()))
    }

    float_value(
        positive_sign, integral_digits, fractional_digits, exp,
        is_double, replace_non_finite, require_finite
    )
}


// ================================================================
//      Calculations of values
// ================================================================

fn integral_value(
    positive_sign: bool,
    radix: u32,
    digits: Vec<u8>,
    unsigned: bool,
    suffix: IntSuffix,
) -> Result<Token, NumericParseError> {

    // First, try to read the digits into a u64. We can worry about the rest later.
    let mut num: u64 = 0;
    for digit in digits {
        num = num
            .checked_mul(u64::from(radix)).ok_or(NumericParseError::IntegerTooLarge)?
            .checked_add(u64::from(digit)).ok_or(NumericParseError::IntegerTooLarge)?;
    }

    let pos_oor = |expected_type: &'static str| {
        NumericParseError::OutOfRangeInteger {
            negative: false,
            num,
            expected_type,
        }
    };
    let neg_oor = |expected_type: &'static str| {
        NumericParseError::OutOfRangeInteger {
            negative: false,
            num,
            expected_type,
        }
    };

    Ok(match (positive_sign, unsigned) {
        // The int has to fit in a smaller number.
        // Note that the "as i8/i16/i32" are no-ops added to make "as i64" sign-extend.
        (true, true) => match suffix {
            IntSuffix::B => Token::Byte(  u8::try_from(num).map_err(|_| pos_oor("u8"))? as i8),
            IntSuffix::S => Token::Short(u16::try_from(num).map_err(|_| pos_oor("u16"))? as i16),
            IntSuffix::I => Token::Int(  u32::try_from(num).map_err(|_| pos_oor("u32"))? as i32),
            // The full range is allowed
            IntSuffix::L => Token::Long(num as i64),
            IntSuffix::None => Token::UnsuffixedInt(num as i64),
        },

        // The negative half of the range would have a minus sign
        (true, false) => match suffix {
            IntSuffix::B => Token::Byte(  i8::try_from(num).map_err(|_| pos_oor("i8"))?),
            IntSuffix::S => Token::Short(i16::try_from(num).map_err(|_| pos_oor("i16"))?),
            IntSuffix::I => Token::Int(  i32::try_from(num).map_err(|_| pos_oor("i32"))?),
            IntSuffix::L => Token::Long( i64::try_from(num).map_err(|_| pos_oor("i64"))?),
            IntSuffix::None => Token::UnsuffixedInt(
                i64::try_from(num).map_err(|_| pos_oor("i64"))?
            ),
        }

        // -0 is the only unsigned integer with a minus sign
        (false, true) => if num == 0 {
            match suffix {
                IntSuffix::B    => Token::Byte(0),
                IntSuffix::S    => Token::Short(0),
                IntSuffix::I    => Token::Int(0),
                IntSuffix::L    => Token::Long(0),
                IntSuffix::None => Token::UnsuffixedInt(0),
            }
        } else {
            return Err(NumericParseError::NegativeUnsignedInteger(num))
        },

        // The value is signed and has a minus sign in front. We can have, speaking in
        // terms of mathematical value (ignoring concerns about bit widths) in the i8 case,
        // -0 >= -num >= i8::MIN = -(i8::MAX + 1).
        (false, false) => match suffix {
            IntSuffix::B => if num <= i8::MAX as u64 + 1 {
                Token::Byte((num as i8).wrapping_neg())
            } else {
                return Err(neg_oor("i8"))
            },
            IntSuffix::S => if num <= i16::MAX as u64 + 1 {
                Token::Short((num as i16).wrapping_neg())
            } else {
                return Err(neg_oor("i16"))
            },
            IntSuffix::I => if num <= i32::MAX as u64 + 1 {
                Token::Int((num as i32).wrapping_neg())
            } else {
                return Err(neg_oor("i32"))
            },
            // Note i64::MAX is less than u64::MAX, this doesn't overflow
            IntSuffix::L => if num <= i64::MAX as u64 + 1 {
                Token::Long((num as i64).wrapping_neg())
            } else {
                return Err(neg_oor("i64"))
            },
            IntSuffix::None => if num <= i64::MAX as u64 + 1 {
                Token::UnsuffixedInt((num as i64).wrapping_neg())
            } else {
                return Err(neg_oor("i64"))
            },
        }
    })
}

fn float_value(
    positive_sign: bool,
    integral_digits: Vec<u8>,
    fractional_digits: Vec<u8>,
    exp: Option<(bool, Vec<u8>)>,
    is_double: bool,
    replace_non_finite: bool,
    require_finite: bool,
) -> Result<Token, NumericParseError> {

    let mut num: f64 = 0.;

    // No need to worry about panics, thanks to infinity.
    for digit in integral_digits {
        num = (num * 10.) + digit as f64;
    }

    let mut factor = 0.1;
    for digit in fractional_digits {
        num += digit as f64 * factor;
        factor *= 0.1;
    }

    if !positive_sign {
        num = -num;
    }

    if num == 0. {
        // Don't bother with the exponent stuff.
        return Ok(if is_double {
            Token::Double(num)
        } else {
            Token::Float(num as f32)
        })
    }

    if let Some((exp_sign, exp_digits)) = exp {

        // Allow leading zeroes in the exponent digits. Technically this is an
        // "integral value", sort of, but the spec makes it seem like this is fine.
        let digits = exp_digits.into_iter().skip_while(|d| d == &0);

        // Parse the first six exponent digits after the leading zeroes into a i32. If the exponent has
        // more than six digits,
        // then it's more than enough to push any nonzero non-NaN value to an infinity.
        // It might have zero nonzero digits, in which case the exponent is 0, which does nothing.

        // Even if someone put `.0000000[...]00001e99999999[...]`, the fractional part would
        // round to zero. Sure, that just means the analytically correct thing would require
        // parsing the exponent and coefficient at the same time.
        // NOTE: this would probably be a funny edge case for a test. But seriously, nobody
        // should actually use `.000000[ 400 zeroes ]01e500` and expect it to work.

        // Also, these calculations won't overflow since 999_999 is much less than 2^31
        let mut exponent: i32 = 0;
        for digit in digits.take(6) {
            exponent = exponent * 10 + digit as i32;
        }

        if !exp_sign {
            exponent = -exponent;
        }

        num = num.powi(exponent);
    }

    if is_double {
        let num = if replace_non_finite {
            if num.is_finite() {
                num

            } else if num.is_infinite() {
                if num > 0. {
                    f64::MAX
                } else {
                    f64::MIN
                }

            } else {
                // NaN
                0.
            }
        } else {
            num
        };

        if !num.is_finite() && require_finite {
            return Err(NumericParseError::NonfiniteFloat)
        }
        Ok(Token::Double(num))

    } else {
        let num = num as f32;
        let num = if replace_non_finite {
            if num.is_finite() {
                num

            } else if num.is_infinite() {
                if num > 0. {
                    f32::MAX
                } else {
                    f32::MIN
                }

            } else {
                // NaN
                0.
            }
        } else {
            num
        };

        if !num.is_finite() && require_finite {
            return Err(NumericParseError::NonfiniteFloat)
        }
        Ok(Token::Float(num))
    }
}

// ================================================================
//      Helper functions
// ================================================================

/// Returns `Some(true)` if a plus sign or no sign was read,
/// and `Some(false)` if a minus sign was read. Consumes a `+` or `-`, otherwise peeks.
/// Returns `None` if the `chars` iterator ended.
fn read_sign(chars: &mut CharIter) -> Option<bool> {
    match chars.peek() {
        Some('+') => {
            chars.next();
            Some(true)
        }
        Some('-') => {
            chars.next();
            Some(false)
        }
        Some(_) => Some(true),
        None => None
    }
}

#[derive(Debug)]
enum ReadDigitsResult {
    FirstPeekedNone,
    FirstUnlistedChar(char),
    LoopPeekedNone,
    LoopNonDigit,
    InvalidUnderscore,
}

/// Utility function for reading in characters as digits, possibly with underscores,
/// until some condition causes the function to return.
/// The first character is treated differently, and may be skipped. If it is not skipped
/// and is not an underscore or character, then `FirstPeekedNone` or `FirstUnlistedChar`
/// is returned as appropriate.
///
/// Neither the first nor last character may be underscores. If `underscores_allowed` is false,
/// an underscore in any position will result in `InvalidUnderscore` being returned.
///
/// If the function reaches its final loop and successfully halts, then the function returns
/// whether the loop halted because end-of-input was reached (`LoopPeekedNone`)
/// or because a character that was neither a digit nor ignored underscore
/// was reached (`LoopNonDigit`).
///
/// Note that `first_chars_to_skip` is not intended to include '_' or a digit valid in the radix;
/// returning true on such characters may cause unexpected results.
/// ## Panics
/// Panics if radix is not between 2 and 36, inclusive.
fn read_digits(
    chars: &mut CharIter,
    radix: u32,
    allow_underscores: bool,
    allow_first_none: bool,
    first_chars_to_skip: impl FnOnce(char) -> bool,
) -> (Vec<u8>, ReadDigitsResult) {

    let mut digits = Vec::new();

    // The first has to be handled a bit differently.
    match chars.next() {
        // The callee can choose which first characters allow us to proceed to the loop
        Some(ch) if first_chars_to_skip(ch) => {},
        Some(ch) => {
            if let Some(digit) = ch.to_digit(10) {
                // Maximum digit value is far less than 255
                digits.push(digit as u8);

            } else if ch == '_' {
                return (digits, ReadDigitsResult::InvalidUnderscore)
            } else {
                return (digits, ReadDigitsResult::FirstUnlistedChar(ch))
            }
        }

        None => if !allow_first_none {
            return (digits, ReadDigitsResult::FirstPeekedNone)
        }
    }

    let mut last_is_underscore = false;

    // Note that if the character read above is in first_chars_to_skip and not '_' or a digit,
    // this loop will correctly not consume any characters.
    let halt_cause = loop {
        match chars.peek() {
            Some('_') => {
                if !allow_underscores {
                    return (digits, ReadDigitsResult::InvalidUnderscore)
                }
                chars.next();
                last_is_underscore = true;
            }
            Some(&ch) => {
                if let Some(digit) = ch.to_digit(radix) {
                    chars.next();
                    last_is_underscore = false;
                    // Maximum digit value is far less than 255
                    digits.push(digit as u8);

                } else {
                    break ReadDigitsResult::LoopNonDigit;
                }
            }
            None => break ReadDigitsResult::LoopPeekedNone,
        }
    };

    // No string of digits should end in an underscore.
    if last_is_underscore {
        return (digits, ReadDigitsResult::InvalidUnderscore)
    }

    (digits, halt_cause)
}

/// Helper function for UpdatedJava integer parsing.
/// If successful, returns whether the integer is unsigned and what its suffix is.
/// (True indicates unsigned, false indicates signed.)
/// A `None` return value indicates that no suffix could be parsed,
/// and this may or may not be an error.
fn finish_integer(mut input: CharIter, radix: u32) -> Option<(bool, IntSuffix)> {
    // Read the suffix before the 'U' or 'S' that might prefix it,
    // because a single 'S' should be interpreted as a Short (of some signedness),
    // not a Signed [integer].
    let suffix = match input.next_back() {
        Some('b' | 'B') => IntSuffix::B,
        Some('s' | 'S') => IntSuffix::S,
        Some('i' | 'I') => IntSuffix::I,
        None            => IntSuffix::None,
        Some('l' | 'L') => IntSuffix::L,
        Some('u' | 'U') => {
            // If we finished reading the characters, then good, we read the 'U' a bit early,
            // but that's fine. Also, suffix is `None`
            return if input.next().is_none() {
                Some((true, IntSuffix::None))
            } else {
                // Failed to parse
                None
            }
        }
        Some(_) => return None
    };

    let unsigned = match input.next() {
        Some('u' | 'U') => true,
        Some('s' | 'S') => false,
        None => radix != 10, // Decimal defaults to signed, the others to unsigned
        Some(_) => return None
    };

    if input.next().is_none() {
        Some((unsigned, suffix))
    } else {
        None
    }
}

// ================================================================
//      Numeric Parse Error
// ================================================================

#[derive(Error, Debug, Clone)]
pub enum NumericParseError {
    // Initial parsing
    #[error("empty strings are not valid numbers")]
    EmptyString,
    #[error("the numeric literal had no digits, only a sign, suffix, invalid characters, and similar")]
    NoDigits,
    #[error("underscores are only permitted in the UpdatedJava version, and must be between digits")]
    InvalidUnderscore,
    #[error("the numeric literal was not a valid integer, and '{0}' cannot occur in floats")]
    InvalidFloatCharacter(char),
    #[error(
        "the numeric literal was not a valid integer, and found '{0}' when expecting a float or double suffix")]
    InvalidFloatSuffix(char),
    #[error("the numeric literal was nearly parsed as a valid float, but had {0} additional characters at the end")]
    AdditionalCharacters(usize),
    #[error("an exponent occurred in a float literal, which the Original version does not permit")]
    ExponentProhibited,
    #[error("an empty string is not a valid exponent")]
    EmptyExponent,
    #[error("the exponent had no digits, only a sign")]
    NoExponentDigits,
    #[error("invalid character '{0}' occurred in the exponent of a float literal")]
    InvalidExponentCharacter(char),
    // Computing values
    #[error("value is a syntactically correct integer, but out of range of any integer type")]
    IntegerTooLarge,
    #[error("parsed value -{0} is not in the range of any unsigned integer")]
    NegativeUnsignedInteger(u64),
    #[error(
        "parsed value {}{} is not in the range of the expected {} type",
        if *negative { "-" } else { "" },
        num, expected_type
    )]
    OutOfRangeInteger {
        negative: bool,
        num: u64,
        expected_type: &'static str,
    },
    #[error("the floating-point value was infinite or NaN, but was required to be finite")]
    NonfiniteFloat,
}
