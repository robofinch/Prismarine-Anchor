// Specialized functions for lexing particular sorts of tokens
mod numeric;
mod other_utils;


use std::mem;
use std::{borrow::Cow, iter::Peekable, str::CharIndices};

use crate::tag::NbtTag;
use crate::settings::{
    DepthLimit, ParseNonFinite, ParseTrueFalse, SnbtParseOptions, SnbtVersion,
};
use super::SnbtError;


pub use self::numeric::NumericParseError;
pub use self::other_utils::{allowed_unquoted, starts_unquoted_number};


pub struct Lexer<'a> {
    raw:        &'a str,
    chars:      Peekable<CharIndices<'a>>,
    index:      usize,
    peek_stack: Vec<Result<TokenData, SnbtError>>,
    opts:       SnbtParseOptions,
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
    pub fn depth_limit(&self) -> DepthLimit {
        self.opts.depth_limit
    }

    #[inline]
    pub fn byte_strings_enabled(&self) -> bool {
        self.opts.enable_byte_strings
    }

    #[inline]
    pub fn raw(&self) -> &str {
        self.raw
    }

    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn peek(&mut self, expecting_string: bool) -> Option<&Result<TokenData, SnbtError>> {
        if self.peek_stack.is_empty() {
            if let Some(res) = self.next(expecting_string) {
                self.peek_stack.push(res);
            } else {
                // We got None from self.next(), end of the input string.
                return None;
            }
        }

        // The peek_stack is nonempty (either it already was, or we pushed to it).
        // We should be able to do `Some(self.peek_stack.last().unwrap())`,
        // but it's already the correct data type.
        self.peek_stack.last()
    }

    pub fn next(&mut self, expecting_string: bool) -> Option<Result<TokenData, SnbtError>> {
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
        token:            &Token,
        expecting_string: bool,
    ) -> Result<TokenData, SnbtError> {
        match self.next(expecting_string).transpose()? {
            // We found a token so check the token type
            Some(td) => {
                if mem::discriminant(&td.token) == mem::discriminant(token) {
                    Ok(td)
                } else {
                    Err(SnbtError::unexpected_token(
                        self.raw,
                        Some(&td),
                        token.as_expectation(),
                    ))
                }
            }
            // No tokens were left so return an unexpected end of string error
            None => Err(SnbtError::unexpected_eos(token.as_expectation())),
        }
    }

    /// Collects a multi-character token from the character iterator and parses it
    /// with `parse_token`.
    fn slurp_token(&mut self, expecting_string: bool) -> Result<TokenData, SnbtError> {
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
                        self.raw, self.index, char_width, ch,
                    ));
                }
            }
            None => unreachable!("slurp_token called on an empty token"),
        };

        let (char_width, raw_token_buffer) = if quoted {
            self.slurp_quoted_symbol(self.raw, first_ch, start, char_width)?
        } else {
            // Unquoted string.
            // Loop until we reach a character which isn't allowed in the string
            while let Some(ch) = self.peek_ch() {
                if !allowed_unquoted(ch) {
                    break;
                } else {
                    // We read a valid Some(char)
                    char_width += 1;
                    self.next_ch();
                }
            }

            // If we reached the end of the string, we want to parse the entire rest
            // of the string. In that case, self.index is at the end of the string.
            // Otherwise, we want to parse all but the last character (the one which
            // isn't allowed in an unquoted string). We know that self.index is positioned
            // just before the character returned by self.peek_ch().
            //
            // TLDR: in either case self.index is the end index of the unquoted string
            (char_width, Cow::Borrowed(&self.raw[start..self.index]))
        };

        self.parse_token(
            raw_token_buffer,
            start,
            char_width,
            quoted,
            expecting_string,
        )
    }

    /// Collect a quoted symbol from the character iterator.
    // self.raw is passed in separately to avoid borrowing issues
    fn slurp_quoted_symbol<'b>(
        &mut self,
        raw:            &'b str,
        initial_quote:  char,
        start:          usize,
        mut char_width: usize,
    ) -> Result<(usize, Cow<'b, str>), SnbtError> {
        // Sort of silly that clippy doesn't let me put this closer to the place it matters
        #![allow(
            clippy::needless_continue,
            reason = "in case something is added after the match in a below loop",
        )]

        let mut raw_token_buffer = Cow::Owned(String::new());
        // Don't include the quotes in the final buffer
        let mut flush_start = start + 1;

        #[inline]
        fn flush<'a>(raw: &'a str, buffer: &mut Cow<'a, str>, start: usize, end: usize) {
            if start == end {
                return;
            }

            assert!(
                start < end,
                "Internal SNBT parsing error: start < end in `flush`",
            );

            if buffer.is_empty() {
                *buffer = Cow::Borrowed(&raw[start..end]);
            } else {
                buffer.to_mut().push_str(&raw[start..end]);
            }
        }

        loop {
            // We won't stop until we read at least one character, a closing quote
            char_width += 1;
            match self.next_ch() {
                Some('\\') => {
                    if let Some((ch, escape_char_len)) =
                        self.parse_escape_sequence(self.index - 1)?
                    {
                        if let Some(ch) = ch {
                            // An escape sequence was parsed normally

                            // Need to flush before pushing to self.raw_token_buffer
                            flush(
                                raw,
                                &mut raw_token_buffer,
                                flush_start,
                                self.index - 1, // Flush everything up to the '\\'
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
                            raw,
                            &mut raw_token_buffer,
                            flush_start,
                            self.index - 1, // Flush everything up to the '\\'
                        );
                        char_width -= 1;
                        flush_start = self.index;
                    }
                }
                Some(ch) if ch == initial_quote => {
                    // Flush remaning characters
                    flush(
                        raw,
                        &mut raw_token_buffer,
                        flush_start,
                        self.index - 1, // note: first_ch is '\'' or '"' with len_utf8 of 1
                    );
                    break;
                }
                // A later flush will handle this character
                Some(..) => {
                    continue;
                }
                // Expected the quote to be matched
                None => return Err(SnbtError::unmatched_quote(raw, start)),
            }
        }

        Ok((char_width, raw_token_buffer))
    }

    /// Parses an isolated token
    #[expect(clippy::fn_params_excessive_bools, reason = "internal function")]
    fn parse_token(
        &mut self,
        token_string:     Cow<'_, str>,
        start:            usize,
        char_width:       usize,
        quoted:           bool,
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
            ));
        }

        // Check if the unquoted token is ambiguous
        if let Some(ambiguous) = AmbiguousWord::new(&token_string) {
            return ambiguous.disambiguate(
                self.opts,
                expecting_string,
                self.raw,
                start,
                char_width,
            );
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
                #[expect(
                    clippy::unwrap_used,
                    reason = "if `token_string.is_empty()`, then we return early above",
                )]
                let ch = token_string.chars().next().unwrap();
                if starts_unquoted_number(ch) {
                    return self.parse_updated_numeric(start, char_width, &token_string);
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
    fn from_lossless(value: T) -> Result<Self, (T, Option<NumericParseError>)>;
}

