use std::fmt;
use std::{cell::Cell, marker::PhantomData};
use std::{
    fmt::{Debug, Formatter},
    io::{Cursor, Write},
};

use serde::Serialize;
use serde::ser::{
    Impossible, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant,
    SerializeTuple, SerializeTupleStruct, SerializeTupleVariant,
};

use crate::raw;
use crate::{io::NbtIoError, settings::IoOptions};
use crate::raw::{
    BYTE_ARRAY_ID, BYTE_ID, COMPOUND_ID, DOUBLE_ID, FLOAT_ID, INT_ARRAY_ID,
    INT_ID, LIST_ID, LONG_ID, LONG_ARRAY_ID, SHORT_ID, STRING_ID, TAG_END_ID,
};
use super::{
    array::{BYTE_ARRAY_NICHE, INT_ARRAY_NICHE, LONG_ARRAY_NICHE},
    util::{DefaultSerializer, Ser},
};


// TODO: actually implement DepthLimit checks. I haven't managed to figure out the boundary
// between compound or list tags and their elements, so I don't know where to add checks.
// Some messing around with debug prints should reveal useful info, probably.

// TODO: there are some things here that really, really, really need to be tested.
// I think I trust the io and snbt code. This, not as much.


/// The serializer type for writing binary NBT data.
pub type Serializer<'a, W> = Ser<SerializerImpl<'a, W, Homogenous>>;

/// An alternative serializer type for writing binary NBT data which elides checks for
/// sequence homogeneity. Using this type could result in bogus NBT data.
pub type UncheckedSerializer<'a, W> = Ser<SerializerImpl<'a, W, Unchecked>>;


impl<'a, W: Write> Serializer<'a, W> {
    /// Constructs a new serializer with the given writer, IO options, and root name.
    /// If no root name is specified, then an empty string is written to the header.
    pub fn new(writer: &'a mut W, opts: IoOptions, root_name: Option<&'a str>) -> Self {
        SerializerImpl::new(writer, opts, BorrowedPrefix::new(root_name.unwrap_or("")))
            .into_serializer()
    }
}

impl<'a, W: Write> UncheckedSerializer<'a, W> {
    /// Constructs a new unchecked serializer with the given writer, IO options, and root name.
    /// If no root name is specified, then an empty string is written to the header.
    pub fn new(writer: &'a mut W, opts: IoOptions, root_name: Option<&'a str>) -> Self {
        SerializerImpl::new(writer, opts, BorrowedPrefix::new(root_name.unwrap_or("")))
            .into_serializer()
    }
}

pub struct SerializerImpl<'a, W, C> {
    writer:    &'a mut W,
    opts:      IoOptions,
    root_name: BorrowedPrefix<&'a str>,
    _phantom:  PhantomData<C>,
}

impl<W, C> Debug for SerializerImpl<'_, W, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SerializerImpl with root_name \"{:?}\" and opts: {:?}",
            self.root_name,
            self.opts,
        )
    }
}

impl<'a, W: Write, C: TypeChecker> SerializerImpl<'a, W, C> {
    fn new(writer: &'a mut W, opts: IoOptions, root_name: BorrowedPrefix<&'a str>) -> Self {
        SerializerImpl {
            writer,
            opts,
            root_name,
            _phantom: PhantomData,
        }
    }
}

impl<'a, W: Write, C: TypeChecker> DefaultSerializer for SerializerImpl<'a, W, C> {
    type Error = NbtIoError;
    type Ok    = ();
    type SerializeMap           = SerializeCompound<'a, W, C>;
    type SerializeSeq           = Impossible<Self::Ok, Self::Error>;
    type SerializeStruct        = SerializeCompound<'a, W, C>;
    type SerializeStructVariant = SerializeCompound<'a, W, C>;
    type SerializeTuple         = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct   = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant  = SerializeList<'a, W, C>;

    #[cold]
    fn unimplemented(self, _ty: &'static str) -> Self::Error {
        NbtIoError::MissingRootTag
    }

