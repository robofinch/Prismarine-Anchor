use std::{array, slice};
use std::{collections::BTreeSet, convert::Infallible, error::Error as StdError};
use std::io::{Error as IoError, Read};

use thiserror::Error;
use zerocopy::transmute;


// ================================
//  Structs
// ================================

/// Palettized storage for one subchunk, with other two special cases.
#[derive(Debug, Clone)]
pub enum PalettizedStorage<T> {
    Empty,
    Uniform(T),
    Palettized(PalettizedSubchunk<T>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaletteHeader {
    pub palette_type:   PaletteType,
    pub bits_per_index: HeaderBitsPerIndex,
}

#[derive(Debug, Clone)]
pub struct PalettizedSubchunk<T> {
    bits_per_index: PaletteBitsPerIndex,
    packed_indices: Vec<u32>,
    palette:        Vec<T>,
}

/// Either the (nonzero) number of bits per index into a palette,
/// or a special case for when the subchunk is uniformly a single value
/// or is empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderBitsPerIndex {
    Empty,
    Uniform,
    Palettized(PaletteBitsPerIndex),
}

/// The (nonzero) number of bits per index into a palette.
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
    // NOTE: some code below assumes that the maximum value is at most 16,
    // so if that changes, it will need to be modified.
}

/// In practice, `Data3D` data uses only `Runtime`, and `SubchunkBlocks` data uses
/// `Persistent`. There could be exceptions, but they should be rare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteType {
    Persistent,
    Runtime,
}

