#![expect(unsafe_code)]

use std::{fmt, io, ptr};
use std::{borrow::Cow, error::Error, marker::PhantomData};
use std::{
    fmt::{Debug, Display, Formatter},
    io::{Cursor, ErrorKind, Read},
};

use serde::de;
use serde::forward_to_deserialize_any;
use serde::de::{
    DeserializeSeed, EnumAccess, IntoDeserializer as _, MapAccess, SeqAccess,
    value::CowStrDeserializer, VariantAccess, Visitor,
};

use crate::raw;
use crate::{io::NbtIoError, settings::IoOptions};
use super::array::TYPE_HINT_NICHE;
use crate::raw::{
    BYTE_ARRAY_ID, BYTE_ID, COMPOUND_ID, DOUBLE_ID, FLOAT_ID, INT_ARRAY_ID,
    INT_ID, LIST_ID, LONG_ID, LONG_ARRAY_ID, SHORT_ID, STRING_ID, TAG_END_ID,
};


/// The deserializer type for reading binary NBT data.
pub struct Deserializer<'a, R, B> {
    reader:    &'a mut R,
    opts:      IoOptions,
    _buffered: PhantomData<B>,
}

impl<R, B> Debug for Deserializer<'_, R, B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Deserializer with opts: {:?}", self.opts)
    }
}

impl<'a, R: Read> Deserializer<'a, R, Unbuffered> {
    /// Attempts to construct a new deserializer with the given reader. If the data in the reader
    /// does not start with a valid compound tag, an error is returned. Otherwise, the root name
    /// is returned along with the deserializer.
    pub fn new(reader: &'a mut R, opts: IoOptions) -> Result<(Self, String), NbtIoError> {
        if raw::read_u8(reader, opts)? != COMPOUND_ID {
            return Err(NbtIoError::MissingRootTag);
        }

        let root_name = raw::read_string(reader, opts)?;
        Ok((
            Deserializer {
                reader,
                opts,
                _buffered: PhantomData,
            },
            root_name,
        ))
    }
}

impl<'a, 'buffer> Deserializer<'a, Cursor<&'buffer [u8]>, BufferedCursor<'buffer>>
where
// This is just here explicitly to clarify what's going on. In reality, having a reference to
// a cursor in and of itself is a certificate of this constraint 'buffer: 'a
{
    /// Similar to [`new`], however constructing a deserializer with this method will allow for
    /// data to be borrowed during deserialization.
    ///
    /// If the data in the reader does not start with a valid compound tag, an error is returned.
    /// Otherwise, the root name is returned along with the deserializer.
    ///
    /// [`new`]: crate::serde::Deserializer::new
    pub fn from_cursor(
        reader: &'a mut Cursor<&'buffer [u8]>,
        opts:   IoOptions,
    ) -> Result<(Self, Cow<'buffer, str>), NbtIoError> {
        if raw::read_u8(reader, opts)? != COMPOUND_ID {
            return Err(NbtIoError::MissingRootTag);
        }

        let root_name_len = raw::read_string_len(reader, opts)?;
        let bytes = read_bytes_from_cursor(reader, root_name_len)?;

        let root_name = raw::string_from_bytes(bytes, opts)?;

        Ok((
            Deserializer {
                reader,
                opts,
                _buffered: PhantomData,
            },
            root_name,
        ))
    }
}

impl<'de, 'a, 'buffer, R, B> de::Deserializer<'de> for Deserializer<'a, R, B>
where
    'de: 'a,
    'buffer: 'de,
    R: Read,
    B: BufferSpecialization<'buffer>,
{
    type Error = NbtIoError;

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct identifier ignored_any
    }

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    #[inline]
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        DeserializeTag::<_, B, COMPOUND_ID>::new(self.reader, self.opts, 0)
            .deserialize_map(visitor)
    }

    #[inline]
    fn deserialize_struct<V>(
        self,
        _name:   &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    #[inline]
    fn deserialize_enum<V>(
        self,
        name:     &'static str,
        variants: &'static [&'static str],
        visitor:  V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        DeserializeTag::<_, B, COMPOUND_ID>::new(self.reader, self.opts, 0)
            .deserialize_enum(name, variants, visitor)
    }

    #[inline]
    fn is_human_readable(&self) -> bool {
        false
    }
}

