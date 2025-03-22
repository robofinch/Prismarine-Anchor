// Specialized functions for lexing particular sorts of tokens
mod numeric;
mod other_utils;


use std::mem;
use std::{borrow::Cow, iter::Peekable, str::CharIndices};

use crate::tag::NbtTag;
use crate::settings::{
    ParseNonFinite, ParseTrueFalse, SnbtParseOptions, SnbtVersion
};
use super::SnbtError;


pub use self::other_utils::{allowed_unquoted, starts_unquoted_number};


pub struct Lexer<'a> {
    raw: &'a str,
    chars: Peekable<CharIndices<'a>>,
    index: usize,
    peek_stack: Vec<Result<TokenData, SnbtError>>,
    opts: SnbtParseOptions,
}

impl<'a> Lexer<'a> {
    pub fn new(raw: &'a str, opts: SnbtParseOptions) -> Self {
        Lexer {
            raw,
            chars: raw.char_indices().peekable(),
            index: 0,
            peek_stack: Vec::new(),
            opts,
        }
    }

    #[inline]
    pub fn snbt_version(&self) -> SnbtVersion {
        self.opts.version
    }

    #[inline]
    pub fn raw(&self) -> &str {
        self.raw
    }

    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn peek(
        &mut self,
        expecting_string: bool,
    ) -> Option<&Result<TokenData, SnbtError>> {

        if self.peek_stack.is_empty() {

            if let Some(res) = self.next(expecting_string) {
                self.peek_stack.push(res);

            } else {
                // We got None from self.next(), end of the input string.
                return None
            }
        }

        // The peek_stack is nonempty (either it already was, or we pushed to it).
        // We should be able to do `Some(self.peek_stack.last().unwrap())`,
        // but it's already the correct data type.
        self.peek_stack.last()
    }

    pub fn next(
        &mut self,
        expecting_string: bool,
    ) -> Option<Result<TokenData, SnbtError>> {
        // Manage the peeking function
        if let Some(item) = self.peek_stack.pop() {
            return Some(item);
        }

        // Skip whitespace
        while self.peek_ch()?.is_ascii_whitespace() {
            self.next_ch();
        }

        // Manage single-char tokens and pass multi-character tokens to a designated function
        let tk = match self.peek_ch()? {
            '{' => TokenData::new(Token::OpenCurly, self.index, 1),
            '}' => TokenData::new(Token::ClosedCurly, self.index, 1),
            '[' => TokenData::new(Token::OpenSquare, self.index, 1),
            ']' => TokenData::new(Token::ClosedSquare, self.index, 1),
            ',' => TokenData::new(Token::Comma, self.index, 1),
            ':' => TokenData::new(Token::Colon, self.index, 1),
            ';' => TokenData::new(Token::Semicolon, self.index, 1),
            _ => return Some(self.slurp_token(expecting_string)),
        };

        self.next_ch();
        Some(Ok(tk))
    }

    #[inline]
    fn peek_ch(&mut self) -> Option<char> {
        self.chars.peek().map(|&(_, ch)| ch)
    }

    #[inline]
    fn next_ch(&mut self) -> Option<char> {
        let next = self.chars.next();
        if let Some((index, ch)) = next {
            self.index = index + ch.len_utf8();
        }
        next.map(|(_, ch)| ch)
    }

    /// Asserts that the next token is the same type as the provided token.
    pub fn assert_next(
        &mut self,
        token: Token,
        expecting_string: bool,
    ) -> Result<TokenData, SnbtError> {

        match self.next(expecting_string).transpose()? {
            // We found a token so check the token type
            Some(td) =>
                if mem::discriminant(&td.token) == mem::discriminant(&token) {
                    Ok(td)
                } else {
                    Err(SnbtError::unexpected_token(
                        self.raw,
                        Some(&td),
                        token.as_expectation(),
                    ))
                },

            // No tokens were left so return an unexpected end of string error
            None => Err(SnbtError::unexpected_eos(token.as_expectation())),
        }
    }

