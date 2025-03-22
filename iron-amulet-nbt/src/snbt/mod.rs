//! Module for parsing SNBT into NBT data

mod lexer;


use std::{fmt, mem};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

use crate::{
    settings::{DepthLimit, SnbtParseOptions, SnbtVersion},
    tag::{NbtCompound, NbtList, NbtTag},
};
use self::lexer::{FromExact, FromLossless, Lexer, Token, TokenData};


pub use self::lexer::{allowed_unquoted, starts_unquoted_number};
pub(crate) use self::lexer::is_ambiguous;


// Should add module-wide documentation about the specification and implementation.



// ================================
//      Convenience functions
// ================================

/// Parses the given string into an NBT tag.
/// See [`SnbtVersion`] for some specifics of the standard and this implementation.
#[inline]
pub fn parse_any<T: AsRef<str> + ?Sized>(
    string_nbt: &T,
    opts: SnbtParseOptions,
) -> Result<NbtTag, SnbtError> {
    parse_any_and_size(
        string_nbt,
        opts,
    ).map(|(tag, _)| tag)
}

/// Parses the given string into an NBT tag, using the original SNBT version.
/// See [`SnbtVersion`] for some specifics of the standard and this implementation.
#[inline]
pub fn parse_any_original<T: AsRef<str> + ?Sized>(
    string_nbt: &T,
) -> Result<NbtTag, SnbtError> {
    parse_any_and_size(
        string_nbt,
        SnbtParseOptions::default_original(),
    ).map(|(tag, _)| tag)
}

/// Parses the given string into an NBT tag, using the newer SNBT version.
/// See [`SnbtVersion`] for some specifics of the standard and this implementation.
#[inline]
pub fn parse_any_updated<T: AsRef<str> + ?Sized>(
    string_nbt: &T,
) -> Result<NbtTag, SnbtError> {
    parse_any_and_size(
        string_nbt,
        SnbtParseOptions::default_updated(),
    ).map(|(tag, _)| tag)
}

/// Parses the given string into a compound NBT tag.
/// See [`SnbtVersion`] for some specifics of the standard and this implementation.
#[inline]
pub fn parse_compound<T: AsRef<str> + ?Sized>(
    string_nbt: &T,
    opts: SnbtParseOptions,
) -> Result<NbtCompound, SnbtError> {
    parse_compound_and_size(
        string_nbt,
        opts,
    ).map(|(tag, _)| tag)
}

/// Parses the given string into a compound NBT tag, using the newer SNBT version.
/// See [`SnbtVersion`] for some specifics of the standard and this implementation.
#[inline]
pub fn parse_compound_updated<T: AsRef<str> + ?Sized>(
    string_nbt: &T,
) -> Result<NbtCompound, SnbtError> {
    parse_compound_and_size(
        string_nbt,
        SnbtParseOptions::default_updated(),
    ).map(|(tag, _)| tag)
}

/// Parses the given string into a compound NBT tag, using the original SNBT version.
/// See [`SnbtVersion`] for some specifics of the standard and this implementation.
#[inline]
pub fn parse_compound_original<T: AsRef<str> + ?Sized>(
    string_nbt: &T,
) -> Result<NbtCompound, SnbtError> {
    parse_compound_and_size(
        string_nbt,
        SnbtParseOptions::default_original(),
    ).map(|(tag, _)| tag)
}

// ================================
//      Actual parsing
// ================================

/// Parses the given string into a tag just like [`parse_any`],
/// but also returns the number of parsed characters.
pub fn parse_any_and_size<T: AsRef<str> + ?Sized>(
    string_nbt: &T,
    opts: SnbtParseOptions,
) -> Result<(NbtTag, usize), SnbtError> {
    let mut tokens = Lexer::new(string_nbt.as_ref(), opts);
    let tag = parse_next_value(&mut tokens, false, 0)?;

    Ok((tag, tokens.index()))
}

/// Parses the given string just like [`parse_compound`],
/// but also returns the number of parsed characters.
pub fn parse_compound_and_size<T: AsRef<str> + ?Sized>(
    string_nbt: &T,
    opts: SnbtParseOptions,
) -> Result<(NbtCompound, usize), SnbtError> {
    let mut tokens = Lexer::new(string_nbt.as_ref(), opts);
    let open_curly = tokens.assert_next(Token::OpenCurly, false)?;
    parse_compound_tag(&mut tokens, &open_curly, 0)
}

