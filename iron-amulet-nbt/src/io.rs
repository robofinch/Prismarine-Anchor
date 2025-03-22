use std::{fmt, io};
use std::error::Error;
use std::{
    fmt::{Display, Formatter},
    io::{Read, Write},
};

use flate2::Compression;
use flate2::{
    read::{GzDecoder, ZlibDecoder},
    write::{GzEncoder, ZlibEncoder},
};

use crate::raw;
use crate::{
    settings::{DepthLimit, IoOptions, NBTCompression},
    tag::{NbtCompound, NbtList, NbtTag},
};


// todo: implement allow_*_root features

/// Reads the given encoding of NBT data from the given reader, returning the resulting NBT
/// compound and associated root name.
pub fn read_nbt<R: Read>(
    reader: &mut R,
    opts: IoOptions
) -> Result<(NbtCompound, String), NbtIoError> {

    match opts.compression {
        NBTCompression::Uncompressed => read_nbt_uncompressed(reader, opts),
        NBTCompression::ZlibCompressed | NBTCompression::ZlibCompressedWith(_) =>
            read_nbt_uncompressed(&mut ZlibDecoder::new(reader), opts),
        NBTCompression::GzipCompressed | NBTCompression::GzipCompressedWith(_) =>
            read_nbt_uncompressed(&mut GzDecoder::new(reader), opts),
    }
}

fn read_nbt_uncompressed<R: Read>(
    reader: &mut R, opts: IoOptions
) -> Result<(NbtCompound, String), NbtIoError> {

    let root_id = raw::read_u8(reader, opts)?;
    if root_id != 0xA {
        return Err(NbtIoError::TagTypeMismatch {
            expected: 0xA,
            found: root_id,
        });
    }

    let root_name = raw::read_string(reader, opts)?;
    match read_tag_body_const::<_, 0xA>(reader, opts, 0) {
        Ok(NbtTag::Compound(compound)) => Ok((compound, root_name)),
        Err(e) => Err(e),
        _ => unreachable!(),
    }
}

fn read_tag_body_dyn<R: Read>(
    reader: &mut R, opts: IoOptions, tag_id: u8, current_depth: u32
) -> Result<NbtTag, NbtIoError> {

    macro_rules! drive_reader {
        ($($id:literal)*) => {
            match tag_id {
                $( $id => read_tag_body_const::<_, $id>(reader, opts, current_depth), )*
                _ => Err(NbtIoError::InvalidTagId(tag_id))
            }
        };
    }

    drive_reader!(0x1 0x2 0x3 0x4 0x5 0x6 0x7 0x8 0x9 0xA 0xB 0xC)
}

