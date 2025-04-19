use std::array;
use std::collections::BTreeSet;
use std::io::{Cursor, Read};

use zerocopy::transmute;

use super::all_read;


#[derive(Debug, Clone)]
pub struct Data3D {
    /// The inner array is indexed by Z values. The outer array is indexed by X values.
    /// Therefore, the correct indexing order is `heightmap[X][Z]`.
    pub heightmap: [[u16; 16]; 16],
    /// The biomes are stored in subchunks starting from the bottom of the world.
    /// In the Overworld, it should have length 24; in the Nether, 8; and in the End, 16.
    pub biomes: Vec<Data3DSubchunkBiomes>,
}

#[derive(Debug, Clone)]
pub enum Data3DSubchunkBiomes {
    Empty,
    Uniform(u32),
    Palettized(PalettizedBiomes),
    // TODO: figure out what happens when PaletteType is Persistent instead of Runtime,
    // and make another enum variant if needed. RN, if it were to occur,
    // it would just result in an opaque RawValue instead of a Data3D.
    // Note that unlike Subchunk data, only Runtime IDs are supported here, which is the opposite.
}

#[derive(Debug, Clone)]
pub struct PalettizedBiomes {
    bits_per_index: PaletteBitsPerIndex,
    packed_biome_indices: Vec<u32>,
    biome_id_palette: Vec<u32>,
}

/// The (nonzero) number of bits per index into a palette.
/// Used by [`PalettizedBiomes`] and [`SubchunkBlocks`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteBitsPerIndex {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Eight,
    // I find it annoying that 10 isn't an option (for 3 indices per u32)
    Sixteen,
}

/// Used to explicitly describe these two states,
/// but it's basically a bool for "is runtime?" / "is not persistent?"
enum PaletteType {
    Persistent,
    Runtime,
}

impl Data3D {
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() <= 512 {
            return None;
        }

        // The .try_into().unwrap() converts a slice of length 512 into an array of
        // length 512, which does not fail.
        let heightmap: [u8; 512] = value[0..512].try_into().unwrap();
        let heightmap: [[u8; 2]; 256] = transmute!(heightmap);
        let heightmap = heightmap.map(u16::from_le_bytes);
        let heightmap: [[u16; 16]; 16] = transmute!(heightmap);

        // We know that value.len() > 512
        let mut reader = Cursor::new(&value[512..]);
        let mut subchunks = Vec::new();

        let remaining_len = value.len() - 512;

        while !all_read(reader.position(), remaining_len) {
            subchunks.push(Data3DSubchunkBiomes::parse(&mut reader)?);
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
            subchunk.extend_serialized(bytes);
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes);
        bytes
    }
}

