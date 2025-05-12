mod compound;
mod list;
#[cfg(feature = "comparable")]
pub mod comparable;
#[cfg(feature = "serde")]
mod serde_impl;


use std::fmt;
use std::{borrow::Cow, hash::Hash};
use std::fmt::{Debug, Display, Formatter};

use crate::{raw, snbt};
use crate::repr::NbtStructureError;
use crate::{
    settings::{EscapeSequence, SnbtParseOptions, SnbtWriteOptions, WriteNonFinite},
    snbt::{SnbtError, allowed_unquoted, is_ambiguous, starts_unquoted_number},
};


pub use self::{compound::NbtCompound, list::NbtList};


/// The hash map type utilized in this crate.
///
/// If `preserve_order` is enabled, the map will iterate over keys and values
/// in the order they were inserted by using the `IndexMap` type
/// from the crate <https://docs.rs/indexmap/latest/indexmap/>.
/// Otherwise, `std`'s `HashMap` is used.
#[cfg(feature = "preserve_order")]
pub type Map<T> = indexmap::IndexMap<String, T>;

/// The hash map type utilized in this crate.
///
/// If `preserve_order` is enabled, the map will iterate over keys and values
/// in the order they were inserted by using the `IndexMap` type
/// from the crate <https://docs.rs/indexmap/latest/indexmap/>.
/// Otherwise, `std`'s `HashMap` is used.
#[cfg(not(feature = "preserve_order"))]
pub type Map<T> = std::collections::HashMap<String, T>;


