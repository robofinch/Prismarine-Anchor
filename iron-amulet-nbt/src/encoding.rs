use flate2::Compression;


/// Encoding options for reading/writing NBT data from/to bytes (e.g. from/to a file).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncodingOptions {
    // Note for possible improvement / change:
    // It might end up better for performance to leave Endianness in the type system
    // instead of having it be an enum; however, that could monomorphize most or all of the looooong
    // serde impl and raw.rs functions into multiple copies

    /// Bedrock Edition is LittleEndian, Java is BigEndian
    pub endianness: Endianness,
    /// Compression usage varies in Minecraft
    pub compression: NBTCompression,
    /// Java is CESU-8, Bedrock is probably always UTF-8
    pub string_encoding: StringEncoding,
}

impl EncodingOptions {
    /// Default Java encoding for NBT
    pub fn java() -> Self {
        Self {
            endianness: Endianness::BigEndian,
            compression: NBTCompression::GzCompressed,
            string_encoding: StringEncoding::Cesu8,
        }
    }

    /// Default Bedrock encoding for NBT
    pub fn bedrock() -> Self {
        Self {
            endianness: Endianness::LittleEndian,
            compression: NBTCompression::GzCompressed,
            string_encoding: StringEncoding::Utf8,
        }
    }
}

/// Endianness of stored NBT
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
/// Describes the compression options for NBT data: uncompressed, Zlib compressed and Gz compressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NBTCompression {
    /// Uncompressed NBT data.
    Uncompressed,
    /// Zlib compressed NBT data. When writing, the default compression level will be used.
    ZlibCompressed,
    /// Zlib compressed NBT data with the given compression level.
    ZlibCompressedWith(CompressionLevel),
    /// Gz compressed NBT data. When writing, the default compression level will be used.
    GzCompressed,
    /// Gz compressed NBT data with the given compression level.
    GzCompressedWith(CompressionLevel),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        Compression::new(value.0 as u32)
    }
}

/// String encodings used by Minecraft. Java is CESU-8, Bedrock is probably always UTF-8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringEncoding {
    /// Used by Bedrock
    Utf8,
    /// Used by Java
    Cesu8
}
