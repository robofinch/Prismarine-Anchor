#![allow(unsafe_code)]

use std::{ptr, slice, str};
use std::{borrow::Cow, mem::ManuallyDrop};
use std::io::{Read, Result as IoResult, Write};

use byteorder::{BigEndian, LittleEndian, ReadBytesExt as _, WriteBytesExt as _};
use varint_rs::{VarintReader as _, VarintWriter as _};

use crate::{io::NbtIoError, tag::NbtTag};
use crate::settings::{Endianness, IoOptions, StringEncoding};


type NbtResult<T> = Result<T, NbtIoError>;


#[inline]
pub const fn id_for_tag(tag: Option<&NbtTag>) -> u8 {
    match tag {
        None => 0x0, // TAG_End
        Some(NbtTag::Byte(..))       => 0x1,
        Some(NbtTag::Short(..))      => 0x2,
        Some(NbtTag::Int(..))        => 0x3,
        Some(NbtTag::Long(..))       => 0x4,
        Some(NbtTag::Float(..))      => 0x5,
        Some(NbtTag::Double(..))     => 0x6,
        Some(NbtTag::ByteArray(..))  => 0x7,
        Some(NbtTag::String(..))     => 0x8,
        Some(NbtTag::ByteString(..)) => 0x8,
        Some(NbtTag::List(..))       => 0x9,
        Some(NbtTag::Compound(..))   => 0xA,
        Some(NbtTag::IntArray(..))   => 0xB,
        Some(NbtTag::LongArray(..))  => 0xC,
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
        Endianness::BigEndian
            => reader.read_u16::<BigEndian>(),
        Endianness::LittleEndian | Endianness::NetworkLittleEndian
            => reader.read_u16::<LittleEndian>()
    }
}

#[inline]
pub fn read_i8<R: Read>(reader: &mut R, _opts: IoOptions) -> IoResult<i8> {
    reader.read_i8()
}

#[inline]
pub fn read_i16<R: Read>(reader: &mut R, opts: IoOptions) -> IoResult<i16> {
    match opts.endianness {
        Endianness::BigEndian
            => reader.read_i16::<BigEndian>(),
        Endianness::LittleEndian | Endianness::NetworkLittleEndian
            => reader.read_i16::<LittleEndian>()
    }
}

#[inline]
pub fn read_i32<R: Read>(reader: &mut R, opts: IoOptions) -> IoResult<i32> {
    match opts.endianness {
        Endianness::BigEndian    => reader.read_i32::<BigEndian>(),
        Endianness::LittleEndian => reader.read_i32::<LittleEndian>(),
        Endianness::NetworkLittleEndian => reader.read_i32_varint()
    }
}

#[inline]
pub fn read_i64<R: Read>(reader: &mut R, opts: IoOptions) -> IoResult<i64> {
    match opts.endianness {
        Endianness::BigEndian    => reader.read_i64::<BigEndian>(),
        Endianness::LittleEndian => reader.read_i64::<LittleEndian>(),
        Endianness::NetworkLittleEndian => reader.read_i64_varint()
    }
}

#[inline]
pub fn read_f32<R: Read>(reader: &mut R, opts: IoOptions) -> IoResult<f32> {
    match opts.endianness {
        Endianness::BigEndian
            => reader.read_f32::<BigEndian>(),
        Endianness::LittleEndian | Endianness::NetworkLittleEndian
            => reader.read_f32::<LittleEndian>()
    }
}

#[inline]
pub fn read_f64<R: Read>(reader: &mut R, opts: IoOptions) -> IoResult<f64> {
    match opts.endianness {
        Endianness::BigEndian
            => reader.read_f64::<BigEndian>(),
        Endianness::LittleEndian | Endianness::NetworkLittleEndian
            => reader.read_f64::<LittleEndian>()
    }
}

#[inline]
pub fn string_from_bytes(bytes: &[u8], opts: IoOptions) -> NbtResult<Cow<'_, str>> {
    match opts.string_encoding {
        StringEncoding::Utf8 => match str::from_utf8(bytes) {
            Ok(string) => Ok(Cow::Borrowed(string)),
            Err(_) => Err(NbtIoError::InvalidUtf8String)
        },
        StringEncoding::Cesu8 => match cesu8::from_java_cesu8(bytes) {
            Ok(string) => Ok(string),
            Err(_) => Err(NbtIoError::InvalidCesu8String),
        }
    }
}

