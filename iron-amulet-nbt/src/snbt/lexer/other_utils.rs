//! Specialized lexing functions for parsing tokens that require
//! manipulating strings and characters.

use std::array;

use crate::{settings::SnbtVersion, snbt::SnbtError};
use super::{Lexer, Token, TokenData};


// This module contains the following items:
// - Utils
// - Operation parsing
// - Escape sequence parsing


// ================================
//      Utils
// ================================
/// Returns whether a character is in `[0-9a-zA-Z]` or is `_`, `-`, `.`, or `+`,
/// which are the characters allowed to be in unquoted strings.
pub fn allowed_unquoted(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+')
}

/// Returns whether a character is in `[0-9]` or is `-`, `.`, or `+`,
/// which can be the first character of a valid integer or float tag in SNBT.
pub fn starts_unquoted_number(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, '-' | '.' | '+')
}

fn chars_to_u8(chars: [char; 2]) -> Option<u8> {
    let nibbles = [
        chars[0].to_digit(16)? as u8,
        chars[1].to_digit(16)? as u8,
    ];

    Some((nibbles[0] << 4) + nibbles[1])
}

fn chars_to_u16(chars: [char; 4]) -> Option<u16> {
    let nibbles = chars.map(|c| c.to_digit(16));

    let mut sum = nibbles[0]?;
    for i in 1..4 {
        sum = (sum << 4) +  nibbles[i]?;
    }

    Some(sum as u16)
}

fn chars_to_u32(chars: [char; 8]) -> Option<u32> {
    let nibbles = chars.map(|c| c.to_digit(16));

    let mut sum = nibbles[0]?;
    for i in 1..8 {
        sum = (sum << 4) +  nibbles[i]?;
    }

    Some(sum)
}

// Based on an answer from the Rust users forum
fn concat_arrays<T, const A: usize, const B: usize, const C: usize>(
    a: [T; A],
    b: [T; B],
) -> [T; C] {

    const {
        assert!(A + B == C, "incorrect output array length in call to `concat_arrays`");
    }

    let mut iter = a.into_iter().chain(b);
    array::from_fn(|_| iter.next().unwrap())
}


// ================================
//      Operation parsing
// ================================

// The names of the bool and uuid functions. An unquoted string prefixed with
// BOOL_FUNC or UUID_FUNC and suffixed with FUNC_SUFFIX will be interpreted as
// the bool(num) or uuid(str) operation, respectively.
//
// Note that code below assumes that any prefix/suffix match cannot overlap,
// which does hold since the the last character of the prefixes
// don't match the first character of the suffix.
const BOOL_FUNC: &str = "bool(";
const UUID_FUNC: &str = "uuid(";
const FUNC_SUFFIX: &str = ")";

