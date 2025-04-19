use flate2::Compression;
#[cfg(feature = "derive_serde")]
use serde::{Serialize, Deserialize};


// ================================
//      Limits
// ================================

/// The recursive NBT tags (Compounds and Lists) can be nested up to (and including)
/// 512 levels deep in the standard specification.
/// The limit may be increased here if the `configurable_depth` feature is enabled,
/// but note that this crate uses recursive functions to read and write NBT data;
/// if the limit is too high and unreasonably nested data is received,
/// a crash could occur from the nested function calls exceeding the maximum stack size.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct DepthLimit(pub(crate) u32);

impl Default for DepthLimit {
    /// The maximum depth that NBT compounds and tags can be nested in the standard Minecraft specification.
    fn default() -> Self {
        Self(512)
    }
}

impl DepthLimit {
    pub fn limit(self) -> u32 {
        self.0
    }
}

#[cfg(feature = "configurable_depth")]
impl DepthLimit {
    /// A limit on how deeply the recursive NBT tags (Compounds and Lists) may be nested.
    /// Note that this crate uses recursive functions to read and write NBT data;
    /// if the limit is too high and unreasonably nested data is received,
    /// a crash could occur from the nested function calls exceeding the maximum stack size.
    pub fn new(limit: u32) -> Self {
        Self(limit)
    }
}

// This section could be expanded to add one or two more limits on parsing and writing data,
// in particular in regards to length (of files/bytes/strings, multicharacter tokens).
// It wouldn't be too hard to give functions here data that results in too much memory allocation
// and ultimately a crash.
// Is that truly important to fix? Compared to functional tasks, no,
// and it would be time-consuming to implement, so it's left as an idea.
// And too much heap memory usage isn't even likely to be *too* much of a problem
// thanks to swap space, and catch_unwind is there if needed (if panic isn't set to abort).
// Stack memory usage is worse, we can limit that with DepthLimit.
// Might need to place a TotalLengthLimit on parsing files, though, or place a limit
// on how many items can be in a compound tag or list tag and have an option to ignore ones
// that are too long, in case a user is trying to handle some really bad NBT data,
// trying to delete parts of it. Likewise, perhaps this library could use features to try to
// recover corrupt NBT byte data as much as possible...
// but until that becomes a problem someone has, it doesn't seem worth tackling.


// ================================
//      IO Settings
// ================================

// Note for possible improvement / change:
// It might end up better for performance to leave Endianness in the type system
// instead of having it be an enum; however, that could monomorphize most or all of the looooong
// serde impl and raw.rs functions into multiple copies. Will take the easier option
// until benchmarks are set up.

/// Encoding options for reading/writing NBT data from/to bytes (e.g. from/to a file).
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct IoOptions {
    /// [Endianness] of some NBT data, primarily numeric data. Moreover, in the `NetworkLitleEndian`
    /// variant, `i32` and `i64` values are written or read with a variable-length varint encoding.
    ///
    /// Bedrock Edition is LittleEndian, Java is BigEndian
    ///
    /// [Endianness]: https://en.wikipedia.org/wiki/Endianness
    pub endianness: Endianness,
    /// Compression of NBT bytes.
    ///
    /// Default: Gzip compression with the default compression level
    /// ([`NBTCompression::GzCompressed`]).
    pub compression: NbtCompression,
    /// The byte encoding used by strings. Note that the NBT tags in this crate always
    /// use Rust's encoding, UTF-8.
    ///
    /// Default: CESU-8 for Java, UTF-8 for Bedrock
    pub string_encoding: StringEncoding,
    /// Whether invalid strings should be read as `ByteString`s instead of causing errors to be
    /// thrown.
    ///
    /// Default: false.
    pub allow_invalid_strings: bool,
    /// The maximum depth that NBT compounds and tags can be recursively nested.
    ///
    /// Default: 512, the limit used by Minecraft.
    pub depth_limit: DepthLimit,
}

impl IoOptions {
    /// Default Java encoding for NBT bytes
    #[inline]
    pub fn java() -> Self {
        Self {
            endianness:             Endianness::BigEndian,
            compression:            NbtCompression::GzipCompressed,
            string_encoding:        StringEncoding::Cesu8,
            allow_invalid_strings:  false,
            depth_limit:            DepthLimit::default(),
        }
    }