    /// Collects a multi-character token from the character iterator and parses it
    /// with `parse_token`.
    fn slurp_token(
        &mut self,
        expecting_string: bool,
    ) -> Result<TokenData, SnbtError> {

        let start = self.index;
        // Width of *raw input* in chars. The string passed to parse_token
        // is not necessarily of size char_width, since that has escapes applied,
        // and excludes any quotes.
        let mut char_width = 1;

        let (first_ch, quoted) = match self.next_ch() {
            Some('"')  => ('"',  true),
            Some('\'') => ('\'', true),
            Some(ch) => {
                if allowed_unquoted(ch) {
                    (ch, false)
                } else {
                    return Err(SnbtError::invalid_unquoted_character(
                        self.raw,
                        self.index,
                        char_width,
                        ch
                    ))
                }
            }
            None => unreachable!("slurp_token called on an empty token"),
        };

        let mut raw_token_buffer = Cow::Owned(String::new());

        if quoted {
            // Don't include the quotes in the final buffer
            let mut flush_start = start + 1;

            #[inline]
            fn flush<'a>(raw: &'a str, buffer: &mut Cow<'a, str>, start: usize, end: usize) {
                if start == end {
                    return;
                }

                assert!(
                    start < end,
                    "Internal SNBT parsing error: start < end in `flush`"
                );

                if buffer.is_empty() {
                    *buffer = Cow::Borrowed(&raw[start .. end]);
                } else {
                    buffer.to_mut().push_str(&raw[start .. end]);
                }
            }

            loop {
                // We won't stop until we read at least one character, a closing quote
                char_width += 1;
                match self.next_ch() {
                    Some('\\') => {
                        if let Some((ch, escape_char_len))
                            = self.parse_escape_sequence(self.index - 1)?
                        {
                            if let Some(ch) = ch {
                                // An escape sequence was parsed normally

                                // Need to flush before pushing to self.raw_token_buffer
                                flush(
                                    self.raw,
                                    &mut raw_token_buffer,
                                    flush_start,
                                    self.index - 1 // Flush everything up to the '\\'
                                );

                                raw_token_buffer.to_mut().push(ch);
                                char_width += escape_char_len;
                                flush_start = self.index;

                            } else {
                                // Invalid escape sequence, and should be copied verbatim,
                                // so no need to flush.
                                char_width += escape_char_len;
                            }

                        } else {
                            // Invalid escape sequence, and should be ignored.
                            flush(
                                self.raw,
                                &mut raw_token_buffer,
                                flush_start,
                                self.index - 1 // Flush everything up to the '\\'
                            );
                            char_width -= 1;
                            flush_start = self.index;
                        }
                    }
                    Some(ch) if ch == first_ch => {
                        // Flush remaning characters
                        flush(
                            self.raw,
                            &mut raw_token_buffer,
                            flush_start,
                            self.index - 1 // note: first_ch is '\'' or '"' with len_utf8 of 1
                        );
                        break;
                    }
                    // A later flush will handle this character
                    Some(..) => continue,
                    // Expected the quote to be matched
                    None => return Err(SnbtError::unmatched_quote(self.raw, start))
                }
            }

        } else {
            // Unquoted string.
            // Loop until we reach a character which isn't allowed in the string
            loop {
                if let Some(ch) = self.peek_ch() {
                    if !allowed_unquoted(ch) {
                        break;
                    } else {
                        // We read a valid Some(char)
                        char_width += 1;
                        self.next_ch();
                    }
                } else {
                    // End of string
                    break;
                }
            };

            // If we reached the end of the string, we want to parse the entire rest
            // of the string. In that case, self.index is at the end of the string.
            // Otherwise, we want to parse all but the last character (the one which
            // isn't allowed in an unquoted string). We know that self.index is positioned
            // just before the character returned by self.peek_ch().
            //
            // TLDR: in either case self.index is the end index of the unquoted string
            raw_token_buffer = Cow::Borrowed(&self.raw[start .. self.index]);
        }