// Parses the next value in the token stream
fn parse_next_value(
    tokens: &mut Lexer<'_>,
    expecting_string: bool,
    current_depth: u32,
) -> Result<NbtTag, SnbtError> {
    let token = tokens.next(expecting_string).transpose()?;
    parse_value(tokens, token, current_depth)
}

/// Parses a token into a value
fn parse_value(
    tokens: &mut Lexer<'_>,
    token: Option<TokenData>,
    current_depth: u32,
) -> Result<NbtTag, SnbtError> {

    if let Some(td) = token {
        match td {
            // Open curly brace indicates a compound tag is present
            TokenData {
                token: Token::OpenCurly,
                ..
            } => parse_compound_tag(tokens, &td, current_depth).map(|(tag, _)| tag.into()),

            // Open square brace indicates that some kind of list is present
            TokenData {
                token: Token::OpenSquare,
                ..
            } => parse_list(tokens, &td, current_depth),

            // Could be a value token or delimiter token
            _ => {
                td.into_tag().map_err(|td| {
                    SnbtError::unexpected_token(tokens.raw(), Some(&td), "value")
                })
            }
        }

    } else {
        // We expected a value but ran out of data
        Err(SnbtError::unexpected_eos("value"))
    }
}

// Parses a list, which can be either a generic tag list or vector of primitives
fn parse_list(
    tokens: &mut Lexer<'_>,
    open_square: &TokenData,
    current_depth: u32,
) -> Result<NbtTag, SnbtError> {

    match tokens.next(false).transpose()? {
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
            match tokens.peek(false) {
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
                    tokens.next(false);

                    // Determine the primitive type and parse it
                    match string.as_str() {
                        "b" | "B" => parse_prim_list::<i8>(tokens, open_square),
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

                _ => {
                    if current_depth >= tokens.depth_limit().0 {
                        Err(SnbtError::exceeded_depth_limit(
                            tokens.raw(),
                            index,
                            tokens.depth_limit()
                        ))
                    } else {
                        // Parse as a tag list (token errors are delegated to this function)
                        parse_tag_list(tokens, NbtTag::String(string), current_depth)
                            .map(Into::into)
                    }
                }
            }
        }

        // Any other pattern is delegated to the general tag list parser
        td => {
            // Check the depth limit
            if let Some(td) = &td {
                if current_depth >= tokens.depth_limit().0 {
                    return Err(SnbtError::exceeded_depth_limit(
                        tokens.raw(),
                        td.index,
                        tokens.depth_limit()
                    ));
                }
            }

            let first_element = parse_value(tokens, td, current_depth + 1)?;
            parse_tag_list(tokens, first_element, current_depth).map(Into::into)
        }
    }
}