#[inline]
pub fn bytes_from_string(string: &str, opts: IoOptions) -> Cow<'_, [u8]> {
    match opts.string_encoding {
        StringEncoding::Utf8 => Cow::Borrowed(string.as_bytes()),
        StringEncoding::Cesu8 => cesu8::to_java_cesu8(string)
    }
}

#[inline]
pub fn read_i32_as_usize<R: Read>(reader: &mut R, opts: IoOptions) -> NbtResult<usize> {
    usize::try_from(read_i32(reader, opts)?).map_err(|_| NbtIoError::ExcessiveLength)
}

#[inline]
pub fn read_string_len<R: Read>(reader: &mut R, opts: IoOptions) -> NbtResult<usize> {
    match opts.endianness {
        Endianness::BigEndian | Endianness::LittleEndian
            => Ok(usize::from(read_u16(reader, opts)?)),
        Endianness::NetworkLittleEndian
            => usize::try_from(reader.read_u32_varint()?)
                .map_err(|_| NbtIoError::ExcessiveLength),
    }
}

pub fn read_string<R: Read>(reader: &mut R, opts: IoOptions) -> NbtResult<String> {
    let len = read_string_len(reader, opts)?;
    let mut bytes = vec![0; len];
    reader.read_exact(&mut bytes)?;

    Ok(string_from_bytes(bytes.as_slice(), opts)?.into_owned())
}

pub fn read_string_or_bytes(reader: &mut impl Read, opts: IoOptions) -> NbtResult<NbtTag> {
    let len = read_string_len(reader, opts)?;
    let mut bytes = vec![0; len];
    reader.read_exact(&mut bytes)?;

    Ok(if let Ok(string) = string_from_bytes(bytes.as_slice(), opts) {
        NbtTag::String(string.into_owned())
    } else {
        NbtTag::ByteString(bytes)
    })
}

#[cfg(feature = "serde")]
pub fn read_string_into<'a, R: Read>(
    reader: &mut R,
    opts: IoOptions,
    dest: &'a mut Vec<u8>
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
        Endianness::BigEndian
            => writer.write_u16::<BigEndian>(value),
        Endianness::LittleEndian | Endianness::NetworkLittleEndian
            => writer.write_u16::<LittleEndian>(value),
    }
}

#[inline]
pub fn write_i8<W: Write>(writer: &mut W, _opts: IoOptions, value: i8) -> IoResult<()> {
    writer.write_i8(value)
}

#[inline]
pub fn write_i16<W: Write>(writer: &mut W, opts: IoOptions, value: i16) -> IoResult<()> {
    match opts.endianness {
        Endianness::BigEndian
            => writer.write_i16::<BigEndian>(value),
        Endianness::LittleEndian | Endianness::NetworkLittleEndian
            => writer.write_i16::<LittleEndian>(value),
    }
}

#[inline]
pub fn write_i32<W: Write>(writer: &mut W, opts: IoOptions, value: i32) -> IoResult<()> {
    match opts.endianness {
        Endianness::BigEndian    => writer.write_i32::<BigEndian>(value),
        Endianness::LittleEndian => writer.write_i32::<LittleEndian>(value),
        Endianness::NetworkLittleEndian => writer.write_i32_varint(value)
    }
}

#[inline]
pub fn write_i64<W: Write>(writer: &mut W, opts: IoOptions, value: i64) -> IoResult<()> {
    match opts.endianness {
        Endianness::BigEndian    => writer.write_i64::<BigEndian>(value),
        Endianness::LittleEndian => writer.write_i64::<LittleEndian>(value),
        Endianness::NetworkLittleEndian => writer.write_i64_varint(value)
    }
}

#[inline]
pub fn write_f32<W: Write>(writer: &mut W, opts: IoOptions, value: f32) -> IoResult<()> {
    match opts.endianness {
        Endianness::BigEndian
            => writer.write_f32::<BigEndian>(value),
        Endianness::LittleEndian | Endianness::NetworkLittleEndian
            => writer.write_f32::<LittleEndian>(value),
    }
}

