//! Module for parsing SNBT into NBT data

// Note to anyone reading the source: parsing functions begin at around line 400.

mod lexer;


use std::{char, fmt, mem, str};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

use crate::{
    settings::{DepthLimit, SnbtVersion},
    tag::{NbtCompound, NbtList, NbtTag},
};
use self::lexer::{Lexer, Token, TokenData};


pub use self::lexer::{allowed_unquoted, starts_unquoted_number};


// Should add module-wide documentation about the specification and implementation.



// ================================
//      Convenience functions
// ================================

/// Parses the given string into an NBT tag.
/// See [`SnbtVersion`] for some specifics of the standard and this implementation.
#[inline]
pub fn parse_any<T: AsRef<str> + ?Sized>(
    string_nbt: &T,
    version: SnbtVersion,
) -> Result<NbtTag, SnbtError> {
    parse_any_limited(
        string_nbt,
        version,
        DepthLimit::default()
    ).map(|(tag, _)| tag)
}

/// Parses the given string into an NBT tag, using the original SNBT version.
/// See [`SnbtVersion`] for some specifics of the standard and this implementation.
#[inline]
pub fn parse_any_original<T: AsRef<str> + ?Sized>(
    string_nbt: &T,
) -> Result<NbtTag, SnbtError> {
    parse_any_limited(
        string_nbt,
        SnbtVersion::Original,
        DepthLimit::default()
    ).map(|(tag, _)| tag)
}

/// Parses the given string into an NBT tag, using the newer SNBT version.
/// See [`SnbtVersion`] for some specifics of the standard and this implementation.
#[inline]
pub fn parse_any_updated<T: AsRef<str> + ?Sized>(
    string_nbt: &T,
) -> Result<NbtTag, SnbtError> {
    parse_any_limited(
        string_nbt,
        SnbtVersion::UpdatedJava,
        DepthLimit::default()
    ).map(|(tag, _)| tag)
}

/// Parses the given string into a compound NBT tag.
/// See [`SnbtVersion`] for some specifics of the standard and this implementation.
#[inline]
pub fn parse_compound<T: AsRef<str> + ?Sized>(
    string_nbt: &T,
    version: SnbtVersion,
) -> Result<NbtCompound, SnbtError> {
    parse_compound_limited(
        string_nbt,
        version,
        DepthLimit::default()
    ).map(|(tag, _)| tag)
}

/// Parses the given string into a compound NBT tag, using the newer SNBT version.
/// See [`SnbtVersion`] for some specifics of the standard and this implementation.
#[inline]
pub fn parse_compound_updated<T: AsRef<str> + ?Sized>(
    string_nbt: &T,
) -> Result<NbtCompound, SnbtError> {
    parse_compound_limited(
        string_nbt,
        SnbtVersion::UpdatedJava,
        DepthLimit::default()
    ).map(|(tag, _)| tag)
}

/// Parses the given string into a compound NBT tag, using the original SNBT version.
/// See [`SnbtVersion`] for some specifics of the standard and this implementation.
#[inline]
pub fn parse_compound_original<T: AsRef<str> + ?Sized>(
    string_nbt: &T,
) -> Result<NbtCompound, SnbtError> {
    parse_compound_limited(
        string_nbt,
        SnbtVersion::Original,
        DepthLimit::default()
    ).map(|(tag, _)| tag)
}

// ================================
//      Actual parsing
// ================================

/// Parses the given string into a tag just like [`parse_any`],
/// but also returns the amount of parsed characters, and takes a `DepthLimit` setting.
pub fn parse_any_limited<T: AsRef<str> + ?Sized>(
    string_nbt: &T,
    version: SnbtVersion,
    depth_limit: DepthLimit,
) -> Result<(NbtTag, usize), SnbtError> {
    let mut tokens = Lexer::new(string_nbt.as_ref(), version);
    let tag = parse_next_value(&mut tokens, None)?;

    Ok((tag, tokens.index()))
}

/// Parses the given string just like [`parse_compound`],
/// but also returns the amount of parsed characters, and takes a `DepthLimit` setting.
pub fn parse_compound_limited<T: AsRef<str> + ?Sized>(
    string_nbt: &T,
    version: SnbtVersion,
    depth_limit: DepthLimit,
) -> Result<(NbtCompound, usize), SnbtError> {
    let mut tokens = Lexer::new(string_nbt.as_ref(), version);
    let open_curly = tokens.assert_next(Token::OpenCurly)?;
    parse_compound_tag(&mut tokens, &open_curly)
}

