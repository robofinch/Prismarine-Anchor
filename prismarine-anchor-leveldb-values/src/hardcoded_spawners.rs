use prismarine_anchor_translation::datatypes::BlockPosition;
use prismarine_anchor_util::bijective_enum_map;
use prismarine_anchor_util::slice_to_array;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardcodedSpawners(pub Vec<HardcodedSpawner>);

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
            let next_spawner = slice_to_array::<0, 25, _, 25>(value);
            value = &value[25..];

            hardcoded_spawners.push(HardcodedSpawner::parse(next_spawner)?);
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
        for harcoded_spawner in self.0.iter().take(len_usize) {
            harcoded_spawner.extend_serialized(bytes);
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
pub struct HardcodedSpawner {
    pub low_corner:   BlockPosition,
    pub high_corner:  BlockPosition,
    pub spawner_type: HardcodedSpawnerType,
}

impl HardcodedSpawner {
    pub fn parse(value: [u8; 25]) -> Option<Self> {
        let spawner_type = HardcodedSpawnerType::try_from(value[24]).ok()?;

        let low_x  = i32::from_le_bytes([value[0],  value[ 1], value[ 2], value[ 3]]);
        let low_y  = i32::from_le_bytes([value[4],  value[ 5], value[ 6], value[ 7]]);
        let low_z  = i32::from_le_bytes([value[8],  value[ 9], value[10], value[11]]);
        let high_x = i32::from_le_bytes([value[12], value[13], value[14], value[15]]);
        let high_y = i32::from_le_bytes([value[16], value[17], value[18], value[19]]);
        let high_z = i32::from_le_bytes([value[20], value[21], value[22], value[23]]);

        if low_x <= high_x && low_y <= high_y && low_z <= high_z {
            Some(Self {
                low_corner: BlockPosition {
                    x: low_x,
                    y: low_y,
                    z: low_z,
                },
                high_corner: BlockPosition {
                    x: high_x,
                    y: high_y,
                    z: high_z,
                },
                spawner_type,
            })
        } else {
            None
        }
    }

    #[inline]
    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) {
        // Hypothetically, this would be useful in isolation, but in practice
        // we already reserve space in `HardcodedSpawners`.
        // bytes.reserve(25);
        bytes.extend(self.low_corner.x.to_le_bytes());
        bytes.extend(self.low_corner.y.to_le_bytes());
        bytes.extend(self.low_corner.z.to_le_bytes());
        bytes.extend(self.high_corner.x.to_le_bytes());
        bytes.extend(self.high_corner.y.to_le_bytes());
        bytes.extend(self.high_corner.z.to_le_bytes());
        bytes.push(u8::from(self.spawner_type));
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