/// The generic NBT tag type, containing all supported tag variants
/// which wrap around a corresponding Rust type.
///
/// This type will implement both `Serialize` and `Deserialize` when the serde feature is enabled,
/// however this type should still be read and written with the utilities in the [`io`] module when
/// possible if speed is the main priority. When linking into the serde ecosystem, we ensured that
/// all tag types would have their data inlined into the resulting NBT output of our Serializer.
/// Because of this, NBT tags are only compatible with self-describing formats, and also have
/// slower deserialization implementations due to this restriction.
///
/// [`io`]: crate::io
#[derive(Clone, PartialEq)]
pub enum NbtTag {
    /// A signed, one-byte integer.
    Byte(i8),
    /// A signed, two-byte integer.
    Short(i16),
    /// A signed, four-byte integer.
    Int(i32),
    /// A signed, eight-byte integer.
    Long(i64),
    /// A 32-bit floating point value.
    Float(f32),
    /// A 64-bit floating point value.
    Double(f64),
    /// An array (vec) of one-byte integers. Minecraft treats this as an array of signed bytes.
    ByteArray(Vec<i8>),
    /// A UTF-8 string.
    String(String),
    /// A "string" tag that may be invalid UTF-8. This **does not** have strong support
    /// in this library; it exists solely to handle strange edge cases.
    /// The methods in the `io` and `snbt` modules support it.
    /// It is serialized into invalid but human-readable SNBT using a syntax similar to
    /// byte arrays: `[ByteString;1b;2b;2b]`, for instance. When serialized through serde,
    /// it is treated as a `ByteArray` instead.
    // TODO: provide support for ByteString in the serde module as well.
    ByteString(Vec<u8>),
    /// An NBT tag list.
    List(NbtList),
    /// An NBT tag compound.
    Compound(NbtCompound),
    /// An array (vec) of signed, four-byte integers.
    IntArray(Vec<i32>),
    /// An array (vec) of signed, eight-byte integers.
    LongArray(Vec<i64>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NbtContainerType {
    Compound,
    List,
    ByteArray,
    IntArray,
    LongArray,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NbtType {
    Byte,
    Short,
    Int,
    Long,
    Float,
    Double,
    ByteArray,
    String,
    List,
    Compound,
    IntArray,
    LongArray,
}

impl NbtTag {
    /// Returns the single character denoting this tag's type, or `None` if this tag has no type
    /// specifier.
    #[inline]
    pub fn type_specifier(&self) -> Option<&'static str> {
        match self {
            Self::Short(_)                     => Some("S"),
            Self::Float(_)                     => Some("F"),
            Self::Double(_)                    => Some("D"),
            Self::IntArray(_)                  => Some("I"),
            Self::Byte(_) | Self::ByteArray(_) => Some("B"),
            Self::Long(_) | Self::LongArray(_) => Some("L"),
            // Note that in particular, `Self::Int` has no type specifier.
            _ => None,
        }
    }

    /// Returns this tag's type.
    #[inline]
    pub fn tag_type(&self) -> NbtType {
        match self {
            Self::Byte(_)       => NbtType::Byte,
            Self::Short(_)      => NbtType::Short,
            Self::Int(_)        => NbtType::Int,
            Self::Long(_)       => NbtType::Long,
            Self::Float(_)      => NbtType::Float,
            Self::Double(_)     => NbtType::Double,
            Self::ByteArray(_)  => NbtType::ByteArray,
            #[expect(clippy::match_same_arms)]
            Self::String(_)     => NbtType::String,
            Self::ByteString(_) => NbtType::String,
            Self::List(_)       => NbtType::List,
            Self::Compound(_)   => NbtType::Compound,
            Self::IntArray(_)   => NbtType::IntArray,
            Self::LongArray(_)  => NbtType::LongArray,
        }
    }

    /// Returns this tag's numeric ID.
    #[inline]
    pub fn numeric_tag_id(&self) -> u8 {
        raw::id_for_tag(Some(self))
    }

    /// Returns which type of container this tag is, or `None` if it is not a container.
    #[inline]
    pub fn container_type(&self) -> Option<NbtContainerType> {
        match self {
            Self::Compound(_)  => Some(NbtContainerType::Compound),
            Self::List(_)      => Some(NbtContainerType::List),
            Self::ByteArray(_) => Some(NbtContainerType::ByteArray),
            Self::IntArray(_)  => Some(NbtContainerType::IntArray),
            Self::LongArray(_) => Some(NbtContainerType::LongArray),
            _ => None,
        }
    }

    #[inline]
    pub(crate) fn tag_name(&self) -> &'static str {
        match self {
            Self::Byte(_)       => "Byte",
            Self::Short(_)      => "Short",
            Self::Int(_)        => "Int",
            Self::Long(_)       => "Long",
            Self::Float(_)      => "Float",
            Self::Double(_)     => "Double",
            #[expect(clippy::match_same_arms)]
            Self::String(_)     => "String",
            Self::ByteString(_) => "String",
            Self::ByteArray(_)  => "ByteArray",
            Self::IntArray(_)   => "IntArray",
            Self::LongArray(_)  => "LongArray",
            Self::Compound(_)   => "Compound",
            Self::List(_)       => "List",
        }
    }

    /// Parses an NBT tag from SNBT
    #[inline]
    pub fn from_snbt(input: &str, opts: SnbtParseOptions) -> Result<Self, SnbtError> {
        snbt::parse_any(input, opts)
    }

    /// Converts this NBT tag into a valid, parsable SNBT string with no extraneous spacing. This
    /// method should not be used to generate user-facing text, rather [`to_pretty_snbt`] should
    /// be used instead. Additionally, this function uses the default `SnbtWriteOptions`
    /// for the original version of SNBT.
    ///
    /// If control over SNBT features is desired, use `to_snbt_with_options`.
    ///
    /// If finer control over the output is desired, then the tag can be formatted
    /// via the standard library's [`format!`] macro to pass additional formatting parameters.
    /// Note that some formatting parameters may result in invalid SNBT.
    pub fn to_snbt(&self) -> String {
        format!("{self:?}")
    }

    /// Converts this NBT tag into a valid, parsable SNBT string with extra spacing for
    /// readability. Additionally, this function uses the default `SnbtWriteOptions`
    /// for the original version of SNBT.
    ///
    /// If control over SNBT features is desired, use `to_pretty_snbt_with_options`.
    ///
    /// If a more compact SNBT representation is desired, then use [`to_snbt`].
    ///
    /// If finer control over the output is desired, then the tag can be formatted via the standard
    /// library's [`format!`] macro to pass additional formatting parameters.
    /// Note that some formatting parameters may result in invalid SNBT.
    pub fn to_pretty_snbt(&self) -> String {
        format!("{self:#?}")
    }

    /// Converts this NBT tag into a valid, parsable SNBT string with no extraneous spacing.
    /// This method should not be used to generate user-facing text,
    /// rather [`to_pretty_snbt`] should be used instead.
    ///
    /// If finer control over the output is desired, then the tag can be formatted
    /// via the standard library's [`format!`] macro to pass additional formatting parameters.
    /// Note that some formatting parameters may result in invalid SNBT.
    pub fn to_snbt_with_options(&self, opts: SnbtWriteOptions) -> String {
        format!("{:?}", TagWithOptions::new(self, opts))
    }

    /// Converts this NBT tag into a valid, parsable SNBT string with extra spacing for readability.
    ///
    /// If a more compact SNBT representation is desired, then use [`to_snbt`].
    ///
    /// If finer control over the output is desired, then the tag can be formatted via the standard
    /// library's [`format!`] macro to pass additional formatting parameters.
    /// Note that some formatting parameters may result in invalid SNBT.
    // TODO: test if providing weird formatting parameters can actually affect output
    pub fn to_pretty_snbt_with_limit(&self, opts: SnbtWriteOptions) -> String {
        format!("{:#?}", TagWithOptions::new(self, opts))
    }

    /// Returns whether or not the given string needs to be quoted to form valid SNBT.
    #[inline]
    pub fn should_quote(string: &str) -> bool {
        // Empty strings, strings whose first char collides with numbers,
        // and strings with more than the restricted set of characters that may be unquoted
        // all need to be quoted.

        if string.is_empty() {
            return true;
        }

        if let Some(first) = string.chars().next() {
            // The older SNBT versions might still allow this string to be unquoted
            // if it's "not confused with other data types" according to minecraft.wiki,
            // and the newer SNBT version requires it to be quoted.
            // The simplest and most compatible option is to quote it.
            if starts_unquoted_number(first) {
                return true;
            }
        }

        // If any of the characters aren't allowed to be unquoted, then the string must
        // be quoted
        for ch in string.chars() {
            if !allowed_unquoted(ch) {
                return true;
            }
        }

        // If the string would have ambiguous type if it were unquoted, like "true",
        // then it should be quoted.
        is_ambiguous(string)
    }

    /// If necessary, wraps the given string in quotes and escapes any quotes
    /// contained in the original string.
    /// May also apply other escape sequences based on provided options.
    pub fn string_to_snbt(string: &str, opts: SnbtWriteOptions) -> Cow<'_, str> {
        if !Self::should_quote(string) {
            return Cow::Borrowed(string);
        }

        // Determine the best option for the surrounding quotes to minimize escape sequences
        let surrounding = if string.contains('"') { '\'' } else { '"' };

        let mut snbt_string = String::with_capacity(2 + string.len());
        snbt_string.push(surrounding);

        // Note that the newer SNBT version supports more unicode escapes,
        // but they don't seem to be mandatory, as the strings should already
        // allow most UTF-8 or CESU-8 characters, at least in theory.
        let escapes = opts.enabled_escape_sequences;

        // Construct the string accounting for escape sequences
        for ch in string.chars() {
            match ch {
                // Escapes for '\'', '"', and '\\' cannot be controlled by options.
                _ if ch == surrounding || ch == '\\' => snbt_string.push('\\'),

                // Escapes to these characters aren't applied unless directly enabled
                '\n' => {
                    if escapes.is_enabled(EscapeSequence::N) {
                        snbt_string.push_str("\\n");
                        continue;
                    }
                }
                '\r' => {
                    if escapes.is_enabled(EscapeSequence::R) {
                        snbt_string.push_str("\\r");
                        continue;
                    }
                }
                ' ' => {
                    if escapes.is_enabled(EscapeSequence::S) {
                        snbt_string.push_str("\\s");
                        continue;
                    }
                }
                '\t' => {
                    if escapes.is_enabled(EscapeSequence::T) {
                        snbt_string.push_str("\\t");
                        continue;
                    }
                }

                // Note the slight difference in syntax; these characters could be escaped
                // with unicode escapes
                '\x08' if escapes.is_enabled(EscapeSequence::B) => {
                    snbt_string.push_str("\\b");
                    continue;
                }
                '\x0c' if escapes.is_enabled(EscapeSequence::F) => {
                    snbt_string.push_str("\\f");
                    continue;
                }

                _ => {
                    if !ch.is_ascii_graphic() {
                        let num = format!("{:x}", ch as u32);
                        // Note that each hex digit has a length of 1 byte
                        snbt_string.push_str(match num.len() {
                            1 => "\\x0",
                            2 => "\\x",
                            3 => "\\u0",
                            4 => "\\u",
                            5 => "\\U000",
                            6 => "\\U00",
                            7 => "\\U0",
                            // 0 and strictly greater than 8 are impossible,
                            // but might as well throw them in this case.
                            _ => "\\U",
                        });
                        snbt_string.push_str(&num);
                    }
                }
            }
            snbt_string.push(ch);
        }

        snbt_string.push(surrounding);
        Cow::Owned(snbt_string)
    }

