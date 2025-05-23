use std::fmt;
use std::marker::PhantomData;
use std::borrow::{Borrow, BorrowMut};

use serde::{Deserialize, Deserializer, Serialize};
use serde::de::{EnumAccess, Error, MapAccess, SeqAccess, Visitor};

use crate::raw;


pub(crate) const BYTE_ARRAY_NICHE:    &str = "b_nbt_array";
pub(crate) const INT_ARRAY_NICHE:     &str = "i_nbt_array";
pub(crate) const LONG_ARRAY_NICHE:    &str = "l_nbt_array";
// TODO: try to figure out some way to support ByteStrings in the serde impl
// pub(crate) const BSTRING_ARRAY_NICHE: &str = "s_nbt_array";
pub(crate) const TYPE_HINT_NICHE:     &str = "__nbt_array_type_hint";


#[expect(clippy::too_long_first_doc_paragraph)]
/// A transparent wrapper around sequential types to allow the NBT serializer to automatically
/// select an appropriate array type, favoring specialized array types like [`IntArray`] and
/// [`ByteArray`]. You can construct an array using `Array::from`.
///
/// Currently this type can only wrap vectors, slices, and arrays, however homogenous tuples may
/// be supported in the future.
///
/// [`IntArray`]: crate::tag::NbtTag::IntArray
/// [`ByteArray`]: crate::tag::NbtTag::ByteArray
// TODO: consider supporting homogenous tuples
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Array<T>(T);

impl<T> Array<T> {
    /// Returns the inner value wrapped by this type.
    #[inline]
    pub fn into_inner(array: Self) -> T {
        array.0
    }
}

impl<T: Serialize + ArrayNiche> Serialize for Array<T> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_newtype_struct(T::NICHE, self.0.as_ser_repr())
    }
}

impl<'de, T: Deserialize<'de> + ArrayNiche> Deserialize<'de> for Array<T> {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<T>(PhantomData<T>);

        impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Visitor<T> {
            type Value = Array<T>;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "A newtype struct type")
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                Ok(Array(Deserialize::deserialize(deserializer)?))
            }
        }

        deserializer.deserialize_newtype_struct(T::NICHE, Visitor(PhantomData))
    }
}

impl<T> AsRef<T> for Array<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> AsMut<T> for Array<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> Borrow<T> for Array<T> {
    #[inline]
    fn borrow(&self) -> &T {
        &self.0
    }
}

impl<T> BorrowMut<T> for Array<T> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> From<T> for Array<T>
where
    T: ArrayNiche,
{
    #[inline]
    fn from(array: T) -> Self {
        Self(array)
    }
}

pub trait ArrayNiche {
    type SerRepr: ?Sized + Serialize;
    const NICHE: &'static str;

    fn as_ser_repr(&self) -> &Self::SerRepr;
}

impl<T> ArrayNiche for &T
where
    T: ArrayNiche + ?Sized,
{
    type SerRepr = T::SerRepr;

    const NICHE: &'static str = T::NICHE;

    #[inline]
    fn as_ser_repr(&self) -> &Self::SerRepr {
        T::as_ser_repr(self)
    }
}

impl<T> ArrayNiche for &mut T
where
    T: ArrayNiche + ?Sized,
{
    type SerRepr = T::SerRepr;

    const NICHE: &'static str = T::NICHE;

    #[inline]
    fn as_ser_repr(&self) -> &Self::SerRepr {
        T::as_ser_repr(self)
    }
}

impl ArrayNiche for Vec<i8> {
    type SerRepr = [u8];

    const NICHE: &'static str = BYTE_ARRAY_NICHE;

    #[inline]
    fn as_ser_repr(&self) -> &Self::SerRepr {
        raw::cast_bytes_to_unsigned(self.as_slice())
    }
}

impl ArrayNiche for Vec<u8> {
    type SerRepr = [u8];

    const NICHE: &'static str = BYTE_ARRAY_NICHE;

    #[inline]
    fn as_ser_repr(&self) -> &Self::SerRepr {
        self.as_slice()
    }
}

impl ArrayNiche for Vec<i32> {
    type SerRepr = [i32];

    const NICHE: &'static str = INT_ARRAY_NICHE;

    #[inline]
    fn as_ser_repr(&self) -> &Self::SerRepr {
        self.as_slice()
    }
}

impl ArrayNiche for Vec<i64> {
    type SerRepr = [i64];

