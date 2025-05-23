use prismarine_anchor_mc_datatypes::{BlockPosInSubchunk, ChunkColumn};


/// An array of an even number of 4-bit values. Within each byte, the less significant
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

    #[inline]
    pub fn iter(&self) -> Iter<'_, N> {
        self.into_iter()
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

#[derive(Debug, Clone)]
pub struct IntoIter<const N: usize> {
    array:        NibbleArray<N>,
    index:        usize,
    upper_nibble: bool,
}

impl<const N: usize> IntoIter<N> {
    #[inline]
    fn new(array: NibbleArray<N>) -> Self {
        Self {
            array,
            index:        0,
            upper_nibble: false,
        }
    }
}

impl<const N: usize> Iterator for IntoIter<N> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < N {
            let byte = self.array.0[self.index];

            let nibble = if self.upper_nibble {
                // Advance the iterator to the first nibble of the next byte
                self.index += 1;
                self.upper_nibble = false;

                byte >> 4
            } else {
                // Advance the iterator to the next nibble
                self.upper_nibble = true;

                byte & 0b1111
            };

            Some(nibble)
        } else {
            None
        }
    }
}

impl<const N: usize> IntoIterator for NibbleArray<N> {
    type IntoIter = IntoIter<N>;
    type Item = u8;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter::new(self)
    }
}

#[derive(Debug, Clone)]
pub struct Iter<'a, const N: usize> {
    array:        &'a NibbleArray<N>,
    index:        usize,
    upper_nibble: bool,
}

impl<'a, const N: usize> Iter<'a, N> {
    #[inline]
    fn new(array: &'a NibbleArray<N>) -> Self {
        Self {
            array,
            index:        0,
            upper_nibble: false,
        }
    }
}

impl<const N: usize> Iterator for Iter<'_, N> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < N {
            let byte = self.array.0[self.index];

            let nibble = if self.upper_nibble {
                // Advance the iterator to the first nibble of the next byte
                self.index += 1;
                self.upper_nibble = false;

                byte >> 4
            } else {
                // Advance the iterator to the next nibble
                self.upper_nibble = true;

                byte & 0b1111
            };

            Some(nibble)
        } else {
            None
        }
    }
}

impl<'a, const N: usize> IntoIterator for &'a NibbleArray<N> {
    type IntoIter = Iter<'a, N>;
    type Item = u8;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}