    /// Used in the `display_and_debug` macro below
    #[inline]
    fn to_formatted_snbt(&self, f: &mut Formatter<'_>, opts: SnbtWriteOptions) -> fmt::Result {
        self.recursively_format_snbt(&mut String::new(), f, 0, opts)
    }

    /// Helper function for `Self::recursively_format_snbt`
    #[expect(clippy::write_with_newline, reason = "clarity")]
    fn write_prim_list<D: Display>(
        f:                   &mut Formatter<'_>,
        list:                &[D],
        indent:              &mut String,
        list_header:         &str,
        element_type_suffix: &str,
    ) -> fmt::Result {
        if list.is_empty() {
            return write!(f, "[{list_header};]");
        }

        if f.alternate() {
            indent.push_str("    ");
            write!(f, "[\n{indent}{list_header};\n")?;
        } else {
            write!(f, "[{list_header};")?;
        }

        let last_index = list.len() - 1;
        for (index, element) in list.iter().enumerate() {
            if f.alternate() {
                write!(f, "{indent}")?;
            }
            Display::fmt(element, f)?;
            write!(f, "{element_type_suffix}")?;

            if index != last_index {
                if f.alternate() {
                    write!(f, ",\n")?;
                } else {
                    write!(f, ",")?;
                }
            }
        }

        if f.alternate() {
            indent.truncate(indent.len() - 4);
            write!(f, "\n{indent}]")
        } else {
            write!(f, "]")
        }
    }