#[derive(Debug)]
pub struct TokenData {
    pub token:      Token,
    pub index:      usize,
    pub char_width: usize,
}

impl TokenData {
    #[inline]
    pub fn new(token: Token, index: usize, char_width: usize) -> Self {
        Self {
            token,
            index,
            char_width,
        }
    }

    #[inline]
    pub fn into_tag(self) -> Result<NbtTag, Self> {
        match self.token.into_tag() {
            Ok(tag) => Ok(tag),
            Err(tk) => Err(Self::new(tk, self.index, self.char_width)),
        }
    }
}

impl<T: FromExact<Token>> FromExact<TokenData> for T {
    #[inline]
    fn from_exact(td: TokenData) -> Result<Self, TokenData> {
        match T::from_exact(td.token) {
            Ok(value) => Ok(value),
            Err(tk) => Err(TokenData::new(tk, td.index, td.char_width)),
        }
    }
}

impl<T: FromLossless<Token>> FromLossless<TokenData> for T {
    fn from_lossless(td: TokenData) -> Result<Self, (TokenData, Option<NumericParseError>)> {
        match T::from_lossless(td.token) {
            Ok(value) => Ok(value),
            Err((tk, cause)) => Err((
                TokenData::new(tk, td.index, td.char_width),
                cause,
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
    Int(i32),
    Long(i64),
    UnsuffixedInt(i64),
    Float(f32),
    Double(f64),
}

impl Token {
    pub fn as_expectation(&self) -> &'static str {
        match self {
            Self::OpenCurly    => "'{'",
            Self::ClosedCurly  => "'}'",
            Self::OpenSquare   => "'['",
            Self::ClosedSquare => "']'",
            Self::Comma        => "','",
            Self::Colon        => "':'",
            Self::Semicolon    => "';'",
            _ => "value",
        }
    }

    pub fn into_tag(self) -> Result<NbtTag, Self> {
        match self {
            Self::String { value, .. } => Ok(NbtTag::String(value)),
            Self::Byte(value)          => Ok(NbtTag::Byte(value)),
            Self::Short(value)         => Ok(NbtTag::Short(value)),
            Self::Int(value)           => Ok(NbtTag::Int(value)),
            Self::Long(value)          => Ok(NbtTag::Long(value)),
            Self::Float(value)         => Ok(NbtTag::Float(value)),
            Self::Double(value)        => Ok(NbtTag::Double(value)),
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
    #[expect(clippy::use_self, reason = "clarity; it's an i32")]
    fn from_exact(tk: Token) -> Result<Self, Token> {
        match tk {
            Token::Int(value) => Ok(value),
            // Additional information from the i64 being out-of-range is unimportant,
            // we just return the token on error.
            #[expect(clippy::map_err_ignore, reason = "error information is unnecessary")]
            Token::UnsuffixedInt(value) => i32::try_from(value).map_err(|_| tk),
            _ => Err(tk),
        }
    }
}

fn out_of_range(
    tk:            Token,
    value:         i64,
    expected_type: &'static str,
) -> (Token, Option<NumericParseError>) {
    (
        tk,
        Some(NumericParseError::OutOfRangeInteger {
            negative: value < 0,
            num: value.unsigned_abs(),
            expected_type,
        }),
    )
}

impl FromLossless<Token> for i8 {
    #[expect(clippy::use_self, reason = "clarity; it's an i8")]
    fn from_lossless(tk: Token) -> Result<Self, (Token, Option<NumericParseError>)> {
        match tk {
            Token::Byte(value) => Ok(value),
            #[expect(
                clippy::map_err_ignore,
                reason = "only possible error is out-of-range i64",
            )]
            Token::UnsuffixedInt(value) => {
                i8::try_from(value).map_err(|_| out_of_range(tk, value, "i8"))
            }
            _ => Err((tk, None)),
        }
    }
}

impl FromLossless<Token> for i16 {
    #[expect(clippy::use_self, reason = "clarity; it's an i16")]
    fn from_lossless(tk: Token) -> Result<Self, (Token, Option<NumericParseError>)> {
        match tk {
            Token::Byte(value)  => Ok(i16::from(value)),
            Token::Short(value) => Ok(value),
            #[expect(
                clippy::map_err_ignore,
                reason = "only possible error is out-of-range i64",
            )]
            Token::UnsuffixedInt(value) => {
                i16::try_from(value).map_err(|_| out_of_range(tk, value, "i16"))
            }
            _ => Err((tk, None)),
        }
    }
}

impl FromLossless<Token> for i32 {
    #[expect(clippy::use_self, reason = "clarity")]
    fn from_lossless(tk: Token) -> Result<Self, (Token, Option<NumericParseError>)> {
        match tk {
            Token::Byte(value)  => Ok(i32::from(value)),
            Token::Short(value) => Ok(i32::from(value)),
            Token::Int(value)   => Ok(value),
            #[expect(
                clippy::map_err_ignore,
                reason = "only possible error is out-of-range i64",
            )]
            Token::UnsuffixedInt(value) => {
                i32::try_from(value).map_err(|_| out_of_range(tk, value, "i32"))
            }
            _ => Err((tk, None)),
        }
    }
}

