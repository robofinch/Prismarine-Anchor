mod array;
mod de;
mod ser;
mod util;


use std::borrow::Cow;
use std::io::{Cursor, Read, Write};

use flate2::Compression;
use flate2::{
    read::{GzDecoder, ZlibDecoder},
    write::{GzEncoder, ZlibEncoder},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::io::NbtIoError;
use crate::settings::{IoOptions, NbtCompression};


pub(crate) use self::array::TypeHint;
pub use self::{array::Array, de::Deserializer, util::Ser};
pub use self::ser::{Serializer, UncheckedSerializer};


/// Serializes the given value as binary NBT data, returning the resulting Vec. The value must
/// be a struct or non-unit enum variant, else the serializer will return with an error.
pub fn serialize<T: Serialize>(
    value:     &T,
    opts:      IoOptions,
    root_name: Option<&str>,
) -> Result<Vec<u8>, NbtIoError> {
    let mut cursor = Cursor::new(Vec::<u8>::new());
    serialize_into(&mut cursor, value, opts, root_name)?;
    Ok(cursor.into_inner())
}

/// Similar to [`serialize`], but elides checks for homogeneity on sequential types.
/// This means that there are some `T` for which this method will write invalid
/// NBT data to the given writer.
///
/// [`serialize`]: crate::serde::serialize
pub fn serialize_unchecked<T: Serialize>(
    value:     &T,
    opts:      IoOptions,
    root_name: Option<&str>,
) -> Result<Vec<u8>, NbtIoError> {
    let mut cursor = Cursor::new(Vec::<u8>::new());
    serialize_into_unchecked(&mut cursor, value, opts, root_name)?;
    Ok(cursor.into_inner())
}

/// Serializes the given value as binary NBT data, writing to the given writer.
///
/// The value must be a struct or non-unit enum variant, else the serializer will return with an
/// error.
pub fn serialize_into<W: Write, T: Serialize>(
    writer:    &mut W,
    value:     &T,
    opts:      IoOptions,
    root_name: Option<&str>,
) -> Result<(), NbtIoError> {
    let (mode, compression) = match opts.compression {
        NbtCompression::Uncompressed => {
            return value.serialize(Serializer::new(writer, opts, root_name));
        }
        NbtCompression::ZlibCompressed => (2, Compression::default()),
        NbtCompression::ZlibCompressedWith(compression) => (2, compression.into()),
        NbtCompression::GzipCompressed => (1, Compression::default()),
        NbtCompression::GzipCompressedWith(compression) => (1, compression.into()),
    };

    if mode == 1 {
        value.serialize(Serializer::new(
            &mut GzEncoder::new(writer, compression),
            opts,
            root_name,
        ))
    } else {
        value.serialize(Serializer::new(
            &mut ZlibEncoder::new(writer, compression),
            opts,
            root_name,
        ))
    }
}

/// Similar to [`serialize_into`], but elides checks for homogeneity on sequential types.
/// This means that there are some `T` for which this method will write invalid
/// NBT data to the given writer.
///
/// [`serialize_into`]: crate::serde::serialize_into
pub fn serialize_into_unchecked<W: Write, T: Serialize>(
    writer:    &mut W,
    value:     &T,
    opts:      IoOptions,
    root_name: Option<&str>,
) -> Result<(), NbtIoError> {
    let (mode, compression) = match opts.compression {
        NbtCompression::Uncompressed => {
            return value.serialize(UncheckedSerializer::new(writer, opts, root_name));
        }
        NbtCompression::ZlibCompressed => (2, Compression::default()),
        NbtCompression::ZlibCompressedWith(compression) => (2, compression.into()),
        NbtCompression::GzipCompressed => (1, Compression::default()),
        NbtCompression::GzipCompressedWith(compression) => (1, compression.into()),
    };

    if mode == 1 {
        value.serialize(UncheckedSerializer::new(
            &mut GzEncoder::new(writer, compression),
            opts,
            root_name,
        ))
    } else {
        value.serialize(UncheckedSerializer::new(
            &mut ZlibEncoder::new(writer, compression),
            opts,
            root_name,
        ))
    }
}

/// Deserializes the given type from uncompressed, binary NBT data, allowing for the type to borrow
/// from the given buffer.
///
/// The NBT data must be uncompressed, start with a compound tag, and represent the type `T`
/// correctly, else the deserializer will return with an error.
pub fn deserialize_from_buffer<'de, T: Deserialize<'de>>(
    buffer: &'de [u8],
    opts:   IoOptions,
) -> Result<(T, Cow<'de, str>), NbtIoError> {
    let mut cursor = Cursor::new(buffer);
    let (de, root_name) = Deserializer::from_cursor(&mut cursor, opts)?;
    Ok((T::deserialize(de)?, root_name))
}

/// Deserializes the given type from binary NBT data.
///
/// The NBT data must start with a compound tag and represent the type `T` correctly, else the
/// deserializer will return with an error.
pub fn deserialize<T: DeserializeOwned>(
    bytes: &[u8],
    opts:  IoOptions,
) -> Result<(T, String), NbtIoError> {
    deserialize_from(&mut Cursor::new(bytes), opts)
}

/// Deserializes the given type from binary NBT data read from the given reader.
///
/// The NBT data must start with a compound tag and represent the type `T` correctly, else the
/// deserializer will return with an error.
pub fn deserialize_from<R: Read, T: DeserializeOwned>(
    reader: &mut R,
    opts:   IoOptions,
) -> Result<(T, String), NbtIoError> {
    match opts.compression {
        NbtCompression::Uncompressed => deserialize_from_raw(reader, opts),
        NbtCompression::ZlibCompressed | NbtCompression::ZlibCompressedWith(_) => {
            deserialize_from_raw(&mut ZlibDecoder::new(reader), opts)
        }
        NbtCompression::GzipCompressed | NbtCompression::GzipCompressedWith(_) => {
            deserialize_from_raw(&mut GzDecoder::new(reader), opts)
        }
    }
}

fn deserialize_from_raw<'de: 'a, 'a, R: Read, T: Deserialize<'de>>(
    reader: &'a mut R,
    opts:   IoOptions,
) -> Result<(T, String), NbtIoError> {
    let (de, root_name) = Deserializer::new(reader, opts)?;
    Ok((T::deserialize(de)?, root_name))
}