impl Data3DSubchunkBiomes {
    pub fn parse(reader: &mut impl Read) -> Option<Self> {
        let mut header = [0; 1];
        reader.read_exact(&mut header).ok()?;
        let header = header[0];
        let palette_type = PaletteType::from(header & 1);
        let bits_per_index_or_special = header >> 1;

        if let PaletteType::Persistent = palette_type {
            // This basically never happens, or so I've heard.
            // TODO: figure it out
            return None;
        }

        match bits_per_index_or_special {
            127 => Some(Self::Empty),
            0 => {
                let mut biome_id = [0; 4];
                reader.read_exact(&mut biome_id).ok()?;
                let biome_id = u32::from_le_bytes(biome_id);
                Some(Self::Uniform(biome_id))
            }
            bits_per_index => {

                fn read_le_u32s(reader: &mut impl Read, num_u32s: usize) -> Option<Vec<u32>> {
                    let mut u32s = vec![0; num_u32s * 4];
                    reader.read_exact(&mut u32s).ok()?;

                    let mut u32s = u32s.into_iter();
                    Some(Vec::from_iter((0..num_u32s).map(|_| {
                        // We know that u32s has length exactly num_u32s * 4,
                        // so these unwraps succeed.
                        let block = [
                            u32s.next().unwrap(),
                            u32s.next().unwrap(),
                            u32s.next().unwrap(),
                            u32s.next().unwrap(),
                        ];
                        u32::from_le_bytes(block)
                    })))
                }

                let bits_per_index = PaletteBitsPerIndex::new_exact(bits_per_index)?;
                let packed_indices_len = bits_per_index.num_u32s_for_4096_indices();

                let packed_indices = read_le_u32s(reader, packed_indices_len)?;

                let mut palette_len = [0; 4];
                reader.read_exact(&mut palette_len).ok()?;
                let palette_len = u32::from_le_bytes(palette_len);
                let palette_len = usize::try_from(palette_len).ok()?;

                let palette = read_le_u32s(reader, palette_len)?;

                Some(Self::Palettized(PalettizedBiomes::new_packed_checked(
                    bits_per_index,
                    packed_indices,
                    palette,
                )?))
            }
        }
    }

    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) {
        match self {
            Self::Empty => {
                let bits_per_index = 127u8;
                let palette_version = 1;
                let header = (bits_per_index << 1) | palette_version;
                bytes.push(header);
            }
            Self::Uniform(biome_id) => {
                let bits_per_index = 0u8;
                let palette_version = 1;
                let header = (bits_per_index << 1) | palette_version;
                bytes.reserve(5);
                bytes.push(header);
                bytes.extend(biome_id.to_le_bytes());
            }
            Self::Palettized(palettized_biomes) => {
                let bits_per_index = u8::from(palettized_biomes.bits_per_index);
                let palette_version = 1;
                let header = (bits_per_index << 1) | palette_version;


                let num_u32s = palettized_biomes.packed_indices_len();
                let palette_len = palettized_biomes.palette_len();
                let (_, packed_indices, palette) = palettized_biomes.packed();

                let reserve_len = 1 + num_u32s*4 + 4 + palette_len*4;
                // In the worst case, this value is
                // 1 + 2048*4 + 4 + 4096*4 = 24_851 < 65_536, which is 2 to the 16.
                // Thus, it definitely fits in a usize, which has at least 16 bits.
                let reserve_len = usize::try_from(reserve_len).unwrap();

                // ...This could easily panic on 16 bit systems, though.
                bytes.reserve(reserve_len);
                bytes.push(header);
                for block in packed_indices {
                    bytes.extend(block.to_le_bytes());
                }
                bytes.extend(palette_len.to_le_bytes());
                for biome_id in palette {
                    bytes.extend(biome_id.to_le_bytes());
                }
            }
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes);
        bytes
    }
}

impl PalettizedBiomes {
    /// The provided data should be the biome IDs of a subchunk,
    /// where Y is the innermost index, Z is the middle index, and X is the outermost index.
    /// In other words, the correct indexing order should be `unpacked_biome_ids[X][Z][Y]`.
    pub fn new_unpacked(unpacked_biome_ids: [[[u32; 16]; 16]; 16]) -> Self {
        Self::new_unpacked_flattened(transmute!(unpacked_biome_ids))
    }