impl FromLossless<Token> for i64 {
    #[expect(clippy::use_self, reason = "clarity")]
    fn from_lossless(tk: Token) -> Result<Self, (Token, Option<NumericParseError>)> {
        #[expect(clippy::match_same_arms)]
        match tk {
            Token::Byte(value)          => Ok(i64::from(value)),
            Token::Short(value)         => Ok(i64::from(value)),
            Token::Int(value)           => Ok(i64::from(value)),
            Token::Long(value)          => Ok(value),
            Token::UnsuffixedInt(value) => Ok(value),
            _ => Err((tk, None)),
        }
    }
}

pub fn is_ambiguous(string: &str) -> bool {
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
            Self::True         => "true",
            Self::False        => "false",
            Self::InfinityD    => "Infinityd",
            Self::InfinityF    => "Infinityf",
            Self::NegInfinityD => "-Infinityd",
            Self::NegInfinityF => "-Infinityf",
            Self::NaND         => "NaNd",
            Self::NaNF         => "NaNf",
        }
        .to_owned()
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
        opts:             SnbtParseOptions,
        expecting_string: bool,
        input:            &str,
        index:            usize,
        char_width:       usize,
    ) -> Result<TokenData, SnbtError> {

        match self {
            Self::True | Self::False => match (opts.true_false, expecting_string) {
                (ParseTrueFalse::AsString, _) | (ParseTrueFalse::AsDetected, true) => {
                    let token = Token::String {
                        value:  self.string_val(),
                        quoted: false,
                    };
                    Ok(TokenData::new(token, index, char_width))
                }
                _ => Ok(TokenData::new(self.numeric_val(), index, char_width)),
            },

            _ => match (opts.non_finite, expecting_string) {
                (ParseNonFinite::Error, _) => {
                    Err(SnbtError::ambiguous_token(input, index, char_width))
                }
                (ParseNonFinite::AsDetected, true) | (ParseNonFinite::AsString, _) => {
                    Ok(TokenData {
                        token: Token::String {
                            value:  self.string_val(),
                            quoted: false,
                        },
                        index,
                        char_width,
                    })
                }
                (ParseNonFinite::AsDetected, false) | (ParseNonFinite::AsFloat, _) => {
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
                    } else if matches!(opts.version, SnbtVersion::UpdatedJava) {
                        Err(SnbtError::invalid_number(
                            input,
                            index,
                            char_width,
                            NumericParseError::NonfiniteFloat,
                        ))
                    } else {
                        Ok(TokenData::new(self.numeric_val(), index, char_width))
                    }
                }
            },
        }
    }
}