#[inline]
fn drive_visitor_seq_const<'de, 'a, 'buffer, R, V, B, const TAG_ID: u8>(
    reader:        &'a mut R,
    opts:          IoOptions,
    current_depth: u32,
    visitor:       V,
) -> Result<V::Value, NbtIoError>
where
    'de: 'a,
    'buffer: 'de,
    R: Read,
    V: Visitor<'de>,
    B: BufferSpecialization<'buffer>,
{
    match TAG_ID {
        BYTE_ARRAY_ID => {
            let len = raw::read_i32_as_usize(reader, opts)?;
            visitor.visit_seq(DeserializeSeq::<_, _, BYTE_ID, TAG_ID>::new(
                DeserializeTag::<_, B, BYTE_ID>::new(reader, opts, current_depth),
                len,
            ))
        }
        LIST_ID => drive_visitor_seq_tag::<_, _, B>(reader, opts, current_depth, visitor),
        INT_ARRAY_ID => {
            let len = raw::read_i32_as_usize(reader, opts)?;
            visitor.visit_seq(DeserializeSeq::<_, _, INT_ID, TAG_ID>::new(
                DeserializeTag::<_, B, INT_ID>::new(reader, opts, current_depth),
                len,
            ))
        }
        LONG_ARRAY_ID => {
            let len = raw::read_i32_as_usize(reader, opts)?;
            visitor.visit_seq(DeserializeSeq::<_, _, LONG_ID, TAG_ID>::new(
                DeserializeTag::<_, B, LONG_ID>::new(reader, opts, current_depth),
                len,
            ))
        }
        _ => Err(NbtIoError::ExpectedSeq),
    }
}

fn drive_visitor_seq_tag<'de, 'a, 'buffer, R, V, B>(
    reader:        &'a mut R,
    opts:          IoOptions,
    current_depth: u32,
    visitor:       V,
) -> Result<V::Value, NbtIoError>
where
    'de: 'a,
    'buffer: 'de,
    R: Read,
    V: Visitor<'de>,
    B: BufferSpecialization<'buffer>,
{
    let id = raw::read_u8(reader, opts)?;
    let len = raw::read_i32_as_usize(reader, opts)?;

    if len != 0 && current_depth >= opts.depth_limit.0 {
        return Err(NbtIoError::ExceededDepthLimit {
            limit: opts.depth_limit,
        });
    }

    macro_rules! drive_visitor {
        ($($id:literal)*) => {
            match id {
                TAG_END_ID => {
                    if len == 0 {
                        visitor.visit_seq(DeserializeSeq::<_, _, TAG_END_ID, LIST_ID>::new(
                            DeserializeTag::<_, B, TAG_END_ID>::new(
                                reader,
                                opts,
                                current_depth + 1,
                            ),
                            len,
                        ))
                    } else {
                        Err(NbtIoError::InvalidTagId(TAG_END_ID))
                    }
                }
                $( $id => visitor.visit_seq(DeserializeSeq::<_, _, $id, LIST_ID>::new(
                    DeserializeTag::<_, B, $id>::new(reader, opts, current_depth + 1), len)
                ), )*
                _ => Err(NbtIoError::InvalidTagId(id))
            }
        };
    }

    drive_visitor!(0x1 0x2 0x3 0x4 0x5 0x6 0x7 0x8 0x9 0xA 0xB 0xC)
}

struct DeserializeEnum<'a, R, B, const TAG_ID: u8> {
    reader:        &'a mut R,
    opts:          IoOptions,
    current_depth: u32,
    variant:       Cow<'a, str>,
    _buffered:     PhantomData<B>,
}

impl<'a, R, B, const TAG_ID: u8> DeserializeEnum<'a, R, B, TAG_ID> {
    #[inline]
    fn new(reader: &'a mut R, opts: IoOptions, current_depth: u32, variant: Cow<'a, str>) -> Self {
        DeserializeEnum {
            reader,
            opts,
            current_depth,
            variant,
            _buffered: PhantomData,
        }
    }
}

impl<'de, 'a, 'buffer, R, B, const TAG_ID: u8> EnumAccess<'de>
    for DeserializeEnum<'a, R, B, TAG_ID>