    #[inline]
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        let mut map = self.serialize_map(Some(1))?;
        map.serialize_entry(variant, value)?;
        SerializeMap::end(map)
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.root_name.write(self.writer, self.opts, COMPOUND_ID)?;
        let prefix = BorrowedPrefix::new(variant);
        SerializeCompoundEntry::new(self.writer, self.opts, 0, prefix)
            .serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.root_name.write(self.writer, self.opts, COMPOUND_ID)?;
        Ok(SerializeCompound::new(self.writer, self.opts, 0))
    }

    #[inline]
    fn serialize_struct(
        self,
        _name: &'static str,
        len:   usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        <Self as DefaultSerializer>::serialize_map(self, Some(len))
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name:          &'static str,
        _variant_index: u32,
        variant:        &'static str,
        _len:           usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.root_name.write(self.writer, self.opts, COMPOUND_ID)?;
        raw::write_u8(self.writer, self.opts, COMPOUND_ID)?;
        raw::write_string(self.writer, self.opts, variant)?;
        // The extra closing tag is added by the SerializeStructVariant impl
        Ok(SerializeCompound::new(self.writer, self.opts, 0))
    }

    #[inline]
    fn is_human_readable(&self) -> bool {
        false
    }
}

struct SerializeArray<'a, W> {
    writer:        &'a mut W,
    opts:          IoOptions,
    current_depth: u32,
}

impl<'a, W> SerializeArray<'a, W>
where
    W: Write,
{
    #[inline]
    fn new(writer: &'a mut W, opts: IoOptions, current_depth: u32) -> Self {
        SerializeArray {
            writer,
            opts,
            current_depth,
        }
    }
}

impl<W> DefaultSerializer for SerializeArray<'_, W>
where
    W: Write,
{
    type Error = NbtIoError;
    type Ok    = ();
    type SerializeMap           = Impossible<Self::Ok, Self::Error>;
    type SerializeSeq           = Self;
    type SerializeStruct        = Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeTuple         = Self::SerializeSeq;
    type SerializeTupleStruct   = Self::SerializeSeq;
    type SerializeTupleVariant  = Impossible<Self::Ok, Self::Error>;

    #[cold]
    fn unimplemented(self, _ty: &'static str) -> Self::Error {
        panic!("Array<T> wrapper incorrectly used on non-sequential type")
    }

    #[inline]
    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        raw::write_usize_as_i32(self.writer, self.opts, value.len())?;
        self.writer.write_all(value)?;
        Ok(())
    }

    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let len = len.ok_or(NbtIoError::MissingLength)?;
        raw::write_usize_as_i32(self.writer, self.opts, len)?;
        Ok(self)
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len:   usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }
}

impl<W> SerializeSeq for SerializeArray<'_, W>
where
    W: Write,
{
    type Error = NbtIoError;
    type Ok = ();

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(
            SerializeListElement::new(
                self.writer,
                self.opts,
                self.current_depth,
                NoPrefix,
                &UNCHECKED,
            )
            .into_serializer(),
        )
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W> SerializeTuple for SerializeArray<'_, W>
where
    W: Write,
{
    type Error = NbtIoError;
    type Ok = ();

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        <Self as SerializeSeq>::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as SerializeSeq>::end(self)
    }
}

impl<W> SerializeTupleStruct for SerializeArray<'_, W>
where
    W: Write,
{
    type Error = NbtIoError;
    type Ok = ();

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        <Self as SerializeSeq>::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as SerializeSeq>::end(self)
    }
}

pub struct SerializeList<'a, W, C> {
    writer:        &'a mut W,
    opts:          IoOptions,
    current_depth: u32,
    length:        Option<i32>,
    type_checker:  C,
}

impl<W, C> Debug for SerializeList<'_, W, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SerializeList with length {:?} at nesting depth {:?} with opts: {:?}",
            self.length,
            self.current_depth,
            self.opts,
        )
    }
}

impl<'a, W, C> SerializeList<'a, W, C>
where
    W: Write,
    C: TypeChecker,
{
    #[expect(
        clippy::unnecessary_wraps,
        reason = "for consistency with similar structs here, for which new() can fail",
    )]
    fn new(
        writer:        &'a mut W,
        opts:          IoOptions,
        current_depth: u32,
        length:        i32,
    ) -> Result<Self, NbtIoError> {
        Ok(SerializeList {
            writer,
            opts,
            current_depth,
            length:        Some(length),
            type_checker:  C::new(),
        })
    }
}