    /// The provided data should be the biome IDs of a subchunk in YZX order (Y increments first).
    pub fn new_unpacked_flattened(unpacked_biome_ids: [u32; 4096]) -> Self {
        let mut biome_id_palette = BTreeSet::new();

        for &biome_id in &unpacked_biome_ids {
            biome_id_palette.insert(biome_id);
        }

        let biome_id_palette: Vec<u32> = biome_id_palette.into_iter().collect();

        if biome_id_palette.len() == 1 {
            // We can, as a special case, do this very quickly.

            let bits_per_index = PaletteBitsPerIndex::One;

            // The number of u32 blocks is the number of indices, 4096, divided by
            // the number of 1-bit indices which can be stored in a 32-bit block, which is 32.
            let num_u32s = 4096 / 32;

            return Self {
                bits_per_index,
                packed_biome_indices: vec![0; num_u32s],
                biome_id_palette,
            };
        }

        // Note that we *cannot* have `biome_id_palette.len() == 0`. We added things
        // to `biome_id_palette`, so even if it was always the same value, there's at least 1.
        // We know `biome_id_palette.len() > 0`, and is at most 4096, which is 2^12,
        // so it's less than 2 to the 16. Thus, the below does not panic.
        let bits_per_index = PaletteBitsPerIndex::new_from_usize(biome_id_palette.len()).unwrap();

        let num_u32s = bits_per_index.num_u32s_for_4096_indices();
        // `2^bits_per_index - 1` has the least-significant `bits_per_index` bits set.
        // This fits in a u32, which is used below.
        let index_mask = (1usize << u8::from(bits_per_index)) - 1;

        let mut packed = Vec::with_capacity(num_u32s);
        let mut u32_block = 0u32;
        let mut num_indices_in_block = 0;

        for biome_id in unpacked_biome_ids {
            u32_block <<= u8::from(bits_per_index);

            // This unwrapping does not panic since we added every biome_id
            // to the BTreeSet which was then converted to a sorted Vec.
            let index = biome_id_palette.binary_search(&biome_id).unwrap();

            // index_mask fits in a u32, so this doesn't overflow.
            let index = (index & index_mask) as u32;

            u32_block |= index;

            num_indices_in_block += 1;
            if num_indices_in_block >= bits_per_index.indices_per_u32() {
                packed.push(u32_block);
                u32_block = 0;
                num_indices_in_block = 0;
            }
        }

        Self {
            packed_biome_indices: packed,
            bits_per_index,
            biome_id_palette,
        }
    }

    /// Creates a new `PalettizedBiomes` struct which stores the biome IDs of a subchunk
    /// in a condensed way. This performs the following checks,
    /// which includes iterating over all 4096 indices:
    ///
    /// `packed_biome_indices` has length exactly `bits_per_index.num_u32s_for_4096_indices()`;
    ///
    /// `biome_id_palette` has length greater than the maximum biome index, and at most `4096`.
    pub fn new_packed_checked(
        bits_per_index: PaletteBitsPerIndex,
        packed_biome_indices: Vec<u32>,
        biome_id_palette: Vec<u32>,
    ) -> Option<Self> {

        if packed_biome_indices.len() != bits_per_index.num_u32s_for_4096_indices() {
            return None;
        }

        if biome_id_palette.len() == 0 || biome_id_palette.len() > 4096 {
            return None;
        }

        let max_permissible_index = biome_id_palette.len() - 1;
        // This unwrap does not panic, by the above checks on biome_id_palette.len().
        let max_permissible_index = u32::try_from(max_permissible_index).unwrap();

        let indices_per_u32 = bits_per_index.indices_per_u32();
        // let padding_bits = bits_per_index.padding_bits();
        // `2^bits_per_index - 1` has the least-significant `bits_per_index` bits set.
        let index_mask = (1u32 << u8::from(bits_per_index)) - 1;

        for &dword in &packed_biome_indices {
            let mut dword = dword;

            for _ in 0..indices_per_u32 {

                let index = dword & index_mask;
                if index > max_permissible_index {
                    return None;
                }

                dword >>= u8::from(bits_per_index);
            }
        }

        // All the checks are done.
        Some(Self::new_packed_unchecked(bits_per_index, packed_biome_indices, biome_id_palette))
    }

    /// Creates a new `PalettizedBiomes` struct which stores the biome IDs of a subchunk
    /// in a condensed way. This performs no checks for correctness.
    /// While no memory unsoundness will occur if assumptions are not met, panics
    /// may occur, or invalid biome data that Minecraft may reject may be written.
    /// It is assumed that:
    ///
    /// `packed_biome_indices` has length exactly `bits_per_index.num_u32s_for_4096_indices()`;
    ///
    /// `biome_id_palette` has length greater than the maximum biome index, and at most `4096`.
    pub fn new_packed_unchecked(
        bits_per_index: PaletteBitsPerIndex,
        packed_biome_indices: Vec<u32>,
        biome_id_palette: Vec<u32>,
    ) -> Self {
        Self {
            bits_per_index,
            packed_biome_indices,
            biome_id_palette,
        }
    }