        Ok(self.parse_token(
            raw_token_buffer,
            start,
            char_width,
            quoted,
            expecting_string
        )?)
    }

    /// Parses an isolated token
    fn parse_token(
        &mut self,
        token_string: Cow<'_, str>,
        start: usize,
        char_width: usize,
        quoted: bool,
        expecting_string: bool,
    ) -> Result<TokenData, SnbtError> {
        if quoted || token_string.is_empty() {
            // Only strings can be quoted or be empty
            return Ok(TokenData::new(
                Token::String {
                    value: token_string.into_owned(),
                    quoted,
                },
                start,
                char_width,
            ))
        }

        // Check if the unquoted token is ambiguous
        if let Some(ambiguous) = AmbiguousWord::new(&token_string) {
            return ambiguous.disambiguate(self.opts, expecting_string, self.raw, start, char_width);
        }

        // Check if the token is the bool(..) or uuid(..) operation
        if let Some(res) = self.try_parse_operations(start, char_width, &token_string) {
            return res;
        }

        // Try parsing as a number
        match self.snbt_version() {
            SnbtVersion::UpdatedJava => {
                // Check the first character of the string. Note that we checked above
                // whether token_string is empty, so it's nonempty here.
                let ch = token_string.chars().next().unwrap();
                if starts_unquoted_number(ch) {
                    return self.parse_updated_numeric(start, char_width, &token_string)
                }
            }
            SnbtVersion::Original => {
                if let Ok(tk) = self.parse_original_numeric(start, char_width, &token_string) {
                    return Ok(tk);
                }
            }
        }

        // By elimination, it cannot be anything but an unquoted string.
        // Note that slurp_token only gave us things that are allowed to be quoted,
        // so no additional validation is needed.
        Ok(TokenData::new(
            Token::String {
                value: token_string.into_owned(),
                quoted,
            },
            start,
            char_width,
        ))
    }
}

/// Intended for converting an integer-valued token into the integer type
/// of the same size.
pub trait FromExact<T>: Sized {
    /// Intended for converting an integer-valued token into the integer type
    /// of the same size.
    fn from_exact(value: T) -> Result<Self, T>;
}

/// Intended for converting an integer-valued token into the integer type
/// of the same or greater size.
pub trait FromLossless<T>: Sized {
    /// Intended for converting an integer-valued token into the integer type
    /// of the same or greater size.
    /// The return value being `Err((_, true))` should indicate that the token
    /// had no suffix, but was too large to fit into this integer type.
    fn from_lossless(value: T) -> Result<Self, (T, bool)>;
}

#[derive(Debug)]
pub struct TokenData {
    pub token: Token,
    pub index: usize,
    pub char_width: usize,
}

impl TokenData {
    #[inline]
    fn new(token: Token, index: usize, char_width: usize) -> Self {
        TokenData {
            token,
            index,
            char_width,
        }
    }

    pub fn into_tag(self) -> Result<NbtTag, Self> {
        match self.token.into_tag() {
            Ok(tag) => Ok(tag),
            Err(tk) => Err(Self::new(tk, self.index, self.char_width)),
        }
    }
}

impl<T: FromExact<Token>> FromExact<TokenData> for T {
    fn from_exact(td: TokenData) -> Result<Self, TokenData> {
        match T::from_exact(td.token) {
            Ok(value) => Ok(value),
            Err(tk) => Err(TokenData::new(tk, td.index, td.char_width)),
        }
    }
}

impl<T: FromLossless<Token>> FromLossless<TokenData> for T {
    fn from_lossless(td: TokenData) -> Result<Self, (TokenData, bool)> {
        match T::from_lossless(td.token) {
            Ok(value) => Ok(value),
            Err((tk, cause)) => Err((
                TokenData::new(tk, td.index, td.char_width),
                cause
            )),
        }
    }
}