    /// Default Java encoding for NBT bytes, but with no compression
    #[inline]
    pub fn java_uncompressed() -> Self {
        Self {
            compression: NbtCompression::Uncompressed,
            ..Self::java()
        }
    }

    /// Default Bedrock encoding for NBT bytes
    #[inline]
    pub fn bedrock() -> Self {
        Self {
            endianness:             Endianness::LittleEndian,
            compression:            NbtCompression::GzipCompressed,
            string_encoding:        StringEncoding::Utf8,
            allow_invalid_strings:  false,
            depth_limit:            DepthLimit::default(),
        }
    }

    /// Default Bedrock encoding for NBT bytes, but with no compression
    #[inline]
    pub fn bedrock_uncompressed() -> Self {
        Self {
            compression: NbtCompression::Uncompressed,
            ..Self::bedrock()
        }
    }

    /// Bedrock encoding for NBT bytes with `NetworkEndian` endianness
    /// and no compression.
    #[inline]
    pub fn bedrock_network_uncompressed() -> Self {
        Self {
            endianness:             Endianness::NetworkLittleEndian,
            compression:            NbtCompression::Uncompressed,
            string_encoding:        StringEncoding::Utf8,
            allow_invalid_strings:  false,
            depth_limit:            DepthLimit::default(),
        }
    }
}

/// [Endianness] of NBT bytes.
///
/// [Endianness]: https://en.wikipedia.org/wiki/Endianness
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub enum Endianness {
    /// Used by Java
    BigEndian,
    /// Used by Bedrock for numeric information
    LittleEndian,
    /// Used by Bedrock to serialize NBT over a network with variable-length encodings
    /// of 32- and 64-bit integers.
    /// See https://wiki.bedrock.dev/nbt/nbt-in-depth#network-little-endian
    /// for more information.
    NetworkLittleEndian,
}

// Note that there's also an option to include/exclude the Zlib header, which should not matter
// for NBT as far as I'm aware, but does matter for Bedrock's LevelDB.
/// Describes the compression options for NBT data:
/// uncompressed, Zlib-compressed and Gzip-compressed.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub enum NbtCompression {
    /// Uncompressed NBT data.
    Uncompressed,
    /// Zlib-compressed NBT data. When writing, the default compression level will be used.
    ZlibCompressed,
    /// Zlib-compressed NBT data with the given compression level.
    ZlibCompressedWith(CompressionLevel),
    /// Gzip-compressed NBT data. When writing, the default compression level will be used.
    GzipCompressed,
    /// Gzip-compressed NBT data with the given compression level.
    GzipCompressedWith(CompressionLevel),
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct CompressionLevel(u8);

impl From<Compression> for CompressionLevel {
    fn from(value: Compression) -> Self {
        // Only values 0-9 should actually be used, and miniz-oxide uses 10 at most.
        // 0-255 is more than enough.
        Self(value.level() as u8)
    }
}

impl From<CompressionLevel> for Compression {
    fn from(value: CompressionLevel) -> Self {
        Compression::new(u32::from(value.0))
    }
}

/// String encodings used by Minecraft. Java is CESU-8, Bedrock is probably always UTF-8.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub enum StringEncoding {
    /// Used by Bedrock
    Utf8,
    /// Used by Java
    Cesu8,
}


// ================================
//      SNBT Options
// ================================

