use subslice_to_array::{SubsliceToArray as _, SubsliceToArrayRef as _};

use super::helpers::{
    biome_data_from_parts, biome_data_to_parts,
    Heightmap, LegacyBiomeColors, LegacyBiomeIds,
};


/// Not written since 1.0.0
// TODO: exactly when?
// And could a world end up with both LegacyData2D and Data2D keys?
// (In such a circumstance, I assume the LegacyData2D would be ignored.)
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq))]
#[derive(Debug, Clone)]
pub struct LegacyData2D {
    pub heightmap:    Heightmap,
    pub biome_ids:    LegacyBiomeIds,
    pub biome_colors: LegacyBiomeColors,
}

impl LegacyData2D {
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() != 512 + 1024 {
            log::warn!("LegacyData2D didn't have length 1536. Received length: {}", value.len());
            return None;
        }

        let heightmap: [u8; 512] = value.subslice_to_array::<0, 512>();
        let heightmap = Heightmap::from_flattened_le_bytes(heightmap);

        // 512 + 1024 == 1536
        let biomes: &[u8; 1024] = value.subslice_to_array_ref::<512, 1536>();
        let (biome_ids, biome_colors) = biome_data_to_parts(biomes);

        Some(Self {
            heightmap,
            biome_ids,
            biome_colors,
        })
    }

    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) {
        let heightmap = self.heightmap.flattened_le_bytes_cow();
        let biomes = biome_data_from_parts(&self.biome_ids, &self.biome_colors);

        bytes.reserve(1536); // 512 + 1024
        bytes.extend(heightmap.as_slice());
        bytes.extend(biomes);
    }

    #[inline]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes);
        bytes
    }
}
