use std::io::{Cursor, Read};

use subslice_to_array::SubsliceToArray as _;
use vecmap::VecMap;

use prismarine_anchor_mc_datatypes::identifier::{IdentifierParseOptions, NamespacedIdentifier};
use prismarine_anchor_util::u64_equals_usize;

use crate::{block_volume::BlockVolume, ValueToBytesOptions};


#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq))]
#[derive(Debug, Clone)]
pub enum AabbVolumes {
    /// First used in 1.21.20 (or, some preview of 1.21.20)
    V1(AabbVolumesV1),
}

impl AabbVolumes {
    #[inline]
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() < 4 {
            return None;
        }

        let version = u32::from_le_bytes(value.subslice_to_array::<0, 4>());

        match version {
            1 => Some(Self::V1(AabbVolumesV1::parse(value)?)),
            _ => None,
        }
    }

    #[inline]
    pub fn extend_serialized(
        &self,
        bytes: &mut Vec<u8>,
        opts: ValueToBytesOptions,
    ) -> Result<(), VolumesToBytesError> {
        match self {
            Self::V1(volumes) => volumes.extend_serialized(bytes, opts),
        }
    }

    #[inline]
    pub fn to_bytes(&self, opts: ValueToBytesOptions) -> Result<Vec<u8>, VolumesToBytesError> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes, opts)?;
        Ok(bytes)
    }
}

// TODO: make a more strictly-typed version of this
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq))]
#[derive(Debug, Clone)]
pub struct AabbVolumesV1 {
    /// Map with `structure_id` keys (the IDs are used in `DynamicSpawnArea` and `StaticSpawnArea`)
    pub structure_types:     VecMap<u32, NamespacedIdentifier>,
    // TODO: does every structure need to have at least one bounding box?
    // will Minecraft simply remove unneeded structures with no bounding boxes?
    // I have the impression that the bounding box map is *probably* the most crucial one.
    /// Map with `box_id` keys (linking a `BlockVolume` to a `DynamicSpawnArea`
    /// or `StaticSpawnArea`)
    pub bounding_boxes:      VecMap<u32, BlockVolume>,
    // TODO: is *every* bounding box either a dynamic or static spawn area, and not both?
    // does Minecraft error if one is None or Both?
    /// Map with `box_id` keys (linking a `BlockVolume` to a `DynamicSpawnArea`
    /// or `StaticSpawnArea`)
    pub dynamic_spawn_areas: VecMap<u32, DynamicSpawnArea>,
    /// Map with `box_id` keys (linking a `BlockVolume` to a `DynamicSpawnArea`
    /// or `StaticSpawnArea`)
    pub static_spawn_areas:  VecMap<u32, StaticSpawnArea>,
}

impl AabbVolumesV1 {
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() < 4 {
            return None;
        }

        let version = u32::from_le_bytes(value.subslice_to_array::<0, 4>());
        if version != 1 {
            return None;
        }

        let mut reader = Cursor::new(&value[4..]);

        let structure_types_len = read_len(&mut reader)?;
        let mut structure_types = VecMap::with_capacity(structure_types_len);

        for _ in 0..structure_types_len {
            let structure_id = read_u32(&mut reader)?;
            let name_len = usize::from(read_u16(&mut reader)?);

            let mut name = vec![0; name_len];
            reader.read_exact(&mut name).ok()?;
            let name = String::from_utf8(name).ok()?;

            let opts = IdentifierParseOptions {
                default_namespace:          None,
                java_character_constraints: false,
            };
            let structure_identifier = NamespacedIdentifier::parse_string(name, opts).ok()?;

            if structure_types.insert(structure_id, structure_identifier).is_some() {
                // There shouldn't have been duplicate keys
                return None;
            }
        }

        let bounding_boxes = read_map(&mut reader, BlockVolume::parse)?;
        let dynamic_spawn_areas = read_map(&mut reader, |value: [u8; 8]| {
            let structure_id      = u32::from_le_bytes(value.subslice_to_array::<0, 4>());
            let full_bounding_box = u32::from_le_bytes(value.subslice_to_array::<4, 8>());

            let full_bounding_box = match full_bounding_box {
                0 => false,
                1 => true,
                _ => return None,
            };

            Some(DynamicSpawnArea {
                structure_id,
                full_bounding_box,
            })
        })?;
        let static_spawn_areas = read_map(&mut reader, |value: [u8; 12]| {
            let structure_id      = u32::from_le_bytes(value.subslice_to_array::<0, 4>());
            let height_difference = i32::from_le_bytes(value.subslice_to_array::<4, 8>());
            let full_bounding_box = u32::from_le_bytes(value.subslice_to_array::<8, 12>());

            let full_bounding_box = match full_bounding_box {
                0 => false,
                1 => true,
                _ => return None,
            };

            Some(StaticSpawnArea {
                structure_id,
                height_difference,
                full_bounding_box,
            })
        })?;

