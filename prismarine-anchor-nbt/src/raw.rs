#![expect(unsafe_code)]

use std::{ptr, slice, str};
use std::{borrow::Cow, mem::ManuallyDrop};
use std::io::{Read, Result as IoResult, Write};

use byteorder::{BigEndian, LittleEndian, ReadBytesExt as _, WriteBytesExt as _};
use varint_rs::{VarintReader as _, VarintWriter as _};

use crate::{io::NbtIoError, tag::NbtTag};
use crate::settings::{Endianness, IoOptions, StringEncoding};


type NbtResult<T> = Result<T, NbtIoError>;

pub const TAG_END_ID:    u8 = 0x0;
pub const BYTE_ID:       u8 = 0x1;
pub const SHORT_ID:      u8 = 0x2;
pub const INT_ID:        u8 = 0x3;
pub const LONG_ID:       u8 = 0x4;
pub const FLOAT_ID:      u8 = 0x5;
pub const DOUBLE_ID:     u8 = 0x6;
pub const BYTE_ARRAY_ID: u8 = 0x7;
pub const STRING_ID:     u8 = 0x8;
pub const LIST_ID:       u8 = 0x9;
pub const COMPOUND_ID:   u8 = 0xA;
pub const INT_ARRAY_ID:  u8 = 0xB;
pub const LONG_ARRAY_ID: u8 = 0xC;


#[inline]
pub const fn id_for_tag(tag: Option<&NbtTag>) -> u8 {
    match tag {
        None                         => TAG_END_ID,
        Some(NbtTag::Byte(..))       => BYTE_ID,
        Some(NbtTag::Short(..))      => SHORT_ID,
        Some(NbtTag::Int(..))        => INT_ID,
        Some(NbtTag::Long(..))       => LONG_ID,
        Some(NbtTag::Float(..))      => FLOAT_ID,
        Some(NbtTag::Double(..))     => DOUBLE_ID,
        Some(NbtTag::ByteArray(..))  => BYTE_ARRAY_ID,
        #[expect(clippy::match_same_arms)]
        Some(NbtTag::String(..))     => STRING_ID,
        Some(NbtTag::ByteString(..)) => STRING_ID,
        Some(NbtTag::List(..))       => LIST_ID,
        Some(NbtTag::Compound(..))   => COMPOUND_ID,
        Some(NbtTag::IntArray(..))   => INT_ARRAY_ID,
        Some(NbtTag::LongArray(..))  => LONG_ARRAY_ID,
    }
}

#[cfg(feature = "serde")]
#[inline]
pub fn read_bool<R: Read>(reader: &mut R, opts: IoOptions) -> IoResult<bool> {
    Ok(read_u8(reader, opts)? != 0)
}

#[inline]
pub fn read_u8<R: Read>(reader: &mut R, _opts: IoOptions) -> IoResult<u8> {
    reader.read_u8()
}

#[inline]
pub fn read_u16<R: Read>(reader: &mut R, opts: IoOptions) -> IoResult<u16> {
    match opts.endianness {
        Endianness::BigEndian => reader.read_u16::<BigEndian>(),
        Endianness::LittleEndian | Endianness::NetworkLittleEndian => {
            reader.read_u16::<LittleEndian>()
        }
    }
}

#[inline]
pub fn read_i8<R: Read>(reader: &mut R, _opts: IoOptions) -> IoResult<i8> {
    reader.read_i8()
}

#[inline]
pub fn read_i16<R: Read>(reader: &mut R, opts: IoOptions) -> IoResult<i16> {
    match opts.endianness {
        Endianness::BigEndian => reader.read_i16::<BigEndian>(),
        Endianness::LittleEndian | Endianness::NetworkLittleEndian => {
            reader.read_i16::<LittleEndian>()
        }
    }
}

#[inline]
pub fn read_i32<R: Read>(reader: &mut R, opts: IoOptions) -> IoResult<i32> {
    match opts.endianness {
        Endianness::BigEndian           => reader.read_i32::<BigEndian>(),
        Endianness::LittleEndian        => reader.read_i32::<LittleEndian>(),
        Endianness::NetworkLittleEndian => reader.read_i32_varint(),
    }
}

