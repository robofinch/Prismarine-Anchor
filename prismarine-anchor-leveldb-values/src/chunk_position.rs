#[cfg(feature = "dimensions")]
use crate::dimensions::NumericDimension;


/// The location of a chunk in a dimension of a world.
///
/// Note that this is not the block position;
/// multiply this position by 16 to find the positions of its blocks. For example
/// `ChunkPosition { x: 1, z: 2 }` refers to the chunk from `(16, 32)` to `(31, 47)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChunkPosition {
    pub x: i32,
    pub y: i32,
}

/// The location of a chunk in a world, including its dimension.
#[cfg(feature = "dimensions")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DimensionedChunkPos(ChunkPosition, NumericDimension);

#[cfg(feature = "dimensions")]
impl DimensionedChunkPos {
    /// Attempt to parse the bytes as a `ChunkPosition` followed by an optional `NumericDimension`.
    /// The dimension defaults to the Overworld if not present.
    ///
    /// Warning: the `NumericDimension` might not be a vanilla dimension, which could indicate
    /// that an unintentionally successful parse occurred.
    pub fn new_raw(bytes: &[u8]) -> Option<Self> {
        if bytes.len() == 8 {
            Some(Self(
                ChunkPosition {
                    x: i32::from_le_bytes(bytes[0..4].try_into().unwrap()),
                    y: i32::from_le_bytes(bytes[4..8].try_into().unwrap()),
                },
                NumericDimension::OVERWORLD,
            ))

        } else if bytes.len() == 12 {

            let dimension_id = u32::from_le_bytes(bytes[8..12].try_into().unwrap());

            Some(Self(
                ChunkPosition {
                    x: i32::from_le_bytes(bytes[0..4].try_into().unwrap()),
                    y: i32::from_le_bytes(bytes[4..8].try_into().unwrap()),
                },
                NumericDimension::from_bedrock_numeric(dimension_id),
            ))

        } else {
            None
        }
    }

    /// Extend the provided bytes with the byte format of a `DimensionedChunkPos`, namely
    /// a `ChunkPosition` followed by a `NumericDimension`. If the dimension
    /// is the Overworld, its dimension ID doesn't need to be serialized, but if
    /// `write_overworld_id` is true, then it will be.
    pub fn extend_serialized(self, bytes: &mut Vec<u8>, write_overworld_id: bool) {
        bytes.reserve(12);
        bytes.extend(self.0.x.to_le_bytes());
        bytes.extend(self.0.y.to_le_bytes());
        if write_overworld_id || self.1.to_bedrock_numeric() != 0 {
            bytes.extend(self.1.to_bedrock_numeric().to_le_bytes());
        }
    }

    /// Write a `DimensionedChunkPos` to bytes for a `ChunkPosition` followed by
    /// a `NumericDimension`. If the dimension is the Overworld,
    /// its dimension ID doesn't need to be serialized,
    /// but if `write_overworld_id` is true, then it will be.
    pub fn to_bytes(self, write_overworld_id: bool) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes, write_overworld_id);
        bytes
    }
}

#[cfg(feature = "dimensions")]
impl TryFrom<&[u8]> for DimensionedChunkPos {
    type Error = ();

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::new_raw(value).ok_or(())
    }
}