fn parse_prim_list<'a, T>(
    tokens: &mut Lexer<'a>,
    open_square: &TokenData,
) -> Result<NbtTag, SnbtError>
where
    T: FromExact<TokenData> + FromLossless<TokenData>,
    NbtTag: From<Vec<T>>,
{
    let mut list: Vec<T> = Vec::new();
    // Zero is used as a niche value so the first iteration of the loop runs correctly
    let mut comma: Option<usize> = Some(0);

    loop {
        match tokens.next(false).transpose()? {
            // Finish off the list
            Some(TokenData {
                token: Token::ClosedSquare,
                ..
            }) => match comma {
                Some(0) | None => return Ok(list.into()),
                // For some reason, even in the updated version, trailing commas are
                // still not allowed for numeric arrays, if I'm reading the spec correctly.
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
                        match tokens.snbt_version() {
                            // The numeric array can accept data of the same size or smaller
                            SnbtVersion::UpdatedJava => {
                                match T::from_lossless(td) {
                                    Ok(value) => list.push(value),
                                    Err((td, true)) =>
                                        return Err(SnbtError::invalid_number(
                                            tokens.raw(),
                                            td.index,
                                            td.char_width,
                                        )),
                                    Err((td, false)) =>
                                        return Err(SnbtError::non_homogenous_numeric_list(
                                            tokens.raw(),
                                            td.index,
                                            td.char_width,
                                        )),
                                }
                            }
                            // The numeric array can accept data only of the same size
                            SnbtVersion::Original => {
                                match T::from_exact(td) {
                                    Ok(value) => list.push(value),
                                    Err(td) =>
                                        return Err(SnbtError::non_homogenous_numeric_list(
                                            tokens.raw(),
                                            td.index,
                                            td.char_width,
                                        )),
                                }
                            }
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

// Depth limit should be checked before entering this function
fn parse_tag_list(
    tokens: &mut Lexer<'_>,
    first_element: NbtTag,
    current_depth: u32,
) -> Result<NbtList, SnbtError> {
    // Construct the list and use the first element to determine the list's type
    let mut list = NbtList::new();
    let mut descrim = mem::discriminant(&first_element);
    let expecting_strings = matches!(&first_element, &NbtTag::String(_));
    let mut list_holds_compounds = matches!(&first_element, &NbtTag::Compound{..});
    list.push(first_element);

    loop {
        match tokens.next(expecting_strings).transpose()? {
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
                let (index, char_width) = match tokens.peek(expecting_strings) {
                    // The comma could be a trailing comma at the end of the list
                    Some(&Ok(TokenData {
                        index, token: Token::ClosedSquare, ..
                    })) => match tokens.snbt_version() {
                        SnbtVersion::UpdatedJava => return Ok(list),
                        SnbtVersion::Original => return Err(
                            SnbtError::trailing_comma(tokens.raw(), index)
                        ),
                    }
                    Some(&Ok(TokenData {
                        index, char_width, ..
                    })) => (index, char_width),
                    _ => (0, 0),
                };
                let element = parse_next_value(tokens, expecting_strings, current_depth + 1)?;

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
    current_depth: u32,
) -> Result<(NbtCompound, usize), SnbtError> {
    let mut compound = NbtCompound::new();
    // Zero is used as a niche value so the first iteration of the loop runs correctly
    let mut comma: Option<usize> = Some(0);

    loop {
        if let Some(td) = tokens.next(true).transpose()? {
            match td {
                // Finish off the compound tag
                TokenData {
                    token: Token::ClosedCurly,
                    ..
                } => {
                    match comma {
                        // First loop iteration or no comma
                        Some(0) | None => return Ok((compound, tokens.index())),
                        // Later iteration with a trailing comma
                        Some(index) => if tokens.snbt_version() == SnbtVersion::Original {
                            return Err(SnbtError::trailing_comma(tokens.raw(), index));
                        }
                    }
                }

                // Parse a new key-value pair
                TokenData {
                    token: Token::String { value: key, .. },
                    index,
                    char_width,
                } => {
                    match comma {
                        // First loop iteration or a comma indicated that more data is present
                        Some(_) => {
                            // Check current_depth. If we're at the limit, then this is
                            // an error.
                            if current_depth >= tokens.depth_limit().0 {
                                return Err(SnbtError::exceeded_depth_limit(
                                    tokens.raw(),
                                    index,
                                    tokens.depth_limit()
                                ))
                            }

                            tokens.assert_next(Token::Colon, false)?;
                            compound.insert(
                                key,
                                parse_next_value(tokens, false, current_depth + 1)?,
                            );
                            comma = None;
                        }

                        // There was not a comma before this string
                        // so therefore the token is unexpected
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
                TokenData {
                    token: Token::Comma,
                    index,
                    ..
                } => match comma {
                    None => comma = Some(index),
                    // This comma came before any valid element, or after another comma;
                    // this is not valid in either version.
                    Some(_) =>
                        return Err(SnbtError::unexpected_token_at(
                            tokens.raw(),
                            index,
                            1,
                            "compound key or '}'",
                        )),
                }

                // Catch-all for unexpected tokens
                td =>
                    return Err(SnbtError::unexpected_token(
                        tokens.raw(),
                        Some(&td),
                        "compound key, '}', or ','",
                    )),
            }

        } else {
            // End of input / unmatched brace
            return Err(SnbtError::unmatched_brace(tokens.raw(), open_curly.index))
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
    fn exceeded_depth_limit(input: &str, index: usize, limit: DepthLimit) -> Self {
        Self {
            segment: Self::segment(input, index, 1, 4, 4),
            index,
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

    fn ambiguous_token(input: &str, index: usize, char_width: usize) -> Self {
        Self {
            segment: Self::segment(input, index, char_width, 0, 0),
            index,
            error: ParserErrorType::AmbiguousToken,
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
                    f, "Exceeded depth limit {} for nested tag lists and compound tags",
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
            ParserErrorType::AmbiguousToken
                => write!(f, "Ambiguous token '{}' at column {}", self.segment, self.index),
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
    /// An unquoted token which could be numeric or a string,
    /// which was prohibited in parsing options.
    AmbiguousToken,
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