impl<W, C> SerializeSeq for SerializeList<'_, W, C>
where
    W: Write,
    C: TypeChecker,
{
    type Error = NbtIoError;
    type Ok = ();

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        match self.length.take() {
            None => value.serialize(
                SerializeListElement::new(
                    self.writer,
                    self.opts,
                    self.current_depth,
                    NoPrefix,
                    &self.type_checker,
                )
                .into_serializer(),
            ),
            Some(length) => value.serialize(
                SerializeListElement::new(
                    self.writer,
                    self.opts,
                    self.current_depth,
                    LengthPrefix::new(length),
                    &self.type_checker,
                )
                .into_serializer(),
            ),
        }
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        // Empty list
        if self.length.is_some() {
            raw::write_u8(self.writer, self.opts, TAG_END_ID)?;
            raw::write_usize_as_i32(self.writer, self.opts, 0)?;
        }

        Ok(())
    }
}

impl<W, C> SerializeTuple for SerializeList<'_, W, C>
where
    W: Write,
    C: TypeChecker,
{
    type Error = NbtIoError;
    type Ok = ();

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        <Self as SerializeSeq>::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as SerializeSeq>::end(self)
    }
}

impl<W, C> SerializeTupleStruct for SerializeList<'_, W, C>
where
    W: Write,
    C: TypeChecker,
{
    type Error = NbtIoError;
    type Ok = ();

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        <Self as SerializeSeq>::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as SerializeSeq>::end(self)
    }
}

impl<W, C> SerializeTupleVariant for SerializeList<'_, W, C>
where
    W: Write,
    C: TypeChecker,
{
    type Error = NbtIoError;
    type Ok = ();

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        <Self as SerializeSeq>::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        // Empty list
        if self.length.is_some() {
            raw::write_u8(self.writer, self.opts, TAG_END_ID)?;
            raw::write_usize_as_i32(self.writer, self.opts, 0)?;
        }

        // Add a TAG_End because tuple variants are serialized as { name: [data...] }
        raw::write_u8(self.writer, self.opts, TAG_END_ID)?;
        Ok(())
    }
}

struct SerializeListElement<'a, W, P, C> {
    writer:        &'a mut W,
    opts:          IoOptions,
    current_depth: u32,
    prefix:        P,
    type_checker:  &'a C,
}

impl<'a, W, P, C> SerializeListElement<'a, W, P, C>
where
    W: Write,
    P: Prefix,
    C: TypeChecker,
{
    #[inline]
    fn new(
        writer:             &'a mut W,
        opts:               IoOptions,
        current_depth:      u32,
        inner_prefix:       P,
        inner_type_checker: &'a C,
    ) -> Self {
        SerializeListElement {
            writer,
            opts,
            current_depth,
            prefix: inner_prefix,
            type_checker: inner_type_checker,
        }
    }
}