// Parses the next value in the token stream
fn parse_next_value(
    tokens: &mut Lexer<'_>,
    delimiter: Option<fn(char) -> bool>,
) -> Result<NbtTag, SnbtError> {
    let token = tokens.next(delimiter).transpose()?;
    parse_value(tokens, token)
}

/// Parses a token into a value
fn parse_value(tokens: &mut Lexer<'_>, token: Option<TokenData>) -> Result<NbtTag, SnbtError> {
    match token {
        // Open curly brace indicates a compound tag is present
        #[rustfmt::skip]
        Some(
            td @ TokenData {
                token: Token::OpenCurly,
                ..
            },
        ) => parse_compound_tag(tokens, &td).map(|(tag, _)| tag.into()),

        // Open square brace indicates that some kind of list is present
        #[rustfmt::skip]
        Some(
            td @ TokenData {
                token: Token::OpenSquare,
                ..
            },
        ) => parse_list(tokens, &td),

        // Could be a value token or delimiter token
        Some(td) => {
            td.into_tag().map_err(|td| {
                SnbtError::unexpected_token(tokens.raw(), Some(&td), "value")
            })
        },

        // We expected a value but ran out of data
        None => Err(SnbtError::unexpected_eos("value")),
    }
}

// Parses a list, which can be either a generic tag list or vector of primitives
fn parse_list(tokens: &mut Lexer<'_>, open_square: &TokenData) -> Result<NbtTag, SnbtError> {
    const DELIMITER: Option<fn(char) -> bool> = Some(|ch| matches!(ch, ',' | ']' | ';'));

    match tokens.next(DELIMITER).transpose()? {
        // Empty list ('[]') with no type specifier is treated as an empty NBT tag list
        Some(TokenData {
            token: Token::ClosedSquare,
            ..
        }) => Ok(NbtList::new().into()),

        // A string as the first "element" can either be a type specifier such as in [I; 1, 2], or
        // a regular string in a tag list, such as in ['i', 'j', 'k'],
        // or an operation in an integer list, such as in [bool(2), 4b, 0b]
        Some(TokenData {
            token:
                Token::String {
                    value: string,
                    quoted,
                },
            index,
            char_width,
        }) => {
            // Peek at the next token to see if it's a semicolon, which would indicate a primitive vector
            match tokens.peek(DELIMITER) {
                // Parse as a primitive vector
                Some(Ok(TokenData {
                    token: Token::Semicolon,
                    ..
                })) => {
                    if quoted {
                        return Err(SnbtError::unexpected_token_at(
                            tokens.raw(),
                            index,
                            char_width,
                            "'B', 'I', or 'L'",
                        ));
                    }

                    // Moves past the peeked semicolon
                    tokens.next(None);

                    // Determine the primitive type and parse it
                    match string.as_str() {
                        "b" | "B" => parse_prim_list::<u8>(tokens, open_square),
                        "i" | "I" => parse_prim_list::<i32>(tokens, open_square),
                        "l" | "L" => parse_prim_list::<i64>(tokens, open_square),
                        _ => Err(SnbtError::unexpected_token_at(
                            tokens.raw(),
                            index,
                            char_width,
                            "'B', 'I', or 'L'",
                        )),
                    }
                }

                // Parse as a tag list (token errors are delegated to this function)
                _ => parse_tag_list(tokens, NbtTag::String(string)).map(Into::into),
            }
        }

        // Any other pattern is delegated to the general tag list parser
        td => {
            let first_element = parse_value(tokens, td, )?;
            parse_tag_list(tokens, first_element).map(Into::into)
        }
    }
}

fn parse_prim_list<'a, T>(
    tokens: &mut Lexer<'a>,
    open_square: &TokenData,
) -> Result<NbtTag, SnbtError>
where
    Token: Into<Result<T, Token>>,
    NbtTag: From<Vec<T>>,
{
    let mut list: Vec<T> = Vec::new();
    // Zero is used as a niche value so the first iteration of the loop runs correctly
    let mut comma: Option<usize> = Some(0);

    loop {
        match tokens.next(Some(|ch| ch == ',' || ch == ']')).transpose()? {
            // Finish off the list
            Some(TokenData {
                token: Token::ClosedSquare,
                ..
            }) => match comma {
                Some(0) | None => return Ok(list.into()),
                Some(index) => return Err(SnbtError::trailing_comma(tokens.raw(), index)),
            },

            // Indicates another value should be parsed
            Some(TokenData {
                token: Token::Comma,
                index,
                ..
            }) => comma = Some(index),

            // Attempt to convert the token into a value
            Some(td) => {
                // Make sure a value was expected
                match comma {
                    Some(_) => {
                        match td.into_value::<T>() {
                            Ok(value) => list.push(value),
                            Err(td) =>
                                return Err(SnbtError::non_homogenous_numeric_list(
                                    tokens.raw(),
                                    td.index,
                                    td.char_width,
                                )),
                        }

                        comma = None;
                    }

                    None =>
                        return Err(SnbtError::unexpected_token(
                            tokens.raw(),
                            Some(&td),
                            Token::Comma.as_expectation(),
                        )),
                }
            }

            None => return Err(SnbtError::unmatched_brace(tokens.raw(), open_square.index)),
        }
    }
}

