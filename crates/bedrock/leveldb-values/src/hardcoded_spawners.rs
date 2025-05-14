use bijective_enum_map::injective_enum_map;
use subslice_to_array::SubsliceToArray as _;

use crate::{block_volume::BlockVolume, ValueToBytesOptions};


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardcodedSpawners(pub Vec<(BlockVolume, HardcodedSpawnerType)>);

impl HardcodedSpawners {
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() < 4 {
            log::warn!(
                "HardcodedSpawners had length {}, too short for the 4-byte length header",
                value.len(),
            );
            return None;
        }

        let num_entries = u32::from_le_bytes(value.subslice_to_array::<0, 4>());
        let num_entries = usize::try_from(num_entries).ok()?;

        if value.len() != 4 + num_entries * 25 {
            log::warn!(
                "HardcodedSpawners with {} entries (according to header) was expected \
                 to have length {}, but had length {}",
                num_entries,
                4 + num_entries * 25,
                value.len(),
            );
            return None;
        }

        // We can process value in chunks of 25 bytes
        let mut value = &value[4..];
        let mut hardcoded_spawners = Vec::with_capacity(num_entries);
        for _ in 0..num_entries {
            let volume = value.subslice_to_array::<0, 24>();
            let spawner_type = value[24];
            value = &value[25..];

            let spawner_type = HardcodedSpawnerType::try_from(spawner_type)
                .inspect_err(|()| log::warn!("Invalid HardcodedSpawnerType: {spawner_type}"))
                .ok()?;

            hardcoded_spawners.push((
                BlockVolume::parse(volume)?,
                spawner_type,
            ));
        }

        Some(Self(hardcoded_spawners))
    }

    pub fn extend_serialized(
        &self,
        bytes: &mut Vec<u8>,
        opts: ValueToBytesOptions,
    ) -> Result<(), SpawnersToBytesError> {
        let (len, len_usize) = opts
            .handle_excessive_length
            .length_to_u32(self.0.len())
            .ok_or(SpawnersToBytesError::ExcessiveLength)?;

        bytes.reserve(4 + len_usize * 25);
        bytes.extend(len.to_le_bytes());
        for (volume, spawner_type) in self.0.iter().take(len_usize) {
            volume.extend_serialized(bytes);
            bytes.push(u8::from(*spawner_type));
        }

        Ok(())
    }

    #[inline]
    pub fn to_bytes(
        &self,
        opts: ValueToBytesOptions,
    ) -> Result<Vec<u8>, SpawnersToBytesError> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes, opts)?;
        Ok(bytes)
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

injective_enum_map! {
    HardcodedSpawnerType, u8,
    NetherFortress        <=> 1,
    WitchHut              <=> 2,
    OceanMonument         <=> 3,
    LegacyVillageCat      <=> 4,
    PillagerOutpost       <=> 5,
    NewerLegacyVillageCat <=> 6,
}

#[derive(Debug, Clone, Copy)]
pub enum SpawnersToBytesError {
    ExcessiveLength,
}
