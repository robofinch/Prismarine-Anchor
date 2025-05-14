use subslice_to_array::SubsliceToArray as _;

use prismarine_anchor_mc_datatypes::positions::ChunkPosition;
use prismarine_anchor_mc_datatypes::dimensions::{NumericDimension, OverworldElision};

/// The location of a chunk in a world, including its dimension.
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct DimensionedChunkPos(pub ChunkPosition, pub Option<NumericDimension>);

impl DimensionedChunkPos {
    /// Attempt to parse the bytes as a `ChunkPosition` followed by an optional `NumericDimension`.
    /// The dimension being `None` implicitly indicates the Overworld; currently, Bedrock
    /// always elides the numeric dimension ID for the Overworld.
    ///
    /// Warning: the `NumericDimension` might not be a vanilla dimension, which could indicate
    /// that an unintentionally successful parse occurred.
    pub fn new_raw(bytes: &[u8]) -> Option<Self> {
        if bytes.len() == 8 {
            Some(Self(
                ChunkPosition {
                    x: i32::from_le_bytes(bytes.subslice_to_array::<0, 4>()),
                    z: i32::from_le_bytes(bytes.subslice_to_array::<4, 8>()),
                },
                None,
            ))

        } else if bytes.len() == 12 {

            let dimension_id = u32::from_le_bytes(bytes.subslice_to_array::<8, 12>());

            Some(Self(
                ChunkPosition {
                    x: i32::from_le_bytes(bytes.subslice_to_array::<0, 4>()),
                    z: i32::from_le_bytes(bytes.subslice_to_array::<4, 8>()),
                },
                Some(NumericDimension::from_bedrock_numeric(dimension_id)),
            ))

        } else {
            None
        }
    }

    /// Extend the provided bytes with the byte format of a `DimensionedChunkPos`, namely
    /// a `ChunkPosition` followed by a `NumericDimension`.
    ///
    /// If the dimension is the Overworld, its dimension ID doesn't need to be serialized;
    /// whether or not it is serialized is controlled by the `OverworldElision` option.
    pub fn extend_serialized(self, bytes: &mut Vec<u8>, write_overworld_id: OverworldElision) {
        bytes.reserve(12);
        bytes.extend(self.0.x.to_le_bytes());
        bytes.extend(self.0.z.to_le_bytes());

        let dimension_id = write_overworld_id.maybe_elide_id(self.1);
        if let Some(dimension_id) = dimension_id {
            bytes.extend(dimension_id.to_bedrock_numeric().to_le_bytes());
        }
    }

    /// Write a `DimensionedChunkPos` to bytes for a `ChunkPosition` followed by
    /// a `NumericDimension`.
    ///
    /// If the dimension is the Overworld, its dimension ID doesn't need to be serialized;
    /// whether or not it is serialized is controlled by the `OverworldElision` option.
    #[inline]
    pub fn to_bytes(self, write_overworld_id: OverworldElision) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes, write_overworld_id);
        bytes
    }
}

impl TryFrom<&[u8]> for DimensionedChunkPos {
    type Error = ();

    #[inline]
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::new_raw(value).ok_or(())
    }
}