fn parse_tag_list(tokens: &mut Lexer<'_>, first_element: NbtTag) -> Result<NbtList, SnbtError> {
    const DELIMITER: Option<fn(char) -> bool> = Some(|ch| ch == ',' || ch == ']');

    // Construct the list and use the first element to determine the list's type
    let mut list = NbtList::new();
    let mut descrim = mem::discriminant(&first_element);
    let mut list_holds_compounds = matches!(&first_element, &NbtTag::Compound{..});
    list.push(first_element);

    loop {
        // No delimiter needed since we only expect ']' and ','
        match tokens.next(None).transpose()? {
            // Finish off the list
            Some(TokenData {
                token: Token::ClosedSquare,
                ..
            }) => return Ok(list),

            // Indicates another value should be parsed
            Some(TokenData {
                token: Token::Comma,
                ..
            }) => {
                let (index, char_width) = match tokens.peek(DELIMITER) {
                    Some(&Ok(TokenData {
                        index, char_width, ..
                    })) => (index, char_width),
                    _ => (0, 0),
                };
                let element = parse_next_value(tokens, DELIMITER)?;

                if mem::discriminant(&element) == descrim {
                    list.push(element);

                } else {
                    match tokens.snbt_version() {
                        SnbtVersion::UpdatedJava => {
                            // In order to preserve list homogeneity, convert everything
                            // to a compound tag.

                            #[inline]
                            fn to_compound(tag: NbtTag) -> NbtTag {
                                if let NbtTag::Compound{..} = tag {
                                    tag
                                } else {
                                    let mut compound = NbtCompound::with_capacity(1);
                                    compound.insert("", tag);
                                    NbtTag::Compound(compound)
                                }
                            }

                            let compounded_element = to_compound(element);

                            if !list_holds_compounds {
                                // Convert the rest of the list to compound tags
                                list = list.into_iter().map(|tag| to_compound(tag)).collect();

                                descrim = mem::discriminant(&compounded_element);
                                list_holds_compounds = true;
                            }

                            list.push(compounded_element);
                        },

                        SnbtVersion::Original =>
                            return Err(SnbtError::non_homogenous_tag_list(
                                tokens.raw(), index, char_width,
                            ))
                    };
                }
            }

            // Some invalid token
            td =>
                return Err(SnbtError::unexpected_token(
                    tokens.raw(),
                    td.as_ref(),
                    "',' or ']'",
                )),
        }
    }
}

fn parse_compound_tag<'a>(
    tokens: &mut Lexer<'a>,
    open_curly: &TokenData,
) -> Result<(NbtCompound, usize), SnbtError> {
    let mut compound = NbtCompound::new();
    // Zero is used as a niche value so the first iteration of the loop runs correctly
    let mut comma: Option<usize> = Some(0);

    loop {
        match tokens.next(Some(|ch| ch == ':')).transpose()? {
            // Finish off the compound tag
            Some(TokenData {
                token: Token::ClosedCurly,
                ..
            }) => {
                match comma {
                    // First loop iteration or no comma
                    Some(0) | None => return Ok((compound, tokens.index())),
                    // Later iteration with a trailing comma
                    Some(index) => return Err(SnbtError::trailing_comma(tokens.raw(), index)),
                }
            }

            // Parse a new key-value pair
            Some(TokenData {
                token: Token::String { value: key, .. },
                index,
                char_width,
            }) => {
                match comma {
                    // First loop iteration or a comma indicated that more data is present
                    Some(_) => {
                        tokens.assert_next(Token::Colon)?;
                        compound.insert(
                            key,
                            parse_next_value(tokens, Some(|ch| ch == ',' || ch == '}'))?,
                        );
                        comma = None;
                    }

                    // There was not a comma before this string so therefore the token is unexpected
                    None =>
                        return Err(SnbtError::unexpected_token_at(
                            tokens.raw(),
                            index,
                            char_width,
                            Token::Comma.as_expectation(),
                        )),
                }
            }

            // Denote that another key-value pair is anticipated
            Some(TokenData {
                token: Token::Comma,
                index,
                ..
            }) => comma = Some(index),

            // Catch-all for unexpected tokens
            Some(td) =>
                return Err(SnbtError::unexpected_token(
                    tokens.raw(),
                    Some(&td),
                    "compound key, '}', or ','",
                )),

            // End of file / unmatched brace
            None => return Err(SnbtError::unmatched_brace(tokens.raw(), open_curly.index)),
        }
    }
}