    const NICHE: &'static str = LONG_ARRAY_NICHE;

    #[inline]
    fn as_ser_repr(&self) -> &Self::SerRepr {
        self.as_slice()
    }
}

impl ArrayNiche for [i8] {
    type SerRepr = [u8];

    const NICHE: &'static str = BYTE_ARRAY_NICHE;

    #[inline]
    fn as_ser_repr(&self) -> &Self::SerRepr {
        raw::cast_bytes_to_unsigned(self)
    }
}

impl ArrayNiche for [u8] {
    type SerRepr = [u8];

    const NICHE: &'static str = BYTE_ARRAY_NICHE;

    #[inline]
    fn as_ser_repr(&self) -> &Self::SerRepr {
        self
    }
}

impl ArrayNiche for [i32] {
    type SerRepr = [i32];

    const NICHE: &'static str = INT_ARRAY_NICHE;

    #[inline]
    fn as_ser_repr(&self) -> &Self::SerRepr {
        self
    }
}

impl ArrayNiche for [i64] {
    type SerRepr = [i64];

    const NICHE: &'static str = LONG_ARRAY_NICHE;

    #[inline]
    fn as_ser_repr(&self) -> &Self::SerRepr {
        self
    }
}

impl<const N: usize> ArrayNiche for [i8; N] {
    type SerRepr = [u8];

    const NICHE: &'static str = BYTE_ARRAY_NICHE;

    #[inline]
    fn as_ser_repr(&self) -> &Self::SerRepr {
        let slice: &[i8] = self;
        raw::cast_bytes_to_unsigned(slice)
    }
}

impl<const N: usize> ArrayNiche for [u8; N] {
    type SerRepr = [u8];

    const NICHE: &'static str = BYTE_ARRAY_NICHE;

    #[inline]
    fn as_ser_repr(&self) -> &Self::SerRepr {
        self
    }
}

impl<const N: usize> ArrayNiche for [i32; N] {
    type SerRepr = [i32];

    const NICHE: &'static str = INT_ARRAY_NICHE;

    #[inline]
    fn as_ser_repr(&self) -> &Self::SerRepr {
        self
    }
}

impl<const N: usize> ArrayNiche for [i64; N] {
    type SerRepr = [i64];

    const NICHE: &'static str = LONG_ARRAY_NICHE;

    #[inline]
    fn as_ser_repr(&self) -> &Self::SerRepr {
        self
    }
}

pub(crate) struct TypeHint {
    pub hint: Option<u8>,
}

impl<'de> Deserialize<'de> for TypeHint {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            hint: deserializer.deserialize_newtype_struct(TYPE_HINT_NICHE, TypeHintVisitor)?,
        })
    }
}

struct TypeHintVisitor;

impl<'de> Visitor<'de> for TypeHintVisitor {
    type Value = Option<u8>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "A prismarine-anchor-nbt array type hint")
    }

    #[inline]
    fn visit_bool<E: Error>(self, _v: bool) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_i8<E: Error>(self, _v: i8) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_i16<E: Error>(self, _v: i16) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_i32<E: Error>(self, _v: i32) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_i64<E: Error>(self, _v: i64) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_i128<E: Error>(self, _v: i128) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_u8<E: Error>(self, v: u8) -> Result<Self::Value, E> {
        Ok(Some(v))
    }

    #[inline]
    fn visit_u16<E: Error>(self, _v: u16) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_u32<E: Error>(self, _v: u32) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_u64<E: Error>(self, _v: u64) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_u128<E: Error>(self, _v: u128) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_f32<E: Error>(self, _v: f32) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_f64<E: Error>(self, _v: f64) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_char<E: Error>(self, _v: char) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_str<E: Error>(self, _v: &str) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_borrowed_str<E: Error>(self, _v: &'de str) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_string<E: Error>(self, _v: String) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_bytes<E: Error>(self, _v: &[u8]) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_borrowed_bytes<E: Error>(self, _v: &'de [u8]) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_byte_buf<E: Error>(self, _v: Vec<u8>) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_none<E: Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_some<D>(self, _deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(None)
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(None)
    }

    #[inline]
    fn visit_newtype_struct<D>(self, _deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(None)
    }

    #[inline]
    fn visit_seq<A>(self, _seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        Ok(None)
    }

    #[inline]
    fn visit_map<A>(self, _map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        Ok(None)
    }

    #[inline]
    fn visit_enum<A>(self, _data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        Ok(None)
    }
}