#[derive(Debug)]
pub enum Token {
    OpenCurly,
    ClosedCurly,
    OpenSquare,
    ClosedSquare,
    Comma,
    Colon,
    Semicolon,
    String { value: String, quoted: bool },
    Byte(i8),
    Short(i16),
    Int { value: i32, suffixed: bool },
    Long(i64),
    Float(f32),
    Double(f64),
}

impl Token {
    #[inline]
    pub fn int(value: i32, suffixed: bool) -> Self {
        Self::Int { value, suffixed }
    }

    pub fn as_expectation(&self) -> &'static str {
        match self {
            Token::OpenCurly    => "'{'",
            Token::ClosedCurly  => "'}'",
            Token::OpenSquare   => "'['",
            Token::ClosedSquare => "']'",
            Token::Comma        => "','",
            Token::Colon        => "':'",
            Token::Semicolon    => "';'",
            _ => "value",
        }
    }

    pub fn into_tag(self) -> Result<NbtTag, Self> {
        match self {
            Token::String { value, .. } => Ok(NbtTag::String(value)),
            Token::Byte(value)          => Ok(NbtTag::Byte(value)),
            Token::Short(value)         => Ok(NbtTag::Short(value)),
            Token::Int { value, .. }    => Ok(NbtTag::Int(value)),
            Token::Long(value)          => Ok(NbtTag::Long(value)),
            Token::Float(value)         => Ok(NbtTag::Float(value)),
            Token::Double(value)        => Ok(NbtTag::Double(value)),
            tk => Err(tk),
        }
    }
}

impl From<Token> for Result<String, Token> {
    fn from(tk: Token) -> Self {
        match tk {
            Token::String { value, .. } => Ok(value),
            tk => Err(tk),
        }
    }
}

macro_rules! impl_from_exact {
    ($int:ty, $token:ident) => {
        impl FromExact<Token> for $int {
            fn from_exact(tk: Token) -> Result<Self, Token> {
                match tk {
                    Token::$token(value) => Ok(value),
                    _ => Err(tk),
                }
            }
        }
    };
}

impl_from_exact!(i8,  Byte);
impl_from_exact!(i16, Short);
impl_from_exact!(i64, Long);

impl FromExact<Token> for i32 {
    fn from_exact(tk: Token) -> Result<Self, Token> {
        match tk {
            Token::Int { value, .. } => Ok(value),
            _ => Err(tk),
        }
    }
}

impl FromLossless<Token> for i8 {
    fn from_lossless(tk: Token) -> Result<Self, (Token, bool)> {
        match tk {
            Token::Byte(value) => Ok(value),
            Token::Int { value, suffixed: false } => {
                if value < 1 << 8 {
                    Ok(value as i8)
                } else {
                    Err((tk, true))
                }
            }
            _ => Err((tk, false)),
        }
    }
}

impl FromLossless<Token> for i16 {
    fn from_lossless(tk: Token) -> Result<Self, (Token, bool)> {
        match tk {
            Token::Byte(value)  => Ok(value as i16),
            Token::Short(value) => Ok(value),
            Token::Int { value, suffixed: false } => {
                if value < 1 << 16 {
                    Ok(value as i16)
                } else {
                    Err((tk, true))
                }
            }
            _ => Err((tk, false)),
        }
    }
}

impl FromLossless<Token> for i32 {
    fn from_lossless(tk: Token) -> Result<Self, (Token, bool)> {
        match tk {
            Token::Byte(value)       => Ok(value as i32),
            Token::Short(value)      => Ok(value as i32),
            Token::Int { value, .. } => Ok(value),
            _ => Err((tk, false)),
        }
    }
}

impl FromLossless<Token> for i64 {
    fn from_lossless(tk: Token) -> Result<Self, (Token, bool)> {
        match tk {
            Token::Byte(value)       => Ok(value as i64),
            Token::Short(value)      => Ok(value as i64),
            Token::Int { value, .. } => Ok(value as i64),
            Token::Long(value)       => Ok(value),
            _ => Err((tk, false)),
        }
    }
}