/// An error that occurs during the parsing process. This error contains a copy of a segment
/// of the input where the error occurred as well as metadata about the specific error. See
/// [`ParserErrorType`](crate::snbt::ParserErrorType) for the different error types.
pub struct SnbtError {
    segment: String,
    index: usize,
    error: ParserErrorType,
}

impl SnbtError {
    fn exceeded_depth_limit(limit: DepthLimit) -> Self {
        Self {
            segment: String::new(),
            index: 0,
            error: ParserErrorType::ExceededDepthLimit { limit }
        }
    }

    fn unexpected_eos(expected: &'static str) -> Self {
        Self {
            segment: String::new(),
            index: 0,
            error: ParserErrorType::UnexpectedEOS { expected },
        }
    }

    fn unexpected_token(input: &str, token: Option<&TokenData>, expected: &'static str) -> Self {
        match token {
            Some(token) => Self::unexpected_token_at(
                input, token.index, token.char_width, expected
            ),
            None => Self::unexpected_eos(expected),
        }
    }

    fn unexpected_token_at(
        input: &str,
        index: usize,
        char_width: usize,
        expected: &'static str,
    ) -> Self {
        Self {
            segment: Self::segment(input, index, char_width, 15, 0),
            index,
            error: ParserErrorType::UnexpectedToken { expected },
        }
    }

    fn unsupported_escape_sequence(input: &str, index: usize, char_width: usize) -> Self {
        Self {
            segment: Self::segment(input, index, char_width, 0, 0),
            index,
            error: ParserErrorType::UnsupportedEscapeSequence,
        }
    }

    #[cfg(not(feature = "named_escapes"))]
    fn named_escape_sequence(input: &str, index: usize, char_width: usize) -> Self {
        Self {
            segment: Self::segment(input, index, char_width, 0, 0),
            index,
            error: ParserErrorType::NamedEscapeSequence,
        }
    }

    fn unknown_escape_sequence(input: &str, index: usize, char_width: usize) -> Self {
        Self {
            segment: Self::segment(input, index, char_width, 0, 0),
            index,
            error: ParserErrorType::UnknownEscapeSequence,
        }
    }

    fn invalid_unquoted_character(input: &str, index: usize, char_width: usize, ch: char) -> Self {
        Self {
            segment: Self::segment(input, index, char_width, 10, 5),
            index,
            error: ParserErrorType::InvalidUnquotedCharacter { ch }
        }
    }

    fn invalid_number(input: &str, index: usize, char_width: usize) -> Self {
        Self {
            segment: Self::segment(input, index, char_width, 0, 0),
            index,
            error: ParserErrorType::InvalidNumber,
        }
    }

    fn invalid_uuid(input: &str, index: usize, char_width: usize) -> Self {
        Self {
            segment: Self::segment(input, index, char_width, 0, 0),
            index,
            error: ParserErrorType::InvalidUUID,
        }
    }

    fn trailing_comma(input: &str, index: usize) -> Self {
        Self {
            segment: Self::segment(input, index, 1, 15, 1),
            index,
            error: ParserErrorType::TrailingComma,
        }
    }

    fn unmatched_quote(input: &str, index: usize) -> Self {
        Self {
            segment: Self::segment(input, index, 1, 7, 7),
            index,
            error: ParserErrorType::UnmatchedQuote,
        }
    }

    fn unmatched_brace(input: &str, index: usize) -> Self {
        Self {
            segment: Self::segment(input, index, 1, 0, 15),
            index,
            error: ParserErrorType::UnmatchedBrace,
        }
    }

    fn non_homogenous_numeric_list(input: &str, index: usize, char_width: usize) -> Self {
        Self {
            segment: Self::segment(input, index, char_width, 15, 0),
            index,
            error: ParserErrorType::NonHomogenousNumericList,
        }
    }

    fn non_homogenous_tag_list(input: &str, index: usize, char_width: usize) -> Self {
        Self {
            segment: Self::segment(input, index, char_width, 15, 0),
            index,
            error: ParserErrorType::NonHomogenousTagList,
        }
    }

