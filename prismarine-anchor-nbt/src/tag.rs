#[cfg(feature = "comparable")]
pub mod comparable;


use std::fmt;
use std::hash::Hash;
use std::{
    borrow::{Borrow, BorrowMut, Cow},
    fmt::{Debug, Display, Formatter},
    ops::{Deref, DerefMut, Index, IndexMut},
};

use crate::{raw, snbt};
use crate::{
    repr::{NbtReprError, NbtStructureError},
    settings::{EscapeSequence, SnbtParseOptions, SnbtWriteOptions, WriteNonFinite},
    snbt::{allowed_unquoted, is_ambiguous, SnbtError, starts_unquoted_number},
};


/// The hash map type utilized in this crate.
/// If `preserve_order` is enabled, the map will iterate over keys and values
/// in the order they were inserted by using the `IndexMap` type
/// from the crate <https://docs.rs/indexmap/latest/indexmap/>.
/// Otherwise, `std`'s `HashMap` is used.
#[cfg(feature = "preserve_order")]
pub type Map<T> = indexmap::IndexMap<String, T>;

/// The hash map type utilized in this crate.
/// If `preserve_order` is enabled, the map will iterate over keys and values
/// in the order they were inserted by using the `IndexMap` type
/// from the crate <https://docs.rs/indexmap/latest/indexmap/>.
/// Otherwise, `std`'s `HashMap` is used.
#[cfg(not(feature = "preserve_order"))]
pub type Map<T> = std::collections::HashMap<String, T>;


