use std::fmt;
use std::fmt::Formatter;

use serde::de;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{MapAccess, Visitor};

use crate::raw;
use crate::{
    raw::{BYTE_ARRAY_ID, INT_ARRAY_ID, LIST_ID, LONG_ARRAY_ID},
    serde::{Array, TypeHint},
};
use super::{Map, NbtCompound, NbtList, NbtTag};


impl Serialize for NbtTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            &Self::Byte(value)       => serializer.serialize_i8(value),
            &Self::Short(value)      => serializer.serialize_i16(value),
            &Self::Int(value)        => serializer.serialize_i32(value),
            &Self::Long(value)       => serializer.serialize_i64(value),
            &Self::Float(value)      => serializer.serialize_f32(value),
            &Self::Double(value)     => serializer.serialize_f64(value),
            Self::ByteArray(array)   => Array::from(array).serialize(serializer),
            Self::ByteString(array)  => Array::from(array).serialize(serializer),
            Self::String(value)      => serializer.serialize_str(value),
            Self::List(list)         => list.serialize(serializer),
            Self::Compound(compound) => compound.serialize(serializer),
            Self::IntArray(array)    => Array::from(array).serialize(serializer),
            Self::LongArray(array)   => Array::from(array).serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for NbtTag {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
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
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
        Ok(NbtTag::Byte(if v { 1 } else { 0 }))
    }

    #[inline]
    fn visit_i8<E: de::Error>(self, v: i8) -> Result<Self::Value, E> {
        Ok(NbtTag::Byte(v))
    }

    #[inline]
    fn visit_u8<E: de::Error>(self, v: u8) -> Result<Self::Value, E> {
        Ok(NbtTag::Byte(v as i8))
    }

    #[inline]
    fn visit_i16<E: de::Error>(self, v: i16) -> Result<Self::Value, E> {
        Ok(NbtTag::Short(v))
    }

    #[inline]
    fn visit_i32<E: de::Error>(self, v: i32) -> Result<Self::Value, E> {
        Ok(NbtTag::Int(v))
    }

    #[inline]
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(NbtTag::Long(v))
    }

    #[inline]
    fn visit_f32<E: de::Error>(self, v: f32) -> Result<Self::Value, E> {
        Ok(NbtTag::Float(v))
    }

    #[inline]
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        Ok(NbtTag::Double(v))
    }

    #[inline]
    fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
        self.visit_byte_buf(v.to_owned())
    }

    #[inline]
    fn visit_byte_buf<E: de::Error>(self, v: Vec<u8>) -> Result<Self::Value, E> {
        Ok(NbtTag::ByteArray(raw::cast_byte_buf_to_signed(v)))
    }

    #[inline]
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(NbtTag::String(v.to_owned()))
    }

    #[inline]
    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        Ok(NbtTag::String(v))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut dest = match map.size_hint() {
            Some(hint) => Map::with_capacity(hint),
            None       => Map::new(),
        };
        while let Some((key, tag)) = map.next_entry::<String, NbtTag>()? {
            dest.insert(key, tag);
        }
        Ok(NbtTag::Compound(NbtCompound(dest)))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
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
                    Self::Byte(list)    => NbtTag::ByteArray(list),
                    Self::Int(list)     => NbtTag::IntArray(list),
                    Self::Long(list)    => NbtTag::LongArray(list),
                    Self::Tag(list)     => NbtTag::List(NbtList(list)),
                    Self::Indeterminate => NbtTag::List(NbtList::new()),
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
                (NbtTag::Int(value),  ArbitraryList::Int(list))  => list.push(value),
                (NbtTag::Long(value), ArbitraryList::Long(list)) => list.push(value),
                (tag,                 ArbitraryList::Tag(list))  => list.push(tag),
                (tag, list @ ArbitraryList::Indeterminate) => {
                    let size = seq.size_hint();
                    match tag {
                        NbtTag::Byte(value) => {
                            *list = ArbitraryList::Byte(init_vec(value, size));
                        }
                        NbtTag::Int(value) => {
                            *list = ArbitraryList::Int(init_vec(value, size));
                        }
                        NbtTag::Long(value) => {
                            *list = ArbitraryList::Long(init_vec(value, size));
                        }
                        tag => {
                            *list = ArbitraryList::Tag(init_vec(tag, size));
                        }
                    }
                }
                _ => {
                    return Err(de::Error::custom(
                        "tag type mismatch when deserializing array",
                    ));
                }
            }
        }

        match seq.next_element::<TypeHint>() {
            Ok(Some(TypeHint { hint: Some(tag_id) })) => match (list, tag_id) {
                (ArbitraryList::Byte(list), LIST_ID) => {
                    Ok(NbtTag::List(NbtList(
                        list.into_iter().map(Into::into).collect(),
                    )))
                }
                (ArbitraryList::Int(list), LIST_ID) => {
                    Ok(NbtTag::List(NbtList(
                        list.into_iter().map(Into::into).collect(),
                    )))
                }
                (ArbitraryList::Long(list), LIST_ID) => {
                    Ok(NbtTag::List(NbtList(
                        list.into_iter().map(Into::into).collect(),
                    )))
                }
                (ArbitraryList::Indeterminate, BYTE_ARRAY_ID) => {
                    Ok(NbtTag::ByteArray(Vec::new()))
                }
                (ArbitraryList::Indeterminate, INT_ARRAY_ID)  => {
                    Ok(NbtTag::IntArray(Vec::new()))
                }
                (ArbitraryList::Indeterminate, LONG_ARRAY_ID) => {
                    Ok(NbtTag::LongArray(Vec::new()))
                }
                (list, _) => Ok(list.into_tag()),
            },
            _ => Ok(list.into_tag()),
        }
    }
}

impl Serialize for NbtList {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for NbtList {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(Deserialize::deserialize(deserializer)?))
    }
}

impl Serialize for NbtCompound {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for NbtCompound {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(Deserialize::deserialize(deserializer)?))
    }
}