where
    'de: 'a,
    'buffer: 'de,
    R: Read,
    B: BufferSpecialization<'buffer>,
{
    type Error = NbtIoError;
    type Variant = DeserializeVariant<'a, R, B, TAG_ID>;

    #[inline]
    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let de: CowStrDeserializer<'a, Self::Error> = self.variant.into_deserializer();
        Ok((
            seed.deserialize(de)?,
            DeserializeVariant::new(self.reader, self.opts, self.current_depth),
        ))
    }
}

struct DeserializeVariant<'a, R, B, const TAG_ID: u8> {
    reader:        &'a mut R,
    opts:          IoOptions,
    current_depth: u32,
    _buffered:     PhantomData<B>,
}

impl<'a, 'buffer, R, B, const TAG_ID: u8> DeserializeVariant<'a, R, B, TAG_ID>
where
    R: Read,
    B: BufferSpecialization<'buffer>,
{
    #[inline]
    fn new(reader: &'a mut R, opts: IoOptions, current_depth: u32) -> Self {
        DeserializeVariant {
            reader,
            opts,
            current_depth,
            _buffered: PhantomData,
        }
    }
}

impl<'de, 'a, 'buffer, R, B, const TAG_ID: u8> VariantAccess<'de>
    for DeserializeVariant<'a, R, B, TAG_ID>
where
    'de: 'a,
    'buffer: 'de,
    R: Read,
    B: BufferSpecialization<'buffer>,
{
    type Error = NbtIoError;

    #[expect(
        clippy::unimplemented, clippy::disallowed_macros,
        reason = "this feels more descriptive than panic! or unreachable!",
    )]
    #[cold]
    fn unit_variant(self) -> Result<(), Self::Error> {
        unimplemented!("Unit variant should have been handled by deserialize_enum")
    }

    #[inline]
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut DeserializeTag::<_, B, TAG_ID>::new(
            self.reader,
            self.opts,
            self.current_depth,
        ))
    }

    #[inline]
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        drive_visitor_seq_const::<_, _, B, TAG_ID>(
            self.reader,
            self.opts,
            self.current_depth,
            visitor,
        )
    }

    #[inline]
    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if TAG_ID == COMPOUND_ID {
            visitor.visit_map(DeserializeMap::<_, B>::new(
                self.reader,
                self.opts,
                self.current_depth,
            ))
        } else {
            Err(NbtIoError::TagTypeMismatch {
                expected: COMPOUND_ID,
                found:    TAG_ID,
            })
        }
    }
}

struct DeserializeSeq<'a, R, B, const TAG_ID: u8, const LIST_ID: u8> {
    inner:          DeserializeTag<'a, R, B, TAG_ID>,
    remaining:      usize,
    dispatch_state: TypeHintDispatchState,
    _buffered:      PhantomData<B>,
}

impl<'a, 'buffer, R, B, const TAG_ID: u8, const LIST_ID: u8>
    DeserializeSeq<'a, R, B, TAG_ID, LIST_ID>
where
    R: Read,
    B: BufferSpecialization<'buffer>,
{
    #[inline]
    fn new(inner: DeserializeTag<'a, R, B, TAG_ID>, len: usize) -> Self {
        DeserializeSeq {
            inner,
            remaining: len,
            dispatch_state: TypeHintDispatchState::Waiting,
            _buffered: PhantomData,
        }
    }
}

impl<'de, 'a, 'buffer, R, B, const TAG_ID: u8, const LIST_ID: u8> SeqAccess<'de>
    for DeserializeSeq<'a, R, B, TAG_ID, LIST_ID>