    fn segment(
        input: &str,
        index: usize,
        char_width: usize,
        before: usize,
        after: usize,
    ) -> String {
        let start = input[.. index]
                .char_indices()
                .rev()
                .nth(before.saturating_sub(1))
                .map(|(index, _)| index)
                .unwrap_or(0);
        let end = (index + input[index ..]
                .char_indices()
                .nth(char_width.min(20) + after)
                .map(|(index, _)| index)
                .unwrap_or(input.len())
            ).min(input.len());

        input[start .. end].to_owned()
    }
}

impl Display for SnbtError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.error {
            ParserErrorType::ExceededDepthLimit { limit }
                => write!(
                    f, "Exceeded depth limit {} of nested tag lists and compound tags",
                    limit.0
                ),
            ParserErrorType::UnexpectedEOS { expected }
                => write!(f, "Reached end of input but expected {}", expected),
            ParserErrorType::UnexpectedToken { expected }
                => write!(
                    f, "Unexpected token at column {} near '{}', expected {}",
                    self.index, self.segment, expected
                ),
            ParserErrorType::UnsupportedEscapeSequence
                => write!(
                    f, "Escape sequence only supported in a different SNBT version: '{}'",
                    self.segment
                ),
            ParserErrorType::NamedEscapeSequence
            => write!(
                f, "Named sequence support is not enabled; could not parse escape sequence '{}'",
                self.segment
            ),
            ParserErrorType::UnknownEscapeSequence
                => write!(f, "Unknown escape sequence: '{}'", self.segment),
            ParserErrorType::InvalidUnquotedCharacter { ch }
                => write!(
                    f, "Character '{}' disallowed in unquoted strings at column {} near '{}'",
                    ch, self.index, self.segment
                ),
            ParserErrorType::InvalidNumber
                => write!(f, "Invalid number: {}", self.segment),
            ParserErrorType::InvalidUUID
                => write!(f, "Invalid string representation of a UUID: {}", self.segment),
            ParserErrorType::TrailingComma
                => write!(f, "Forbidden trailing comma at column {}: '{}'", self.index, self.segment),
            ParserErrorType::UnmatchedQuote
                => write!(f, "Unmatched quote: column {} near '{}'", self.index, self.segment),
            ParserErrorType::UnmatchedBrace
                => write!(f, "Unmatched brace at column {} near '{}'", self.index, self.segment),
            ParserErrorType::NonHomogenousNumericList
                => write!(
                    f, "Non-homogenous typed array of numbers at column {} near '{}'",
                    self.index, self.segment
                ),
            ParserErrorType::NonHomogenousTagList
                => write!(
                    f,
                    "Heterogenous tag list (only supported in new SNBT version) at column {} near '{}'",
                    self.index, self.segment
                ),
        }
    }
}

impl Debug for SnbtError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.error, f)
    }
}

impl Error for SnbtError {}

/// A specific type of parser error.
#[derive(Debug, Clone)]
pub enum ParserErrorType {
    /// The limit on recursive nesting depth of NBT lists and compounds was exceeded.
    ExceededDepthLimit {
        /// The limit which was exceeded.
        limit: DepthLimit
    },
    /// The end of the string (EOS) was encountered before it was expected.
    UnexpectedEOS {
        /// The expected token or sequence of tokens.
        expected: &'static str,
    },
    /// An unexpected token was encountered.
    UnexpectedToken {
        /// The expected token or sequence of tokens.
        expected: &'static str,
    },
    /// An escape sequence supported in some SNBT version, but not the one selected.
    UnsupportedEscapeSequence,
    /// A named escape sequence was encountered, but named escape sequence support wasn't enabled.
    NamedEscapeSequence,
    /// An unknown or invalid escape sequence.
    UnknownEscapeSequence,
    /// A non-alphanumeric character other than `_`, `-`, `.`, or `+` appeared in an unquoted string.
    InvalidUnquotedCharacter {
        /// The encountered character which should not appear in unquoted strings.
        ch: char
    },
    /// An invalid number.
    InvalidNumber,
    /// An invalid string representation of a UUID.
    InvalidUUID,
    /// A trailing comma was encountered in a list or compound when it shouldn't have been.
    TrailingComma,
    /// An unmatched single or double quote was encountered.
    UnmatchedQuote,
    /// An unmatched curly bracket, square bracket, or parenthesis was encountered.
    UnmatchedBrace,
    /// A non-homogenous array of numbers was encountered.
    NonHomogenousNumericList,
    /// A non-homogenous array of NBT tags was encountered.
    NonHomogenousTagList,
}