    /// Returns the length of `packed_biome_indices` as a `u32`. By the assumptions
    /// of this struct, that value's length is at most 2048 and thus fits in a `u32`.
    /// Note that `bits_per_index.num_u32s_for_4096_indices() <= 2048` for all possible
    /// values of `bits_per_index: PaletteBitsPerIndex`.
    pub fn packed_indices_len(&self) -> u32 {
        u32::try_from(self.packed_biome_indices.len())
            .expect(
                "a PalettizedBiomes' packed_biome_indices should have length at most 2048"
            )
    }

    /// Returns the length of `biome_id_palette` as a `u32`. By the assumptions
    /// of this struct, that value's length is at most 4096 and thus fits in a `u32`.
    pub fn palette_len(&self) -> u32 {
        u32::try_from(self.biome_id_palette.len())
            .expect(
                "a PalettizedBiomes' biome_id_palette should have length at most 4096"
            )
    }

    /// Return the `bits_per_index` and references to the
    /// `packed_biome_indices` and `biome_id_palette` of
    /// this `PalettizedBiomes`.
    pub fn packed(&self) -> (PaletteBitsPerIndex, &Vec<u32>, &Vec<u32>) {
        (self.bits_per_index, &self.packed_biome_indices, &self.biome_id_palette)
    }

    /// Returns the `bits_per_index`, `packed_biome_indices`, and `biome_id_palette` of
    /// this `PalettizedBiomes`, consuming it.
    pub fn into_packed(self) -> (PaletteBitsPerIndex, Vec<u32>, Vec<u32>) {
        (self.bits_per_index, self.packed_biome_indices, self.biome_id_palette)
    }

    /// Compute the biome IDs of a subchunk which are stored in a condensed way in this struct.
    /// In the output data, Y is the innermost index, Z is the middle index,
    /// and X is the outermost index.
    /// In other words, the correct indexing order should be `self.unpacked()[X][Z][Y]`.
    pub fn unpacked(&self) -> [[[u32; 16]; 16]; 16] {
        transmute!(self.unpacked_flattened())
    }

    /// Compute the biome IDs of a subchunk which are stored in a condensed way in this struct.
    /// The output data is in YZX order (Y increments first).
    pub fn unpacked_flattened(&self) -> [u32; 4096] {

        // `2^bits_per_index - 1` has the least-significant `bits_per_index` bits set.
        let index_mask = (1u32 << u8::from(self.bits_per_index)) - 1;

        let mut packed_ids = self.packed_biome_indices.iter();
        let mut u32_block = 0;
        let mut num_indices_in_block = 0;

        array::from_fn(|_| {
            if num_indices_in_block == 0 {
                // Since there must be enough blocks for all 4096 indices, this does not panic.
                u32_block = *packed_ids.next().unwrap();

                // Note that this value is not accurate for the final block, but that's fine,
                // since the final block is stopped by the 4096-index-limit of array::from_fn
                // (That is to say, the callback won't be called more than the correct
                // number of times in the final block.)
                num_indices_in_block = self.bits_per_index.indices_per_u32();
            }

            let index = u32_block & index_mask;
            u32_block >>= u8::from(self.bits_per_index);
            num_indices_in_block -= 1;

            // Note that self.bits_per_index is at most 16, so this does not overflow.
            let index = index as u16;

            self.biome_id_palette[usize::from(index)]
        })
    }
}