    /// Helper function for `Self::recursively_format_snbt`
    #[inline]
    fn write<D: Display>(
        f:           &mut Formatter<'_>,
        value:       &D,
        type_suffix: &str,
    ) -> fmt::Result {
        Display::fmt(value, f)?;
        write!(f, "{type_suffix}")
    }

    /// Helper function for `Self::recursively_format_snbt`
    fn check_depth_limit(
        f:                  &mut Formatter<'_>,
        current_depth:      u32,
        opts:               SnbtWriteOptions,
        recursive_tag_name: &'static str,
    ) -> fmt::Result {
        // Note that depths 0 ..= depth_limit.0 are the valid depths.
        // if depth == depth_limit.limit, then this is the last depth level
        // we are allowed to write anything to. If the next tag would recurse,
        // we need to stop here and try to return an error or something.
        // (We use >= instead of == just in case, but > should never occur.)
        if current_depth >= opts.depth_limit.0 {

            // Converting to a string should be infallible; we can't simply error out.
            // Instead, unfortunately, we must just print something that
            // hopefully indicates the issue.
            log::warn!(
                "Depth limit of {} reached; could not add {} tag",
                opts.depth_limit.0,
                recursive_tag_name,
            );

            let err_msg_str = format!(
                "Depth limit of {} reached; could not add {} tag",
                opts.depth_limit.0,
                recursive_tag_name,
            );
            let err_msg_tag = Self::string_to_snbt(&err_msg_str, opts);

            write!(f, "{err_msg_tag}")

        } else {
            Ok(())
        }
    }