#[inline]
pub fn write_f64<W: Write>(writer: &mut W, opts: IoOptions, value: f64) -> IoResult<()> {
    match opts.endianness {
        Endianness::BigEndian
            => writer.write_f64::<BigEndian>(value),
        Endianness::LittleEndian | Endianness::NetworkLittleEndian
            => writer.write_f64::<LittleEndian>(value),
    }
}

#[inline]
pub fn write_usize_as_i32<W: Write>(writer: &mut W, opts: IoOptions, value: usize) -> NbtResult<()> {
    let value = i32::try_from(value).map_err(|_| NbtIoError::ExcessiveLength)?;
    write_i32(writer, opts, value)?;
    Ok(())
}

#[inline]
pub fn write_string_len<W: Write>(writer: &mut W, opts: IoOptions, len: usize) -> NbtResult<()> {
    // Error if the length can't be written
    match opts.endianness {
        Endianness::BigEndian | Endianness::LittleEndian => {
            let len = u16::try_from(len).map_err(|_| NbtIoError::ExcessiveLength)?;
            write_u16(writer, opts, len)
        }
        Endianness::NetworkLittleEndian => {
            let len = u32::try_from(len).map_err(|_| NbtIoError::ExcessiveLength)?;
            writer.write_u32_varint(len)
        }
    }.map_err(NbtIoError::StdIo)
}

#[inline]
pub fn write_string<W: Write>(writer: &mut W, opts: IoOptions, string: &str) -> NbtResult<()> {
    let string = bytes_from_string(string, opts);
    write_string_len(writer, opts, string.len())?;
    writer.write_all(&string).map_err(NbtIoError::StdIo)
}

#[inline]
pub fn write_byte_string(
    writer: &mut impl Write,
    opts: IoOptions,
    string: &[u8],
) -> NbtResult<()> {
    if opts.allow_invalid_strings {
        write_string_len(writer, opts, string.len())?;
        writer.write_all(string).map_err(NbtIoError::StdIo)
    } else {
        Err(NbtIoError::InvalidUtf8String)
    }
}

#[inline]
pub fn cast_byte_buf_to_signed(buf: Vec<u8>) -> Vec<i8> {
    let mut me = ManuallyDrop::new(buf);
    // Pointer cast is valid because i8 and u8 have the same layout
    let ptr = me.as_mut_ptr() as *mut i8;
    let length = me.len();
    let capacity = me.capacity();

    // Safety
    // * `ptr` was allocated by a Vec
    // * i8 has the same size and alignment as u8
    // * `length` and `capacity` came from a valid Vec
    unsafe { Vec::from_raw_parts(ptr, length, capacity) }
}

#[inline]
pub fn cast_byte_buf_to_unsigned(buf: Vec<i8>) -> Vec<u8> {
    let mut me = ManuallyDrop::new(buf);
    // Pointer cast is valid because i8 and u8 have the same layout
    let ptr = me.as_mut_ptr() as *mut u8;
    let length = me.len();
    let capacity = me.capacity();

    // Safety
    // * `ptr` was allocated by a Vec
    // * u8 has the same size and alignment as i8
    // * `length` and `capacity` came from a valid Vec
    unsafe { Vec::from_raw_parts(ptr, length, capacity) }
}

// Currently unused, but might be used later
#[inline]
#[allow(dead_code)]
pub fn cast_bytes_to_signed(bytes: &[u8]) -> &[i8] {
    let data = bytes.as_ptr() as *const i8;
    let len = bytes.len();

    // Safety
    // * `data` is valid for len * 1 bytes
    //     * The entire memory range of `data` is contained in a single
    //       allocated object since it came from a valid slice
    //     * `data` is non-null and aligned correctly for u8 (and thus i8)
    // * `data` points to exactly `len` consecutive bytes
    // * The constructed reference adopts the lifetime of the provided reference
    // * `len` <= isize::MAX because `len` came from a valid slice
    unsafe { slice::from_raw_parts(data, len) }
}

#[inline]
pub fn cast_bytes_to_unsigned(bytes: &[i8]) -> &[u8] {
    let data = bytes.as_ptr() as *const u8;
    let len = bytes.len();

    // Safety
    // * `data` is valid for len * 1 bytes
    //     * The entire memory range of `data` is contained in a single
    //       allocated object since it came from a valid slice
    //     * `data` is non-null and aligned correctly for i8 (and thus u8)
    // * `data` points to exactly `len` consecutive bytes
    // * The constructed reference adopts the lifetime of the provided reference
    // * `len` <= isize::MAX because `len` came from a valid slice
    unsafe { slice::from_raw_parts(data, len) }
}