#[inline]
pub fn read_i64<R: Read>(reader: &mut R, opts: IoOptions) -> IoResult<i64> {
    match opts.endianness {
        Endianness::BigEndian           => reader.read_i64::<BigEndian>(),
        Endianness::LittleEndian        => reader.read_i64::<LittleEndian>(),
        Endianness::NetworkLittleEndian => reader.read_i64_varint(),
    }
}

#[inline]
pub fn read_f32<R: Read>(reader: &mut R, opts: IoOptions) -> IoResult<f32> {
    match opts.endianness {
        Endianness::BigEndian => reader.read_f32::<BigEndian>(),
        Endianness::LittleEndian | Endianness::NetworkLittleEndian => {
            reader.read_f32::<LittleEndian>()
        }
    }
}

#[inline]
pub fn read_f64<R: Read>(reader: &mut R, opts: IoOptions) -> IoResult<f64> {
    match opts.endianness {
        Endianness::BigEndian => reader.read_f64::<BigEndian>(),
        Endianness::LittleEndian | Endianness::NetworkLittleEndian => {
            reader.read_f64::<LittleEndian>()
        }
    }
}

#[inline]
pub fn string_from_bytes(bytes: &[u8], opts: IoOptions) -> NbtResult<Cow<'_, str>> {
    match opts.string_encoding {
        StringEncoding::Utf8 => match str::from_utf8(bytes) {
            Ok(string) => Ok(Cow::Borrowed(string)),
            Err(_)     => Err(NbtIoError::InvalidUtf8String),
        },
        StringEncoding::Cesu8 => match cesu8::from_java_cesu8(bytes) {
            Ok(string) => Ok(string),
            Err(_)     => Err(NbtIoError::InvalidCesu8String),
        },
    }
}

#[inline]
pub fn bytes_from_string(string: &str, opts: IoOptions) -> Cow<'_, [u8]> {
    match opts.string_encoding {
        StringEncoding::Utf8  => Cow::Borrowed(string.as_bytes()),
        StringEncoding::Cesu8 => cesu8::to_java_cesu8(string),
    }
}

#[inline]
pub fn read_i32_as_usize<R: Read>(reader: &mut R, opts: IoOptions) -> NbtResult<usize> {
    #[expect(
        clippy::map_err_ignore,
        reason = "out-of-range i32 is the only possible error ignored",
    )]
    usize::try_from(read_i32(reader, opts)?).map_err(|_| NbtIoError::ExcessiveLength)
}

#[inline]
pub fn read_string_len<R: Read>(reader: &mut R, opts: IoOptions) -> NbtResult<usize> {
    #[expect(
        clippy::map_err_ignore,
        reason = "out-of-range u32 is the only possible error ignored",
    )]
    match opts.endianness {
        Endianness::BigEndian | Endianness::LittleEndian => {
            Ok(usize::from(read_u16(reader, opts)?))
        }
        Endianness::NetworkLittleEndian => {
            usize::try_from(reader.read_u32_varint()?).map_err(|_| NbtIoError::ExcessiveLength)
        }
    }
}

pub fn read_string<R: Read>(reader: &mut R, opts: IoOptions) -> NbtResult<String> {
    let len = read_string_len(reader, opts)?;
    let mut bytes = vec![0; len];
    reader.read_exact(&mut bytes)?;

    Ok(string_from_bytes(bytes.as_slice(), opts)?.into_owned())
}

pub fn read_string_or_bytes<R: Read>(reader: &mut R, opts: IoOptions) -> NbtResult<NbtTag> {
    let len = read_string_len(reader, opts)?;
    let mut bytes = vec![0; len];
    reader.read_exact(&mut bytes)?;

    Ok(
        if let Ok(string) = string_from_bytes(bytes.as_slice(), opts) {
            NbtTag::String(string.into_owned())
        } else {
            NbtTag::ByteString(bytes)
        },
    )
}

#[cfg(feature = "serde")]
pub fn read_string_into<'a, R: Read>(
    reader: &mut R,
    opts:   IoOptions,
    dest:   &'a mut Vec<u8>,
) -> NbtResult<Cow<'a, str>> {
    let len = read_string_len(reader, opts)?;
    dest.resize(len, 0);
    reader.read_exact(dest)?;
    string_from_bytes(dest, opts)
}

#[cfg(feature = "serde")]
#[inline]
pub fn write_bool<W: Write>(writer: &mut W, opts: IoOptions, value: bool) -> IoResult<()> {
    write_u8(writer, opts, if value { 1 } else { 0 })
}