impl<'a, W, P, C> DefaultSerializer for SerializeListElement<'a, W, P, C>
where
    W: Write,
    P: Prefix,
    C: TypeChecker,
{
    type Error = NbtIoError;
    type Ok    = ();
    type SerializeMap           = SerializeCompound<'a, W, C>;
    type SerializeSeq           = SerializeList<'a, W, C>;
    type SerializeStruct        = SerializeCompound<'a, W, C>;
    type SerializeStructVariant = SerializeCompound<'a, W, C>;
    type SerializeTuple         = SerializeList<'a, W, C>;
    type SerializeTupleStruct   = SerializeList<'a, W, C>;
    type SerializeTupleVariant  = SerializeList<'a, W, C>;

    #[cold]
    fn unimplemented(self, ty: &'static str) -> Self::Error {
        NbtIoError::UnsupportedType(ty)
    }

    #[inline]
    fn serialize_bool(self, value: bool) -> Result<Self::Ok, Self::Error> {
        self.type_checker.verify(BYTE_ID)?;
        self.prefix.write(self.writer, self.opts, BYTE_ID)?;
        raw::write_bool(self.writer, self.opts, value)?;
        Ok(())
    }

    #[inline]
    fn serialize_i8(self, value: i8) -> Result<Self::Ok, Self::Error> {
        self.type_checker.verify(BYTE_ID)?;
        self.prefix.write(self.writer, self.opts, BYTE_ID)?;
        raw::write_i8(self.writer, self.opts, value)?;
        Ok(())
    }

    #[inline]
    fn serialize_u8(self, value: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i8(value as i8)
    }

    #[inline]
    fn serialize_i16(self, value: i16) -> Result<Self::Ok, Self::Error> {
        self.type_checker.verify(SHORT_ID)?;
        self.prefix.write(self.writer, self.opts, SHORT_ID)?;
        raw::write_i16(self.writer, self.opts, value)?;
        Ok(())
    }

    #[inline]
    fn serialize_i32(self, value: i32) -> Result<Self::Ok, Self::Error> {
        self.type_checker.verify(INT_ID)?;
        self.prefix.write(self.writer, self.opts, INT_ID)?;
        raw::write_i32(self.writer, self.opts, value)?;
        Ok(())
    }

    #[inline]
    fn serialize_i64(self, value: i64) -> Result<Self::Ok, Self::Error> {
        self.type_checker.verify(LONG_ID)?;
        self.prefix.write(self.writer, self.opts, LONG_ID)?;
        raw::write_i64(self.writer, self.opts, value)?;
        Ok(())
    }

    #[inline]
    fn serialize_f32(self, value: f32) -> Result<Self::Ok, Self::Error> {
        self.type_checker.verify(FLOAT_ID)?;
        self.prefix.write(self.writer, self.opts, FLOAT_ID)?;
        raw::write_f32(self.writer, self.opts, value)?;
        Ok(())
    }

    #[inline]
    fn serialize_f64(self, value: f64) -> Result<Self::Ok, Self::Error> {
        self.type_checker.verify(DOUBLE_ID)?;
        self.prefix.write(self.writer, self.opts, DOUBLE_ID)?;
        raw::write_f64(self.writer, self.opts, value)?;
        Ok(())
    }

    #[inline]
    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        self.type_checker.verify(STRING_ID)?;
        self.prefix.write(self.writer, self.opts, STRING_ID)?;
        raw::write_string(self.writer, self.opts, value)?;
        Ok(())
    }

    #[inline]
    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.type_checker.verify(BYTE_ARRAY_ID)?;
        self.prefix.write(self.writer, self.opts, BYTE_ARRAY_ID)?;
        raw::write_usize_as_i32(self.writer, self.opts, value.len())?;
        self.writer.write_all(value)?;
        Ok(())
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(NbtIoError::OptionInList)
    }

    #[inline]
    fn serialize_some<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        Err(NbtIoError::OptionInList)
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name:         &'static str,
        variant_index: u32,
        _variant:      &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.type_checker.verify(INT_ID)?;
        self.prefix.write(self.writer, self.opts, INT_ID)?;
        raw::write_i32(self.writer, self.opts, variant_index as i32)?;
        Ok(())
    }

    #[inline]
    fn serialize_newtype_struct<T>(
        self,
        name:  &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        match name {
            BYTE_ARRAY_NICHE => {
                self.type_checker.verify(BYTE_ARRAY_ID)?;
                self.prefix.write(self.writer, self.opts, BYTE_ARRAY_ID)?;
            }
            INT_ARRAY_NICHE => {
                self.type_checker.verify(INT_ARRAY_ID)?;
                self.prefix.write(self.writer, self.opts, INT_ARRAY_ID)?;
            }
            LONG_ARRAY_NICHE => {
                self.type_checker.verify(LONG_ARRAY_ID)?;
                self.prefix.write(self.writer, self.opts, LONG_ARRAY_ID)?;
            }
            _ => return value.serialize(self.into_serializer()),
        }
        value.serialize(
            SerializeArray::new(self.writer, self.opts, self.current_depth).into_serializer(),
        )
    }

    #[inline]
    fn serialize_newtype_variant<T>(
        self,
        _name:          &'static str,
        _variant_index: u32,
        variant:        &'static str,
        value:          &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.type_checker.verify(COMPOUND_ID)?;
        self.prefix.write(self.writer, self.opts, COMPOUND_ID)?;
        value.serialize(
            SerializeCompoundEntry::<_, C, _>::new(
                self.writer,
                self.opts,
                self.current_depth,
                BorrowedPrefix::new(variant),
            )
            .into_serializer(),
        )?;
        raw::write_u8(self.writer, self.opts, raw::id_for_tag(None))?;
        Ok(())
    }

    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.type_checker.verify(LIST_ID)?;
        self.prefix.write(self.writer, self.opts, LIST_ID)?;
        let len = len.ok_or(NbtIoError::MissingLength)?;
        #[expect(
            clippy::map_err_ignore,
            reason = "the only possible error ignored is that the usize is too large",
        )]
        let len = i32::try_from(len).map_err(|_| NbtIoError::ExcessiveLength)?;

        SerializeList::new(self.writer, self.opts, self.current_depth, len)
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len:   usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name:          &'static str,
        _variant_index: u32,
        variant:        &'static str,
        len:            usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        // [{name: []}]

        // Check that we're allowed to have compounds in this list
        self.type_checker.verify(COMPOUND_ID)?;
        self.prefix.write(self.writer, self.opts, COMPOUND_ID)?;

        // Write the compound
        let prefix = BorrowedPrefix::new(variant);
        SerializeCompoundEntry::new(self.writer, self.opts, self.current_depth, prefix)
            .serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.type_checker.verify(COMPOUND_ID)?;
        self.prefix.write(self.writer, self.opts, COMPOUND_ID)?;
        Ok(SerializeCompound::new(
            self.writer,
            self.opts,
            self.current_depth,
        ))
    }

    #[inline]
    fn serialize_struct(
        self,
        _name: &'static str,
        len:   usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name:          &'static str,
        _variant_index: u32,
        variant:        &'static str,
        _len:           usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.type_checker.verify(COMPOUND_ID)?;
        self.prefix.write(self.writer, self.opts, COMPOUND_ID)?;
        raw::write_u8(self.writer, self.opts, COMPOUND_ID)?;
        raw::write_string(self.writer, self.opts, variant)?;
        // The extra closing tag is added by the SerializeStructVariant impl
        Ok(SerializeCompound::new(
            self.writer,
            self.opts,
            self.current_depth,
        ))
    }

    #[inline]
    fn is_human_readable(&self) -> bool {
        false
    }
}

