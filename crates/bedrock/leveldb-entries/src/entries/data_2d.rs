use subslice_to_array::SubsliceToArray as _;

use super::helpers::{Heightmap, NewLegacyBiomeIds, LegacyBiomeIds};


/// Not written since 1.18.0, except in Old-style worlds.
///
/// At some point (possibly 1.21.40, or perhaps when `Data3D` was introduced),
/// biome IDs were changed to be 16 bits instead of 8 bits, and this also impacted `Data2D`.
// TODO: when exactly did it stop being used? And when did it change bit size?
#[derive(Debug, Clone)]
pub enum Data2D {
    Original(Box<Data2DOriginal>),
    New(Box<Data2DNew>),
}

impl Data2D {
    #[inline]
    pub fn parse(value: &[u8]) -> Option<Self> {
        match value.len() {
            // 512 for heightmap + 256 for biomes
            768  => Some(Self::Original(Box::new(Data2DOriginal::parse(value)?))),
            // 512 each for heightmap and biomes
            1024 => Some(Self::New(Box::new(Data2DNew::parse(value)?))),
            _    => None,
        }
    }

    #[inline]
    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) {
        match self {
            Self::Original(data) => data.extend_serialized(bytes),
            Self::New(data)      => data.extend_serialized(bytes),
        }
    }

    #[inline]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes);
        bytes
    }
}

#[derive(Debug, Clone)]
pub struct Data2DOriginal {
    pub heightmap: Heightmap,
    pub biome_ids: LegacyBiomeIds,
}

impl Data2DOriginal {
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() != 512 + 256 {
            return None;
        }

        let heightmap: [u8; 512] = value.subslice_to_array::<0, 512>();
        let heightmap = Heightmap::from_flattened_le_bytes(heightmap);

        // 768 == 512 + 256
        let biome_ids: [u8; 256] = value.subslice_to_array::<512, 768>();
        let biome_ids = LegacyBiomeIds::from_flattened(biome_ids);

        Some(Self {
            heightmap,
            biome_ids,
        })
    }

    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) {
        let heightmap = self.heightmap.flattened_le_bytes_cow();
        let biome_ids = self.biome_ids.flattened_ref();

        bytes.reserve(768); // 512 + 256
        bytes.extend(heightmap.as_slice());
        bytes.extend(biome_ids);
    }

    #[inline]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes);
        bytes
    }
}

#[derive(Debug, Clone)]
pub struct Data2DNew {
    pub heightmap: Heightmap,
    pub biome_ids: NewLegacyBiomeIds,
}

impl Data2DNew {
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() != 1024 {
            return None;
        }

        let heightmap: [u8; 512] = value.subslice_to_array::<0, 512>();
        let biome_ids: [u8; 512] = value.subslice_to_array::<512, 1024>();

        Some(Self {
            heightmap: Heightmap::from_flattened_le_bytes(heightmap),
            biome_ids: NewLegacyBiomeIds::from_flattened_le_bytes(biome_ids),
        })
    }

    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) {
        bytes.reserve(768); // 512 + 256
        let heightmap = self.heightmap.flattened_le_bytes_cow();
        let biome_ids = self.biome_ids.flattened_le_bytes_cow();

        bytes.extend(heightmap.as_slice());
        bytes.extend(biome_ids.as_slice());
    }

    #[inline]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes);
        bytes
    }
}
