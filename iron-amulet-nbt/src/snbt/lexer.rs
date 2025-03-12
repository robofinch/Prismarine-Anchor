use std::str::Chars;
use std::{char, mem, str};
use std::{borrow::Cow, iter::Peekable, str::CharIndices};

use crate::tag::NbtTag;
use super::{SnbtVersion, SnbtError};
use super::utils::{allowed_unquoted, chars_to_u8, chars_to_u16, chars_to_u32, concat_arrays};


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


pub struct Lexer<'a> {
    raw: &'a str,
    chars: Peekable<CharIndices<'a>>,
    index: usize,
    peek_stack: Vec<Result<TokenData, SnbtError>>,
    version: SnbtVersion,
}

impl<'a> Lexer<'a> {
    pub fn new(raw: &'a str, version: SnbtVersion) -> Self {
        Lexer {
            raw,
            chars: raw.char_indices().peekable(),
            index: 0,
            peek_stack: Vec::new(),
            version,
        }
    }

    #[inline]
    pub fn snbt_version(&self) -> SnbtVersion {
        self.version
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
        delimiter: Option<fn(char) -> bool>,
    ) -> Option<&Result<TokenData, SnbtError>> {

        if self.peek_stack.is_empty() {

            if let Some(res) = self.next(delimiter) {
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
        delimiter: Option<fn(char) -> bool>,
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
            _ => return Some(self.slurp_token(delimiter)),
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

    // Asserts that the next token is the same type as the provided token.
    pub fn assert_next(&mut self, token: Token) -> Result<TokenData, SnbtError> {
        match self.next(None).transpose()? {
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

    // Collects a token from the character iterator.
    // It's only called once, so it's marked inline just in case.
    #[inline]
    fn slurp_token(&mut self, delimiter: Option<fn(char) -> bool>) -> Result<TokenData, SnbtError> {
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
                        let (ch, escape_char_len) = self.parse_escape_sequence(self.index - 1)?;

                        // Need to flush before pushing to self.raw_token_buffer
                        flush(
                            self.raw,
                            &mut raw_token_buffer,
                            flush_start,
                            self.index - 1
                        );

                        raw_token_buffer.to_mut().push(ch);
                        char_width += escape_char_len;
                        flush_start = self.index;
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

        Ok(self.parse_token(raw_token_buffer, start, char_width, quoted)?)
    }

    // Parses an isolated token
    // It's only called once, so it's marked inline just in case.
    #[inline]
    fn parse_token(
        &mut self,
        token_string: Cow<'_, str>,
        start: usize,
        char_width: usize,
        quoted: bool,
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

        // Check if the string is the bool(..) or uuid(..) operation
        if self.version == SnbtVersion::UpdatedJava && token_string.ends_with(FUNC_SUFFIX) {
            if token_string.starts_with(BOOL_FUNC) {
                return self.parse_bool_func(start, char_width, &token_string)

            } else if token_string.starts_with(UUID_FUNC) {
                return self.parse_uuid_func(start, char_width, &token_string)
            }
        }

        // Try parsing as a number
        match self.version {
            SnbtVersion::UpdatedJava => {
                // Check the first character of the string. Note that we checked above
                // whether token_string is empty, so it's nonempty here.
                let ch = token_string.chars().next().unwrap();
                if ch.is_ascii_digit() || matches!(ch, '.' | '+' | '-') {
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

    // Parses the body of an escape sequence (i.e., excluding the initial backslash),
    // and returns the character indicated by the escape as well as the number
    // of characters in the escape sequence's body.
    // `index` should be the index of the escape sequence's start, i.e., the backslash.
    fn parse_escape_sequence(
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

    // Parses a numeric token, in the UpdatedJava version.
    // Requires token_string to be nonempty. (Which is the case at its two call sites.)
    fn parse_updated_numeric(
        &self,
        index: usize,
        char_width: usize,
        num_string: &str,
    ) -> Result<TokenData, SnbtError> {

        todo!()
    }

    // Parse a numeric token, in the Original version.
    // Requires token_string to be nonempty. (Which is the case at its two call sites.)
    fn parse_original_numeric(
        &self,
        index: usize,
        char_width: usize,
        num_string: &str,
    ) -> Result<TokenData, SnbtError> {

        todo!()
    }

    // Parse the bool(..) operation
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

    // Parse the uuid(..) operation
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
            Token::Int(int_array[0] as i64),
            Token::Comma,
            Token::Int(int_array[0] as i64),
            Token::Comma,
            Token::Int(int_array[0] as i64),
            Token::Comma,
            Token::Int(int_array[0] as i64),
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

#[derive(Debug)]
pub struct TokenData {
    pub token: Token,
    pub index: usize,
    pub char_width: usize,
}

impl TokenData {
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

    pub fn into_value<T>(self) -> Result<T, Self>
    where Token: Into<Result<T, Token>> {
        match self.token.into() {
            Ok(value) => Ok(value),
            Err(tk) => Err(Self::new(tk, self.index, self.char_width)),
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
    Byte(i64),
    Short(i64),
    Int(i64),
    Long(i64),
    Float(f64),
    Double(f64),
}

impl Token {
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
            Token::Byte(value)   => Ok(NbtTag::Byte(value as i8)),
            Token::Short(value)  => Ok(NbtTag::Short(value as i16)),
            Token::Int(value)    => Ok(NbtTag::Int(value as i32)),
            Token::Long(value)   => Ok(NbtTag::Long(value)),
            Token::Float(value)  => Ok(NbtTag::Float(value as f32)),
            Token::Double(value) => Ok(NbtTag::Double(value)),
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

macro_rules! opt_int_from_token {
    ($int:ty) => {
        impl From<Token> for Result<$int, Token> {
            fn from(tk: Token) -> Self {
                match tk {
                    Token::Byte(x)  => Ok(x as $int),
                    Token::Short(x) => Ok(x as $int),
                    Token::Int(x)   => Ok(x as $int),
                    Token::Long(x)  => Ok(x as $int),
                    tk => Err(tk),
                }
            }
        }
    };
}

opt_int_from_token!(i8);
opt_int_from_token!(u8);
opt_int_from_token!(i16);
opt_int_from_token!(i32);
opt_int_from_token!(i64);

macro_rules! opt_float_from_token {
    ($float:ty) => {
        impl From<Token> for Result<$float, Token> {
            fn from(tk: Token) -> Self {
                match tk {
                    Token::Float(x)  => Ok(x as $float),
                    Token::Double(x) => Ok(x as $float),
                    tk => Err(tk),
                }
            }
        }
    };
}

opt_float_from_token!(f32);
opt_float_from_token!(f64);