/// The generic NBT tag type, containing all supported tag variants which wrap around a corresponding
/// rust type.
///
/// This type will implement both `Serialize` and `Deserialize` when the serde feature is enabled,
/// however this type should still be read and written with the utilities in the [`io`] module when
/// possible if speed is the main priority. When linking into the serde ecosystem, we ensured that all
/// tag types would have their data inlined into the resulting NBT output of our Serializer. Because of
/// this, NBT tags are only compatible with self-describing formats, and also have slower deserialization
/// implementations due to this restriction.
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
    pub fn type_specifier(&self) -> Option<&'static str> {
        match self {
            Self::Byte(_)      => Some("B"),
            Self::Short(_)     => Some("S"),
            Self::Long(_)      => Some("L"),
            Self::Float(_)     => Some("F"),
            Self::Double(_)    => Some("D"),
            Self::ByteArray(_) => Some("B"),
            Self::IntArray(_)  => Some("I"),
            Self::LongArray(_) => Some("L"),
            _ => None,
        }
    }

    /// Returns this tag's type.
    pub fn tag_type(&self) -> NbtType {
        match self {
            Self::Byte(_)      => NbtType::Byte,
            Self::Short(_)     => NbtType::Short,
            Self::Int(_)       => NbtType::Int,
            Self::Long(_)      => NbtType::Long,
            Self::Float(_)     => NbtType::Float,
            Self::Double(_)    => NbtType::Double,
            Self::ByteArray(_) => NbtType::ByteArray,
            Self::String(_)    => NbtType::String,
            Self::List(_)      => NbtType::List,
            Self::Compound(_)  => NbtType::Compound,
            Self::IntArray(_)  => NbtType::IntArray,
            Self::LongArray(_) => NbtType::LongArray,
        }
    }

    /// Returns which type of container this tag is, or `None` if it is not a container.
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

    pub(crate) fn tag_name(&self) -> &'static str {
        match self {
            NbtTag::Byte(_) => "Byte",
            NbtTag::Short(_) => "Short",
            NbtTag::Int(_) => "Int",
            NbtTag::Long(_) => "Long",
            NbtTag::Float(_) => "Float",
            NbtTag::Double(_) => "Double",
            NbtTag::String(_) => "String",
            NbtTag::ByteArray(_) => "ByteArray",
            NbtTag::IntArray(_) => "IntArray",
            NbtTag::LongArray(_) => "LongArray",
            NbtTag::Compound(_) => "Compound",
            NbtTag::List(_) => "List",
        }
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
        format!("{:?}", self)
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
        format!("{:#?}", self)
    }

    /// Converts this NBT tag into a valid, parsable SNBT string with no extraneous spacing.
    /// This method should not be used to generate user-facing text,
    /// rather [`to_pretty_snbt`] should be used instead.
    ///
    /// If finer control over the output is desired, then the tag can be formatted
    /// via the standard library's [`format!`] macro to pass additional formatting parameters.
    /// Note that some formatting parameters may result in invalid SNBT.
    pub fn to_snbt_with_options(&self, opts: SnbtWriteOptions) -> String {
        format!("{:?}", TagWithOptions::new(&self, opts))
    }

    /// Converts this NBT tag into a valid, parsable SNBT string with extra spacing for readability.
    ///
    /// If a more compact SNBT representation is desired, then use [`to_snbt`].
    ///
    /// If finer control over the output is desired, then the tag can be formatted via the standard
    /// library's [`format!`] macro to pass additional formatting parameters.
    /// Note that some formatting parameters may result in invalid SNBT.
    pub fn to_pretty_snbt_with_limit(&self, opts: SnbtWriteOptions) -> String {
        format!("{:#?}", TagWithOptions::new(&self, opts))
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
            if !allowed_unquoted(ch)  {
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
        let surrounding: char;
        if string.contains('"') {
            surrounding = '\'';
        } else {
            surrounding = '"';
        }

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
                '\n' => if escapes.is_enabled(EscapeSequence::N) {
                    snbt_string.push_str("\\n");
                    continue;
                }
                '\r' => if escapes.is_enabled(EscapeSequence::R) {
                    snbt_string.push_str("\\r");
                    continue;
                }
                ' ' => if escapes.is_enabled(EscapeSequence::S) {
                    snbt_string.push_str("\\s");
                    continue;
                }
                '\t' => if escapes.is_enabled(EscapeSequence::T) {
                    snbt_string.push_str("\\t");
                    continue;
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

                _ => if !ch.is_ascii_graphic() {
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
            snbt_string.push(ch);
        }

        snbt_string.push(surrounding);
        Cow::Owned(snbt_string)
    }

    /// Parses an NBT tag from SNBT
    #[inline]
    pub fn from_snbt(input: &str, opts: SnbtParseOptions) -> Result<Self, SnbtError> {
        snbt::parse_any(input, opts)
    }

    #[inline]
    fn to_formatted_snbt(&self, f: &mut Formatter<'_>, opts: SnbtWriteOptions) -> fmt::Result {
        self.recursively_format_snbt(&mut String::new(), f, 0, opts)
    }

    #[allow(clippy::write_with_newline)]
    fn recursively_format_snbt(
        &self,
        indent: &mut String,
        f: &mut Formatter<'_>,
        current_depth: u32,
        opts: SnbtWriteOptions,
    ) -> fmt::Result {

        fn write_list(
            list: &[impl Display],
            indent: &mut String,
            ts: &str,
            f: &mut Formatter<'_>,
        ) -> fmt::Result {
            if list.is_empty() {
                return write!(f, "[{};]", ts);
            }

            if f.alternate() {
                indent.push_str("    ");
                write!(f, "[\n{}{};\n", indent, ts)?;
            } else {
                write!(f, "[{};", ts)?;
            }

            let last_index = list.len() - 1;
            for (index, element) in list.iter().enumerate() {
                if f.alternate() {
                    write!(f, "{}", indent)?;
                }
                Display::fmt(element, f)?;
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
                write!(f, "\n{}]", &indent)
            } else {
                write!(f, "]")
            }
        }

        #[inline]
        fn write(value: &impl Display, ts: Option<&str>, f: &mut Formatter<'_>) -> fmt::Result {
            match ts {
                Some(ts) => {
                    Display::fmt(value, f)?;
                    write!(f, "{}", ts)
                }
                None => Display::fmt(value, f),
            }
        }

        let ts = self.type_specifier();

        match self {
            NbtTag::Byte(value)  => write(value, ts, f),
            NbtTag::Short(value) => write(value, ts, f),
            NbtTag::Int(value)   => write(value, ts, f),
            NbtTag::Long(value)  => write(value, ts, f),
            NbtTag::Float(value) => match opts.non_finite {
                WriteNonFinite::PrintFloats => {
                    let float = if value.is_finite() {
                        value
                    } else if value.is_infinite() {
                        if *value > 0. {
                            &f32::MAX
                        } else {
                            &f32::MIN
                        }
                    } else {
                        &f32::NAN
                    };
                    write(float, ts, f)
                }
                WriteNonFinite::PrintStrings => {
                    if value.is_finite() {
                        write(value, ts, f)
                    } else {
                        let string = if value.is_infinite() {
                            if *value > 0. {
                                "Infinityf"
                            } else {
                                "-Infinityf"
                            }
                        } else {
                            "NaNf"
                        };
                        write(&string, ts, f)
                    }
                }
            }
            NbtTag::Double(value) => match opts.non_finite {
                WriteNonFinite::PrintFloats => {
                    let float = if value.is_finite() {
                        value
                    } else if value.is_infinite() {
                        if *value > 0. {
                            &f64::MAX
                        } else {
                            &f64::MIN
                        }
                    } else {
                        &f64::NAN
                    };
                    write(float, ts, f)
                }
                WriteNonFinite::PrintStrings => {
                    if value.is_finite() {
                        write(value, ts, f)
                    } else {
                        let string = if value.is_infinite() {
                            if *value > 0. {
                                "Infinityd"
                            } else {
                                "-Infinityd"
                            }
                        } else {
                            "NaNd"
                        };
                        write(&string, ts, f)
                    }
                }
            }
            NbtTag::ByteArray(value) => write_list(&**value, indent, ts.unwrap(), f),
            NbtTag::String(value) => write!(f, "{}", Self::string_to_snbt(value, opts)),
            NbtTag::List(value) => if current_depth >= opts.depth_limit.0 {
                // Converting to a string should be infallible; we can't simply error out.
                // Instead, unfortunately, we must just print something that
                // hopefully indicates the issue.

                // Note that depths 0 ..= depth_limit.0 are the valid depths.
                // if depth == depth_limit.limit, then this is the last depth level
                // we are allowed to write anything to. If the next tag would recurse,
                // we need to stop here.
                // (We use >= above instead of == just in case, but > should never occur.)

                write!(f, "{}", Self::string_to_snbt(
                    &format!(
                        "Depth limit of {} reached; could not add List tag",
                        opts.depth_limit.0
                    ),
                    opts,
                ))
            } else {
                // Note that List and Compound increment current_depth for their child members,
                // so incrementing it here would be a logic error.
                // Conceptually, current_depth is the depth of that list tag,
                // and that list tag *is* the current NbtTag, more or less.
                value.recursively_format_snbt(indent, f, current_depth, opts)
            },
            NbtTag::Compound(value) => if current_depth >= opts.depth_limit.0 {
                write!(f, "{}", Self::string_to_snbt(
                    &format!(
                        "Depth limit of {} reached; could not add Compound tag",
                        opts.depth_limit.0
                    ),
                    opts,
                ))
            } else {
                value.recursively_format_snbt(indent, f, current_depth, opts)
            },
            NbtTag::IntArray(value)  => write_list(&**value, indent, ts.unwrap(), f),
            NbtTag::LongArray(value) => write_list(&**value, indent, ts.unwrap(), f),
        }
    }
}

// Implement the from trait for all the tag's internal types
macro_rules! tag_from {
    ($($type:ty, $tag:ident);*) => {
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
    i8, Byte;
    i16, Short;
    i32, Int;
    i64, Long;
    f32, Float;
    f64, Double;
    Vec<i8>, ByteArray;
    String, String;
    NbtList, List;
    NbtCompound, Compound;
    Vec<i32>, IntArray;
    Vec<i64>, LongArray
);

impl From<&str> for NbtTag {
    #[inline]
    fn from(value: &str) -> NbtTag {
        NbtTag::String(value.to_owned())
    }
}

impl From<&String> for NbtTag {
    #[inline]
    fn from(value: &String) -> NbtTag {
        NbtTag::String(value.clone())
    }
}

impl From<bool> for NbtTag {
    #[inline]
    fn from(value: bool) -> NbtTag {
        NbtTag::Byte(if value { 1 } else { 0 })
    }
}

impl From<u8> for NbtTag {
    #[inline]
    fn from(value: u8) -> Self {
        NbtTag::Byte(value as i8)
    }
}

impl From<Vec<u8>> for NbtTag {
    #[inline]
    fn from(value: Vec<u8>) -> Self {
        NbtTag::ByteArray(raw::cast_byte_buf_to_signed(value))
    }
}

macro_rules! prim_from_tag {
    ($($type:ty, $tag:ident);*) => {
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
    i8, Byte;
    i16, Short;
    i32, Int;
    i64, Long;
    f32, Float;
    f64, Double
);

impl TryFrom<&NbtTag> for bool {
    type Error = NbtStructureError;

    fn try_from(tag: &NbtTag) -> Result<Self, Self::Error> {
        match *tag {
            NbtTag::Byte(value) => Ok(value != 0),
            NbtTag::Short(value) => Ok(value != 0),
            NbtTag::Int(value) => Ok(value != 0),
            NbtTag::Long(value) => Ok(value != 0),
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
        match *tag {
            NbtTag::Byte(value) => Ok(value as u8),
            _ => Err(NbtStructureError::type_mismatch("Byte", tag.tag_name())),
        }
    }
}

macro_rules! ref_from_tag {
    ($($type:ty, $tag:ident);*) => {
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
    i8, Byte;
    i16, Short;
    i32, Int;
    i64, Long;
    f32, Float;
    f64, Double;
    Vec<i8>, ByteArray;
    [i8], ByteArray;
    String, String;
    str, String;
    NbtList, List;
    NbtCompound, Compound;
    Vec<i32>, IntArray;
    [i32], IntArray;
    Vec<i64>, LongArray;
    [i64], LongArray
);

impl<'a> TryFrom<&'a NbtTag> for &'a u8 {
    type Error = NbtStructureError;

    #[inline]
    fn try_from(tag: &'a NbtTag) -> Result<Self, Self::Error> {
        if let NbtTag::Byte(value) = tag {
            Ok(unsafe { &*(value as *const i8 as *const u8) })
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
    ($($type:ty, $tag:ident);*) => {
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
    i8, Byte;
    i16, Short;
    i32, Int;
    i64, Long;
    f32, Float;
    f64, Double;
    Vec<i8>, ByteArray;
    String, String;
    NbtList, List;
    NbtCompound, Compound;
    Vec<i32>, IntArray;
    Vec<i64>, LongArray
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

/// The NBT tag list type which is essentially just a wrapper for a vec of NBT tags.
///
/// This type will implement both `Serialize` and `Deserialize` when the serde feature is enabled,
/// however this type should still be read and written with the utilities in the [`io`] module when
/// possible if speed is the main priority. See [`NbtTag`] for more details.
///
/// [`io`]: crate::io
/// [`NbtTag`]: crate::tag::NbtTag
#[repr(transparent)]
#[derive(Clone, PartialEq)]
pub struct NbtList(pub(crate) Vec<NbtTag>);

impl NbtList {
    /// Returns a new NBT tag list with an empty internal vec.
    #[inline]
    pub const fn new() -> Self {
        NbtList(Vec::new())
    }

    /// Returns a mutable reference to the internal vector of this NBT list.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut Vec<NbtTag> {
        &mut self.0
    }

    /// Returns the internal vector of this NBT list.
    #[inline]
    pub fn into_inner(self) -> Vec<NbtTag> {
        self.0
    }

    /// Returns a new NBT tag list with the given initial capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        NbtList(Vec::with_capacity(capacity))
    }

    /// Clones the data in the given list and converts it into an [`NbtList`].
    #[inline]
    pub fn clone_from<'a, T, L>(list: L) -> Self
    where
        T: Clone + Into<NbtTag> + 'a,
        L: IntoIterator<Item = &'a T>,
    {
        NbtList(list.into_iter().map(|x| x.clone().into()).collect())
    }

    /// Iterates over this tag list, converting each tag reference into the specified type.
    #[inline]
    pub fn iter_map<'a, T: TryFrom<&'a NbtTag>>(
        &'a self,
    ) -> impl Iterator<Item = Result<T, <T as TryFrom<&'a NbtTag>>::Error>> + 'a {
        self.0.iter().map(|tag| T::try_from(tag))
    }

    /// Iterates over mutable references to the tags in this list, converting each tag reference into
    /// the specified type. See [`iter_map`](crate::tag::NbtList::iter_map) for usage details.
    #[inline]
    pub fn iter_mut_map<'a, T: TryFrom<&'a mut NbtTag>>(
        &'a mut self,
    ) -> impl Iterator<Item = Result<T, <T as TryFrom<&'a mut NbtTag>>::Error>> + 'a {
        self.0.iter_mut().map(|tag| T::try_from(tag))
    }

    /// Converts this tag list into a valid SNBT string. See `NbtTag::`[`to_snbt`] for details.
    ///
    /// [`to_snbt`]: crate::NbtTag::to_snbt
    pub fn to_snbt(&self) -> String {
        format!("{:?}", self)
    }

    /// Converts this tag list into a valid SNBT string with extra spacing for readability.
    /// See `NbtTag::`[`to_pretty_snbt`] for details.
    ///
    /// [`to_pretty_snbt`]: crate::NbtTag::to_pretty_snbt
    pub fn to_pretty_snbt(&self) -> String {
        format!("{:#?}", self)
    }

    /// Converts this tag list into a valid SNBT string.
    /// See `NbtTag::`[`to_snbt`] for details.
    ///
    /// [`to_snbt`]: crate::tag::NbtTag::to_snbt
    #[cfg(feature = "configurable_depth")]
    pub fn to_snbt_with_options(&self, opts: SnbtWriteOptions) -> String {
        format!("{:?}", ListWithOptions::new(&self, opts))
    }

    /// Converts this tag list into a valid SNBT string with extra spacing for readability.
    /// See `NbtTag::`[`to_pretty_snbt`] for details.
    ///
    /// [`to_pretty_snbt`]: crate::tag::NbtTag::to_pretty_snbt
    #[cfg(feature = "configurable_depth")]
    pub fn to_pretty_snbt_with_options(&self, opts: SnbtWriteOptions) -> String {
        format!("{:#?}", ListWithOptions::new(&self, opts))
    }

    /// Returns the length of this list.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if this tag list has a length of zero, false otherwise.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the value of the tag at the given index, or an error if the index is out of bounds
    /// or the the tag type does not match the type specified. This method should be used for
    /// obtaining primitives and shared references to lists and compounds.
    #[inline]
    pub fn get<'a, T>(&'a self, index: usize) -> Result<T, NbtReprError>
    where
        T: TryFrom<&'a NbtTag>,
        T::Error: Into<anyhow::Error>,
    {
        T::try_from(
            self.0
                .get(index)
                .ok_or_else(|| NbtStructureError::invalid_index(index, self.len()))?,
        )
        .map_err(NbtReprError::from_any)
    }

    /// Returns a mutable reference to the tag at the given index, or an error if the index is out
    /// of bounds or tag type does not match the type specified. This method should be used for
    /// obtaining mutable references to elements.
    #[inline]
    pub fn get_mut<'a, T>(&'a mut self, index: usize) -> Result<T, NbtReprError>
    where
        T: TryFrom<&'a mut NbtTag>,
        T::Error: Into<anyhow::Error>,
    {
        let len = self.len();
        T::try_from(
            self.0
                .get_mut(index)
                .ok_or_else(|| NbtStructureError::invalid_index(index, len))?,
        )
        .map_err(NbtReprError::from_any)
    }

    /// Returns a reference to the tag at the given index without any casting,
    /// or `None` if the index is out of bounds.
    pub fn get_tag(&self, index: usize) -> Option<&NbtTag> {
        self.0.get(index)
    }

    /// Returns a mutable reference to the tag at the given index without any casting,
    /// or `None` if the index is out of bounds.
    pub fn get_tag_mut(&mut self, index: usize) -> Option<&mut NbtTag> {
        self.0.get_mut(index)
    }

    /// While preserving the order of the `NbtList`, removes and returns the tag at the given index
    /// without any casting, or returns `None` if the index is out of bounds.
    pub fn remove_tag(&mut self, index: usize) -> Option<NbtTag> {
        if index < self.0.len() {
            Some(self.0.remove(index))
        } else {
            None
        }
    }

    /// Removes and returns the tag at the given index without any casting,
    /// or returns `None` if the index is out of bounds. Does not preserve the order
    /// of the `NbtList`.
    pub fn swap_remove_tag(&mut self, index: usize) -> Option<NbtTag> {
        if index < self.0.len() {
            Some(self.0.swap_remove(index))
        } else {
            None
        }
    }

    /// Pushes the given value to the back of the list after wrapping it in an `NbtTag`.
    #[inline]
    pub fn push<T: Into<NbtTag>>(&mut self, value: T) {
        self.0.push(value.into());
    }

    #[inline]
    fn to_formatted_snbt(&self, f: &mut Formatter<'_>, opts: SnbtWriteOptions) -> fmt::Result {
        self.recursively_format_snbt(&mut String::new(), f, 0, opts)
    }

    #[allow(clippy::write_with_newline)]
    fn recursively_format_snbt(
        &self,
        indent: &mut String,
        f: &mut Formatter<'_>,
        current_depth: u32,
        opts: SnbtWriteOptions,
    ) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "[]");
        }

        if f.alternate() {
            indent.push_str("    ");
            write!(f, "[\n")?;
        } else {
            write!(f, "[")?;
        }

        let last_index = self.len() - 1;
        for (index, element) in self.0.iter().enumerate() {
            if f.alternate() {
                write!(f, "{}", indent)?;
            }

            // Conceptually, current_depth is the depth of this List itself;
            // its elements are one recursive tag deeper.
            element.recursively_format_snbt(indent, f, current_depth+1, opts)?;

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
            write!(f, "\n{}]", indent)
        } else {
            write!(f, "]")
        }
    }
}

impl Default for NbtList {
    #[inline]
    fn default() -> Self {
        NbtList::new()
    }
}

impl<T: Into<NbtTag>> From<Vec<T>> for NbtList {
    #[inline]
    fn from(list: Vec<T>) -> Self {
        NbtList(list.into_iter().map(|x| x.into()).collect())
    }
}

impl IntoIterator for NbtList {
    type IntoIter = <Vec<NbtTag> as IntoIterator>::IntoIter;
    type Item = NbtTag;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a NbtList {
    type IntoIter = <&'a Vec<NbtTag> as IntoIterator>::IntoIter;
    type Item = &'a NbtTag;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut NbtList {
    type IntoIter = <&'a mut Vec<NbtTag> as IntoIterator>::IntoIter;
    type Item = &'a mut NbtTag;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl FromIterator<NbtTag> for NbtList {
    #[inline]
    fn from_iter<T: IntoIterator<Item = NbtTag>>(iter: T) -> Self {
        NbtList(Vec::from_iter(iter))
    }
}

impl AsRef<[NbtTag]> for NbtList {
    #[inline]
    fn as_ref(&self) -> &[NbtTag] {
        &self.0
    }
}

impl AsMut<[NbtTag]> for NbtList {
    #[inline]
    fn as_mut(&mut self) -> &mut [NbtTag] {
        &mut self.0
    }
}

impl Borrow<[NbtTag]> for NbtList {
    #[inline]
    fn borrow(&self) -> &[NbtTag] {
        &self.0
    }
}

impl BorrowMut<[NbtTag]> for NbtList {
    #[inline]
    fn borrow_mut(&mut self) -> &mut [NbtTag] {
        &mut self.0
    }
}

impl Deref for NbtList {
    type Target = [NbtTag];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for NbtList {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

impl Extend<NbtTag> for NbtList {
    #[inline]
    fn extend<T: IntoIterator<Item = NbtTag>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl Index<usize> for NbtList {
    type Output = NbtTag;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for NbtList {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

/// The NBT tag compound type which is essentially just a wrapper for a hash map of string keys
/// to tag values.
///
/// This type will implement both `Serialize` and `Deserialize` when the serde feature is enabled,
/// however this type should still be read and written with the utilities in the [`io`] module when
/// possible if speed is the main priority. See [`NbtTag`] for more details.
///
/// [`NbtTag`]: crate::NbtTag
/// [`io`]: crate::io
#[repr(transparent)]
#[derive(Clone, PartialEq)]
pub struct NbtCompound(pub(crate) Map<NbtTag>);

impl NbtCompound {
    /// Returns a new NBT tag compound with an empty internal hash map.
    #[inline]
    pub fn new() -> Self {
        NbtCompound(Map::new())
    }

    /// Returns a reference to the internal hash map of this compound.
    #[inline]
    pub fn inner(&self) -> &Map<NbtTag> {
        &self.0
    }

    /// Returns a mutable reference to the internal hash map of this compound.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut Map<NbtTag> {
        &mut self.0
    }

    /// Returns the internal hash map of this NBT compound.
    #[inline]
    pub fn into_inner(self) -> Map<NbtTag> {
        self.0
    }

    /// Returns a new NBT tag compound with the given initial capacity, unless the `comparable`
    /// feature is enabled, in which case no allocation is performed.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        #[cfg(not(feature = "comparable"))]
        {
            NbtCompound(Map::with_capacity(capacity))
        }
        #[cfg(feature = "comparable")]
        {
            let _ = capacity;
            NbtCompound(Map::new())
        }
    }

    /// Clones the data in the given map and converts it into an [`NbtCompound`](crate::tag::NbtCompound).
    #[inline]
    pub fn clone_from<'a, K, V, M>(map: &'a M) -> Self
    where
        K: Clone + Into<String> + 'a,
        V: Clone + Into<NbtTag> + 'a,
        &'a M: IntoIterator<Item = (&'a K, &'a V)>,
    {
        NbtCompound(
            map.into_iter()
                .map(|(key, value)| (key.clone().into(), value.clone().into()))
                .collect(),
        )
    }

    /// Iterates over this tag compound, converting each tag reference into the specified type. Each key is
    /// paired with the result of the attempted conversion into the specified type. The iterator will not
    /// terminate even if some conversions fail.
    #[inline]
    pub fn iter_map<'a, T: TryFrom<&'a NbtTag>>(
        &'a self,
    ) -> impl Iterator<Item = (&'a str, Result<T, <T as TryFrom<&'a NbtTag>>::Error>)> + 'a {
        self.0
            .iter()
            .map(|(key, tag)| (key.as_str(), T::try_from(tag)))
    }

    /// Iterates over this tag compound, converting each mutable tag reference into the specified type. See
    /// [`iter_map`](crate::tag::NbtCompound::iter_map) for details.
    #[inline]
    pub fn iter_mut_map<'a, T: TryFrom<&'a mut NbtTag>>(
        &'a mut self,
    ) -> impl Iterator<Item = (&'a str, Result<T, <T as TryFrom<&'a mut NbtTag>>::Error>)> + 'a
    {
        self.0
            .iter_mut()
            .map(|(key, tag)| (key.as_str(), T::try_from(tag)))
    }

    /// Converts this tag compound into a valid SNBT string. See `NbtTag::`[`to_snbt`] for details.
    ///
    /// [`to_snbt`]: crate::tag::NbtTag::to_snbt
    pub fn to_snbt(&self) -> String {
        format!("{:?}", self)
    }

    /// Converts this tag compound into a valid SNBT string with extra spacing for readability.
    /// See `NbtTag::`[`to_pretty_snbt`] for details.
    ///
    /// [`to_pretty_snbt`]: crate::tag::NbtTag::to_pretty_snbt
    pub fn to_pretty_snbt(&self) -> String {
        format!("{:#?}", self)
    }

    /// Converts this tag compound into a valid SNBT string.
    /// See `NbtTag::`[`to_snbt`] for details.
    ///
    /// [`to_snbt`]: crate::tag::NbtTag::to_snbt
    #[cfg(feature = "configurable_depth")]
    pub fn to_snbt_with_limit(&self, opts: SnbtWriteOptions) -> String {
        format!("{:?}", CompoundWithOptions::new(&self, opts))
    }

    /// Converts this tag compound into a valid SNBT string with extra spacing for readability.
    /// See `NbtTag::`[`to_pretty_snbt`] for details.
    ///
    /// [`to_pretty_snbt`]: crate::tag::NbtTag::to_pretty_snbt
    #[cfg(feature = "configurable_depth")]
    pub fn to_pretty_snbt_with_options(&self, opts: SnbtWriteOptions) -> String {
        format!("{:#?}", CompoundWithOptions::new(&self, opts))
    }

    /// Returns the number of tags in this compound.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if the length of this compound is zero, false otherwise.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the value of the tag with the given name, or an error if no tag exists with the
    /// given name or specified type. This method should be used to obtain primitives as well as
    /// shared references to lists and compounds.
    #[inline]
    pub fn get<'a, 'b, K, T>(&'a self, name: &'b K) -> Result<T, NbtReprError>
    where
        String: Borrow<K>,
        K: Hash + Ord + Eq + ?Sized,
        &'b K: Into<String>,
        T: TryFrom<&'a NbtTag>,
        T::Error: Into<anyhow::Error>,
    {
        T::try_from(
            self.0
                .get(name)
                .ok_or_else(|| NbtStructureError::missing_tag(name))?,
        )
        .map_err(NbtReprError::from_any)
    }

    /// Returns the value of the tag with the given name, or an error if no tag exists with the
    /// given name or specified type. This method should be used to obtain mutable references to
    /// lists and compounds.
    #[inline]
    pub fn get_mut<'a, 'b, K, T>(&'a mut self, name: &'b K) -> Result<T, NbtReprError>
    where
        String: Borrow<K>,
        K: Hash + Ord + Eq + ?Sized,
        &'b K: Into<String>,
        T: TryFrom<&'a mut NbtTag>,
        T::Error: Into<anyhow::Error>,
    {
        T::try_from(
            self.0
                .get_mut(name)
                .ok_or_else(|| NbtStructureError::missing_tag(name))?,
        )
        .map_err(NbtReprError::from_any)
    }

    /// Returns whether or not this compound has a tag with the given name.
    #[inline]
    pub fn contains_key<K>(&self, key: &K) -> bool
    where
        String: Borrow<K>,
        K: Hash + Ord + Eq + ?Sized,
    {
        self.0.contains_key(key)
    }

    /// Returns a reference to the tag with the given name without any casting,
    /// or `None` if no tag exists with the given name.
    pub fn get_tag<K>(&self, key: &K) -> Option<&NbtTag>
    where
        String: Borrow<K>,
        K: Hash + Ord + Eq + ?Sized,
    {
        self.0.get(key)
    }

    /// Returns a mutable reference to the tag with the given name without any casting,
    /// or `None` if no tag exists with the given name.
    pub fn get_tag_mut<K>(&mut self, key: &K) -> Option<&mut NbtTag>
    where
        String: Borrow<K>,
        K: Hash + Ord + Eq + ?Sized,
    {
        self.0.get_mut(key)
    }

    #[cfg(feature = "preserve_order")]
    /// Removes and returns the tag with the given name without any casting,
    /// or `None` if no tag exists with the given name. If using the `preserve_order`
    /// feature, this method preserves the insertion order.
    pub fn remove_tag<K>(&mut self, key: &K) -> Option<NbtTag>
    where
        String: Borrow<K>,
        K: Hash + Ord + Eq + ?Sized,
    {
        self.0.shift_remove(key)
    }

    /// Removes and returns the tag with the given name without any casting,
    /// or `None` if no tag exists with the given name. This method does not preserve the order
    /// of the map (if tracked with the `preserve_order` feature).
    pub fn swap_remove_tag<K>(&mut self, key: &K) -> Option<NbtTag>
    where
        String: Borrow<K>,
        K: Hash + Ord + Eq + ?Sized,
    {
        #[cfg(feature = "preserve_order")]
        {
            self.0.swap_remove(key)
        }
        #[cfg(not(feature = "preserve_order"))]
        {
            self.0.remove(key)
        }
    }

    /// Adds the given value to this compound with the given name
    /// after wrapping that value in an `NbtTag`.
    #[inline]
    pub fn insert<K: Into<String>, T: Into<NbtTag>>(&mut self, name: K, value: T) {
        self.0.insert(name.into(), value.into());
    }

    /// Parses an NBT compound from SNBT
    #[inline]
    pub fn from_snbt(input: &str, opts: SnbtParseOptions) -> Result<Self, SnbtError> {
        snbt::parse_compound(input, opts)
    }

    #[inline]
    fn to_formatted_snbt(&self, f: &mut Formatter<'_>, opts: SnbtWriteOptions) -> fmt::Result {
        self.recursively_format_snbt(&mut String::new(), f, 0, opts)
    }

    #[allow(clippy::write_with_newline)]
    fn recursively_format_snbt(
        &self,
        indent: &mut String,
        f: &mut Formatter<'_>,
        current_depth: u32,
        opts: SnbtWriteOptions,
    ) -> fmt::Result {

        if self.is_empty() {
            return write!(f, "{{}}");
        }

        if f.alternate() {
            indent.push_str("    ");
            write!(f, "{{\n")?;
        } else {
            write!(f, "{{")?;
        }

        let last_index = self.len() - 1;
        for (index, (key, value)) in self.0.iter().enumerate() {
            let key = NbtTag::string_to_snbt(key, opts);

            if f.alternate() {
                write!(f, "{}{}: ", indent, key)?;
            } else {
                write!(f, "{}:", key)?;
            }

            // Conceptually, current_depth is the depth of this Compound itself;
            // its elements are one recursive tag deeper.
            value.recursively_format_snbt(indent, f, current_depth+1, opts)?;

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
            write!(f, "\n{}}}", indent)
        } else {
            write!(f, "}}")
        }
    }
}

impl Default for NbtCompound {
    #[inline]
    fn default() -> Self {
        NbtCompound::new()
    }
}

impl IntoIterator for NbtCompound {
    type IntoIter = <Map<NbtTag> as IntoIterator>::IntoIter;
    type Item = <Map<NbtTag> as IntoIterator>::Item;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a NbtCompound {
    type IntoIter = <&'a Map<NbtTag> as IntoIterator>::IntoIter;
    type Item = <&'a Map<NbtTag> as IntoIterator>::Item;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut NbtCompound {
    type IntoIter = <&'a mut Map<NbtTag> as IntoIterator>::IntoIter;
    type Item = (&'a String, &'a mut NbtTag);

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl FromIterator<(String, NbtTag)> for NbtCompound {
    #[inline]
    fn from_iter<T: IntoIterator<Item = (String, NbtTag)>>(iter: T) -> Self {
        NbtCompound(Map::from_iter(iter))
    }
}

impl<Q: ?Sized> Index<&Q> for NbtCompound
where
    String: Borrow<Q>,
    Q: Eq + Hash + Ord,
{
    type Output = NbtTag;

    #[inline]
    fn index(&self, key: &Q) -> &NbtTag {
        &self.0[key]
    }
}

impl Extend<(String, NbtTag)> for NbtCompound {
    #[inline]
    fn extend<T: IntoIterator<Item = (String, NbtTag)>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

macro_rules! display_and_debug {
    ($tag:ty, $name: ident) => {
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
            tag: &'a $tag,
            opts: SnbtWriteOptions
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

display_and_debug!(NbtTag, TagWithOptions);
display_and_debug!(NbtList, ListWithOptions);
display_and_debug!(NbtCompound, CompoundWithOptions);


#[cfg(feature = "serde")]
mod serde_impl {
    use super::*;
    use crate::serde::{Array, TypeHint};
    use serde::{
        de::{self, MapAccess, Visitor},
        Deserialize,
        Deserializer,
        Serialize,
        Serializer,
    };

    impl Serialize for NbtTag {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
            match self {
                &NbtTag::Byte(value)       => serializer.serialize_i8(value),
                &NbtTag::Short(value)      => serializer.serialize_i16(value),
                &NbtTag::Int(value)        => serializer.serialize_i32(value),
                &NbtTag::Long(value)       => serializer.serialize_i64(value),
                &NbtTag::Float(value)      => serializer.serialize_f32(value),
                &NbtTag::Double(value)     => serializer.serialize_f64(value),
                NbtTag::ByteArray(array)   => Array::from(array).serialize(serializer),
                NbtTag::String(value)      => serializer.serialize_str(value),
                NbtTag::List(list)         => list.serialize(serializer),
                NbtTag::Compound(compound) => compound.serialize(serializer),
                NbtTag::IntArray(array)    => Array::from(array).serialize(serializer),
                NbtTag::LongArray(array)   => Array::from(array).serialize(serializer),
            }
        }
    }

    impl<'de> Deserialize<'de> for NbtTag {
        #[inline]
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de> {
            deserializer.deserialize_any(NbtTagVisitor)
        }
    }

    struct NbtTagVisitor;

    impl<'de> Visitor<'de> for NbtTagVisitor {
        type Value = NbtTag;

        fn expecting(&self, f: &mut Formatter<'_>) -> fmt::Result {
            write!(f, "a valid NBT type")
        }

        #[inline]
        fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
        where E: de::Error {
            Ok(NbtTag::Byte(if v { 1 } else { 0 }))
        }

        #[inline]
        fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
        where E: de::Error {
            Ok(NbtTag::Byte(v))
        }

        #[inline]
        fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
        where E: de::Error {
            Ok(NbtTag::Byte(v as i8))
        }

        #[inline]
        fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
        where E: de::Error {
            Ok(NbtTag::Short(v))
        }

        #[inline]
        fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
        where E: de::Error {
            Ok(NbtTag::Int(v))
        }

        #[inline]
        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where E: de::Error {
            Ok(NbtTag::Long(v))
        }

        #[inline]
        fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
        where E: de::Error {
            Ok(NbtTag::Float(v))
        }

        #[inline]
        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where E: de::Error {
            Ok(NbtTag::Double(v))
        }

        #[inline]
        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where E: de::Error {
            self.visit_byte_buf(v.to_owned())
        }

        #[inline]
        fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
        where E: de::Error {
            Ok(NbtTag::ByteArray(raw::cast_byte_buf_to_signed(v)))
        }

        #[inline]
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where E: de::Error {
            Ok(NbtTag::String(v.to_owned()))
        }

        #[inline]
        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where E: de::Error {
            Ok(NbtTag::String(v))
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where A: MapAccess<'de> {
            let mut dest = match map.size_hint() {
                Some(hint) => {
                    #[cfg(not(feature = "comparable"))]
                    {
                        Map::with_capacity(hint)
                    }
                    #[cfg(feature = "comparable")]
                    {
                        let _ = hint;
                        Map::new()
                    }
                }
                None => Map::new(),
            };
            while let Some((key, tag)) = map.next_entry::<String, NbtTag>()? {
                dest.insert(key, tag);
            }
            Ok(NbtTag::Compound(NbtCompound(dest)))
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where A: de::SeqAccess<'de> {
            enum ArbitraryList {
                Byte(Vec<i8>),
                Int(Vec<i32>),
                Long(Vec<i64>),
                Tag(Vec<NbtTag>),
                Indeterminate,
            }

            impl ArbitraryList {
                fn into_tag(self) -> NbtTag {
                    match self {
                        ArbitraryList::Byte(list) => NbtTag::ByteArray(list),
                        ArbitraryList::Int(list) => NbtTag::IntArray(list),
                        ArbitraryList::Long(list) => NbtTag::LongArray(list),
                        ArbitraryList::Tag(list) => NbtTag::List(NbtList(list)),
                        ArbitraryList::Indeterminate => NbtTag::List(NbtList::new()),
                    }
                }
            }

            let mut list = ArbitraryList::Indeterminate;

            fn init_vec<T>(element: T, size: Option<usize>) -> Vec<T> {
                match size {
                    Some(size) => {
                        // Add one because the size hint returns the remaining amount
                        let mut vec = Vec::with_capacity(1 + size);
                        vec.push(element);
                        vec
                    }
                    None => vec![element],
                }
            }

            while let Some(tag) = seq.next_element::<NbtTag>()? {
                match (tag, &mut list) {
                    (NbtTag::Byte(value), ArbitraryList::Byte(list)) => list.push(value),
                    (NbtTag::Int(value), ArbitraryList::Int(list)) => list.push(value),
                    (NbtTag::Long(value), ArbitraryList::Long(list)) => list.push(value),
                    (tag, ArbitraryList::Tag(list)) => list.push(tag),
                    (tag, list @ ArbitraryList::Indeterminate) => {
                        let size = seq.size_hint();
                        match tag {
                            NbtTag::Byte(value) =>
                                *list = ArbitraryList::Byte(init_vec(value, size)),
                            NbtTag::Int(value) => *list = ArbitraryList::Int(init_vec(value, size)),
                            NbtTag::Long(value) =>
                                *list = ArbitraryList::Long(init_vec(value, size)),
                            tag => *list = ArbitraryList::Tag(init_vec(tag, size)),
                        }
                    }
                    _ =>
                        return Err(de::Error::custom(
                            "tag type mismatch when deserializing array",
                        )),
                }
            }

            match seq.next_element::<TypeHint>() {
                Ok(Some(TypeHint { hint: Some(tag_id) })) => match (list, tag_id) {
                    (ArbitraryList::Byte(list), 0x9) => Ok(NbtTag::List(NbtList(
                        list.into_iter().map(Into::into).collect(),
                    ))),
                    (ArbitraryList::Int(list), 0x9) => Ok(NbtTag::List(NbtList(
                        list.into_iter().map(Into::into).collect(),
                    ))),
                    (ArbitraryList::Long(list), 0x9) => Ok(NbtTag::List(NbtList(
                        list.into_iter().map(Into::into).collect(),
                    ))),
                    (ArbitraryList::Indeterminate, 0x7) => Ok(NbtTag::ByteArray(Vec::new())),
                    (ArbitraryList::Indeterminate, 0xB) => Ok(NbtTag::IntArray(Vec::new())),
                    (ArbitraryList::Indeterminate, 0xC) => Ok(NbtTag::LongArray(Vec::new())),
                    (list, _) => Ok(list.into_tag()),
                },
                _ => Ok(list.into_tag()),
            }
        }
    }

    impl Serialize for NbtList {
        #[inline]
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
            self.0.serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for NbtList {
        #[inline]
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de> {
            Ok(NbtList(Deserialize::deserialize(deserializer)?))
        }
    }

    impl Serialize for NbtCompound {
        #[inline]
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
            self.0.serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for NbtCompound {
        #[inline]
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de> {
            Ok(NbtCompound(Deserialize::deserialize(deserializer)?))
        }
    }
}