pub struct SerializeCompound<'a, W, C> {
    writer:        &'a mut W,
    opts:          IoOptions,
    current_depth: u32,
    key:           Option<Box<[u8]>>,
    _phantom:      PhantomData<C>,
}

impl<W, C> Debug for SerializeCompound<'_, W, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SerializeCompound with key {:?} at nesting depth {:?} with opts: {:?}",
            self.key,
            self.current_depth,
            self.opts,
        )
    }
}

impl<'a, W: Write, C: TypeChecker> SerializeCompound<'a, W, C> {
    #[inline]
    fn new(writer: &'a mut W, opts: IoOptions, current_depth: u32) -> Self {
        SerializeCompound {
            writer,
            opts,
            current_depth,
            key: None,
            _phantom: PhantomData,
        }
    }
}

impl<W: Write, C: TypeChecker> SerializeMap for SerializeCompound<'_, W, C> {
    type Error = NbtIoError;
    type Ok = ();

    #[inline]
    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        let mut cursor = Cursor::new(Vec::new());
        key.serialize(SerializeKey::new(&mut cursor, self.opts).into_serializer())?;
        self.key = Some(cursor.into_inner().into_boxed_slice());
        Ok(())
    }

    #[inline]
    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        let key = self
            .key
            .take()
            .expect("serialize_value called before key was serialized.");
        let prefix = RawPrefix::new(key);
        value.serialize(
            SerializeCompoundEntry::<_, C, _>::new(
                self.writer,
                self.opts,
                self.current_depth,
                prefix,
            )
            .into_serializer(),
        )
    }

    #[inline]
    fn serialize_entry<K, V>(
        &mut self,
        key:   &K,
        value: &V,
    ) -> Result<(), Self::Error>
    where
        K: Serialize + ?Sized,
        V: Serialize + ?Sized,
    {
        let prefix = BorrowedPrefix::new(key);
        value.serialize(
            SerializeCompoundEntry::<_, C, _>::new(
                self.writer,
                self.opts,
                self.current_depth,
                prefix,
            )
            .into_serializer(),
        )
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        raw::write_u8(self.writer, self.opts, raw::id_for_tag(None))?;
        Ok(())
    }
}