/// Determines which version of the SNBT specification should be used to convert between
/// NBT and SNBT. The updated version is used in Java Edition at or above 1.21.5.
/// The original version is used by Java before 1.21.5, as well as by
/// other versions of Minecraft.
///
/// By default, the version has no impact on converting NBT to SNBT;
/// the output will be compatible with both SNBT versions. Enabling the newer SNBT version
/// expands the parsing features for converting SNBT to NBT, and makes a few strings invalid
/// which were previously valid SNBT.
/// Finer control is possible through fields of [`SnbtParseOptions`] and [`SnbtWriteOptions`],
/// and in particular, outputting character escape sequences only valid in `UpdatedJava` SNBT,
/// such as `\n` or `\t`, can be enabled.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub enum SnbtVersion {
    /// For Java 1.21.5 and later. Adds additional parsing features, and is mostly
    /// backwards-compatible, but the restrictions on numeric values and unquoted strings
    /// are slightly stricter.
    /// Unquoted strings are prohibited from starting with `+`, `-`, `.`, or a digit,
    /// and can otherwise have any characters in `[0-9a-zA-Z]` or `_`, `-`, `.`, `+`.
    /// Only finite float values (not an infinity or NaN) are allowed.
    /// Leading `0`'s are prohibited for integers.
    /// Most other parsing rules are looser than in the original version; only these
    /// stricter exceptions are mentioned as care must be taken with them.
    /// See [minecraft.wiki] for full details.
    ///
    /// Note that by default, this crate's SNBT parsers and writers won't halt with an error
    /// when encountering a non-finite number; they are instead replaced by finite values;
    /// see [`replace_non_finite`] and [`WriteNonFinite`]. Other settings conform
    /// to Minecraft's behavior by default, except for whitespace and sequential lengths.
    /// Here, whitespace between tokens is trimmed, while Minecraft might throw an error on,
    /// say, an unexpected newline in SNBT. Additionally, string and list tags should have lengths
    /// which fit in an i16 or i32, depending on the exact case. No limits are placed on SNBT
    /// string or list lengths here, beyond hardware limitations.
    ///
    /// [minecraft.wiki]: https://minecraft.wiki/w/Java_Edition_1.21.5#:~:text=SNBT%20format
    /// [`replace_non_finite`]: SnbtParseOptions::replace_non_finite
    UpdatedJava,
    /// For Java before 1.21.5, or Bedrock Edition.
    /// Fewer parsing features than the newer SNBT version. The specification for
    /// unquoted strings is also slightly more lenient than with the newer version; for this
    /// implementation, unquoted strings are prohibited from starting with `-` or a digit,
    /// and can otherwise have any characters in `[0-9a-zA-Z]` or `_`, `-`, `.`, `+`. If you
    /// manage to make a SNBT floating point literal larger than `f32::MAX` or `f64::MAX`, it
    /// wouldn't be rejected as invalid and would result in positive infinity.
    /// (You should avoid making infinite or NaN values or SNBT or NBT, as Minecraft does not
    /// like them.)
    ///
    /// Converting NBT to SNBT with this version will still quote strings with the updated version
    /// in mind, for the sake of greater compatibility.
    ///
    /// Note that by default, this crate's SNBT parsers and writers won't halt with an error
    /// when encountering a non-finite number; they are instead replaced by finite values;
    /// see [`replace_non_finite`] and [`WriteNonFinite`]. Other settings conform
    /// to Minecraft's behavior by default, except for whitespace and sequential lengths.
    /// Here, whitespace between tokens is trimmed, while Minecraft might throw an error on,
    /// say, an unexpected newline in SNBT. Additionally, string and list tags should have lengths
    /// which fit in an i16 or i32, depending on the exact case. No limits are placed on SNBT
    /// string or list lengths here, beyond hardware limitations.
    Original,
}