#[inline]
fn read_tag_body_const<R: Read, const TAG_ID: u8>(
    reader: &mut R, opts: IoOptions, current_depth: u32
) -> Result<NbtTag, NbtIoError> {

    let tag = match TAG_ID {
        0x1 => NbtTag::Byte  ( raw::read_i8 ( reader, opts )?),
        0x2 => NbtTag::Short ( raw::read_i16( reader, opts )?),
        0x3 => NbtTag::Int   ( raw::read_i32( reader, opts )?),
        0x4 => NbtTag::Long  ( raw::read_i64( reader, opts )?),
        0x5 => NbtTag::Float ( raw::read_f32( reader, opts )?),
        0x6 => NbtTag::Double( raw::read_f64( reader, opts )?),
        0x7 => {
            let len = raw::read_i32_as_usize(reader, opts)?;
            let mut array = vec![0u8; len];

            reader.read_exact(&mut array)?;

            NbtTag::ByteArray(raw::cast_byte_buf_to_signed(array))
        }
        0x8 => NbtTag::String(raw::read_string(reader, opts)?),
        0x9 => {
            let tag_id = raw::read_u8(reader, opts)?;
            let len = raw::read_i32_as_usize(reader, opts)?;

            // Make sure we don't have an invalid type or a nonempty list of TAG_End
            if tag_id > 0xC || (tag_id == 0 && len > 0) {
                return Err(NbtIoError::InvalidTagId(tag_id));
            }

            if len == 0 {
                return Ok(NbtTag::List(NbtList::new()));
            }

            if current_depth >= opts.depth_limit.0 {
                return Err(NbtIoError::ExceededDepthLimit {
                    limit: opts.depth_limit
                });
            }

            let mut list = NbtList::with_capacity(len);

            macro_rules! drive_reader {
                ($($id:literal)*) => {
                    match tag_id {
                        $(
                            $id => {
                                for _ in 0 .. len {
                                    list.push(read_tag_body_const::<_, $id>(
                                        reader, opts, current_depth + 1
                                    )?);
                                }
                            },
                        )*
                        _ => return Err(NbtIoError::InvalidTagId(tag_id))
                    }
                };
            }

            drive_reader!(0x1 0x2 0x3 0x4 0x5 0x6 0x7 0x8 0x9 0xA 0xB 0xC);

            NbtTag::List(list)
        }
        0xA => {
            let mut compound = NbtCompound::new();
            let mut tag_id = raw::read_u8(reader, opts)?;

            if tag_id != 0x0 && current_depth >= opts.depth_limit.0 {
                return Err(NbtIoError::ExceededDepthLimit {
                    limit: opts.depth_limit
                });
            }

            // Read until TAG_End
            while tag_id != 0x0 {
                let name = raw::read_string(reader, opts)?;
                let tag = read_tag_body_dyn(reader, opts, tag_id, current_depth + 1)?;
                compound.insert(name, tag);
                tag_id = raw::read_u8(reader, opts)?;
            }

            NbtTag::Compound(compound)
        }
        0xB => {
            let len = raw::read_i32_as_usize(reader, opts)?;
            NbtTag::IntArray(raw::read_i32_array(reader, opts, len)?)
        }
        0xC => {
            let len = raw::read_i32_as_usize(reader, opts)?;
            NbtTag::LongArray(raw::read_i64_array(reader, opts, len)?)
        }
        _ => unreachable!("read_tag_body_const called with unchecked TAG_ID"),
    };

    Ok(tag)
}

/// Writes the given encoding of NBT data to the given writer.
/// If no root name is provided, the empty string is used.
pub fn write_nbt<W: Write>(
    writer: &mut W,
    opts: IoOptions,
    root_name: Option<&str>,
    root: &NbtCompound,
) -> Result<(), NbtIoError> {

    let (mode, compression) = match opts.compression {
        NBTCompression::Uncompressed => {
            return write_nbt_uncompressed(writer, opts, root_name, root);
        }
        NBTCompression::ZlibCompressed => (2, Compression::default()),
        NBTCompression::ZlibCompressedWith(compression) => (2, compression.into()),
        NBTCompression::GzipCompressed => (1, Compression::default()),
        NBTCompression::GzipCompressedWith(compression) => (1, compression.into()),
    };

    if mode == 1 {
        write_nbt_uncompressed(&mut GzEncoder::new(writer, compression), opts, root_name, root)
    } else {
        write_nbt_uncompressed(&mut ZlibEncoder::new(writer, compression), opts, root_name, root)
    }
}

/// Writes the given tag compound with the given name to the provided writer, writing only the raw
/// NBT data without any compression.
fn write_nbt_uncompressed<W>(
    writer: &mut W,
    opts: IoOptions,
    root_name: Option<&str>,
    root: &NbtCompound,
) -> Result<(), NbtIoError>
where
    W: Write,
{
    // Compound ID
    raw::write_u8(writer, opts, 0xA)?;
    raw::write_string(writer, opts, root_name.unwrap_or(""))?;

    if opts.depth_limit.0 == 0 && !root.inner().is_empty() {
        return Err(NbtIoError::ExceededDepthLimit {
            limit: opts.depth_limit
        });
    }

    for (name, tag) in root.inner() {
        raw::write_u8(writer, opts, raw::id_for_tag(Some(tag)))?;
        raw::write_string(writer, opts, name)?;
        write_tag_body(writer, opts, tag, 1)?;
    }
    raw::write_u8(writer, opts, raw::id_for_tag(None))?;
    Ok(())
}