where
    'de: 'a,
    'buffer: 'de,
    R: Read,
    B: BufferSpecialization<'buffer>,
{
    type Error = NbtIoError;

    #[inline]
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        // This will optimize out
        if TAG_ID == 0 {
            return Ok(None);
        }

        // This is necessary for the LLVM to consider inlining the outer function
        #[inline(never)]
        fn handle_hint_dispatch<'de, T, const LIST_ID: u8>(
            state: &mut TypeHintDispatchState,
            seed:  T,
        ) -> Result<Option<T::Value>, NbtIoError>
        where
            T: de::DeserializeSeed<'de>,
        {
            match state {
                TypeHintDispatchState::Ready => {
                    *state = TypeHintDispatchState::Sent;
                    Ok(seed.deserialize(TypeHintDeserializer::<LIST_ID>).ok())
                }
                TypeHintDispatchState::Sent => Ok(None),
                TypeHintDispatchState::Waiting => unreachable!(),
            }
        }

        if self.remaining == 0 {
            // If this method gets called again, we'll deserialize a type hint
            if self.dispatch_state == TypeHintDispatchState::Waiting {
                self.dispatch_state = TypeHintDispatchState::Ready;
                return Ok(None);
            } else {
                return handle_hint_dispatch::<_, LIST_ID>(&mut self.dispatch_state, seed);
            }
        }

        self.remaining -= 1;
        seed.deserialize(&mut self.inner).map(Some)
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        if TAG_ID == 0 {
            Some(0)
        } else {
            Some(self.remaining)
        }
    }
}

#[derive(PartialEq, Eq)]
enum TypeHintDispatchState {
    Waiting,
    Ready,
    Sent,
}

struct DeserializeMap<'a, R, B> {
    reader:        &'a mut R,
    opts:          IoOptions,
    current_depth: u32,
    tag_id:        u8,
    _buffered:     PhantomData<B>,
}

impl<'a, 'buffer, R, B> DeserializeMap<'a, R, B>
where
    R: Read,
    B: BufferSpecialization<'buffer>,
{
    #[inline]
    fn new(reader: &'a mut R, opts: IoOptions, current_depth: u32) -> Self {
        DeserializeMap {
            reader,
            opts,
            current_depth,
            tag_id: 0,
            _buffered: PhantomData,
        }
    }

    fn drive_value_visitor<'de, V>(
        &mut self,
        tag_id: u8,
        seed:   V,
    ) -> Result<V::Value, <Self as MapAccess<'de>>::Error>
    where
        'de: 'a,
        'buffer: 'de,
        V: DeserializeSeed<'de>,
    {
        macro_rules! drive_visitor {
            ($($id:literal)*) => {
                match tag_id {
                    $( $id => seed.deserialize(&mut DeserializeTag::<_, B, $id>::new(
                        self.reader, self.opts, self.current_depth + 1
                    )), )*
                    _ => Err(NbtIoError::InvalidTagId(tag_id))
                }
            };
        }

        drive_visitor!(0x1 0x2 0x3 0x4 0x5 0x6 0x7 0x8 0x9 0xA 0xB 0xC)
    }
}

impl<'de, 'a, 'buffer, R, B> MapAccess<'de> for DeserializeMap<'a, R, B>
where
    'de: 'a,
    'buffer: 'de,
    R: Read,
    B: BufferSpecialization<'buffer>,
{
    type Error = NbtIoError;

    #[inline]
    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        self.tag_id = raw::read_u8(self.reader, self.opts)?;

        if self.tag_id == 0 {
            return Ok(None);
        } else if self.current_depth >= self.opts.depth_limit.0 {
            return Err(NbtIoError::ExceededDepthLimit {
                limit: self.opts.depth_limit,
            });
        }

        let mut de = DeserializeTag::<_, B, STRING_ID>::new(
            self.reader,
            self.opts,
            self.current_depth + 1,
        );
        seed.deserialize(&mut de).map(Some)
    }

    #[inline]
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        self.drive_value_visitor(self.tag_id, seed)
    }
}

pub struct DeserializeTag<'a, R, B, const TAG_ID: u8> {
    reader:        &'a mut R,
    opts:          IoOptions,
    current_depth: u32,
    _buffered:     PhantomData<B>,
}

impl<R, B, const TAG_ID: u8> Debug for DeserializeTag<'_, R, B, TAG_ID> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DeserializeTag with TAG_ID {:?} at nesting depth {:?} with opts: {:?}",
            TAG_ID,
            self.current_depth,
            self.opts,
        )
    }
}

impl<'a, 'buffer, R, B, const TAG_ID: u8> DeserializeTag<'a, R, B, TAG_ID>
where
    R: Read,
    B: BufferSpecialization<'buffer>,
{
    #[inline]
    fn new(
        reader:        &'a mut R,
        opts:          IoOptions,
        current_depth: u32,
    ) -> Self {
        DeserializeTag {
            reader,
            opts,
            current_depth,
            _buffered: PhantomData,
        }
    }
}