/// Options for parsing SNBT data into NBT data. See the [`SnbtVersion`] enum and its variants
/// for information about the two versions of SNBT.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct SnbtParseOptions {
    /// Version of the SNBT format used. Has many effects on how SNBT is parsed. Note that
    /// the [`UpdatedJava`] version normally halts with an error if an infinite float or double
    /// is encountered; the below `replace_non_finite` option takes precedence.
    ///
    /// There is no default for this setting; choose which one you need.
    ///
    /// [`UpdatedJava`]: SnbtVersion::UpdatedJava
    pub version: SnbtVersion,
    /// The maximum depth that NBT compounds and tags can be recursively nested.
    ///
    /// Default: 512, the limit used by Minecraft.
    pub depth_limit: DepthLimit,
    /// How the unquoted symbols `true` and `false` should be parsed.
    ///
    /// Default: `AsDetected`
    pub true_false: ParseTrueFalse,
    /// How unquoted symbols like `Infinityf` which likely came from infinite floats
    /// should be parsed.
    ///
    /// Default: `AsDetected`
    pub non_finite: ParseNonFinite,
    /// Whether infinite floating-point numbers should be replaced with finite values
    /// (`MAX` or `MIN` for infinities, `0.` for NaN). Takes precedence over halting with an
    /// error on an infinite float or double literal (e.g. `1e1000`) in the [`UpdatedJava`]
    /// version. Note that parsing an unquoted symbol like `Infinityd` is controlled by
    /// [`ParseNonFinite`] before it could become a nonfinite floating-point number handled
    /// by this setting; selecting `true` for this setting and `Error` for [`ParseNonFinite`]
    /// may result in parsing `Infinityd` to error but `1e1000` to succeed.
    ///
    /// Default: `true`
    ///
    /// [`UpdatedJava`]: SnbtVersion::UpdatedJava
    pub replace_non_finite: bool,
    /// The escape sequences which will be parsed in SNBT quoted strings, putting the escaped
    /// characters in the resulting NBT String tag. For example: the `\n` escape may be replaced
    /// with an actual newline character in the NBT tag.
    /// Note that the escapes `\\`, `\'`, and `\"` are always allowed.
    ///
    /// Default: all escapes on `UpdatedJava`, no escapes on `Original`.
    pub enabled_escape_sequences: EnabledEscapeSequences,
    /// How to handle an escape sequence not in the list of enabled escape sequences
    ///
    /// Default: `Error`
    pub handle_invalid_escape: HandleInvalidEscape,
}

impl SnbtParseOptions {
    /// The default settings for the `UpdatedJava` version
    #[inline]
    pub fn default_updated() -> Self {
        Self {
            version:                    SnbtVersion::UpdatedJava,
            depth_limit:                DepthLimit::default(),
            true_false:                 ParseTrueFalse::AsDetected,
            non_finite:                 ParseNonFinite::AsDetected,
            replace_non_finite:         true,
            enabled_escape_sequences:   EnabledEscapeSequences::all_escapes(),
            handle_invalid_escape:      HandleInvalidEscape::Error,
        }
    }

    /// The default settings for the `Original` version
    #[inline]
    pub fn default_original() -> Self {
        Self {
            version:                    SnbtVersion::Original,
            depth_limit:                DepthLimit::default(),
            true_false:                 ParseTrueFalse::AsDetected,
            non_finite:                 ParseNonFinite::AsDetected,
            replace_non_finite:         true,
            enabled_escape_sequences:   EnabledEscapeSequences::no_escapes(),
            handle_invalid_escape:      HandleInvalidEscape::Error,
        }
    }
}

/// SNBT allows the unquoted symbols `true` and `false` to be used instead of `1b` and `0b`.
/// This enum indicates whether they should always be parsed as bytes, always parsed
/// as unquoted strings, or parsed as bytes unless they are an `NbtCompound` key or
/// an element of an `NbtList` of String tags.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub enum ParseTrueFalse {
    AsByte,
    AsDetected,
    AsString,
}

/// NBT and SNBT aren't meant to support infinite and NaN float or double values, but they
/// could be encountered anyway. When an infinite NBT float is converted to SNBT and back, a normal
/// parser would end up treating the result as a string. With Minecraft Java's default parser,
/// as per [MC-200070], a positive infinite double is printed as `Infinityd`,
/// and an NaN float is printed as `NaNf`, for example. This enum indicates whether such a value
/// (in an unquoted literal) should be parsed as always a (non-finite) number, always a string,
/// or as a number unless it is an `NbtCompound` key or an element of an `NbtList` of String tags.
/// Alternatively, encountering such a value can return an error that halts further parsing
/// Note that [`SnbtParseOptions`] provides the option to replace infinities with the
/// `MAX` or `MIN` constant of `f32` or `f64` and replace NaN values with `0.0` when reading
/// into NBT; combined, the settings can, for instance, parse `Infinityd` as `f64::MAX`.
///
/// [MC-200070]: https://report.bugs.mojang.com/servicedesk/customer/portal/2/MC-200070
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub enum ParseNonFinite {
    AsDetected,
    AsFloat,
    AsString,
    Error,
}