fn write_tag_body<W: Write>(
    writer: &mut W,
    opts: IoOptions,
    tag: &NbtTag,
    current_depth: u32,
) -> Result<(), NbtIoError> {
    match tag {
        &NbtTag::Byte  (value) => raw::write_i8 (writer, opts, value)?,
        &NbtTag::Short (value) => raw::write_i16(writer, opts, value)?,
        &NbtTag::Int   (value) => raw::write_i32(writer, opts, value)?,
        &NbtTag::Long  (value) => raw::write_i64(writer, opts, value)?,
        &NbtTag::Float (value) => raw::write_f32(writer, opts, value)?,
        &NbtTag::Double(value) => raw::write_f64(writer, opts, value)?,
        NbtTag::ByteArray(value) => {
            raw::write_usize_as_i32(writer, opts, value.len())?;
            writer.write_all(raw::cast_bytes_to_unsigned(value.as_slice()))?;
        }
        NbtTag::String(value) => raw::write_string(writer, opts, value)?,
        NbtTag::List(value) =>
            if value.is_empty() {
                writer.write_all(&[raw::id_for_tag(None), 0, 0, 0, 0])?;
            } else {
                let list_type = raw::id_for_tag(Some(&value[0]));
                raw::write_u8(writer, opts, list_type)?;
                raw::write_usize_as_i32(writer, opts, value.len())?;

                if current_depth >= opts.depth_limit.0 && !value.is_empty() {
                    return Err(NbtIoError::ExceededDepthLimit {
                        limit: opts.depth_limit
                    });
                }

                for sub_tag in value.as_ref() {
                    let tag_id = raw::id_for_tag(Some(sub_tag));
                    if tag_id != list_type {
                        return Err(NbtIoError::NonHomogenousList {
                            list_type,
                            encountered_type: tag_id,
                        });
                    }

                    write_tag_body(writer, opts, sub_tag, current_depth + 1)?;
                }
            },
        NbtTag::Compound(value) => {
            if current_depth >= opts.depth_limit.0 && !value.is_empty() {
                return Err(NbtIoError::ExceededDepthLimit {
                    limit: opts.depth_limit
                });
            }

            for (name, tag) in value.inner() {
                raw::write_u8(writer, opts, raw::id_for_tag(Some(tag)))?;
                raw::write_string(writer, opts, name)?;
                write_tag_body(writer, opts, tag, current_depth + 1)?;
            }

            // TAG_End
            raw::write_u8(writer, opts, raw::id_for_tag(None))?;
        }
        NbtTag::IntArray(value) => {
            raw::write_usize_as_i32(writer, opts, value.len())?;

            for &int in value.iter() {
                raw::write_i32(writer, opts, int)?;
            }
        }
        NbtTag::LongArray(value) => {
            raw::write_usize_as_i32(writer, opts, value.len())?;

            for &long in value.iter() {
                raw::write_i64(writer, opts, long)?;
            }
        }
    }

    Ok(())
}