    fn recursively_format_snbt(
        &self,
        indent:        &mut String,
        f:             &mut Formatter<'_>,
        current_depth: u32,
        opts:          SnbtWriteOptions,
    ) -> fmt::Result {

        macro_rules! write_floating_point {
            ($f:expr, $opts:expr, $value:expr, $ts:expr, $non_finite_ts:expr, $float_type:ty) => {
                match $opts.non_finite {
                    WriteNonFinite::PrintFloats => {
                        let float = if $value.is_finite() {
                            $value
                        } else if $value.is_infinite() {
                            if *$value > 0. {
                                &<$float_type>::MAX
                            } else {
                                &<$float_type>::MIN
                            }
                        } else {
                            &<$float_type>::NAN
                        };
                        Self::write($f, float, $ts)
                    }
                    WriteNonFinite::PrintStrings => {
                        if $value.is_finite() {
                            Self::write($f, $value, $ts)
                        } else {
                            let value_str = if $value.is_infinite() {
                                if *$value > 0. {
                                    "Infinity"
                                } else {
                                    "-Infinity"
                                }
                            } else {
                                "NaN"
                            };
                            write!($f, "{}{}", value_str, $non_finite_ts)
                        }
                    }
                }
            };
        }

        let ts = self.type_specifier().unwrap_or("");

        match self {
            Self::Byte(value)       => Self::write(f, value, ts),
            Self::Short(value)      => Self::write(f, value, ts),
            Self::Int(value)        => Self::write(f, value, ts),
            Self::Long(value)       => Self::write(f, value, ts),
            Self::Float(value)      => write_floating_point!(f, opts, value, ts, "f", f32),
            Self::Double(value)     => write_floating_point!(f, opts, value, ts, "d", f64),
            Self::ByteArray(value)  => Self::write_prim_list(f, value, indent, ts, ts),
            Self::String(value)     => write!(f, "{}", Self::string_to_snbt(value, opts)),
            Self::ByteString(value) => {
                if let Ok(string) = String::from_utf8(value.clone()) {
                    write!(f, "{}", Self::string_to_snbt(&string, opts))
                } else {
                    // If you're writing an invalid string to SNBT... well, the output
                    // has to be a valid string. This isn't valid SNBT, but it should be
                    // useful for debugging, I think.
                    // This is printed as `[ByteString; 1B, 2B, 3B, 4B]`, for instance
                    Self::write_prim_list(f, value, indent, "ByteString", "B")
                }
            }
            Self::List(value) => {

                Self::check_depth_limit(f, current_depth, opts, "List")?;

                // Note that List and Compound increment current_depth for their child members,
                // so incrementing it here would be a logic error.
                // Conceptually, current_depth is the depth of that list tag,
                // and that list tag *is* the current NbtTag, more or less.
                value.recursively_format_snbt(indent, f, current_depth, opts)
            }
            Self::Compound(value) => {
                Self::check_depth_limit(f, current_depth, opts, "Compound")?;
                value.recursively_format_snbt(indent, f, current_depth, opts)
            }
            Self::IntArray(value)  => Self::write_prim_list(f, value, indent, ts, ts),
            Self::LongArray(value) => Self::write_prim_list(f, value, indent, ts, ts),
        }
    }
}