impl<'de, 'a, 'buffer, R, B, const TAG_ID: u8> de::Deserializer<'de>
    for &mut DeserializeTag<'a, R, B, TAG_ID>
where
    'de: 'a,
    'buffer: 'de,
    R: Read,
    B: BufferSpecialization<'buffer>,
{
    type Error = NbtIoError;

    forward_to_deserialize_any! {
        i8 i16 i32 i64 i128 u16 u32 u64 u128 char f32 f64 string
    }

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match TAG_ID {
            BYTE_ID   => visitor.visit_i8( raw::read_i8( self.reader, self.opts)?),
            SHORT_ID  => visitor.visit_i16(raw::read_i16(self.reader, self.opts)?),
            INT_ID    => visitor.visit_i32(raw::read_i32(self.reader, self.opts)?),
            LONG_ID   => visitor.visit_i64(raw::read_i64(self.reader, self.opts)?),
            FLOAT_ID  => visitor.visit_f32(raw::read_f32(self.reader, self.opts)?),
            DOUBLE_ID => visitor.visit_f64(raw::read_f64(self.reader, self.opts)?),
            BYTE_ARRAY_ID => {
                let len = raw::read_i32_as_usize(self.reader, self.opts)?;
                visitor.visit_seq(DeserializeSeq::<_, _, BYTE_ID, BYTE_ARRAY_ID>::new(
                    DeserializeTag::<_, B, BYTE_ID>::new(
                        self.reader,
                        self.opts,
                        self.current_depth,
                    ),
                    len,
                ))
            }
            STRING_ID => visitor.visit_string(raw::read_string(self.reader, self.opts)?),
            LIST_ID => drive_visitor_seq_tag::<_, _, B>(
                self.reader,
                self.opts,
                self.current_depth,
                visitor,
            ),
            COMPOUND_ID => visitor.visit_map(DeserializeMap::<_, B>::new(
                self.reader,
                self.opts,
                self.current_depth,
            )),
            INT_ARRAY_ID => {
                let len = raw::read_i32_as_usize(self.reader, self.opts)?;
                visitor.visit_seq(DeserializeSeq::<_, _, INT_ID, INT_ARRAY_ID>::new(
                    DeserializeTag::<_, B, INT_ID>::new(
                        self.reader,
                        self.opts,
                        self.current_depth,
                    ),
                    len,
                ))
            }
            LONG_ARRAY_ID => {
                let len = raw::read_i32_as_usize(self.reader, self.opts)?;
                visitor.visit_seq(DeserializeSeq::<_, _, LONG_ID, LONG_ARRAY_ID>::new(
                    DeserializeTag::<_, B, LONG_ID>::new(
                        self.reader,
                        self.opts,
                        self.current_depth,
                    ),
                    len,
                ))
            }
            _ => unreachable!(),
        }
    }

    #[inline]
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if TAG_ID == BYTE_ID {
            visitor.visit_bool(raw::read_bool(self.reader, self.opts)?)
        } else {
            self.deserialize_any(visitor)
        }
    }

    #[inline]
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if TAG_ID == BYTE_ID {
            visitor.visit_u8(raw::read_u8(self.reader, self.opts)?)
        } else {
            self.deserialize_any(visitor)
        }
    }

    #[inline]
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if TAG_ID == BYTE_ARRAY_ID {
            let len = raw::read_i32_as_usize(self.reader, self.opts)?;
            let mut array = vec![0_u8; len];
            self.reader.read_exact(&mut array)?;
            visitor.visit_byte_buf(array)
        } else {
            self.deserialize_any(visitor)
        }
    }

    #[inline]
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if TAG_ID == BYTE_ARRAY_ID {
            let len = raw::read_i32_as_usize(self.reader, self.opts)?;

            if B::BUFFERED {
                // Safety: R is `&'a mut Cursor<&'buffer [u8]>` and `B` is
                // `BufferedCursor<'buffer>` by the constructor `Deserializer::from_cursor`
                visitor.visit_borrowed_bytes(unsafe { B::read_bytes(self.reader, len) }?)
            } else {
                let mut array = vec![0_u8; len];
                self.reader.read_exact(&mut array)?;
                visitor.visit_bytes(&array)
            }
        } else {
            self.deserialize_any(visitor)
        }
    }

    #[inline]
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if TAG_ID == STRING_ID {
            if B::BUFFERED {
                let len = raw::read_string_len(self.reader, self.opts)?;
                // Safety: R is `&'a mut Cursor<&'buffer [u8]>` and `B` is
                // `BufferedCursor<'buffer>` by the constructor `Deserializer::from_cursor`
                let bytes: &'de [u8] = unsafe { B::read_bytes(self.reader, len) }?;

                let string = raw::string_from_bytes(bytes, self.opts)?;

                match string {
                    Cow::Borrowed(string) => visitor.visit_borrowed_str(string),
                    Cow::Owned(string) => visitor.visit_string(string),
                }
            } else {
                let mut dest = Vec::new();
                match raw::read_string_into(self.reader, self.opts, &mut dest)? {
                    Cow::Borrowed(string) => visitor.visit_str(string),
                    Cow::Owned(string) => visitor.visit_string(string),
                }
            }
        } else {
            self.deserialize_any(visitor)
        }
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    #[inline]
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    #[inline]
    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    #[inline]
    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    #[inline]
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        drive_visitor_seq_const::<_, _, B, TAG_ID>(
            self.reader,
            self.opts,
            self.current_depth,
            visitor,
        )
    }

    #[inline]
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(DeserializeMap::<_, B>::new(
            self.reader,
            self.opts,
            self.current_depth,
        ))
    }

    #[inline]
    fn deserialize_struct<V>(
        self,
        _name:   &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    #[inline]
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    #[inline]
    fn deserialize_tuple_struct<V>(
        self,
        _name:   &'static str,
        _len:    usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name:    &'static str,
        variants: &'static [&'static str],
        visitor:  V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match TAG_ID {
            // Unit variant
            BYTE_ID => visitor.visit_enum(
                variants
                    .get(
                        #[expect(
                            clippy::map_err_ignore,
                            reason = "the only possible error ignored is that the i8 is negative",
                        )]
                        usize::try_from(raw::read_i8(self.reader, self.opts)?)
                            .map_err(|_| NbtIoError::NegativeLength)?,
                    )
                    .ok_or(NbtIoError::InvalidEnumVariant)?
                    .into_deserializer(),
            ),
            SHORT_ID => visitor.visit_enum(
                variants
                    .get(
                        #[expect(
                            clippy::map_err_ignore,
                            reason = "the only possible error ignored is that the i16 is negative",
                        )]
                        usize::try_from(raw::read_i16(self.reader, self.opts)?)
                            .map_err(|_| NbtIoError::NegativeLength)?,
                    )
                    .ok_or(NbtIoError::InvalidEnumVariant)?
                    .into_deserializer(),
            ),
            INT_ID => visitor.visit_enum(
                variants
                    .get(raw::read_i32_as_usize(self.reader, self.opts)?)
                    .ok_or(NbtIoError::InvalidEnumVariant)?
                    .into_deserializer(),
            ),
            STRING_ID => {
                let mut dest = Vec::new();
                visitor.visit_enum(
                    raw::read_string_into(self.reader, self.opts, &mut dest)?.into_deserializer(),
                )
            }
            // Newtype, tuple, and struct variants
            COMPOUND_ID => {
                let id = raw::read_u8(self.reader, self.opts)?;
                let mut buf = Vec::new();
                let variant = raw::read_string_into(self.reader, self.opts, &mut buf)?;

                macro_rules! drive_visitor {
                    ($($id:literal)*) => {
                        match id {
                            $( $id => visitor.visit_enum(DeserializeEnum::<_, B, $id>::new(
                                self.reader, self.opts, self.current_depth, variant
                            )), )*
                            _ => Err(NbtIoError::InvalidTagId(id))
                        }
                    };
                }

                let result = drive_visitor!(0x1 0x2 0x3 0x4 0x5 0x6 0x7 0x8 0x9 0xA 0xB 0xC)?;

                let end = raw::read_u8(self.reader, self.opts)?;
                if end != TAG_END_ID {
                    return Err(NbtIoError::TagTypeMismatch {
                        expected: TAG_END_ID,
                        found:    end,
                    });
                }

                Ok(result)
            }
            _ => Err(NbtIoError::ExpectedEnum),
        }
    }

    #[inline]
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    #[inline]
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct TypeHintDeserializer<const TAG_ID: u8>;