#[derive(Error, Debug)]
pub enum PalettizedStorageParseError<E: StdError> {
    #[error("A palette parser was expected to return a palette of length 1, but did not")]
    InvalidPaletteParser,
    #[error("error while parsing a palette of length {1}: {0}")]
    PaletteParseError(E, usize),
    #[error("an error occured while reading a palette's length and converting it to a usize")]
    PaletteLenError,
    #[error("an error occurred while checking a nonempty, non-uniform palettized subchunk: {0}")]
    CheckError(#[from] PalettizedSubchunkCheckError),
    #[error("an IO error occured while reading indices: {0}")]
    IndexReadError(#[from] IoError)
}

#[derive(Error, Debug, Clone)]
pub enum PalettizedSubchunkCheckError {
    #[error("a subchunk palette had an invalid length of {0}")]
    InvalidPaletteLen(usize),
    #[error("a paletted subchunk was expected to have {expected} u32 blocks, but had {received}")]
    InvalidPaletteIndicesLen {
        expected: usize,
        received: usize,
    },
    #[error("a paletted subchunk had index {index}, but the palette had length {palette_len}")]
    IndexTooLarge {
        palette_len: usize,
        index: u32,
    }
}

#[derive(Error, Debug)]
pub enum PaletteHeaderParseError {
    #[error("invalid bits-per-index for a palette: {0}")]
    InvalidBitsPerIndex(u8),
    #[error(transparent)]
    Io(#[from] IoError),
}

// ================================
//  Standalone Functions
// ================================

/// Attempt to read exactly `num_u32s`-many little-endian `u32`s.
pub fn read_le_u32s<R: Read>(mut reader: R, num_u32s: usize) -> Result<Vec<u32>, IoError> {
    let mut u32s = vec![0; num_u32s * 4];
    reader.read_exact(&mut u32s)?;

    let mut u32s = u32s.into_iter();
    Ok(
        (0..num_u32s)
            .map(|_| {
                // We know that u32s has length exactly num_u32s * 4,
                // so these unwraps succeed.
                #[expect(
                    clippy::unwrap_used,
                    reason = "we call `.next().unwrap()` exactly `num_32s * 4` times",
                )]
                let block = [
                    u32s.next().unwrap(),
                    u32s.next().unwrap(),
                    u32s.next().unwrap(),
                    u32s.next().unwrap(),
                ];
                u32::from_le_bytes(block)
            })
            .collect(),
    )
}

/// Write many `u32`s to little-endian bytes. Infallible.
#[inline]
pub fn write_le_u32s(u32s: &[u32], bytes: &mut Vec<u8>) -> Result<(), Infallible> {
    for block in u32s {
        bytes.extend(block.to_le_bytes());
    }
    Ok(())
}

// ================================
//  Impls
// ================================

impl<T> PalettizedStorage<T> {
    pub fn parse<'a, R, F, FError>(
        reader:                  &'a mut R,
        bits_per_index:          HeaderBitsPerIndex,
        mut parse_palette:       F,
    ) -> Result<Self, PalettizedStorageParseError<FError>>
    where
        R: Read,
        F: FnMut(&'a mut R, usize) -> Result<Vec<T>, FError>,
        FError: StdError,
    {
        match bits_per_index {
            HeaderBitsPerIndex::Empty => Ok(Self::Empty),
            HeaderBitsPerIndex::Uniform => {
                // There are no indices, and the palette should have length 1
                let mut palette = parse_palette(reader, 1)
                    .map_err(|err| PalettizedStorageParseError::PaletteParseError(err, 1))?;
                if palette.len() == 1 {
                    Ok(Self::Uniform(palette.swap_remove(0)))
                } else {
                    Err(PalettizedStorageParseError::InvalidPaletteParser)
                }
            }
            HeaderBitsPerIndex::Palettized(bits_per_index) => {
                let packed_indices_len = bits_per_index.num_u32s_for_4096_indices();

                let packed_indices = read_le_u32s::<&mut R>(reader, packed_indices_len)?;

                let mut palette_len = [0; 4];

                #[expect(clippy::map_err_ignore, reason = "exact error probably doesn't matter")]
                reader.read_exact(&mut palette_len)
                    .map_err(|_| PalettizedStorageParseError::PaletteLenError)?;

                let palette_len = u32::from_le_bytes(palette_len);

                #[expect(clippy::map_err_ignore, reason = "exact error probably doesn't matter")]
                let palette_len = usize::try_from(palette_len)
                    .map_err(|_| PalettizedStorageParseError::PaletteLenError)?;

                let palette = parse_palette(reader, palette_len)
                    .map_err(|err| PalettizedStorageParseError::PaletteParseError(
                        err,
                        palette_len,
                    ))?;

                Ok(Self::Palettized(PalettizedSubchunk::new_packed_checked(
                    bits_per_index,
                    packed_indices,
                    palette,
                )?))
            }
        }
    }

    #[inline]
    pub fn bits_per_index(&self) -> HeaderBitsPerIndex {
        match self {
            Self::Empty => HeaderBitsPerIndex::Empty,
            Self::Uniform(_) => HeaderBitsPerIndex::Uniform,
            Self::Palettized(PalettizedSubchunk { bits_per_index, .. }) => {
                HeaderBitsPerIndex::Palettized(*bits_per_index)
            }
        }
    }

    pub fn extend_serialized<E, F>(
        &self,
        bytes:                    &mut Vec<u8>,
        palette_type:             PaletteType,
        reserve:                  bool,
        mut write_palette_to_vec: F,
    ) -> Result<(), E>
    where
        F: FnMut(&[T], &mut Vec<u8>) -> Result<(), E>,
    {
        let header = PaletteHeader {
            palette_type,
            bits_per_index: self.bits_per_index(),
        };

        match self {
            Self::Empty => {
                bytes.push(u8::from(header));
            }
            Self::Uniform(value) => {
                if reserve {
                    bytes.reserve(1 + size_of::<T>());
                }
                bytes.push(u8::from(header));
                write_palette_to_vec(slice::from_ref(value), bytes)?;
            }
            Self::Palettized(palettized) => {
                let bits_per_index = u8::from(palettized.bits_per_index);
                let palette_version = 1;
                let header = (bits_per_index << 1) | palette_version;

                let palette_len = palettized.palette_len();
                let (_, packed_indices, palette) = palettized.packed();

                if reserve {
                    let num_u32s = palettized.packed_indices_len();
                    let reserve_len = 1 + (num_u32s * 4) + 4 + (palette_len * 4);
                    // If this panics, then the reserve would definitely panic
                    let reserve_len = usize::try_from(reserve_len)
                        .expect("PalettizedStorage consumed too much memory for the hardware");
                    bytes.reserve(reserve_len);
                }

                bytes.push(header);
                // write_le_u32s is infallible
                write_le_u32s(packed_indices.as_slice(), bytes).unwrap();
                bytes.extend(palette_len.to_le_bytes());
                write_palette_to_vec(palette.as_slice(), bytes)?;
            }
        }

        Ok(())
    }

    #[inline]
    pub fn to_bytes<E, F>(
        &self,
        palette_type:             PaletteType,
        reserve:                  bool,
        write_palette_to_vec:     F,
    ) -> Result<Vec<u8>, E>
    where
        F: FnMut(&[T], &mut Vec<u8>) -> Result<(), E>,
    {
        let mut bytes = Vec::new();
        self.extend_serialized::<E, F>(
            &mut bytes,
            palette_type,
            reserve,
            write_palette_to_vec,
        )?;
        Ok(bytes)
    }
}

impl PaletteHeader {
    /// Parses the `PaletteType` and bits per index of the palettized storage,
    /// including special cases.
    pub fn parse_header<R: Read>(mut reader: R) -> Result<Self, PaletteHeaderParseError> {
        let mut header = [0; 1];
        reader.read_exact(&mut header)?;
        let header = header[0];
        let palette_type = PaletteType::from(header & 1);
        let bits_per_index = HeaderBitsPerIndex::parse(header >> 1)
            .ok_or(PaletteHeaderParseError::InvalidBitsPerIndex(header >> 1))?;

        Ok(Self {
            palette_type,
            bits_per_index,
        })
    }
}

impl From<PaletteHeader> for u8 {
    #[expect(clippy::use_self, reason = "clarity and brevity, it's a u8")]
    #[inline]
    fn from(value: PaletteHeader) -> Self {
        let palette_type   = u8::from(value.palette_type);
        let bits_per_index = u8::from(value.bits_per_index);
        (bits_per_index << 1) | (palette_type & 1)
    }
}

impl<T> PalettizedSubchunk<T> {
    /// The provided data should be for one subchunk in YZX order (Y increments first).
    /// Intended for use when `T` is `Copy`.
    ///
    /// Any needed padding bits will be zeroes.
    pub fn new_unpacked_flattened_copy(unpacked_data: [T; 4096]) -> Self
    where
        T: Ord + Copy,
    {
        let mut palette = BTreeSet::new();

        for &value in &unpacked_data {
            palette.insert(value);
        }

        let palette: Vec<T> = palette.into_iter().collect();

        if palette.len() == 1 {
            // We can, as a special case, do this very quickly.

            let bits_per_index = PaletteBitsPerIndex::One;

            // The number of u32 blocks is the number of indices, 4096, divided by
            // the number of 1-bit indices which can be stored in a 32-bit block, which is 32.
            let num_u32s = 4096 / 32;

            return Self {
                bits_per_index,
                packed_indices: vec![0; num_u32s],
                palette,
            };
        }

        // Note that we *cannot* have `palette.len() == 0`. We added things
        // to `palette`, so even if it was always the same value, there's at least 1.
        // We know `palette.len() > 0`, and is at most 4096, which is 2^12,
        // so it's less than 2 to the 16. Thus, the below does not panic.
        #[expect(
            clippy::unwrap_used,
            reason = "`new_from_usize` returns `Some` since 0 < palette.len() <= 4096 == (1 << 12)",
        )]
        let bits_per_index = PaletteBitsPerIndex::new_from_usize(palette.len()).unwrap();