impl<W: Write, C: TypeChecker> SerializeStruct for SerializeCompound<'_, W, C> {
    type Error = NbtIoError;
    type Ok = ();

    #[inline]
    fn serialize_field<T>(
        &mut self,
        key:   &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        let prefix = BorrowedPrefix::new(key);
        value.serialize(
            SerializeCompoundEntry::<_, C, _>::new(
                self.writer,
                self.opts,
                self.current_depth,
                prefix,
            )
            .into_serializer(),
        )
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        raw::write_u8(self.writer, self.opts, raw::id_for_tag(None))?;
        Ok(())
    }
}

impl<W: Write, C: TypeChecker> SerializeStructVariant for SerializeCompound<'_, W, C> {
    type Error = NbtIoError;
    type Ok = ();

    #[inline]
    fn serialize_field<T>(
        &mut self,
        key:   &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        <Self as SerializeStruct>::serialize_field(self, key, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        // Add an extra tag because struct variants are serialized as { name: { fields... } }
        // self.writer.write_all(&[0, 0])?; <- not `&[0, 0, 0, 0, 0]` or just `&[0]`?
        // TODO: this case really, really, really needs to be tested
        raw::write_u8(self.writer, self.opts, TAG_END_ID)?;
        raw::write_usize_as_i32(self.writer, self.opts, 0)?; // not sure if this line is needed
        Ok(())
    }
}

struct SerializeCompoundEntry<'a, W, C, P> {
    writer:        &'a mut W,
    opts:          IoOptions,
    current_depth: u32,
    prefix:        P,
    _phantom:      PhantomData<C>,
}

impl<'a, W: Write, C: TypeChecker, P: Prefix> SerializeCompoundEntry<'a, W, C, P> {
    #[inline]
    fn new(writer: &'a mut W, opts: IoOptions, current_depth: u32, prefix: P) -> Self {
        SerializeCompoundEntry {
            writer,
            opts,
            current_depth,
            prefix,
            _phantom: PhantomData,
        }
    }
}