/// How to handle an invalid or disabled escape sequence in a quoted string when parsing SNBT data.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub enum HandleInvalidEscape {
    /// Copy the escape sequence verbatim into the final string
    CopyVerbatim,
    /// Ignore the escape sequence, act as though it's not there
    Ignore,
    /// Halt parsing and return with an error
    Error,
}

/// Options for writing NBT data to SNBT. See the [`SnbtVersion`] enum and its variants
/// for information about the two versions of SNBT.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct SnbtWriteOptions {
    /// Version of the SNBT format used. Currently has no effect on writing NBT to SNBT.
    // TODO: add warning and error logging throughout the crate, such as
    // warnings if escape sequences are used in the `Original` version.
    pub version: SnbtVersion,
    /// The maximum depth that NBT compounds and tags can be recursively nested.
    ///
    /// Default: 512, the limit used by Minecraft.
    pub depth_limit: DepthLimit,
    /// How to print an infinite or NaN float/double tag, or if writing should
    /// halt with an error (if possible).
    ///
    /// Default: `PrintFloats` ()
    pub non_finite: WriteNonFinite,
    /// Which escape sequences will be used when writing an NBT string tag into a quoted
    /// SNBT string. Note that escapes are not used eagerly when a simpler option is available;
    /// otherwise, enabling unicode escapes would fill the entire string with them. Moreover,
    /// the escapes `\\`, `\'`, and `\"` are always allowed.
    ///
    /// Any graphic ASCII character, as determined by [`is_ascii_graphic`], will never be escaped.
    /// Newlines, carriage returns, spaces, and horizontal tabs will only be escaped if
    /// `\n`, `\r`, `\s`, or `\t` are enabled, respectively.
    ///
    /// Any other character will be escaped if possible using an enabled escape,
    /// using the shortest option (single-character escape, or 2- / 4- /
    /// 8-character unicode escape.) Named escapes are never used for output.
    ///
    /// [`is_ascii_graphic`]: char::is_ascii_graphic
    ///
    /// Default: No escapes in `Original`,
    /// and all escapes **except `\n`, `\r`, and `\s`** in `UpdatedJava`.
    /// Users probably expect their normal whitespace usage to be respected. Plausibly,
    /// some tool with stringent whitespace requirements might require those characters to
    /// be escaped, too, in which case you need to be aware of this exception.
    pub enabled_escape_sequences: EnabledEscapeSequences,
}

impl SnbtWriteOptions {
    /// The default settings for the `UpdatedJava` version
    #[inline]
    pub fn default_updated() -> Self {
        Self {
            version:                    SnbtVersion::UpdatedJava,
            depth_limit:                DepthLimit::default(),
            non_finite:                 WriteNonFinite::PrintFloats,
            enabled_escape_sequences:   EnabledEscapeSequences::from_fn(|e| match e {
                EscapeSequence::N => false,
                EscapeSequence::R => false,
                EscapeSequence::S => false,
                _ => true
            })
        }
    }

    /// The default settings for the `Original` version
    #[inline]
    pub fn default_original() -> Self {
        Self {
            version:                    SnbtVersion::UpdatedJava,
            depth_limit:                DepthLimit::default(),
            non_finite:                 WriteNonFinite::PrintFloats,
            enabled_escape_sequences:   EnabledEscapeSequences::no_escapes(),
        }
    }
}

/// NBT and SNBT aren't meant to support infinite and NaN float or double values, but they
/// could be encountered anyway. When an infinite NBT float is converted to SNBT and back, a normal
/// parser would end up treating the result as a string. With Minecraft Java's default parser,
/// as per [MC-200070], a positive infinite double is printed as `Infinityd`,
/// and an NaN float is printed as `NaNf`, for example. This enum indicates whether such a value
/// in an NBT tag should be displayed in that form, or display positive infinity as though it were
/// the `MAX` constant of `f32` or `f64`, negative infinity as the `MIN` constant, and NaN as `0.`.
///
/// [MC-200070]: https://report.bugs.mojang.com/servicedesk/customer/portal/2/MC-200070
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub enum WriteNonFinite {
    /// Display positive infinity as though it were the `MAX` constant of `f32` or `f64`,
    /// negative infinity as the `MIN` constant, and NaN as `0.`.
    PrintFloats,
    /// Display positive infinity as `Infinityd` or `Infinityf`, negative infinity
    /// as `-Infinityd` or `-Infinityf`, and an NaN value as `NaNf` or  `NaNd`.
    /// Note that `-Infinityd` and `-Infinityf` are not valid unquoted strings or valid floats
    /// in the `UpdatedJava` version, though they are valid unquoted strings
    /// in the `Original` version.
    PrintStrings,
}