/// Describes an error which occurred during the reading or writing of NBT byte data.
#[derive(Debug)]
pub enum NbtIoError {
    /// A native I/O error.
    StdIo(io::Error),
    /// No root tag was found. All NBT byte data must start with a valid root tag,
    /// which by default means a Compound or List tag.
    /// If parsing a certain file used by Minecraft, usually only one of the two is accepted.
    /// Java exclusively uses root compound tags, and in most but not all circumstances,
    /// Bedrock uses root compound tags as well.
    MissingRootTag,
    /// The limit on recursive nesting depth of NBT lists and compounds was exceeded.
    ExceededDepthLimit {
        /// The limit which was exceeded.
        limit: DepthLimit
    },
    /// A sequential data structure was found to be non-homogenous. All sequential structures
    /// in NBT data are homogenous.
    NonHomogenousList {
        /// The list type.
        list_type: u8,
        /// The encountered type.
        encountered_type: u8,
    },
    /// A type requested an option to be read from a list. Since options are indicated by the
    /// absence or presence of a tag, and since all sequential types are length-prefixed,
    /// options cannot exists within arrays in NBT data.
    OptionInList,
    /// A sequential type without a definite length was passed to a serializer.
    MissingLength,
    /// The length of a string or sequential length was too large to fit in the numeric type
    /// it needed to.
    ExcessiveLength,
    /// The length of a string or sequential length was negative.
    NegativeLength,
    /// An invalid tag ID was encountered.
    InvalidTagId(u8),
    /// The first tag ID was expected, but the second was found.
    TagTypeMismatch {
        /// The expected ID.
        expected: u8,
        /// The found ID.
        found: u8,
    },
    /// A sequential type was expected, but another was found.
    ExpectedSeq,
    /// An enum representation was expected, but another was found.
    ExpectedEnum,
    /// An invalid map key was encountered.
    InvalidKey,
    /// An invalid enum variant was encountered.
    InvalidEnumVariant,
    /// An invalid CESU-8 string was encountered.
    InvalidCesu8String,
    /// An invalid UTF-8 string was encountered.
    InvalidUtf8String,
    /// Bytes forming an invalid Network-Endian i32 were encountered.
    InvalidNetI32,
    /// Bytes forming an invalid Network-Endian i64 were encountered.
    InvalidNetI64,
    /// An unsupported type was passed to a serializer or queried from a deserializer.
    UnsupportedType(&'static str),
    /// A custom error message.
    Custom(Box<str>),
}

#[cfg(feature = "serde")]
impl serde::ser::Error for NbtIoError {
    fn custom<T>(msg: T) -> Self
    where T: Display {
        NbtIoError::Custom(msg.to_string().into_boxed_str())
    }
}

#[cfg(feature = "serde")]
impl serde::de::Error for NbtIoError {
    fn custom<T>(msg: T) -> Self
    where T: Display {
        NbtIoError::Custom(msg.to_string().into_boxed_str())
    }
}

impl From<io::Error> for NbtIoError {
    fn from(error: io::Error) -> Self {
        NbtIoError::StdIo(error)
    }
}

impl Display for NbtIoError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            NbtIoError::StdIo(error) => write!(f, "{}", error),
            NbtIoError::MissingRootTag
                => write!(f, "NBT tree does not start with a valid root tag."),
            NbtIoError::ExceededDepthLimit { limit }
                => write!(
                    f, "Exceeded depth limit {} for nested tag lists and compound tags",
                    limit.0
                ),
            &NbtIoError::NonHomogenousList {
                list_type,
                encountered_type,
            } => write!(
                f,
                "Encountered non-homogenous list or sequential type: expected {:X} but found {:X}",
                list_type, encountered_type
            ),
            NbtIoError::OptionInList => write!(
                f,
                "Minecraft's NBT format cannot support options in sequential data structures"
            ),
            NbtIoError::MissingLength => write!(
                f,
                "Sequential types must have an initial computable length to be serializable"
            ),
            NbtIoError::ExcessiveLength => write!(
                f,
                "Length of a string or sequential type must fit in an i16, i32, or usize, depending on situation"
            ),
            NbtIoError::NegativeLength => write!(
                f,
                "Length of a string or sequential type must be nonnegative"
            ),
            &NbtIoError::InvalidTagId(id) => write!(
                f,
                "Encountered invalid tag ID 0x{:X} during deserialization",
                id
            ),
            &NbtIoError::TagTypeMismatch { expected, found } => write!(
                f,
                "Tag type mismatch: expected 0x{:X} but found 0x{:X}",
                expected, found
            ),
            NbtIoError::ExpectedSeq
                => write!(f, "Expected sequential tag type (array)"),
            NbtIoError::ExpectedEnum
                => write!(f, "Encountered invalid enum representation in the NBT tag tree"),
            NbtIoError::InvalidKey
                => write!(f, "Map keys must be a valid string"),
            NbtIoError::InvalidEnumVariant
                => write!(f, "Encountered invalid enum variant while deserializing"),
            NbtIoError::InvalidCesu8String
                => write!(f, "Encountered invalid CESU8 string"),
            NbtIoError::InvalidUtf8String
                => write!(f, "Encountered invalid UTF8 string"),
            NbtIoError::InvalidNetI32
                => write!(f, "Encountered bytes that formed an invalid Network-Endian i32"),
            NbtIoError::InvalidNetI64
                => write!(f, "Encountered bytes that formed an invalid Network-Endian i64"),
            NbtIoError::UnsupportedType(ty)
                => write!(f, "Type {} is not supported by Minecraft's NBT format", ty),
            NbtIoError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl Error for NbtIoError {}
