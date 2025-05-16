use crate::nibble_array::NibbleArray;
use crate::heightmap::Heightmap;
use crate::legacy_biome_data::{LegacyBiomeColors, LegacyBiomeIds};


#[derive(Debug, Clone)]
pub struct LegacyTerrain {
    pub block_ids:    [u8; 4096],
    pub block_data:   NibbleArray<16384>,
    pub skylight:     NibbleArray<16384>,
    pub blocklight:   NibbleArray<16384>,
    pub heightmap:    Heightmap,
    pub biome_ids:    LegacyBiomeIds,
    pub biome_colors: LegacyBiomeColors,
}