        let num_u32s = bits_per_index.num_u32s_for_4096_indices();
        // `2^bits_per_index - 1` has the least-significant `bits_per_index` bits set.
        // This fits in a u32, which is used below.
        let index_mask = (1_usize << u8::from(bits_per_index)) - 1;

        let mut packed_indices = Vec::with_capacity(num_u32s);
        let mut u32_block = 0_u32;
        let mut num_indices_in_block = 0;

        for value in unpacked_data {
            u32_block <<= u8::from(bits_per_index);

            // This unwrapping does not panic since we added every value
            // to the BTreeSet which was then converted to a sorted Vec.
            // Therefore every attempt to search for a value succeeds.
            #[expect(
                clippy::unwrap_used,
                reason = "we inserted everything in `unpacked_data` into `palette`",
            )]
            let index = palette.binary_search(&value).unwrap();

            // index_mask fits in a u32, so this doesn't overflow.
            let index = (index & index_mask) as u32;

            u32_block |= index;

            num_indices_in_block += 1;
            if num_indices_in_block >= bits_per_index.indices_per_u32() {
                packed_indices.push(u32_block);
                u32_block = 0;
                num_indices_in_block = 0;
            }
        }

        Self {
            bits_per_index,
            packed_indices,
            palette,
        }
    }

    /// The provided data should be for one subchunk in YZX order (Y increments first).
    /// If `T` is `Copy`, it is more efficient to use `new_unpacked_flattened_copy`.
    ///
    /// Any needed padding bits will be zeroes.
    pub fn new_unpacked_flattened(unpacked_data: [T; 4096]) -> Self
    where
        T: Ord,
    {
        // Note that we iterate over unpacked_data 3 times instead of 2 times,
        // and create a BTreeSet twice instead of once.

        let mut palette = BTreeSet::new();

        for value in &unpacked_data {
            palette.insert(value);
        }

        let palette: Vec<&T> = palette.into_iter().collect();

        if palette.len() == 1 {
            // We can, as a special case, do this very quickly.

            let bits_per_index = PaletteBitsPerIndex::One;

            // The number of u32 blocks is the number of indices, 4096, divided by
            // the number of 1-bit indices which can be stored in a 32-bit block, which is 32.
            let num_u32s = 4096 / 32;

            return Self {
                bits_per_index,
                packed_indices: vec![0; num_u32s],
                // Every value is the same, so we can choose a random one.
                #[expect(
                    clippy::unwrap_used,
                    reason = "`palette.len() == 1`, so we can call `.next().unwrap()` once",
                )]
                palette: vec![unpacked_data.into_iter().next().unwrap()],
            };
        }

        // Note that we *cannot* have `palette.len() == 0`. We added things
        // to `palette`, so even if it was always the same value, there's at least 1.
        // We know `palette.len() > 0`, and is at most 4096, which is 2^12,
        // so it's less than 2 to the 16. Thus, the below does not panic.
        #[expect(
            clippy::unwrap_used,
            reason = "`new_from_usize` returns `Some` since 0 < palette.len() <= 4096 == (1 << 12)",
        )]
        let bits_per_index = PaletteBitsPerIndex::new_from_usize(palette.len()).unwrap();

        let num_u32s = bits_per_index.num_u32s_for_4096_indices();
        // `2^bits_per_index - 1` has the least-significant `bits_per_index` bits set.
        // This fits in a u32, which is used below.
        let index_mask = (1_usize << u8::from(bits_per_index)) - 1;

        let mut packed_indices = Vec::with_capacity(num_u32s);
        let mut u32_block = 0_u32;
        let mut num_indices_in_block = 0;

        for value in &unpacked_data {
            u32_block <<= u8::from(bits_per_index);

            // This unwrapping does not panic since we added every value
            // to the BTreeSet which was then converted to a sorted Vec.
            #[expect(
                clippy::unwrap_used,
                reason = "we inserted everything in `unpacked_data` into `palette`",
            )]
            let index = palette.binary_search(&value).unwrap();

            // index_mask fits in a u32, so this doesn't overflow.
            let index = (index & index_mask) as u32;

            u32_block |= index;

            num_indices_in_block += 1;
            if num_indices_in_block >= bits_per_index.indices_per_u32() {
                packed_indices.push(u32_block);
                u32_block = 0;
                num_indices_in_block = 0;
            }
        }

        let mut actual_palette = BTreeSet::new();
        for value in unpacked_data {
            actual_palette.insert(value);
        }

        Self {
            packed_indices,
            bits_per_index,
            palette: actual_palette.into_iter().collect(),
        }
    }

    /// Creates a new `PalettizedSubchunk` struct which stores data of a subchunk
    /// in a condensed way. This performs the following checks,
    /// which includes iterating over all 4096 indices:
    ///
    /// `packed_indices` has length exactly `bits_per_index.num_u32s_for_4096_indices()`;
    ///
    /// `palette` has length greater than the maximum index in `packed_indices`,
    /// and is at most `4096`.
    pub fn new_packed_checked(
        bits_per_index: PaletteBitsPerIndex,
        packed_indices: Vec<u32>,
        palette:        Vec<T>,
    ) -> Result<Self, PalettizedSubchunkCheckError> {
        if packed_indices.len() != bits_per_index.num_u32s_for_4096_indices() {
            return Err(PalettizedSubchunkCheckError::InvalidPaletteIndicesLen {
                expected: bits_per_index.num_u32s_for_4096_indices(),
                received: packed_indices.len(),
            });
        }

        let palette_len = palette.len();
        if palette_len == 0 || palette_len > 4096 {
            return Err(PalettizedSubchunkCheckError::InvalidPaletteLen(palette_len));
        }

        let max_permissible_index = palette_len - 1;
        // This unwrap does not panic, by the above checks on palette_len.
        #[expect(
            clippy::unwrap_used,
            reason = "`palette.len() <= 4096 = (1 << 12)`, so it fits in 32 bits",
        )]
        let max_permissible_index = u32::try_from(max_permissible_index).unwrap();

        let indices_per_u32 = bits_per_index.indices_per_u32();
        // let padding_bits = bits_per_index.padding_bits();
        // `2^bits_per_index - 1` has the least-significant `bits_per_index` bits set.
        let index_mask = (1_u32 << u8::from(bits_per_index)) - 1;

        let mut packed_index_iter = packed_indices.iter();
        if let Some(&last_dword) = packed_index_iter.next_back() {
            let mut last_dword = last_dword;

            for _ in 0..bits_per_index.indices_in_last_u32() {
                let index = last_dword & index_mask;
                if index > max_permissible_index {
                    return Err(PalettizedSubchunkCheckError::IndexTooLarge {
                        palette_len,
                        index,
                    });
                }

                last_dword >>= u8::from(bits_per_index);
            }
        }

        for &dword in packed_index_iter {
            let mut dword = dword;

            for _ in 0..indices_per_u32 {
                let index = dword & index_mask;
                if index > max_permissible_index {
                    return Err(PalettizedSubchunkCheckError::IndexTooLarge {
                        palette_len,
                        index,
                    });
                }

                dword >>= u8::from(bits_per_index);
            }
        }

        // All the checks are done.
        Ok(Self::new_packed_unchecked(
            bits_per_index,
            packed_indices,
            palette,
        ))
    }

    /// Creates a new `PalettizedSubchunk` struct which stores the data of a subchunk
    /// in a condensed way. This performs no checks for correctness.
    /// While no memory unsoundness will occur if assumptions are not met, panics
    /// may occur, or invalid data that Minecraft may reject may be written.
    /// It is assumed that:
    ///
    /// `packed_indices` has length exactly `bits_per_index.num_u32s_for_4096_indices()`;
    ///
    /// `palette` has length greater than the maximum index in `packed_indices`,
    /// and is at most `4096`;
    #[inline]
    pub fn new_packed_unchecked(
        bits_per_index: PaletteBitsPerIndex,
        packed_indices: Vec<u32>,
        palette:        Vec<T>,
    ) -> Self {
        Self {
            bits_per_index,
            packed_indices,
            palette,
        }
    }

    /// Returns the length of `packed_indices` as a `u32`. By the assumptions
    /// of this struct, that value's length is at most 2048 and thus fits in a `u32`.
    /// Note that `bits_per_index.num_u32s_for_4096_indices() <= 2048` for all possible
    /// values of `bits_per_index: PaletteBitsPerIndex`.
    #[inline]
    pub fn packed_indices_len(&self) -> u32 {
        u32::try_from(self.packed_indices.len())
            .expect("a PalettizedSubchunk' packed_indices should have length at most 2048")
    }

    /// Returns the length of `palette` as a `u32`. By the assumptions
    /// of this struct, that value's length is at most 4096 and thus fits in a `u32`.
    #[inline]
    pub fn palette_len(&self) -> u32 {
        u32::try_from(self.palette.len())
            .expect("a PalettizedSubchunk' palette should have length at most 4096")
    }

    /// Return the `bits_per_index` and references to the
    /// `packed_indices` and `palette` of
    /// this `PalettizedSubchunk`.
    #[inline]
    pub fn packed(&self) -> (PaletteBitsPerIndex, &Vec<u32>, &Vec<T>) {
        (self.bits_per_index, &self.packed_indices, &self.palette)
    }

    /// Returns the `bits_per_index`, `packed_indices`, and `palette` of
    /// this `PalettizedSubchunk`, consuming it.
    #[inline]
    pub fn into_packed(self) -> (PaletteBitsPerIndex, Vec<u32>, Vec<T>) {
        (self.bits_per_index, self.packed_indices, self.palette)
    }

    /// Compute the subchunk data which is stored in a condensed way in this struct.
    /// The output data is in YZX order (Y increments first).
    pub fn unpacked_flattened(&self) -> [T; 4096]
    where
        T: Clone,
    {
        // `2^bits_per_index - 1` has the least-significant `bits_per_index` bits set.
        let index_mask = (1_u32 << u8::from(self.bits_per_index)) - 1;

        let mut packed_ids = self.packed_indices.iter();
        let mut u32_block = 0;
        let mut num_indices_in_block = 0;

        array::from_fn(|_| {
            if num_indices_in_block == 0 {
                // Since there must be enough blocks for all 4096 indices, this does not panic.
                #[expect(
                    clippy::unwrap_used,
                    reason = "we should have \
                    `packed_ids.len() == self.bits_per_index.num_u32s_for_4096_indices()`",
                )]
                // The extra block is because the compiler
                // complained about applying an attribute to an expression
                {
                    u32_block = *packed_ids.next().unwrap();
                };

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

            self.palette[usize::from(index)].clone()
        })
    }

    /// The provided data should be for one subchunk,
    /// where Y is the innermost index, Z is the middle index, and X is the outermost index.
    /// In other words, the correct indexing order should be `unpacked_data[X][Z][Y]`.
    ///
    /// Any needed padding bits will be zeroes.
    ///
    /// When `T` is `u32`, the `new_unpacked_u32s` function is more efficient.
    pub fn new_unpacked(unpacked_data: [[[T; 16]; 16]; 16]) -> Self
    where
        T: Ord,
    {
        let unpacked: Vec<_> = unpacked_data // [[[T; 16]; 16]; 16]
            .into_iter() // IntoIter<[[T; 16]; 16], 16>
            .flatten()   // impl Iterator<Item = [T; 16]>
            .flatten()   // impl Iterator<Item = T>
            .collect();
        let unpacked: [_; 4096] = match unpacked.try_into() {
            Ok(unpacked) => unpacked,
            Err(_) => unreachable!("16*16*16 == 4096, so the iterator is of the correct length"),
        };
        Self::new_unpacked_flattened(unpacked)
    }

    /// Compute the subchunk data which is stored in a condensed way in this struct.
    /// In the output data, Y is the innermost index, Z is the middle index,
    /// and X is the outermost index.
    /// In other words, the correct indexing order should be `self.unpacked()[X][Z][Y]`.
    ///
    /// When `T` is `u32`, the `unpacked_u32s` function is more efficient.
    pub fn unpacked(&self) -> [[[T; 16]; 16]; 16]
    where
        T: Clone,
    {
        let unpacked: [_; 4096] = self.unpacked_flattened();
        let mut unpacked = unpacked.into_iter();
        array::from_fn(|_| {
            array::from_fn(|_| {
                array::from_fn(|_| {
                    // This doesn't panic since 4096 == 16^3 and `unpacked.len() == 4096`
                    #[expect(
                        clippy::unwrap_used,
                        reason = "we call `.next().unwrap()` exactly `4096 == 16*16*16` times",
                    )]
                    unpacked.next().unwrap()
                })
            })
        })
    }
}