// Implement the from trait for all the tag's internal types
macro_rules! tag_from {
    ($($type:ty, $tag:ident);* $(;)?) => {
        $(
            impl From<$type> for NbtTag {
                #[inline]
                fn from(value: $type) -> NbtTag {
                    NbtTag::$tag(value)
                }
            }
        )*
    };
}

tag_from!(
    i8,  Byte;
    i16, Short;
    i32, Int;
    i64, Long;
    f32, Float;
    f64, Double;
    Vec<i8>,     ByteArray;
    String,      String;
    NbtList,     List;
    NbtCompound, Compound;
    Vec<i32>,    IntArray;
    Vec<i64>,    LongArray;
);

impl From<&str> for NbtTag {
    #[inline]
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<&String> for NbtTag {
    #[inline]
    fn from(value: &String) -> Self {
        Self::String(value.clone())
    }
}

impl From<bool> for NbtTag {
    #[inline]
    fn from(value: bool) -> Self {
        Self::Byte(if value { 1 } else { 0 })
    }
}

impl From<u8> for NbtTag {
    #[inline]
    fn from(value: u8) -> Self {
        Self::Byte(value as i8)
    }
}

impl From<Vec<u8>> for NbtTag {
    #[inline]
    fn from(value: Vec<u8>) -> Self {
        Self::ByteArray(raw::cast_byte_buf_to_signed(value))
    }
}

macro_rules! prim_from_tag {
    ($($type:ty, $tag:ident);* $(;)?) => {
        $(
            impl TryFrom<&NbtTag> for $type {
                type Error = NbtStructureError;

                #[inline]
                fn try_from(tag: &NbtTag) -> Result<Self, Self::Error> {
                    if let NbtTag::$tag(value) = tag {
                        Ok(*value)
                    } else {
                        Err(NbtStructureError::type_mismatch(stringify!($tag), tag.tag_name()))
                    }
                }
            }
        )*
    };
}

prim_from_tag!(
    i8,  Byte;
    i16, Short;
    i32, Int;
    i64, Long;
    f32, Float;
    f64, Double;
);

impl TryFrom<&NbtTag> for bool {
    type Error = NbtStructureError;

    fn try_from(tag: &NbtTag) -> Result<Self, Self::Error> {
        match *tag {
            NbtTag::Byte(value)  => Ok(value != 0),
            NbtTag::Short(value) => Ok(value != 0),
            NbtTag::Int(value)   => Ok(value != 0),
            NbtTag::Long(value)  => Ok(value != 0),
            _ => Err(NbtStructureError::type_mismatch(
                "Byte, Short, Int, or Long",
                tag.tag_name(),
            )),
        }
    }
}

impl TryFrom<&NbtTag> for u8 {
    type Error = NbtStructureError;

    #[inline]
    fn try_from(tag: &NbtTag) -> Result<Self, Self::Error> {
        #[expect(clippy::use_self, reason = "clarity; it's a u8")]
        match *tag {
            NbtTag::Byte(value) => Ok(value as u8),
            _ => Err(NbtStructureError::type_mismatch("Byte", tag.tag_name())),
        }
    }
}

macro_rules! ref_from_tag {
    ($($type:ty, $tag:ident);* $(;)?) => {
        $(
            impl<'a> TryFrom<&'a NbtTag> for &'a $type {
                type Error = NbtStructureError;

                #[inline]
                fn try_from(tag: &'a NbtTag) -> Result<Self, Self::Error> {
                    if let NbtTag::$tag(value) = tag {
                        Ok(value)
                    } else {
                        Err(NbtStructureError::type_mismatch(stringify!($tag), tag.tag_name()))
                    }
                }
            }

            impl<'a> TryFrom<&'a mut NbtTag> for &'a mut $type {
                type Error = NbtStructureError;

                #[inline]
                fn try_from(tag: &'a mut NbtTag) -> Result<Self, Self::Error> {
                    if let NbtTag::$tag(value) = tag {
                        Ok(value)
                    } else {
                        Err(NbtStructureError::type_mismatch(stringify!($tag), tag.tag_name()))
                    }
                }
            }
        )*
    };
}