impl<'a, W, C, P> DefaultSerializer for SerializeCompoundEntry<'a, W, C, P>
where
    W: Write,
    C: TypeChecker,
    P: Prefix,
{
    type Error = NbtIoError;
    type Ok    = ();
    type SerializeMap           = SerializeCompound<'a, W, C>;
    type SerializeSeq           = SerializeList<'a, W, C>;
    type SerializeStruct        = SerializeCompound<'a, W, C>;
    type SerializeStructVariant = SerializeCompound<'a, W, C>;
    type SerializeTuple         = SerializeList<'a, W, C>;
    type SerializeTupleStruct   = SerializeList<'a, W, C>;
    type SerializeTupleVariant  = SerializeList<'a, W, C>;

    #[cold]
    fn unimplemented(self, ty: &'static str) -> Self::Error {
        NbtIoError::UnsupportedType(ty)
    }

    #[inline]
    fn serialize_bool(self, value: bool) -> Result<Self::Ok, Self::Error> {
        self.prefix.write(self.writer, self.opts, BYTE_ID)?;
        raw::write_bool(self.writer, self.opts, value)?;
        Ok(())
    }

    #[inline]
    fn serialize_i8(self, value: i8) -> Result<Self::Ok, Self::Error> {
        self.prefix.write(self.writer, self.opts, BYTE_ID)?;
        raw::write_i8(self.writer, self.opts, value)?;
        Ok(())
    }

    #[inline]
    fn serialize_u8(self, value: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i8(value as i8)
    }

    #[inline]
    fn serialize_i16(self, value: i16) -> Result<Self::Ok, Self::Error> {
        self.prefix.write(self.writer, self.opts, SHORT_ID)?;
        raw::write_i16(self.writer, self.opts, value)?;
        Ok(())
    }

    #[inline]
    fn serialize_i32(self, value: i32) -> Result<Self::Ok, Self::Error> {
        self.prefix.write(self.writer, self.opts, INT_ID)?;
        raw::write_i32(self.writer, self.opts, value)?;
        Ok(())
    }

    #[inline]
    fn serialize_i64(self, value: i64) -> Result<Self::Ok, Self::Error> {
        self.prefix.write(self.writer, self.opts, LONG_ID)?;
        raw::write_i64(self.writer, self.opts, value)?;
        Ok(())
    }

    #[inline]
    fn serialize_f32(self, value: f32) -> Result<Self::Ok, Self::Error> {
        self.prefix.write(self.writer, self.opts, FLOAT_ID)?;
        raw::write_f32(self.writer, self.opts, value)?;
        Ok(())
    }

    #[inline]
    fn serialize_f64(self, value: f64) -> Result<Self::Ok, Self::Error> {
        self.prefix.write(self.writer, self.opts, DOUBLE_ID)?;
        raw::write_f64(self.writer, self.opts, value)?;
        Ok(())
    }

    #[inline]
    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        self.prefix.write(self.writer, self.opts, STRING_ID)?;
        raw::write_string(self.writer, self.opts, value)?;
        Ok(())
    }

    #[inline]
    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.prefix.write(self.writer, self.opts, BYTE_ARRAY_ID)?;
        raw::write_usize_as_i32(self.writer, self.opts, value.len())?;
        self.writer.write_all(value)?;
        Ok(())
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    #[inline]
    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self.into_serializer())
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name:         &'static str,
        variant_index: u32,
        _variant:      &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.prefix.write(self.writer, self.opts, INT_ID)?;
        raw::write_i32(self.writer, self.opts, variant_index as i32)?;
        Ok(())
    }

    #[inline]
    fn serialize_newtype_struct<T>(
        self,
        name:  &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        match name {
            BYTE_ARRAY_NICHE => {
                self.prefix.write(self.writer, self.opts, BYTE_ARRAY_ID)?;
            }
            INT_ARRAY_NICHE => {
                self.prefix.write(self.writer, self.opts, INT_ARRAY_ID)?;
            }
            LONG_ARRAY_NICHE => {
                self.prefix.write(self.writer, self.opts, LONG_ARRAY_ID)?;
            }
            _ => return value.serialize(self.into_serializer()),
        }
        value.serialize(
            SerializeArray::new(self.writer, self.opts, self.current_depth).into_serializer(),
        )
    }

    #[inline]
    fn serialize_newtype_variant<T>(
        self,
        _name:          &'static str,
        _variant_index: u32,
        variant:        &'static str,
        value:          &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.prefix.write(self.writer, self.opts, COMPOUND_ID)?;
        value.serialize(
            SerializeCompoundEntry::<_, C, _>::new(
                self.writer,
                self.opts,
                self.current_depth,
                BorrowedPrefix::new(variant),
            )
            .into_serializer(),
        )?;
        raw::write_u8(self.writer, self.opts, raw::id_for_tag(None))?;
        Ok(())
    }

    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.prefix.write(self.writer, self.opts, LIST_ID)?;
        let len = len.ok_or(NbtIoError::MissingLength)?;
        #[expect(
            clippy::map_err_ignore,
            reason = "the only possible error ignored is that the usize is too large",
        )]
        let len = i32::try_from(len).map_err(|_| NbtIoError::ExcessiveLength)?;

        SerializeList::new(self.writer, self.opts, self.current_depth, len)
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len:   usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name:          &'static str,
        _variant_index: u32,
        variant:        &'static str,
        len:            usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.prefix.write(self.writer, self.opts, COMPOUND_ID)?;
        let prefix = BorrowedPrefix::new(variant);
        SerializeCompoundEntry::new(self.writer, self.opts, self.current_depth, prefix)
            .serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.prefix.write(self.writer, self.opts, COMPOUND_ID)?;
        Ok(SerializeCompound::new(
            self.writer,
            self.opts,
            self.current_depth,
        ))
    }

    #[inline]
    fn serialize_struct(
        self,
        _name: &'static str,
        len:   usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name:          &'static str,
        _variant_index: u32,
        variant:        &'static str,
        _len:           usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.prefix.write(self.writer, self.opts, COMPOUND_ID)?;
        raw::write_u8(self.writer, self.opts, COMPOUND_ID)?;
        raw::write_string(self.writer, self.opts, variant)?;
        // The extra closing tag is added by the SerializeStructVariant impl
        Ok(SerializeCompound::new(
            self.writer,
            self.opts,
            self.current_depth,
        ))
    }

    #[inline]
    fn is_human_readable(&self) -> bool {
        false
    }
}

struct SerializeKey<'a, W> {
    writer: &'a mut W,
    opts:   IoOptions,
}