impl PalettizedSubchunk<u32> {
    /// The provided data should be for one subchunk,
    /// where Y is the innermost index, Z is the middle index, and X is the outermost index.
    /// In other words, the correct indexing order should be `unpacked_data[X][Z][Y]`.
    ///
    /// Any needed padding bits will be zeroes.
    #[inline]
    pub fn new_unpacked_u32s(unpacked_data: [[[u32; 16]; 16]; 16]) -> Self {
        Self::new_unpacked_flattened(transmute!(unpacked_data))
    }

    /// Compute the subchunk data which is stored in a condensed way in this struct.
    /// In the output data, Y is the innermost index, Z is the middle index,
    /// and X is the outermost index.
    /// In other words, the correct indexing order should be `self.unpacked()[X][Z][Y]`.
    #[inline]
    pub fn unpacked_u32s(&self) -> [[[u32; 16]; 16]; 16] {
        transmute!(self.unpacked_flattened())
    }
}

impl HeaderBitsPerIndex {
    #[inline]
    pub fn parse(bits_per_index: u8) -> Option<Self> {
        Some(match bits_per_index {
            127   => Self::Empty,
            0     => Self::Uniform,
            other => Self::Palettized(PaletteBitsPerIndex::new_exact(other)?),
        })
    }
}

impl From<HeaderBitsPerIndex> for u8 {
    #[inline]
    fn from(value: HeaderBitsPerIndex) -> Self {
        match value {
            HeaderBitsPerIndex::Empty   => 127,
            HeaderBitsPerIndex::Uniform => 0,
            HeaderBitsPerIndex::Palettized(bits_per_index) => Self::from(bits_per_index),
        }
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
    #[inline]
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
    #[inline]
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
            _  => return None,
        })
    }

    /// The number of indices which fit in one u32 (4 bytes, a `u32`).
    #[inline]
    pub fn indices_per_u32(self) -> u8 {
        32 / u8::from(self)
    }

    /// The number of u32s (4 bytes, a `u32`) it takes to store 4096 indices
    /// with the given number of bits per index.
    #[inline]
    pub fn num_u32s_for_4096_indices(self) -> usize {
        4096_usize.div_ceil(usize::from(self.indices_per_u32()))
    }

    /// Some bit widths require there to be padding when indices are packed into a u32,
    /// when `self.indices_per_u32() * u8::from(self) < 32`.
    /// This occurs when `32` is not evenly divisible by this number of bits per index.
    ///
    /// This returns the necessary number of padding bits (possibly `0`), such that
    /// `self.indices_per_u32() * u8::from(self) + self.padding_bits() == 32`.
    #[inline]
    pub fn padding_bits(self) -> u8 {
        if [3, 5, 6].contains(&u8::from(self)) {
            2
        } else {
            0
        }
    }

    /// For some bit widths, the number of indices per u32 does not evenly divide 4096,
    /// tht total number of indices. In this case, the final u32 block will contain
    /// fewer indices than normal. This function returns the number of indices in that last block
    /// (which in some cases is simply `self.indices_per_u32()`).
    #[inline]
    pub fn indices_in_last_u32(self) -> u8 {
        let remainder = 4096_u32 % u32::from(self.indices_per_u32());
        if remainder == 0 {
            self.indices_per_u32()
        } else {
            // Since we took a number modulo a u8 value, this fits in a u8 without overflowing
            remainder as u8
        }
    }
}

impl From<PaletteBitsPerIndex> for u8 {
    #[inline]
    fn from(value: PaletteBitsPerIndex) -> Self {
        match value {
            PaletteBitsPerIndex::One     => 1,
            PaletteBitsPerIndex::Two     => 2,
            PaletteBitsPerIndex::Three   => 3,
            PaletteBitsPerIndex::Four    => 4,
            PaletteBitsPerIndex::Five    => 5,
            PaletteBitsPerIndex::Six     => 6,
            PaletteBitsPerIndex::Eight   => 8,
            PaletteBitsPerIndex::Sixteen => 16,
        }
    }
}

impl From<u8> for PaletteType {
    #[inline]
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Persistent,
            _ => Self::Runtime,
        }
    }
}

impl From<PaletteType> for u8 {
    #[inline]
    fn from(value: PaletteType) -> Self {
        match value {
            PaletteType::Persistent => 0,
            PaletteType::Runtime    => 1,
        }
    }
}
