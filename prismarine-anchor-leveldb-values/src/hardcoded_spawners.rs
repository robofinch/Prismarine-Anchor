use std::num::NonZeroU32;

use prismarine_anchor_util::bijective_enum_map;
use prismarine_anchor_util::slice_to_array;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardcodedSpawners(pub Vec<(BlockVolume, HardcodedSpawnerType)>);

impl HardcodedSpawners {
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() < 4 {
            return None;
        }

        let num_entries = u32::from_le_bytes(slice_to_array::<0, 4, _, 4>(value));
        let num_entries = usize::try_from(num_entries).ok()?;

        if value.len() != 4 + num_entries * 25 {
            return None;
        }

        // We can process value in chunks of 25 bytes
        let mut value = &value[4..];
        let mut hardcoded_spawners = Vec::with_capacity(num_entries);
        for _ in 0..num_entries {
            let volume = slice_to_array::<0, 24, _, 24>(value);
            let spawner_type = value[24];
            value = &value[25..];

            hardcoded_spawners.push((
                BlockVolume::parse(volume)?,
                HardcodedSpawnerType::try_from(spawner_type).ok()?,
            ));
        }

        Some(Self(hardcoded_spawners))
    }

    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) {
        let (len, len_usize) = if size_of::<usize>() >= size_of::<u32>() {
            // Casting a u32 to a usize cannot overflow
            let len_u32 = u32::try_from(self.0.len()).unwrap_or(u32::MAX);
            (len_u32, len_u32 as usize)
        } else {
            // Casting a usize to a u32 cannot overflow
            (self.0.len() as u32, self.0.len())
        };

        bytes.reserve(4 + len_usize * 25);
        bytes.extend(len.to_le_bytes());
        for (volume, spawner_type) in self.0.iter().take(len_usize) {
            volume.extend_serialized(bytes);
            bytes.push(u8::from(*spawner_type));
        }
    }

    #[inline]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes);
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockVolume {
    pub low_x:   i32,
    pub low_y:   i32,
    pub low_z:   i32,
    pub width_x: NonZeroU32,
    pub width_y: NonZeroU32,
    pub width_z: NonZeroU32,
}

impl BlockVolume {
    pub fn parse(value: [u8; 24]) -> Option<Self> {
        let low_x  = i32::from_le_bytes([value[0],  value[ 1], value[ 2], value[ 3]]);
        let low_y  = i32::from_le_bytes([value[4],  value[ 5], value[ 6], value[ 7]]);
        let low_z  = i32::from_le_bytes([value[8],  value[ 9], value[10], value[11]]);
        let high_x = i32::from_le_bytes([value[12], value[13], value[14], value[15]]);
        let high_y = i32::from_le_bytes([value[16], value[17], value[18], value[19]]);
        let high_z = i32::from_le_bytes([value[20], value[21], value[22], value[23]]);

        if low_x <= high_x && low_y <= high_y && low_z <= high_z {
            #[expect(
                clippy::unwrap_used,
                reason = "`.saturating_add(1)` ensures that the values are nonzero",
            )]
            Some(Self {
                low_x,
                low_y,
                low_z,
                width_x: NonZeroU32::new(high_x.abs_diff(low_x).saturating_add(1)).unwrap(),
                width_y: NonZeroU32::new(high_y.abs_diff(low_y).saturating_add(1)).unwrap(),
                width_z: NonZeroU32::new(high_z.abs_diff(low_z).saturating_add(1)).unwrap(),
            })
        } else {
            None
        }
    }

    #[inline]
    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) {
        let high_x = self.low_x.saturating_add_unsigned(self.width_x.get() - 1);
        let high_y = self.low_y.saturating_add_unsigned(self.width_y.get() - 1);
        let high_z = self.low_z.saturating_add_unsigned(self.width_z.get() - 1);

        bytes.extend(self.low_x.to_le_bytes());
        bytes.extend(self.low_y.to_le_bytes());
        bytes.extend(self.low_z.to_le_bytes());
        bytes.extend(high_x.to_le_bytes());
        bytes.extend(high_y.to_le_bytes());
        bytes.extend(high_z.to_le_bytes());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardcodedSpawnerType {
    NetherFortress,
    WitchHut,
    OceanMonument,
    LegacyVillageCat,
    PillagerOutpost,
    NewerLegacyVillageCat,
}

bijective_enum_map! {
    HardcodedSpawnerType, u8,
    NetherFortress        <=> 1,
    WitchHut              <=> 2,
    OceanMonument         <=> 3,
    LegacyVillageCat      <=> 4,
    PillagerOutpost       <=> 5,
    NewerLegacyVillageCat <=> 6,
}
