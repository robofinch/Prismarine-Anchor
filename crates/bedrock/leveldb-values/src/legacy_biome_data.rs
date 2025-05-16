use std::array;
use std::borrow::Cow;

use zerocopy::{transmute, transmute_ref, transmute_mut};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};


/// Variant of `LegacyBiomeIds` using 16-bit biomes instead of 8-bit biomes.
///
/// At some point (in versions after `LegacyData2D` and `Data2D` stopped being used normally),
/// biome IDs were changed from 8 bits to 16 bits. This likely occurred either when
/// `Data3D` was introduced, or in 1.21.40.
///
/// The inner array is indexed by Z values. The outer array is indexed by X values.
/// Therefore, the correct indexing order is `biome_ids.0[X][Z]`.
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone)]
pub struct NewLegacyBiomeIds(pub [[u16; 16]; 16]);

impl NewLegacyBiomeIds {
    /// The data should be in ZX order (Z increments first)
    #[inline]
    pub fn from_flattened(biome_ids: [u16; 256]) -> Self {
        Self(transmute!(biome_ids))
    }

    /// The data should be in ZX order (Z increments first). There are, semantically,
    /// 256 values, which are each little-endian u16's.
    #[inline]
    pub fn from_flattened_le_bytes(biome_ids: [u8; 512]) -> Self {
        let biome_ids: [[u8; 2]; 256] = transmute!(biome_ids);
        Self::from_flattened(biome_ids.map(u16::from_le_bytes))
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

/// The inner array is indexed by Z values. The outer array is indexed by X values.
/// Therefore, the correct indexing order is `biome_ids.0[X][Z]`.
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone)]
pub struct LegacyBiomeIds(pub [[u8; 16]; 16]);

impl LegacyBiomeIds {
    #[inline]
    pub fn from_flattened(biome_ids: [u8; 256]) -> Self {
        Self(transmute!(biome_ids))
    }

    /// Gets the data in ZX order (Z increments first)
    #[inline]
    pub fn flattened(self) -> [u8; 256] {
        transmute!(self.0)
    }

    /// Gets the data in ZX order (Z increments first)
    #[inline]
    pub fn flattened_ref(&self) -> &[u8; 256] {
        transmute_ref!(&self.0)
    }

    /// Gets the data in ZX order (Z increments first)
    #[inline]
    pub fn flattened_mut(&mut self) -> &mut [u8; 256] {
        transmute_mut!(&mut self.0)
    }
}

/// The inner array is indexed by Z values. The outer array is indexed by X values.
/// Therefore, the correct indexing order is `biome_colors.0[X][Z]`.
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq))]
#[derive(Debug, Clone)]
pub struct LegacyBiomeColors(pub [[LegacyBiomeColor; 16]; 16]);

impl LegacyBiomeColors {
    /// Gets the data in ZX order (Z increments first)
    #[inline]
    pub fn flattened(self) -> [LegacyBiomeColor; 256] {
        transmute!(self.0)
    }

    /// Gets the data in ZX order (Z increments first)
    #[inline]
    pub fn flattened_ref(&self) -> &[LegacyBiomeColor; 256] {
        transmute_ref!(&self.0)
    }

    /// Gets the data in ZX order (Z increments first)
    #[inline]
    pub fn flattened_mut(&mut self) -> &mut [LegacyBiomeColor; 256] {
        transmute_mut!(&mut self.0)
    }
}

#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(FromBytes, Immutable, IntoBytes, KnownLayout, Debug, Clone, Copy)]
pub struct LegacyBiomeColor {
    pub red:   u8,
    pub green: u8,
    pub blue:  u8,
}

pub fn biome_data_to_parts(biome_data: &[u8; 1024]) -> (LegacyBiomeIds, LegacyBiomeColors) {
    let biome_data: &[[u8; 4]; 256] = transmute_ref!(biome_data);

    let biome_ids = biome_data.map(|[id, ..]| id);
    let biome_ids = transmute!(biome_ids);
    let biome_ids = LegacyBiomeIds(biome_ids);

    let biome_colors: &[[[u8; 4]; 16]; 16] = transmute_ref!(biome_data);
    let biome_colors = biome_colors.map(|inner_arr| {
        inner_arr.map(|[_, red, green, blue]| LegacyBiomeColor { red, green, blue })
    });
    let biome_colors = LegacyBiomeColors(biome_colors);

    (biome_ids, biome_colors)
}

pub fn biome_data_from_parts(
    biome_ids:    &LegacyBiomeIds,
    biome_colors: &LegacyBiomeColors,
) -> [u8; 1024] {
    let biome_ids = biome_ids.flattened_ref();
    let biome_colors = biome_colors.flattened_ref();

    let biomes: [[u8; 4]; 256] = array::from_fn(|idx| {
        let id = biome_ids[idx];
        let color = biome_colors[idx];
        [id, color.red, color.green, color.blue]
    });

    transmute!(biomes)
}