pub(crate) fn is_ambiguous(string: &str) -> bool {
    AmbiguousWord::new(string).is_some()
}

#[derive(Debug)]
pub enum AmbiguousWord {
    True,
    False,
    InfinityD,
    InfinityF,
    NegInfinityD,
    NegInfinityF,
    NaND,
    NaNF,
}

impl AmbiguousWord {
    pub fn new(string: &str) -> Option<Self> {
        match string {
            "true"       => Some(Self::True),
            "false"      => Some(Self::False),
            "Infinityd"  => Some(Self::InfinityD),
            "Infinityf"  => Some(Self::InfinityF),
            "-Infinityd" => Some(Self::NegInfinityD),
            "-Infinityf" => Some(Self::NegInfinityF),
            "NaNd"       => Some(Self::NaND),
            "NaNf"       => Some(Self::NaNF),
            _ => None,
        }
    }

    fn string_val(self) -> String {
        match self {
            Self::True => "true",
            Self::False => "false",
            Self::InfinityD => "Infinityd",
            Self::InfinityF => "Infinityf",
            Self::NegInfinityD => "-Infinityd",
            Self::NegInfinityF => "-Infinityf",
            Self::NaND => "NaNd",
            Self::NaNF => "NaNf",
        }.to_owned()
    }

    fn numeric_val(self) -> Token {
        match self {
            Self::True         => Token::Byte(1),
            Self::False        => Token::Byte(0),
            Self::InfinityD    => Token::Double(f64::INFINITY),
            Self::NegInfinityD => Token::Double(f64::NEG_INFINITY),
            Self::NaND         => Token::Double(f64::NAN),
            Self::InfinityF    => Token::Float(f32::INFINITY),
            Self::NegInfinityF => Token::Float(f32::NEG_INFINITY),
            Self::NaNF         => Token::Float(f32::NAN),
        }
    }

    fn disambiguate(
        self,
        opts: SnbtParseOptions,
        expecting_string: bool,
        input: &str,
        index: usize,
        char_width: usize,
    ) -> Result<TokenData, SnbtError> {

        match self {
            Self::True | Self::False => {
                if opts.true_false == ParseTrueFalse::AsString
                    || (opts.true_false == ParseTrueFalse::AsDetected && expecting_string) {
                        Ok(TokenData {
                            token: Token::String {
                                value: self.string_val(),
                                quoted: false
                            },
                            index,
                            char_width,
                        })
                } else {
                    Ok(TokenData::new(self.numeric_val(), index, char_width))
                }
            }
            _ => match (opts.non_finite, expecting_string) {
                (ParseNonFinite::Error, _) => {
                    Err(SnbtError::ambiguous_token(input, index, char_width))
                }
                (ParseNonFinite::AsDetected, true)
                    | (ParseNonFinite::AsString, _) => {
                        Ok(TokenData {
                            token: Token::String {
                                value: self.string_val(),
                                quoted: false
                            },
                            index,
                            char_width,
                        })
                    }
                (ParseNonFinite::AsDetected, _) | (ParseNonFinite::AsFloat, _) => {
                    if opts.replace_non_finite {
                        Ok(TokenData {
                            token: match self {
                                Self::InfinityD    => Token::Double(f64::MAX),
                                Self::NegInfinityD => Token::Double(f64::MIN),
                                Self::NaND         => Token::Double(f64::NAN),
                                Self::InfinityF    => Token::Float(f32::MAX),
                                Self::NegInfinityF => Token::Float(f32::MIN),
                                Self::NaNF         => Token::Float(f32::NAN),
                                _ => unreachable!(),
                            },
                            index,
                            char_width,
                        })
                    } else if opts.version == SnbtVersion::UpdatedJava {
                        // TODO: Need to implement NumericParseError
                        Err(SnbtError::invalid_number(input, index, char_width))

                    } else {
                        Ok(TokenData::new(self.numeric_val(), index, char_width))
                    }
                }
            }
        }
    }
}