#[inline]
pub fn write_u8<W: Write>(writer: &mut W, _opts: IoOptions, value: u8) -> IoResult<()> {
    writer.write_u8(value)
}

#[inline]
pub fn write_u16<W: Write>(writer: &mut W, opts: IoOptions, value: u16) -> IoResult<()> {
    match opts.endianness {
        Endianness::BigEndian => writer.write_u16::<BigEndian>(value),
        Endianness::LittleEndian | Endianness::NetworkLittleEndian => {
            writer.write_u16::<LittleEndian>(value)
        }
    }
}

#[inline]
pub fn write_i8<W: Write>(writer: &mut W, _opts: IoOptions, value: i8) -> IoResult<()> {
    writer.write_i8(value)
}

#[inline]
pub fn write_i16<W: Write>(writer: &mut W, opts: IoOptions, value: i16) -> IoResult<()> {
    match opts.endianness {
        Endianness::BigEndian => writer.write_i16::<BigEndian>(value),
        Endianness::LittleEndian | Endianness::NetworkLittleEndian => {
            writer.write_i16::<LittleEndian>(value)
        }
    }
}

#[inline]
pub fn write_i32<W: Write>(writer: &mut W, opts: IoOptions, value: i32) -> IoResult<()> {
    match opts.endianness {
        Endianness::BigEndian           => writer.write_i32::<BigEndian>(value),
        Endianness::LittleEndian        => writer.write_i32::<LittleEndian>(value),
        Endianness::NetworkLittleEndian => writer.write_i32_varint(value),
    }
}

#[inline]
pub fn write_i64<W: Write>(writer: &mut W, opts: IoOptions, value: i64) -> IoResult<()> {
    match opts.endianness {
        Endianness::BigEndian           => writer.write_i64::<BigEndian>(value),
        Endianness::LittleEndian        => writer.write_i64::<LittleEndian>(value),
        Endianness::NetworkLittleEndian => writer.write_i64_varint(value),
    }
}

#[inline]
pub fn write_f32<W: Write>(writer: &mut W, opts: IoOptions, value: f32) -> IoResult<()> {
    match opts.endianness {
        Endianness::BigEndian => writer.write_f32::<BigEndian>(value),
        Endianness::LittleEndian | Endianness::NetworkLittleEndian => {
            writer.write_f32::<LittleEndian>(value)
        }
    }
}

#[inline]
pub fn write_f64<W: Write>(writer: &mut W, opts: IoOptions, value: f64) -> IoResult<()> {
    match opts.endianness {
        Endianness::BigEndian => writer.write_f64::<BigEndian>(value),
        Endianness::LittleEndian | Endianness::NetworkLittleEndian => {
            writer.write_f64::<LittleEndian>(value)
        }
    }
}

#[inline]
pub fn write_usize_as_i32<W: Write>(
    writer: &mut W,
    opts:   IoOptions,
    value:  usize,
) -> NbtResult<()> {
    #[expect(
        clippy::map_err_ignore,
        reason = "out-of-range usize is the only possible error ignored",
    )]
    let value = i32::try_from(value).map_err(|_| NbtIoError::ExcessiveLength)?;
    write_i32(writer, opts, value)?;
    Ok(())
}

#[inline]
pub fn write_string_len<W: Write>(writer: &mut W, opts: IoOptions, len: usize) -> NbtResult<()> {
    // Error if the length can't be written
    #[expect(
        clippy::map_err_ignore,
        reason = "out-of-range usize is the only possible error ignored",
    )]
    match opts.endianness {
        Endianness::BigEndian | Endianness::LittleEndian => {
            let len = u16::try_from(len).map_err(|_| NbtIoError::ExcessiveLength)?;
            write_u16(writer, opts, len)
        }
        Endianness::NetworkLittleEndian => {
            let len = u32::try_from(len).map_err(|_| NbtIoError::ExcessiveLength)?;
            writer.write_u32_varint(len)
        }
    }
    .map_err(NbtIoError::StdIo)
}

#[inline]
pub fn write_string<W: Write>(writer: &mut W, opts: IoOptions, string: &str) -> NbtResult<()> {
    let string = bytes_from_string(string, opts);
    write_string_len(writer, opts, string.len())?;
    writer
        .write_all(&string)
        .map_err(NbtIoError::StdIo)
}

