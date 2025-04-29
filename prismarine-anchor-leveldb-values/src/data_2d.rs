use zerocopy::transmute; // Used to convert arrays of arrays into 1D arrays (and back)

use prismarine_anchor_util::slice_to_array;


/// Not written since 1.18.0
// TODO: exactly when?
#[derive(Debug, Clone)]
pub struct Data2D {
    /// The inner array is indexed by Z values. The outer array is indexed by X values.
    /// Therefore, the correct indexing order is `heightmap[X][Z]`.
    pub heightmap: [[u16; 16]; 16],
    /// The inner array is indexed by Z values. The outer array is indexed by X values.
    /// Therefore, the correct indexing order is `biome_ids[X][Z]`.
    pub biome_ids: [[u8; 16]; 16],
}

impl Data2D {
    #[inline]
    pub fn flattened_heightmap(&self) -> [u16; 256] {
        transmute!(self.heightmap)
    }

    #[inline]
    pub fn flattened_biome_ids(&self) -> [u8; 256] {
        transmute!(self.biome_ids)
    }

    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() != 512 + 256 {
            return None;
        }

        let heightmap: [u8; 512] = slice_to_array::<0, 512, _, 512>(value);
        let heightmap: [[u8; 2]; 256] = transmute!(heightmap);
        let heightmap = heightmap.map(u16::from_le_bytes);
        let heightmap: [[u16; 16]; 16] = transmute!(heightmap);

        // 768 == 512 + 256
        let biome_ids: [u8; 256] = slice_to_array::<512, 768, _, 256>(value);
        let biome_ids: [[u8; 16]; 16] = transmute!(biome_ids);

        Some(Self {
            heightmap,
            biome_ids,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let heightmap: [u16; 256] = transmute!(self.heightmap);
        let heightmap: [[u8; 2]; 256] = heightmap.map(u16::to_le_bytes);
        let heightmap: [u8; 512] = transmute!(heightmap);

        let biome_ids: [u8; 256] = transmute!(self.biome_ids);

        let mut output = heightmap.to_vec();
        output.extend(biome_ids);
        output
    }
}
