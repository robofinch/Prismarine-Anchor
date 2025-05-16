use prismarine_anchor_mc_datatypes::positions::{BlockPosInSubchunk, ChunkColumn};


/// An array of an even number of 4-bit values. Each byte is little-endian: the less significant
/// nibble comes before the more significant nibble.
///
/// Note that the array is of length `2 * N`, and uses `N` bytes of space.
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct NibbleArray<const N: usize>(pub [u8; N]);

impl<const N: usize> NibbleArray<N> {
    /// The output, if `Some`, is guaranteed to be at most 15.
    /// Returns `Some` if and only if `index < 2 * N`.
    pub fn get_flattened(&self, index: usize) -> Option<u8> {
        let inner_index = index / 2;
        // The index with `index % 2 == 0` comes first, and uses the less significant nibble.
        // So, the more significant nibble is used iff `index % 2 == 1`.
        let more_significant_nibble = index % 2 == 1;

        let value = *self.0.get(inner_index)?;

        let nibble = if more_significant_nibble {
            value >> 4
        } else {
            value & 0b1111
        };
        Some(nibble)
    }
}

impl NibbleArray<2048> {
    /// The output is at most 15.
    pub fn get_in_subchunk(&self, pos: BlockPosInSubchunk) -> u8 {
        let x = usize::from(pos.x());
        let z = usize::from(pos.z());
        let y = usize::from(pos.y());
        let index = (x << 8) + (z << 4) + y;
        self.get_flattened(index).expect("index is strictly less than 4096")
    }
}

impl NibbleArray<16_384> {
    /// The output, if `Some`, is guaranteed to be at most 15.
    /// Returns `Some` if and only if `y < 128`.
    pub fn get_in_legacy_terrain(&self, column: ChunkColumn, y: u8) -> Option<u8> {
        if y >= 128 {
            return None;
        }

        let x = usize::from(column.x());
        let z = usize::from(column.z());
        let y = usize::from(y);
        let index = (x << 11) | (z << 7) | y;
        // index uses at most 11 + 4 nonzero bits, and 2^15 = 32_768 = 2 * 16_384
        Some(self.get_flattened(index).expect("index is strictly less than 32_768"))
    }
}