ref_from_tag!(
    i8,  Byte;
    i16, Short;
    i32, Int;
    i64, Long;
    f32, Float;
    f64, Double;
    Vec<i8>,     ByteArray;
    [i8],        ByteArray;
    String,      String;
    str,         String;
    NbtList,     List;
    NbtCompound, Compound;
    Vec<i32>,    IntArray;
    [i32],       IntArray;
    Vec<i64>,    LongArray;
    [i64],       LongArray;
);

impl<'a> TryFrom<&'a NbtTag> for &'a u8 {
    type Error = NbtStructureError;

    #[inline]
    fn try_from(tag: &'a NbtTag) -> Result<Self, Self::Error> {
        if let NbtTag::Byte(value) = tag {
            Ok(raw::ref_i8_to_ref_u8(value))
        } else {
            Err(NbtStructureError::type_mismatch("Byte", tag.tag_name()))
        }
    }
}

impl<'a> TryFrom<&'a NbtTag> for &'a [u8] {
    type Error = NbtStructureError;

    #[inline]
    fn try_from(tag: &'a NbtTag) -> Result<Self, Self::Error> {
        if let NbtTag::ByteArray(value) = tag {
            Ok(raw::cast_bytes_to_unsigned(value.as_slice()))
        } else {
            Err(NbtStructureError::type_mismatch(
                "ByteArray",
                tag.tag_name(),
            ))
        }
    }
}

macro_rules! from_tag {
    ($($type:ty, $tag:ident);* $(;)?) => {
        $(
            impl TryFrom<NbtTag> for $type {
                type Error = NbtStructureError;

                #[inline]
                fn try_from(tag: NbtTag) -> Result<Self, Self::Error> {
                    if let NbtTag::$tag(value) = tag {
                        Ok(value)
                    } else {
                        Err(NbtStructureError::type_mismatch(stringify!($tag), tag.tag_name()))
                    }
                }
            }
        )*
    };
}

from_tag!(
    i8,  Byte;
    i16, Short;
    i32, Int;
    i64, Long;
    f32, Float;
    f64, Double;
    Vec<i8>,     ByteArray;
    String,      String;
    NbtList,     List;
    NbtCompound, Compound;
    Vec<i32>,    IntArray;
    Vec<i64>,    LongArray;
);

impl TryFrom<NbtTag> for Vec<u8> {
    type Error = NbtStructureError;

    #[inline]
    fn try_from(tag: NbtTag) -> Result<Self, Self::Error> {
        if let NbtTag::ByteArray(value) = tag {
            Ok(raw::cast_byte_buf_to_unsigned(value))
        } else {
            Err(NbtStructureError::type_mismatch(
                "ByteArray",
                tag.tag_name(),
            ))
        }
    }
}

macro_rules! display_and_debug {
    ($tag:ty, $name:ident) => {
        impl Display for $tag {
            #[inline]
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                self.to_formatted_snbt(f, SnbtWriteOptions::default_original())
            }
        }

        impl Debug for $tag {
            #[inline]
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                self.to_formatted_snbt(f, SnbtWriteOptions::default_original())
            }
        }

        pub struct $name<'a> {
            tag:  &'a $tag,
            opts: SnbtWriteOptions,
        }

        impl<'a> $name<'a> {
            pub fn new(tag: &'a $tag, opts: SnbtWriteOptions) -> Self {
                Self { tag, opts }
            }
        }

        impl<'a> Display for $name<'a> {
            #[inline]
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                self.tag.to_formatted_snbt(f, self.opts)
            }
        }

        impl<'a> Debug for $name<'a> {
            #[inline]
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                self.tag.to_formatted_snbt(f, self.opts)
            }
        }
    };
}

display_and_debug!(NbtTag,      TagWithOptions);
display_and_debug!(NbtList,     ListWithOptions);
display_and_debug!(NbtCompound, CompoundWithOptions);
