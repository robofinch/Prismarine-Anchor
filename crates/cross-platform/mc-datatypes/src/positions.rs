use std::ops::{Index, IndexMut};

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};


/// The location of a chunk in a dimension of a world.
///
/// Note that this is not the block position;
/// multiply this position by 16 to find the positions of its blocks. For example
/// `ChunkPosition { x: 1, z: 2 }` refers to the chunk from `(16, 32)` to `(31, 47)`.
#[cfg_attr(feature = "derive_serde",    derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct ChunkPosition {
    pub x: i32,
    pub z: i32,
}

#[cfg_attr(feature = "derive_serde",    derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct BlockPosition {
    pub x: i32,
    pub y: i16,
    pub z: i32,
}

#[cfg_attr(feature = "derive_serde",    derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq))]
#[derive(Debug, Clone, Copy)]
pub struct FloatingWorldPos {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// An X-Z column within a chunk. The `x` and `z` coordinates are each limited to 4 bits.
/// Since `u8` is the smallest standard integer type, encapsulation is provided to enforce
/// this limit.
#[cfg_attr(feature = "derive_serde",    derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct ChunkColumn(u8);

impl ChunkColumn {
    #[inline]
    pub fn new(x: u8, z: u8) -> Option<Self> {
        if x < 16 && z < 16 {
            Some(Self(x << 4 | z))
        } else {
            None
        }
    }

    #[inline]
    pub fn x(self) -> u8 {
        self.0 >> 4
    }

    #[inline]
    pub fn z(self) -> u8 {
        self.0 & 0b1111
    }

    #[inline]
    pub fn xz(self) -> (u8, u8) {
        (self.x(), self.z())
    }
}

impl<T> Index<ChunkColumn> for [[T; 16]; 16] {
    type Output = T;

    /// Index into an array of the form `[[T; 16]; 16]`, where the inner array is indexed
    /// by Z values and the outer array is indexed by X values,
    /// such that the correct indexing order is `biome_ids[X][Z]`.
    #[inline]
    fn index(&self, index: ChunkColumn) -> &Self::Output {
        &self[usize::from(index.x())][usize::from(index.z())]
    }
}

impl<T> IndexMut<ChunkColumn> for [[T; 16]; 16] {
    /// Index into an array of the form `[[T; 16]; 16]`, where the inner array is indexed
    /// by Z values and the outer array is indexed by X values,
    /// such that the correct indexing order is `biome_ids[X][Z]`.
    #[inline]
    fn index_mut(&mut self, index: ChunkColumn) -> &mut Self::Output {
        &mut self[usize::from(index.x())][usize::from(index.z())]
    }
}

#[cfg_attr(feature = "derive_serde",    derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct BlockPosInChunk {
    pub column: ChunkColumn,
    pub height: i16,
}

impl BlockPosInChunk {
    #[inline]
    pub fn from_subchunk_pos(subchunk_y: i8, pos_in_subchunk: BlockPosInSubchunk) -> Self {
        let height = i16::from(subchunk_y) * 16 + i16::from(pos_in_subchunk.y());
        Self {
            column: pos_in_subchunk.column(),
            height,
        }
    }

    #[inline]
    pub fn to_subchunk_pos(self) -> Option<(i8, BlockPosInSubchunk)> {
        let subchunk_y = i8::try_from(self.height / 16).ok()?;
        let y_in_subchunk = (self.height % 16) as u8;
        let subchunk_pos = BlockPosInSubchunk::from_column(
            y_in_subchunk,
            self.column,
        )?;

        Some((subchunk_y, subchunk_pos))
    }
}

/// An X-Y-Z position within a subchunk. Each coordinate is limited to 4 bits.
/// Since `u8` is the smallest standard integer type, encapsulation is provided to enforce
/// this limit.
#[cfg_attr(feature = "derive_serde",    derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct BlockPosInSubchunk(u16);

impl BlockPosInSubchunk {
    #[inline]
    pub fn new(x: u8, y: u8, z: u8) -> Option<Self> {
        let column = ChunkColumn::new(x, z)?;
        Self::from_column(y, column)
    }

    #[inline]
    pub fn from_column(y: u8, column: ChunkColumn) -> Option<Self> {
        if y < 16 {
            // Using the internal value of the column
            Some(Self(u16::from(y) << 8 | u16::from(column.0)))
        } else {
            None
        }
    }

    #[inline]
    pub fn column(self) -> ChunkColumn {
        // Truncates to bottom 8 bits, which store a column
        ChunkColumn(self.0 as u8)
    }

    #[inline]
    pub fn x(self) -> u8 {
        self.column().x()
    }

    #[inline]
    pub fn y(self) -> u8 {
        // The `& 0b1111` isn't actually needed, but it should more firmly assert to the compiler
        // that this is actually four bits, for length-check optimization purposes and whatnot.
        ((self.0 >> 8) & 0b1111) as u8
    }

    #[inline]
    pub fn z(self) -> u8 {
        self.column().z()
    }

    #[inline]
    pub fn xyz(self) -> (u8, u8, u8) {
        (self.x(), self.y(), self.z())
    }
}

impl<T> Index<BlockPosInSubchunk> for [[[T; 16]; 16]; 16] {
    type Output = T;

    /// Index into an array of the form `[[[T; 16]; 16]; 16]`, where the inner array is indexed
    /// by Y values, the middle array by Z values, and the outer array by X values,
    /// such that the correct indexing order is `biome_ids[X][Z][Y]`.
    #[inline]
    fn index(&self, index: BlockPosInSubchunk) -> &Self::Output {
        &self[usize::from(index.x())][usize::from(index.z())][usize::from(index.y())]
    }
}

impl<T> IndexMut<BlockPosInSubchunk> for [[[T; 16]; 16]; 16] {
    /// Index into an array of the form `[[[T; 16]; 16]; 16]`, where the inner array is indexed
    /// by Y values, the middle array by Z values, and the outer array by X values,
    /// such that the correct indexing order is `biome_ids[X][Z][Y]`.
    #[inline]
    fn index_mut(&mut self, index: BlockPosInSubchunk) -> &mut Self::Output {
        &mut self[usize::from(index.x())][usize::from(index.z())][usize::from(index.y())]
    }
}