#[inline]
pub fn write_byte_string<W: Write>(
    writer: &mut W,
    opts:   IoOptions,
    string: &[u8],
) -> NbtResult<()> {
    if opts.allow_invalid_strings {
        write_string_len(writer, opts, string.len())?;
        writer
            .write_all(string)
            .map_err(NbtIoError::StdIo)
    } else {
        Err(NbtIoError::InvalidUtf8String)
    }
}

#[inline]
pub fn cast_byte_buf_to_signed(buf: Vec<u8>) -> Vec<i8> {
    let mut me = ManuallyDrop::new(buf);
    // Pointer cast is valid because i8 and u8 have the same layout
    let ptr = me.as_mut_ptr().cast::<i8>();
    let length = me.len();
    let capacity = me.capacity();

    // SAFETY:
    // * `ptr` was allocated by a Vec
    // * i8 has the same size and alignment as u8
    // * `length` and `capacity` came from a valid Vec
    unsafe { Vec::from_raw_parts(ptr, length, capacity) }
}

#[inline]
pub fn cast_byte_buf_to_unsigned(buf: Vec<i8>) -> Vec<u8> {
    let mut me = ManuallyDrop::new(buf);
    // Pointer cast is valid because i8 and u8 have the same layout
    let ptr = me.as_mut_ptr().cast::<u8>();
    let length = me.len();
    let capacity = me.capacity();

    // SAFETY:
    // * `ptr` was allocated by a Vec
    // * u8 has the same size and alignment as i8
    // * `length` and `capacity` came from a valid Vec
    unsafe { Vec::from_raw_parts(ptr, length, capacity) }
}

// Currently unused, but might be used later
#[inline]
#[expect(dead_code)]
pub fn cast_bytes_to_signed(bytes: &[u8]) -> &[i8] {
    let data = bytes.as_ptr().cast::<i8>();
    let len = bytes.len();

    // SAFETY:
    // * `data` is valid for len * 1 bytes
    //     * The entire memory range of `data` is contained in a single allocated object
    //       since it came from a valid slice
    //     * `data` is non-null and aligned correctly for u8 (and thus i8)
    // * `data` points to exactly `len` consecutive bytes
    // * The constructed reference adopts the lifetime of the provided reference
    // * `len` <= isize::MAX because `len` came from a valid slice
    unsafe { slice::from_raw_parts(data, len) }
}

#[inline]
pub fn cast_bytes_to_unsigned(bytes: &[i8]) -> &[u8] {
    let data = bytes.as_ptr().cast::<u8>();
    let len = bytes.len();

    // SAFETY:
    // * `data` is valid for len * 1 bytes
    //     * The entire memory range of `data` is contained in a single allocated object
    //       since it came from a valid slice
    //     * `data` is non-null and aligned correctly for i8 (and thus u8)
    // * `data` points to exactly `len` consecutive bytes
    // * The constructed reference adopts the lifetime of the provided reference
    // * `len` <= isize::MAX because `len` came from a valid slice
    unsafe { slice::from_raw_parts(data, len) }
}

pub fn ref_i8_to_ref_u8(byte: &i8) -> &u8 {
    let ptr = ptr::from_ref::<i8>(byte).cast::<u8>();

    // SAFETY:
    // * `ptr` came from a valid reference to an i8, so it is non-null.
    // * u8 has the same size and alignment as i8,
    //   so `ptr` is correctly aligned and dereferenceable.
    // * `ptr` points to a valid byte (no matter what that byte is)
    // * aliasing is satisfied, since we inherit the lifetime of the `byte` argument.
    unsafe { &*ptr }
}