impl<'de, const TAG_ID: u8> de::Deserializer<'de> for TypeHintDeserializer<TAG_ID> {
    type Error = TypeHintDeserializerError;

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(TypeHintDeserializerError)
    }

    #[inline]
    fn deserialize_newtype_struct<V>(
        self,
        name:    &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if name == TYPE_HINT_NICHE {
            visitor.visit_u8(TAG_ID)
        } else {
            Err(TypeHintDeserializerError)
        }
    }
}

#[derive(Debug)]
pub(super) struct TypeHintDeserializerError;

impl Display for TypeHintDeserializerError {
    fn fmt(&self, _f: &mut Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl Error for TypeHintDeserializerError {}

impl de::Error for TypeHintDeserializerError {
    fn custom<T>(_msg: T) -> Self
    where
        T: Display,
    {
        Self
    }
}

/// A trait to implement specialization - sort of.
/// # Safety
/// This trait is used to unsafely get a byte slice from a reader.
/// Whatever invariants the reader has must be upheld.
pub unsafe trait BufferSpecialization<'buffer> {
    const BUFFERED: bool;

    // Safety:
    // In order to avoid unsoundness and memory unsafety, we instead panic by default.
    unsafe fn read_bytes<'de, R>(_reader: &mut R, _len: usize) -> Result<&'de [u8], io::Error>
    where
        'buffer: 'de,
    {
        panic!("read_bytes called on a non-buffered reader")
    }
}

#[derive(Debug)]
pub(super) struct Unbuffered;

// SAFETY:
// This uses the default impl of read_bytes, which panics instead of being unsound.
unsafe impl BufferSpecialization<'static> for Unbuffered {
    const BUFFERED: bool = false;
}

