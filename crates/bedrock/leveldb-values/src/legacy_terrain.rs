use subslice_to_array::{SubsliceToArray as _, SubsliceToArrayRef as _};

use crate::{heightmap::OldHeightmap, nibble_array::NibbleArray};
use crate::legacy_biome_data::{
    biome_data_from_parts, biome_data_to_parts,
    LegacyBiomeColors, LegacyBiomeIds,
};


#[derive(Debug, Clone)]
pub struct LegacyTerrain {
    pub block_ids:    [u8; 32768],
    pub block_data:   NibbleArray<16384>,
    pub skylight:     NibbleArray<16384>,
    pub blocklight:   NibbleArray<16384>,
    pub heightmap:    OldHeightmap,
    pub biome_ids:    LegacyBiomeIds,
    pub biome_colors: LegacyBiomeColors,
}

impl LegacyTerrain {
    pub fn parse(value: &[u8]) -> Option<Self> {
        // 32768 + 16384 + 16384 + 16384 + 512 + 1024 == 83200
        if value.len() != 83200 {
            return None;
        }

        let block_ids  = value.subslice_to_array::<0, 32768>();
        // 32768 + 16384 == 49152
        let block_data = value.subslice_to_array::<32768, 49152>();
        let skylight   = value.subslice_to_array::<49152, 65536>();
        // 65536 + 16384 == 81920
        let blocklight = value.subslice_to_array::<65536, 81920>();
        // 81920 + 512 == 82176
        let heightmap  = value.subslice_to_array::<81920, 82176>();
        // 82176 + 1024 == 83200
        let biome_data = value.subslice_to_array_ref::<82176, 83200>();

        let (biome_ids, biome_colors) = biome_data_to_parts(biome_data);

        Some(Self {
            block_ids,
            block_data: NibbleArray(block_data),
            skylight: NibbleArray(skylight),
            blocklight: NibbleArray(blocklight),
            heightmap: OldHeightmap::from_flattened(heightmap),
            biome_ids,
            biome_colors,
        })
    }

    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) {
        bytes.reserve(83200);
        bytes.extend(self.block_ids);
        bytes.extend(self.block_data.0);
        bytes.extend(self.skylight.0);
        bytes.extend(self.blocklight.0);
        bytes.extend(self.heightmap.flattened_ref());
        bytes.extend(biome_data_from_parts(&self.biome_ids, &self.biome_colors));
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes);
        bytes
    }
}
