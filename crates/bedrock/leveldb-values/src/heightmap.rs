use std::borrow::Cow;

use zerocopy::{transmute, transmute_mut, transmute_ref};


/// The inner array is indexed by Z values. The outer array is indexed by X values.
/// Therefore, the correct indexing order is `heightmap.0[X][Z]`.
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq))]
#[derive(Debug, Clone)]
pub struct Heightmap(pub [[u16; 16]; 16]);

impl Heightmap {
    /// The data should be in ZX order (Z increments first)
    #[inline]
    pub fn from_flattened(heightmap: [u16; 256]) -> Self {
        Self(transmute!(heightmap))
    }

    /// The data should be in ZX order (Z increments first). There are, semantically,
    /// 256 values, which are each little-endian u16's.
    #[inline]
    pub fn from_flattened_le_bytes(heightmap: [u8; 512]) -> Self {
        let heightmap: [[u8; 2]; 256] = transmute!(heightmap);
        Self::from_flattened(heightmap.map(u16::from_le_bytes))
    }

    /// Gets the data in ZX order (Z increments first)
    #[inline]
    pub fn flattened(self) -> [u16; 256] {
        transmute!(self.0)
    }

    /// Gets the data in ZX order (Z increments first)
    #[inline]
    pub fn flattened_ref(&self) -> &[u16; 256] {
        transmute_ref!(&self.0)
    }

    /// Gets the data in ZX order (Z increments first)
    #[inline]
    pub fn flattened_mut(&mut self) -> &mut [u16; 256] {
        transmute_mut!(&mut self.0)
    }

    /// Gets the data in ZX order (Z increments first). There are, semantically,
    /// 256 values, which are each little-endian u16's.
    #[inline]
    pub fn flattened_le_bytes(self) -> [u8; 512] {
        transmute!(self.flattened().map(u16::to_le_bytes))
    }

    /// Gets the data in ZX order (Z increments first). There are, semantically,
    /// 256 values, which are each little-endian u16's.
    #[inline]
    pub fn flattened_le_bytes_cow(&self) -> Cow<'_, [u8; 512]> {
        #[cfg(target_endian = "little")]
        {
            let flattened_le_bytes_ref = transmute_ref!(self.flattened_ref());
            Cow::Borrowed(flattened_le_bytes_ref)
        }
        #[cfg(not(target_endian = "little"))]
        {
            Cow::Owned(self.clone().flattened_le_bytes())
        }
    }
}