/// Escape sequences which are enabled when reading or writing quoted SNBT strings.
/// Note that the escapes `\\`, `\'`, and `\"` are always allowed; these settings do not
/// control those escapes.
///
/// If the `named_escapes` feature is not enabled, the option for enabling
/// unicode escapes will be ignored.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct EnabledEscapeSequences(u16);

impl EnabledEscapeSequences {
    /// Enables the escape sequences for which the provided function returns `true`.
    #[inline]
    pub fn from_fn(f: impl Fn(EscapeSequence) -> bool) -> Self {
        use EscapeSequence as E;

        let mut bits = 0;

        for escape in [
            E::B, E::F, E::N, E::R, E::S, E::T,
            E::UnicodeTwo, E::UnicodeFour, E::UnicodeEight, E::UnicodeNamed
        ] {
            if f(escape) {
                bits |= 1 << (escape as u8)
            }
        }

        Self(bits)
    }

    /// Enables all escape sequences.
    #[inline]
    pub fn all_escapes() -> Self {
        Self(u16::MAX)
    }

    /// Disables all escape sequences.
    #[inline]
    pub fn no_escapes() -> Self {
        Self(0)
    }

    /// Enables `\n` (newline), `\r` (carriage return), and `\t` (horizontal tab).
    #[inline]
    pub fn standard_whitespace_escapes() -> Self {
        Self::from_fn(|escape| match escape {
            EscapeSequence::N => true,
            EscapeSequence::R => true,
            EscapeSequence::T => true,
            _ => false,
        })
    }

    /// Enables `\b` (backspace), `\f` (form feed), `\n` (newline),
    /// `\r` (carriage return), `\s` (space), and `\t` (horizontal tab).
    #[inline]
    pub fn one_character_escapes() -> Self {
        Self::from_fn(|escape| match escape {
            EscapeSequence::B => true,
            EscapeSequence::F => true,
            EscapeSequence::N => true,
            EscapeSequence::R => true,
            EscapeSequence::S => true,
            EscapeSequence::T => true,
            _ => false,
        })
    }

    /// Enables unicode escapes: `\x`, `\u`, and `\U` for two-, four-, or eight-character
    /// escapes, respectively, and `\N{----}` for named unicode escapes.
    /// Note that the named escape setting is ignored if the `named_escapes` feature
    /// is not enabled.
    #[inline]
    pub fn unicode_escapes() -> Self {
        Self::from_fn(|escape| match escape {
            EscapeSequence::UnicodeTwo => true,
            EscapeSequence::UnicodeFour => true,
            EscapeSequence::UnicodeEight => true,
            EscapeSequence::UnicodeNamed => true,
            _ => false,
        })
    }

    /// Whether the provided escape sequence is enabled
    #[inline]
    pub fn is_enabled(self, escape: EscapeSequence) -> bool {
        0 != self.0 & (1 << (escape as u8))
    }
}

/// The various escape sequences allowed in SNBT
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum EscapeSequence {
    /// `\b`, backspace
    B = 0,
    /// `\f`, form feed
    F = 1,
    /// `\n`, newline
    N = 2,
    /// `\r`, carriage return
    R = 3,
    /// `\s`, space
    S = 4,
    /// `\t`, horizontal tab
    T = 5,
    /// `\x--`, two-character unicode escapes
    UnicodeTwo = 6,
    /// `\u----`, four-character unicode escapes
    UnicodeFour = 7,
    /// `\U--------`, eight-character unicode escapes
    UnicodeEight = 8,
    /// `\N{----}`, named unicode escapes. (Note that `----` is a placeholder for a name
    /// of any length.)
    ///
    /// If the `named_escapes` feature is not enabled, this option will be ignored.
    UnicodeNamed = 9,
}
