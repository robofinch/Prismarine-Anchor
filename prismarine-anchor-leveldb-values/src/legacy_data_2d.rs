use std::array;

use subslice_to_array::SubsliceToArray as _;
use zerocopy::transmute; // Used to convert arrays of arrays into 1D arrays (and back)
use zerocopy::{FromBytes, IntoBytes};


/// Not written since 1.0.0
// TODO: exactly when?
// And could a world end up with both LegacyData2D and Data2D keys?
// (In such a circumstance, I assume the LegacyData2D would be ignored.)
#[derive(Debug, Clone)]
pub struct LegacyData2D {
    /// The inner array is indexed by Z values. The outer array is indexed by X values.
    /// Therefore, the correct indexing order is `heightmap[X][Z]`.
    pub heightmap: [[u16; 16]; 16],
    /// The inner array is indexed by Z values. The outer array is indexed by X values.
    /// Therefore, the correct indexing order is `biome_ids[X][Z]`.
    pub biome_ids: [[u8; 16]; 16],
    /// The inner array is indexed by Z values. The outer array is indexed by X values.
    /// Therefore, the correct indexing order is `biome_colors[X][Z]`.
    pub biome_colors: [[LegacyBiomeColor; 16]; 16],
}

impl LegacyData2D {
    pub fn flattened_heightmap(&self) -> [u16; 256] {
        transmute!(self.heightmap)
    }

    pub fn flattened_biome_ids(&self) -> [u8; 256] {
        transmute!(self.biome_ids)
    }

    pub fn flattened_biome_colors(&self) -> [LegacyBiomeColor; 256] {
        transmute!(self.biome_colors)
    }

    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() != 512 + 1024 {
            return None;
        }

        let heightmap: [u8; 512] = value.subslice_to_array::<0, 512>();
        let heightmap: [[u8; 2]; 256] = transmute!(heightmap);
        let heightmap = heightmap.map(u16::from_le_bytes);
        let heightmap: [[u16; 16]; 16] = transmute!(heightmap);

        // 512 + 1024 == 1536
        let biomes: [u8; 1024] = value.subslice_to_array::<512, 1536>();
        let biomes: [[u8; 4]; 256] = transmute!(biomes);

        let biome_ids = biomes.map(|[id, ..]| id);
        let biome_ids = transmute!(biome_ids);

        let biome_colors: [[[u8; 4]; 16]; 16] = transmute!(biomes);
        let biome_colors = biome_colors.map(|inner_arr| {
            inner_arr.map(|[_, red, green, blue]| LegacyBiomeColor { red, green, blue })
        });

        Some(Self {
            heightmap,
            biome_ids,
            biome_colors,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let heightmap: [u16; 256] = transmute!(self.heightmap);
        let heightmap: [[u8; 2]; 256] = heightmap.map(u16::to_le_bytes);
        let heightmap: [u8; 512] = transmute!(heightmap);

        let biome_ids: [u8; 256] = transmute!(self.biome_ids);
        let biome_colors: [LegacyBiomeColor; 256] = transmute!(self.biome_colors);

        let biomes: [[u8; 4]; 256] = array::from_fn(|idx| {
            let id = biome_ids[idx];
            let color = biome_colors[idx];
            [id, color.red, color.green, color.blue]
        });
        let biomes: [u8; 1024] = transmute!(biomes);

        let mut output = heightmap.to_vec();
        output.extend(biomes);
        output
    }
}

#[derive(IntoBytes, FromBytes, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacyBiomeColor {
    pub red:   u8,
    pub green: u8,
    pub blue:  u8,
}
