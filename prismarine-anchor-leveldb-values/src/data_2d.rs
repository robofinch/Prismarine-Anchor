use subslice_to_array::SubsliceToArray as _;
use zerocopy::transmute; // Used to convert arrays of arrays into 1D arrays (and back)


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

        let heightmap: [u8; 512] = value.subslice_to_array::<0, 512>();
        let heightmap: [[u8; 2]; 256] = transmute!(heightmap);
        let heightmap = heightmap.map(u16::from_le_bytes);
        let heightmap: [[u16; 16]; 16] = transmute!(heightmap);

        // 768 == 512 + 256
        let biome_ids: [u8; 256] = value.subslice_to_array::<512, 768>();
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