impl PaletteBitsPerIndex {
    /// Given the length of some palette which will be indexed into, returns the number
    /// of bits needed for an index of an arbitrary entry of that palette.
    ///
    /// Returns `None` precisely if the `palette_len` is `0`, or greater than `2^16`.
    pub fn new_from_usize(palette_len: usize) -> Option<Self> {

        if palette_len <= 1 {
            return None;
        }

        let min_bits_per_index = usize::BITS - (palette_len - 1).leading_zeros();
        let Ok(min_bits_per_index) = u8::try_from(min_bits_per_index) else {
            // If it's greater than 255, then it definitely is too big.
            return None;
        };

        Self::new(min_bits_per_index)
    }

    /// Returns an allowed palette index bit width that is at least as big as the provided
    /// `bits_per_index`. Returns `None` if `bits_per_index` is too large (currently,
    /// returns `None` precisely if `bits_per_index` is at least `2` to the power of `16`).
    pub fn new(bits_per_index: u8) -> Option<Self> {
        Some(match bits_per_index {
            0 | 1 => Self::One,
            2     => Self::Two,
            3     => Self::Three,
            4     => Self::Four,
            5     => Self::Five,
            6     => Self::Six,
            7 | 8 => Self::Eight,
            x if x <= 16 => Self::Sixteen,
            _ => return None,
        })
    }

    /// Returns an allowed palette index bit width that is exactly equal to the provided
    /// `bits_per_index`, if possible.
    pub fn new_exact(bits_per_index: u8) -> Option<Self> {
        Some(match bits_per_index {
            1  => Self::One,
            2  => Self::Two,
            3  => Self::Three,
            4  => Self::Four,
            5  => Self::Five,
            6  => Self::Six,
            8  => Self::Eight,
            16 => Self::Sixteen,
            _ => return None,
        })
    }

    /// The number of indices which fit in one u32 (4 bytes, a `u32`).
    pub fn indices_per_u32(self) -> u8 {
        32 / u8::from(self)
    }

    /// The number of u32s (4 bytes, a `u32`) it takes to store 4096 indices
    /// with the given number of bits per index.
    pub fn num_u32s_for_4096_indices(self) -> usize {
        4096_usize.div_ceil(usize::from(self.indices_per_u32()))
    }

    /// Some bit widths require there to be padding when indices are packed into a u32,
    /// when `self.indices_per_u32() * u8::from(self) < 32`.
    /// This occurs when `32` is not evenly divisible by this number of bits per index.
    ///
    /// This returns the necessary number of padding bits (possibly `0`), such that
    /// `self.indices_per_u32() * u8::from(self) + self.padding_bits() == 32`.
    pub fn padding_bits(self) -> u8 {
        if [3, 5, 6].contains(&u8::from(self)) { 2 } else { 0 }
    }
}

impl From<PaletteBitsPerIndex> for u8 {
    fn from(value: PaletteBitsPerIndex) -> Self {
        match value {
            PaletteBitsPerIndex::One      => 1,
            PaletteBitsPerIndex::Two      => 2,
            PaletteBitsPerIndex::Three    => 3,
            PaletteBitsPerIndex::Four     => 4,
            PaletteBitsPerIndex::Five     => 5,
            PaletteBitsPerIndex::Six      => 6,
            PaletteBitsPerIndex::Eight    => 8,
            PaletteBitsPerIndex::Sixteen  => 16,
        }
    }
}

impl From<bool> for PaletteType {
    fn from(value: bool) -> Self {
        if value {
            PaletteType::Runtime
        } else {
            PaletteType::Persistent
        }
    }
}

impl From<PaletteType> for bool {
    fn from(value: PaletteType) -> Self {
        match value {
            PaletteType::Persistent => false,
            PaletteType::Runtime    => true,
        }
    }
}

impl From<u8> for PaletteType {
    fn from(value: u8) -> Self {
        match value {
            0 => PaletteType::Persistent,
            _ => PaletteType::Runtime,
        }
    }
}

impl From<PaletteType> for u8 {
    fn from(value: PaletteType) -> Self {
        match value {
            PaletteType::Persistent => 0,
            PaletteType::Runtime    => 1,
        }
    }
}