impl Lexer<'_> {
    /// Parse operations (if the token_string is an operation)
    #[inline]
    pub fn try_parse_operations(
        &mut self, start: usize, char_width: usize, token_string: &str
    ) -> Option<Result<TokenData, SnbtError>> {

        if self.version == SnbtVersion::UpdatedJava && token_string.ends_with(FUNC_SUFFIX) {
            if token_string.starts_with(BOOL_FUNC) {
                return Some(self.parse_bool_func(start, char_width, &token_string));

            } else if token_string.starts_with(UUID_FUNC) {
                return Some(self.parse_uuid_func(start, char_width, &token_string));
            }
        }

        None
    }

    /// Parse the bool(..) operation
    fn parse_bool_func(
        &mut self,
        start: usize,
        char_width: usize,
        token_string: &str,
    ) -> Result<TokenData, SnbtError> {
        // Handle nested bool(bool(bool(arg))), just in case,
        // since bool does technically yield a numeric type and accept a numeric type.
        // Thankfully it's idempotent.

        let mut arg = token_string;
        let mut leading_bytes = 0;

        while arg.starts_with(BOOL_FUNC) && arg.ends_with(FUNC_SUFFIX) {
            arg = &arg[BOOL_FUNC.len() .. arg.len() - FUNC_SUFFIX.len()];
            leading_bytes += BOOL_FUNC.len();

            if let Some(whitespace) = arg.find(|c: char| !c.is_whitespace()) {
                leading_bytes += whitespace;
            }
            arg = arg.trim();
        }

        let num_index = start + leading_bytes;
        let num_char_width = arg.chars().count();

        // Make sure we don't have `bool()` or `bool(bool())`
        if arg.is_empty() {
            return Err(SnbtError::unexpected_token_at(
                self.raw,
                num_index,
                1, // the character following arg, ')', has length 1
                "a numeric value"
            ))
        };

        let numeric_tag = match self.version {
            SnbtVersion::UpdatedJava
                => self.parse_updated_numeric(num_index, num_char_width, arg),
            SnbtVersion::Original
                => self.parse_original_numeric(num_index, num_char_width, arg),
        }?;

        let nonzero = match numeric_tag.token {
            Token::Byte(n)   => n != 0,
            Token::Short(n)  => n != 0,
            Token::Int(n)    => n != 0,
            Token::Long(n)   => n != 0,
            Token::Float(n)  => n != 0.,
            Token::Double(n) => n != 0.,
            _ => unreachable!()
        };

        let boolean = if nonzero { 1 } else { 0 };

        Ok(TokenData::new(
            Token::Byte(boolean),
            start,
            char_width
        ))
    }

    /// Parse the uuid(..) operation
    fn parse_uuid_func(
        &mut self,
        start: usize,
        char_width: usize,
        token_string: &str,
    ) -> Result<TokenData, SnbtError> {
        // The UUID is likely of the form 8-4-4-4-12:
        // xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx,
        // where each 'x' is a hex digit, but this implementation also accepts no hyphens
        // (since it looks like Minecraft sometimes uses non-hyphenated hexadecimal).

        // Note that token_string starts with UUID_FUNC and ends with FUNC_SUFFIX,
        // and those matches can't overlap, so the below won't panic.
        let uuid_str_untrimmed = &token_string[
            UUID_FUNC.len() .. token_string.len() - FUNC_SUFFIX.len()
        ];

        let leading_whitespace = uuid_str_untrimmed.chars()
            .take_while(|c| c.is_whitespace())
            .count();
        let uuid_str = uuid_str_untrimmed.trim();

        let invalid_uuid = |quoted: bool| {
            let uuid_index = start + UUID_FUNC.chars().count() + leading_whitespace;
            let uuid_width = uuid_str.chars().count();
            if quoted {
                SnbtError::invalid_uuid(
                    self.raw,
                    uuid_index + 1,
                    uuid_width - 2
                )
            } else {
                SnbtError::invalid_uuid(
                    self.raw,
                    uuid_index,
                    uuid_width
                )
            }
        };

        // Up to 32 digits; up to 4 hyphens; up to 2 quotes; extra character to detect
        // invalid data with extra characters
        let mut uuid_chars: Vec<_> = uuid_str.chars().take(39).collect();
        // At least 32 digits
        if uuid_chars.len() < 32 || uuid_chars.len() > 38 {
            return Err(invalid_uuid(false));
        }

        // We checked the length, the indexing can't panic
        let quoted = matches!(
            (uuid_chars[0], uuid_chars[uuid_chars.len() - 1]),
            ('\'', '\'') | ('"', '"')
        );

        if quoted {
            // Remove the first and last characters, which are matched quotes
            uuid_chars.pop();
            uuid_chars.remove(0);
        }
        let invalid_uuid = || invalid_uuid(quoted);

        // We should now have a plain hexadecimal or hyphenated hexadecimal UUID,
        // of at least length (32 - 2) == 30

        let hyphenated = uuid_chars.get(8) == Some(&'-');

        let int_array = match (hyphenated, uuid_chars.len()) {
            (true, 36) => {
                // The hypens are at 8, 13, 18, 23. We already know the first is a hyphen,
                // so check the others.
                if uuid_chars[13] != '-' || uuid_chars[18] != '-' || uuid_chars[23] != '-' {
                    return Err(invalid_uuid());
                }

                // Split the UUID into its parts
                let first:       [char; 8] = uuid_chars[ 0 .. 8].try_into().unwrap();
                let second:      [char; 4] = uuid_chars[ 9 ..13].try_into().unwrap();
                let third:       [char; 4] = uuid_chars[14 ..18].try_into().unwrap();
                let fourth:      [char; 4] = uuid_chars[19 ..23].try_into().unwrap();
                let fifth_start: [char; 4] = uuid_chars[24 ..28].try_into().unwrap();
                let fifth_end:   [char; 8] = uuid_chars[28 ..36].try_into().unwrap();

                [
                    chars_to_u32(first),
                    chars_to_u32(concat_arrays(second, third)),
                    chars_to_u32(concat_arrays(fourth, fifth_start)),
                    chars_to_u32(fifth_end),
                ]
            },
            (false, 32) => {
                // Parse the 32 characters, which should be hex digits, in groups of 8

                [(0..8), (8..16), (16..24), (24..32)].map(|s| {
                    chars_to_u32(uuid_chars[s].try_into().unwrap())
                })
            },
            _ => return Err(invalid_uuid())
        };

        // This would be shorter if Try were stabilized
        let int_array = [
            int_array[0].ok_or_else(invalid_uuid)? as i32,
            int_array[1].ok_or_else(invalid_uuid)? as i32,
            int_array[2].ok_or_else(invalid_uuid)? as i32,
            int_array[3].ok_or_else(invalid_uuid)? as i32,
        ];

        // Convert int_array into tokens to pass back.
        // Use the entire token_string for the sake of better error messages
        // if an integer array isn't expected.
        // Note that self.peek_stack first reads its later elements,
        // so the tokens are in reverse order compared to how they will be read.
        let tokens = [
            Token::ClosedSquare,
            Token::Int(int_array[3]),
            Token::Comma,
            Token::Int(int_array[2]),
            Token::Comma,
            Token::Int(int_array[1]),
            Token::Comma,
            Token::Int(int_array[0]),
            Token::Semicolon,
            Token::String {
                value: "I".to_owned(),
                quoted: false
            }
        ];
        let first_token = Token::OpenSquare;

        // self.peek_stack should be empty if this function is called,
        // but it can't hurt to future-proof this implementation if that assumption changes.
        self.peek_stack.splice(0..0, tokens.into_iter().map(|token| {
            Ok(TokenData {
                token,
                index: start,
                char_width
            })
        }));

        Ok(TokenData {
            token: first_token,
            index: start,
                char_width
        })
    }
}


