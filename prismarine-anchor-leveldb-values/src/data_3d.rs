use std::io::Cursor;

use subslice_to_array::SubsliceToArray as _;
use zerocopy::transmute;

use crate::all_read;
use crate::palettized_storage::{
    PaletteHeader, PaletteType, PalettizedStorage,
    read_le_u32s, write_le_u32s,
};


#[derive(Debug, Clone)]
pub struct Data3D {
    /// The inner array is indexed by Z values. The outer array is indexed by X values.
    /// Therefore, the correct indexing order is `heightmap[X][Z]`.
    pub heightmap: [[u16; 16]; 16],
    /// The biomes are stored in subchunks starting from the bottom of the world.
    /// In the Overworld, it should have length 24; in the Nether, 8; and in the End, 16.
    pub biomes: Vec<PalettizedStorage<u32>>,
}

impl Data3D {
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() <= 512 {
            return None;
        }

        let heightmap: [u8; 512] = value.subslice_to_array::<0, 512>();
        let heightmap: [[u8; 2]; 256] = transmute!(heightmap);
        let heightmap = heightmap.map(u16::from_le_bytes);
        let heightmap: [[u16; 16]; 16] = transmute!(heightmap);

        // We know that value.len() > 512
        let mut reader = Cursor::new(&value[512..]);
        let mut subchunks = Vec::new();

        let remaining_len = value.len() - 512;

        while !all_read(reader.position(), remaining_len) {
            let header = PaletteHeader::parse_header(&mut reader).ok()?;
            match header.palette_type {
                PaletteType::Persistent => {
                    // Unlike with SubchunkBlocks, only Runtime is usually used for Data3D,
                    // so we only support that.
                    return None;
                }
                PaletteType::Runtime => {
                    subchunks.push(PalettizedStorage::parse(
                        &mut reader,
                        header.bits_per_index,
                        read_le_u32s,
                    ).ok()?);
                }
            }
        }

        Some(Self {
            heightmap,
            biomes: subchunks,
        })
    }

    #[inline]
    pub fn flattened_heightmap(&self) -> [u16; 256] {
        transmute!(self.heightmap)
    }

    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) {
        let heightmap: [u16; 256] = transmute!(self.heightmap);
        let heightmap = heightmap.map(u16::to_le_bytes);
        let heightmap: [u8; 512] = transmute!(heightmap);

        bytes.extend(heightmap);

        for subchunk in &self.biomes {
            // Since write_le_u32s is infallible, the below is infallible.
            subchunk
                .extend_serialized(bytes, PaletteType::Runtime, true, write_le_u32s)
                .expect("write_le_u32s is infallible");
        }
    }

    #[inline]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes);
        bytes
    }
}
