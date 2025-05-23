use subslice_to_array::SubsliceToArray as _;

use crate::interface::ValueToBytesOptions;
use super::{helpers::BlockVolume, wrappers::HardcodedSpawnerTypeWrapper};


#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone)]
pub struct HardcodedSpawners(pub Vec<(BlockVolume, HardcodedSpawnerTypeWrapper)>);

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
        let hardcoded_spawners = value[4..]
            .chunks_exact(25)
            .map(|spawner| {
                let volume = spawner.subslice_to_array::<0, 24>();
                let spawner_type = spawner[24];

                Some((
                    BlockVolume::parse(volume)?,
                    HardcodedSpawnerTypeWrapper(spawner_type),
                ))
            })
            .collect::<Option<_>>()?;

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
            bytes.push(spawner_type.0);
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

#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub enum SpawnersToBytesError {
    ExcessiveLength,
}