// ================================
//      Escape sequence parsing
// ================================

impl Lexer<'_> {
    /// Parses the body of an escape sequence (i.e., excluding the initial backslash),
    /// and returns the character indicated by the escape as well as the number
    /// of characters in the escape sequence's body.
    /// `index` should be the index of the escape sequence's start, i.e., the backslash.
    pub fn parse_escape_sequence(
        &mut self,
        index: usize,
    ) -> Result<(char, usize), SnbtError> {
        // Note that in order to try to produce a more useful error message,
        // the function doesn't try to bail out as soon as possible;
        // instead, it tries to get as far as possible.

        // Also, some of the below char_width usize's for error messages
        // do NOT exclude the backslash

        let snbt_version = self.version;
        // Note that the compiler can inline closures, the below is practically just shorthand.
        let check_supported: _ = |escaped: char, parsed_width: usize| {
            match snbt_version {
                SnbtVersion::UpdatedJava => Ok((escaped, parsed_width)),
                SnbtVersion::Original    => Err(SnbtError::unsupported_escape_sequence(
                    self.raw,
                    index,
                    parsed_width + 1,
                ))
            }
        };

        // This massive match is the return value
        match self.next_ch() {
            Some(ch @ ('\'' | '"' | '\\')) => Ok((ch, 1)),
            Some('b') => check_supported('\x08', 1),
            Some('s') => check_supported('\x20', 1),
            Some('t') => check_supported('\x09', 1),
            Some('n') => check_supported('\x0a', 1),
            Some('f') => check_supported('\x0c', 1),
            Some('r') => check_supported('\x0d', 1),
            Some('x') => {
                // Read two characters and parse
                // The function calls to create errors are cheap and will probably be inlined
                #[allow(clippy::or_fun_call)]
                let chars = [
                    self.next_ch().ok_or(SnbtError::unexpected_eos(
                        "two-character hex unicode value",
                    ))?,
                    self.next_ch().ok_or(SnbtError::unexpected_eos(
                        "two-character hex unicode value",
                    ))?,
                ];

                let utf_val = chars_to_u8(chars).ok_or_else(|| SnbtError::unexpected_token_at(
                    self.raw,
                    index + 2, // Skip the '\\' and 'x', which are each byte length 1
                    2,
                    "two hexadecimal digits",
                ))? as u32;

                let escaped = char::from_u32(utf_val)
                    .ok_or(SnbtError::unknown_escape_sequence(
                        self.raw,
                        index,
                        4,
                    ))?;
                check_supported(escaped, 3)
            }
            Some('u') => {
                let mut get_char = || {
                    // The function calls to create errors are cheap and will probably be inlined
                    #[allow(clippy::or_fun_call)]
                    self.next_ch().ok_or(SnbtError::unexpected_eos(
                        "four-character hex unicode value",
                    ))
                };

                let chars = [get_char()?, get_char()?, get_char()?, get_char()?];

                let utf_val = chars_to_u16(chars).ok_or_else(|| {
                    SnbtError::unexpected_token_at(
                        self.raw,
                        index + 2, // Skip the '\\' and 'u', which are each byte length 1
                        4,
                        "four hexadecimal digits",
                    )
                })? as u32;

                let escaped = char::from_u32(utf_val)
                    .ok_or(SnbtError::unknown_escape_sequence(
                        self.raw,
                        index,
                        6,
                    ))?;
                check_supported(escaped, 5)
            }
            Some('U') => {
                let mut get_char = || {
                    // The function calls to create errors are cheap and will probably be inlined
                    #[allow(clippy::or_fun_call)]
                    self.next_ch().ok_or(SnbtError::unexpected_eos(
                        "eight-character hex unicode value",
                    ))
                };

                let chars = [
                    get_char()?, get_char()?, get_char()?, get_char()?,
                    get_char()?, get_char()?, get_char()?, get_char()?,
                ];

                let utf_val = chars_to_u32(chars).ok_or_else(|| {
                    SnbtError::unexpected_token_at(
                        self.raw,
                        index + 2, // Skip the '\\' and 'U', which are each byte length 1
                        8,
                        "eight hexadecimal digits",
                    )
                })? as u32;

                let escaped = char::from_u32(utf_val)
                    .ok_or(SnbtError::unknown_escape_sequence(
                        self.raw,
                        index,
                        10,
                    ))?;
                check_supported(escaped, 9)
            }
            Some('N') => {
                // Get the name into a string
                if let Some(ch) = self.next_ch() {
                    if ch != '{' {
                        return Err(SnbtError::unexpected_token_at(
                            self.raw,
                            index,
                            1,
                            "an opening curly bracket"
                        ))
                    }
                } else {
                    return Err(SnbtError::unexpected_eos(
                        "a named unicode character escape"
                    ))
                }

                let mut sequence_char_width = 1;
                loop {
                    if let Some(ch) = self.next_ch() {

                        sequence_char_width += 1;

                        if ch == '}' {
                            break;
                        }

                    } else {
                        return Err(SnbtError::unmatched_brace(
                            self.raw,
                            // index would be '\\', index+1 is 'N', and index+2 is '{'
                            index + 2
                        ))
                    }
                }

                #[cfg(feature = "named_escapes")]
                {
                    // skip '\\', 'N', '{'
                    let name_start = index + 3;
                    // ignore '}'
                    let name_end = self.index - 1;

                    // The function calls to create errors are cheap and will probably be inlined
                    #[allow(clippy::or_fun_call)]
                    let escaped = unicode_names2::character(
                        &self.raw[name_start..name_end]
                    ).ok_or(SnbtError::unknown_escape_sequence(
                        self.raw,
                        index,
                        sequence_char_width
                    ))?;

                    check_supported(escaped, sequence_char_width-1)
                }
                #[cfg(not(feature = "named_escapes"))]
                {
                    Err(SnbtError::named_escape_sequence(
                        self.raw,
                        index,
                        sequence_char_width
                    ))
                }
            }
            Some(_) => Err(SnbtError::unknown_escape_sequence(
                self.raw,
                index,
                2
            )),
            None => Err(SnbtError::unexpected_eos("a character escape sequence"))
        }
    }
}
