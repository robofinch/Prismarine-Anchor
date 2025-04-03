use std::cmp::Ordering;

use super::{NbtCompound, NbtList, NbtTag};


#[derive(Debug, Clone)]
pub enum ComparableNbtTag {
    /// A signed, one-byte integer.
    Byte(i8),
    /// A signed, two-byte integer.
    Short(i16),
    /// A signed, four-byte integer.
    Int(i32),
    /// A signed, eight-byte integer.
    Long(i64),
    /// A 32-bit floating point value.
    Float {
        value:   f32,
        epsilon: f32,
    },
    /// A 64-bit floating point value.
    Double {
        value:   f64,
        epsilon: f64,
    },
    /// An array (vec) of one-byte integers. Minecraft treats this as an array of signed bytes.
    ByteArray(Vec<i8>),
    /// A UTF-8 string.
    String(String),
    /// An NBT tag list.
    List(NbtList, f64),
    /// An NBT tag compound.
    Compound(NbtCompound, f64),
    /// An array (vec) of signed, four-byte integers.
    IntArray(Vec<i32>),
    /// An array (vec) of signed, eight-byte integers.
    LongArray(Vec<i64>),
}

impl ComparableNbtTag {
    pub fn new(tag: NbtTag, epsilon: f64) -> Self {
        match tag {
            NbtTag::Byte(n)            => Self::Byte(n),
            NbtTag::Short(n)           => Self::Short(n),
            NbtTag::Int(n)             => Self::Int(n),
            NbtTag::Long(n)            => Self::Long(n),
            NbtTag::Float(value)       => Self::Float { value, epsilon: epsilon as f32 },
            NbtTag::Double(value)      => Self::Double { value, epsilon },
            NbtTag::ByteArray(arr)     => Self::ByteArray(arr),
            NbtTag::String(string)     => Self::String(string),
            NbtTag::List(list)         => Self::List(list, epsilon),
            NbtTag::Compound(compound) => Self::Compound(compound, epsilon),
            NbtTag::IntArray(arr)      => Self::IntArray(arr),
            NbtTag::LongArray(arr)     => Self::LongArray(arr),
        }
    }

    pub fn approx_equal(&self, other: &ComparableNbtTag) -> bool {
        todo!()
    }

    pub fn approx_equal_to_tag(&self, other: &NbtTag) -> bool {
        todo!()
    }

    // range_min

    // range_max
}

impl From<ComparableNbtTag> for NbtTag {
    fn from(tag: ComparableNbtTag) -> Self {
        match tag {
            ComparableNbtTag::Byte(n)               => NbtTag::Byte(n),
            ComparableNbtTag::Short(n)              => NbtTag::Short(n),
            ComparableNbtTag::Int(n)                => NbtTag::Int(n),
            ComparableNbtTag::Long(n)               => NbtTag::Long(n),
            ComparableNbtTag::Float { value, .. }   => NbtTag::Float(value),
            ComparableNbtTag::Double { value, .. }  => NbtTag::Double(value),
            ComparableNbtTag::ByteArray(arr)        => NbtTag::ByteArray(arr),
            ComparableNbtTag::String(string)        => NbtTag::String(string),
            ComparableNbtTag::List(list, _)         => NbtTag::List(list),
            ComparableNbtTag::Compound(compound, _) => NbtTag::Compound(compound),
            ComparableNbtTag::IntArray(arr)         => NbtTag::IntArray(arr),
            ComparableNbtTag::LongArray(arr)        => NbtTag::LongArray(arr),
        }
    }
}

impl PartialEq for ComparableNbtTag {
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}

impl Eq for ComparableNbtTag {}

impl PartialOrd for ComparableNbtTag {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ComparableNbtTag {
    fn cmp(&self, other: &Self) -> Ordering {
        todo!()
    }
}