impl<'a, W: Write> SerializeKey<'a, W> {
    #[inline]
    fn new(writer: &'a mut W, opts: IoOptions) -> Self {
        SerializeKey { writer, opts }
    }
}

impl<W: Write> DefaultSerializer for SerializeKey<'_, W> {
    type Error = NbtIoError;
    type Ok    = ();
    type SerializeMap           = Impossible<Self::Ok, Self::Error>;
    type SerializeSeq           = Impossible<Self::Ok, Self::Error>;
    type SerializeStruct        = Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeTuple         = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct   = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant  = Impossible<Self::Ok, Self::Error>;

    #[cold]
    fn unimplemented(self, _ty: &'static str) -> Self::Error {
        NbtIoError::InvalidKey
    }

    #[inline]
    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        raw::write_string(self.writer, self.opts, value)?;
        Ok(())
    }
}

pub trait TypeChecker: Sized {
    fn new() -> Self;

    fn verify(&self, tag_id: u8) -> Result<(), NbtIoError>;
}

#[derive(Debug)]
pub struct Unchecked;

const UNCHECKED: Unchecked = Unchecked;

impl TypeChecker for Unchecked {
    #[inline]
    fn new() -> Self {
        Self
    }

    #[inline]
    fn verify(&self, _tag_id: u8) -> Result<(), NbtIoError> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct Homogenous {
    id: Cell<Option<u8>>,
}

impl TypeChecker for Homogenous {
    #[inline]
    fn new() -> Self {
        Self {
            id: Cell::new(None),
        }
    }

    #[inline]
    fn verify(&self, tag_id: u8) -> Result<(), NbtIoError> {
        if let Some(id) = self.id.get() {
            if id == tag_id {
                Ok(())
            } else {
                Err(NbtIoError::NonHomogenousList {
                    list_type:        id,
                    encountered_type: tag_id,
                })
            }
        } else {
            self.id.set(Some(tag_id));
            Ok(())
        }
    }
}

pub(super) trait Prefix: Sized {
    fn write_raw<W: Write>(self, writer: &mut W, opts: IoOptions) -> Result<(), NbtIoError>;

    #[inline]
    fn write<W: Write>(
        self,
        writer: &mut W,
        opts: IoOptions,
        tag_id: u8,
    ) -> Result<(), NbtIoError> {
        raw::write_u8(writer, opts, tag_id)?;
        self.write_raw(writer, opts)
    }
}

#[derive(Debug)]
pub(super) struct NoPrefix;

impl Prefix for NoPrefix {
    #[inline]
    fn write_raw<W: Write>(self, _writer: &mut W, _opts: IoOptions) -> Result<(), NbtIoError> {
        Ok(())
    }

    #[inline]
    fn write<W: Write>(
        self,
        _writer: &mut W,
        _opts: IoOptions,
        _tag_id: u8,
    ) -> Result<(), NbtIoError> {
        Ok(())
    }
}

struct LengthPrefix {
    length: i32,
}

impl LengthPrefix {
    #[inline]
    fn new(length: i32) -> Self {
        Self { length }
    }
}

impl Prefix for LengthPrefix {
    #[inline]
    fn write_raw<W: Write>(self, writer: &mut W, opts: IoOptions) -> Result<(), NbtIoError> {
        raw::write_i32(writer, opts, self.length)?;
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct BorrowedPrefix<K> {
    key: K,
}

impl<K: Serialize> BorrowedPrefix<K> {
    #[inline]
    fn new(key: K) -> Self {
        Self { key }
    }
}

impl<K: Serialize> Prefix for BorrowedPrefix<K> {
    #[inline]
    fn write_raw<W: Write>(self, writer: &mut W, opts: IoOptions) -> Result<(), NbtIoError> {
        self.key
            .serialize(SerializeKey::new(writer, opts).into_serializer())
    }
}

struct RawPrefix {
    raw: Box<[u8]>,
}

impl RawPrefix {
    #[inline]
    fn new(raw: Box<[u8]>) -> Self {
        Self { raw }
    }
}

impl Prefix for RawPrefix {
    #[inline]
    fn write_raw<W: Write>(self, writer: &mut W, _opts: IoOptions) -> Result<(), NbtIoError> {
        writer.write_all(&self.raw)?;
        Ok(())
    }
}