/// Convert a mutable reference to a slice of non-zero-sized values to a mutable reference
/// to the bytes of those values.
///
/// # Safety:
/// Any possible bit pattern with the size and alignment of `T` should be a valid bit pattern
/// for `T`.
///
/// # Panics:
/// If `T` is a zero-sized type or somehow has an alignment strictly greater than
/// the alignment of `u8`, the function panics.
unsafe fn non_zst_slice_to_bytes<T>(data: &mut [T]) -> &mut [u8] {
    const {
        // Note that `assert_ne!` currently can't be called in a const block.
        assert!(
            size_of::<T>() != 0,
            "non_zst_slice_to_bytes was called on a slice of a zero-sized type",
        );
        assert!(
            align_of::<u8>() <= align_of::<T>(),
            "You managed to run this on hardware where u8 has alignment that's too high",
        );
    }

    let ptr = data.as_mut_ptr().cast::<u8>();
    let t_len = data.len();

    // NOTE:
    // the byte length of slices and vectors in Rust is in the range
    // `0..=isize::MAX` for non-ZST types.
    // Therefore, the below code neither overflows nor exceeds isize::MAX.
    let byte_len = t_len * size_of::<i32>();

    // SAFETY:
    // * `ptr` is valid for `t_len * size_of::<T>` bytes, a.k.a. `byte_len * 1` bytes
    //   which is `byte_len * size_of::<u8>` bytes.
    //     * The entire memory range of `ptr` is contained in a single allocated object since it
    //       came from a valid slice (`data`)
    //     * `ptr` is non-null and aligned correctly for T (and thus also u8, which has the lowest
    //       alignment requirements).
    // * `ptr` points to exactly `byte_len` consecutive bytes
    // * The constructed reference adopts the lifetime of the provided reference
    // * `byte_len` <= isize::MAX, and as `ptr` and `byte_len` were derived from a valid slice,
    //   `ptr + byte_len` can't wrap around the address space (else `data` would have.. problems)
    unsafe { slice::from_raw_parts_mut(ptr, byte_len) }
}

#[inline]
pub fn read_i32_array<R: Read>(
    reader: &mut R,
    opts:   IoOptions,
    len:    usize,
) -> IoResult<Vec<i32>> {
    if matches!(opts.endianness, Endianness::NetworkLittleEndian) {
        // The number of bytes to read per i32 is variable; we can't do any better than reading
        // the values one-at-a-time
        (0..len)
            .map(|_| reader.read_i32_varint())
            .collect()

    } else {
        let mut data = vec![0_i32; len];
        let data_slice = data.as_mut_slice();

        // SAFETY:
        // * Any possible pattern of 8 bytes with the alignment of an i32 is a valid i32 value.
        let byte_slice = unsafe { non_zst_slice_to_bytes(data_slice) };

        reader.read_exact(byte_slice)?;

        // After the above read, we have now read all the data into the original `data` Vec.
        // But endianness might be messed up, since we directly copied the bytes.

        match opts.endianness {
            Endianness::BigEndian => {
                #[cfg(target_endian = "little")]
                {
                    // We need to swap the endianness.
                    for entry in &mut data {
                        *entry = entry.swap_bytes();
                    }
                }
                // If the target is BigEndian, then the data is already in the correct format.
                Ok(data)
            }
            Endianness::LittleEndian => {
                #[cfg(target_endian = "big")]
                {
                    // We need to swap the endianness.
                    for entry in data.iter_mut() {
                        *entry = entry.swap_bytes();
                    }
                }
                // If the target is LittleEndian, then the data is already in the correct format.
                Ok(data)
            }
            Endianness::NetworkLittleEndian => unreachable!(),
        }
    }
}

#[inline]
pub fn read_i64_array<R: Read>(
    reader: &mut R,
    opts:   IoOptions,
    len:    usize,
) -> IoResult<Vec<i64>> {
    if matches!(opts.endianness, Endianness::NetworkLittleEndian) {
        // The number of bytes to read per i64 is variable; we can't do any better than reading
        // the values one-at-a-time
        (0..len)
            .map(|_| reader.read_i64_varint())
            .collect()

    } else {
        let mut data = vec![0_i64; len];
        let data_slice = data.as_mut_slice();

        // SAFETY:
        // * Any possible pattern of 8 bytes with the alignment of an i64 is a valid i64 value.
        let byte_slice = unsafe { non_zst_slice_to_bytes(data_slice) };

        reader.read_exact(byte_slice)?;

        // After the above read, we have now read all the data into the original `data` Vec.
        // But endianness might be messed up, since we directly copied the bytes.

        match opts.endianness {
            Endianness::BigEndian => {
                #[cfg(target_endian = "little")]
                {
                    for entry in &mut data {
                        *entry = entry.swap_bytes();
                    }
                }
                // If the target is BigEndian, then the data is already in the correct format.
                Ok(data)
            }
            Endianness::LittleEndian => {
                #[cfg(target_endian = "big")]
                {
                    for entry in data.iter_mut() {
                        *entry = entry.swap_bytes();
                    }
                }
                // If the target is LittleEndian, then the data is already in the correct format.
                Ok(data)
            }
            Endianness::NetworkLittleEndian => unreachable!(),
        }
    }
}