#[derive(Debug)]
pub(super) struct BufferedCursor<'buffer> {
    // We are essentially a function which takes a slice and returns a sub-slice, so we need to
    // act like that
    _phantom: PhantomData<fn(&'buffer [u8])>,
}

// SAFETY:
// It is assumed that `R` is `Cursor<&'buffer [u8]>, in which case the safety
// comment for the below unsafe block is satisfied.
unsafe impl<'buffer> BufferSpecialization<'buffer> for BufferedCursor<'buffer> {
    const BUFFERED: bool = true;

    /// Extracts a reference to a slice of bytes out of the given reader.
    ///
    /// # Safety
    ///
    /// The caller must assert that `R` is `Cursor<&'buffer [u8]>`, otherwise unconscionable
    /// amounts of UB will ensue.
    unsafe fn read_bytes<'de, R>(reader: &mut R, len: usize) -> Result<&'de [u8], io::Error>
    where
        'buffer: 'de,
    {
        let reader_ptr = ptr::from_mut::<R>(reader).cast::<Cursor<&'buffer [u8]>>();
        // SAFETY:
        // `R` is assumed to be `Cursor<&'buffer [u8]>`,
        // and thus an `&mut R` reference, converted to a pointer, satisfies:
        // * `reader_ptr` is properly aligned for a `Cursor<&'buffer [u8]>` value
        // * `reader_ptr` is non-null
        // * `reader_ptr` points to a valid value of type `R` a.k.a. `Cursor<&'buffer [u8]>`
        // * we inherit the lifetime of the provided `reader: &mut R` reference,
        //   so aliasing requirements are met.
        let cursor = unsafe { &mut *reader_ptr };
        read_bytes_from_cursor(cursor, len)
    }
}

fn read_bytes_from_cursor<'de, 'a: 'de>(
    cursor: &mut Cursor<&'a [u8]>,
    len:    usize,
) -> Result<&'de [u8], io::Error> {
    let position = cursor.position() as usize;
    let total_len = cursor.get_ref().len();
    let remaining = total_len.saturating_sub(position);

    if len > remaining {
        return Err(io::Error::new(
            ErrorKind::UnexpectedEof,
            format!("Read of {len} bytes requested but only {remaining} remain"),
        ));
    }

    cursor.set_position(u64::try_from(position + len).expect("Cursor position overflowed"));

    let inner: &'a [u8] = cursor.get_ref();
    Ok(&inner[position..position + len])
}