#[inline]
pub fn read_i32_array<R: Read>(reader: &mut R, opts: IoOptions, len: usize) -> IoResult<Vec<i32>> {

    if let Endianness::NetworkLittleEndian = opts.endianness {
        // The number of bytes to read per i32 is variable; we can't do any better than reading
        // the values one-at-a-time
        (0..len).map(|_| reader.read_i32_varint()).collect()

    } else {
        let mut bytes = ManuallyDrop::new(vec![0i32; len]);

        let ptr = bytes.as_mut_ptr() as *mut u8;
        let length = bytes.len() * 4;
        let capacity = bytes.capacity() * 4;

        let mut bytes = unsafe { Vec::from_raw_parts(ptr, length, capacity) };

        reader.read_exact(&mut bytes)?;

        // Safety: the length of the vec is a multiple of 4, and the alignment is 4
        match opts.endianness {
            Endianness::BigEndian => Ok(unsafe {
                convert_int_array_in_place::<i32, 4>(bytes, i32::from_be_bytes)
            }),
            Endianness::LittleEndian => Ok(unsafe {
                convert_int_array_in_place::<i32, 4>(bytes, i32::from_le_bytes)
            }),
            Endianness::NetworkLittleEndian => unreachable!()
        }
    }

}

#[inline]
pub fn read_i64_array<R: Read>(reader: &mut R, opts: IoOptions, len: usize) -> IoResult<Vec<i64>> {

    if let Endianness::NetworkLittleEndian = opts.endianness {
        // The number of bytes to read per i64 is variable; we can't do any better than reading
        // the values one-at-a-time
        (0..len).map(|_| reader.read_i64_varint()).collect()

    } else {
        let mut bytes = ManuallyDrop::new(vec![0i64; len]);

        let ptr = bytes.as_mut_ptr() as *mut u8;
        let length = bytes.len() * 8;
        let capacity = bytes.capacity() * 8;

        let mut bytes = unsafe { Vec::from_raw_parts(ptr, length, capacity) };

        reader.read_exact(&mut bytes)?;

        // Safety: the length of the vec is a multiple of 8, and the alignment is 8
        match opts.endianness {
            Endianness::BigEndian => Ok(unsafe {
                convert_int_array_in_place::<i64, 8>(bytes, i64::from_be_bytes)
            }),
            Endianness::LittleEndian => Ok(unsafe {
                convert_int_array_in_place::<i64, 8>(bytes, i64::from_le_bytes)
            }),
            Endianness::NetworkLittleEndian => unreachable!()
        }
    }
}

/// ## Safety
/// The length of `bytes` must be a multiple of the size of `I` (in bytes),
/// and the alignment of `bytes` must equal the alignment of `I`.
#[inline]
unsafe fn convert_int_array_in_place<I, const SIZE: usize>(
    mut bytes: Vec<u8>,
    convert: fn([u8; SIZE]) -> I,
) -> Vec<I> {
    let mut buf: [u8; SIZE];

    let mut read = bytes.as_ptr() as *const [u8; SIZE];
    let mut write = bytes.as_mut_ptr() as *mut I;

    // Note that if something managed to panic here, the state of bytes
    // might not be correct, but it won't violate memory safety (any 8 bits are a valid u8)
    unsafe {
        let end = bytes.as_ptr().add(bytes.len()) as *const [u8; SIZE];

        while read != end {
            buf = ptr::read(read);
            ptr::write(write, convert(buf));
            read = read.add(1);
            write = write.add(1);
        }
    }

    let mut me = ManuallyDrop::new(bytes);

    let ptr = me.as_mut_ptr() as *mut I;
    let length = me.len();
    let capacity = me.capacity();

    unsafe { Vec::from_raw_parts(ptr, length / SIZE, capacity / SIZE) }
}

pub fn ref_i8_to_ref_u8(byte: &i8) -> &u8 {
    unsafe { &*(byte as *const i8 as *const u8) }
}