        if !u64_equals_usize(reader.position(), reader.get_ref().len()) {
            None
        } else {
            Some(Self {
                structure_types,
                bounding_boxes,
                dynamic_spawn_areas,
                static_spawn_areas,
            })
        }
    }

    pub fn extend_serialized(
        &self,
        bytes: &mut Vec<u8>,
        opts: ValueToBytesOptions,
    ) -> Result<(), VolumesToBytesError> {

        fn len(opts: ValueToBytesOptions, len: usize) -> Result<u32, VolumesToBytesError> {
            opts.handle_excessive_length
                .length_to_u32(len)
                .ok_or(VolumesToBytesError::ExcessiveMapLength)
                .map(|(len, _)| len)
        }

        // Try to error out early (if ever)
        let structure_types_len = len(opts, self.structure_types.len())?;
        let boxes_len           = len(opts, self.bounding_boxes.len())?;
        let dynamic_len         = len(opts, self.dynamic_spawn_areas.len())?;
        let static_len          = len(opts, self.static_spawn_areas.len())?;

        let bbox_data_len = 12
            + self.bounding_boxes.len()      * 28
            + self.dynamic_spawn_areas.len() * 12
            + self.static_spawn_areas.len()  * 16;

        bytes.reserve(8 + self.structure_types.len() * 2 + bbox_data_len);

        // Version
        extend_le(bytes, 1);

        // Structure types
        extend_le(bytes, structure_types_len);
        for (structure_id, identifier) in &self.structure_types {
            let identifier = identifier.to_string();

            let identifier_len = opts
                .handle_excessive_length
                .length_to_u16(identifier.len())
                .ok_or(VolumesToBytesError::ExcessiveStringLength)?
                .0;

            extend_le(bytes, *structure_id);
            bytes.extend(identifier_len.to_le_bytes());
            bytes.extend(identifier.as_bytes());
        }

        bytes.reserve(bbox_data_len);

        // Bounding boxes
        extend_le(bytes, boxes_len);
        for (box_id, volume) in &self.bounding_boxes {
            extend_le(bytes, *box_id);
            volume.extend_serialized(bytes);
        }

        // Dynamic spawn areas
        extend_le(bytes, dynamic_len);
        for (box_id, dynamic_area) in &self.dynamic_spawn_areas {
            let full_bounding_box = if dynamic_area.full_bounding_box {
                1
            } else {
                0
            };

            extend_le(bytes, *box_id);
            extend_le(bytes, dynamic_area.structure_id);
            extend_le(bytes, full_bounding_box);
        }

        // Static spawn areas
        extend_le(bytes, static_len);
        for (box_id, static_area) in &self.static_spawn_areas {
            let full_bounding_box = if static_area.full_bounding_box {
                1
            } else {
                0
            };

            extend_le(bytes, *box_id);
            extend_le(bytes, static_area.structure_id);
            bytes.extend(static_area.height_difference.to_le_bytes());
            extend_le(bytes, full_bounding_box);
        }

        Ok(())
    }

    #[inline]
    pub fn to_bytes(&self, opts: ValueToBytesOptions) -> Result<Vec<u8>, VolumesToBytesError> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes, opts)?;
        Ok(bytes)
    }

    // TODO: provide helper functions for adding/removing a structure or bounding box,
    // figure out what the invariants are, and enforce the invariants with greater encapsulation
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicSpawnArea {
    /// A key in a `structure_types` map which associates the `structure_id` to the structure's
    /// namespaced identifier.
    pub structure_id:      u32,
    /// Whether the structure bounding box is the full bounding box of the structure in the chunk
    /// (as opposed to being a piece of it).
    pub full_bounding_box: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticSpawnArea {
    /// A key in a `structure_types` map which associates the `structure_id` to the structure's
    /// namespaced identifier.
    pub structure_id:      u32,
    /// The bounding box where spawns can occur may be slightly different from the corresponding
    /// bounding box of the structure; in particular, the static spawn area may have a different
    /// height. This value is the spawn area's height minus the structure bounding box's height
    /// (so, if it is negative, the static spawn area is shorter than the full structure).
    ///
    /// The value `-3` is used to prevent pillagers and witches from spawning on the roof of a
    /// pillager outpost or witch hut; for most structures, this is `0`.
    pub height_difference: i32,
    /// Whether the structure bounding box is the full bounding box of the structure in the chunk
    /// (as opposed to being a piece of it).
    pub full_bounding_box: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum VolumesToBytesError {
    ExcessiveMapLength,
    ExcessiveStringLength,
}

#[inline]
fn read_u16<R: Read>(mut reader: R) -> Option<u16> {
    let mut buf = [0; 2];
    reader.read_exact(&mut buf).ok()?;
    Some(u16::from_le_bytes(buf))
}

#[inline]
fn read_u32<R: Read>(mut reader: R) -> Option<u32> {
    let mut buf = [0; 4];
    reader.read_exact(&mut buf).ok()?;
    Some(u32::from_le_bytes(buf))
}

#[inline]
fn read_len<R: Read>(reader: R) -> Option<usize> {
    let len = read_u32(reader)?;
    usize::try_from(len).ok()
}

// For some reason, this lint isn't triggered.
// #[expect(clippy::impl_trait_in_params, reason = "convenience in an internal function")]
fn read_map<T, const N: usize>(
    mut reader: impl Read,
    read_value: impl Fn([u8; N]) -> Option<T>,
) -> Option<VecMap<u32, T>> {
    let map_len = read_len(&mut reader)?;
    let mut map = VecMap::with_capacity(map_len);

    for _ in 0..map_len {
        let key = read_u32(&mut reader)?;
        let mut buf = [0; N];
        reader.read_exact(&mut buf).ok()?;
        let value = read_value(buf)?;

        if map.insert(key, value).is_some() {
            // There shouldn't have been duplicate keys
            return None;
        }
    }

    Some(map)
}

#[inline]
fn extend_le(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend(value.to_le_bytes());
}
