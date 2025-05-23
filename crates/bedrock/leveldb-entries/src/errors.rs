use thiserror::Error;

use prismarine_anchor_nbt::io::NbtIoError;

use super::{DBEntry, DBKey};
use super::entries::{
    ChecksumsToBytesError,
    ExtraBlocksToBytesError,
    SpawnersToBytesError,
    MetaDictToBytesError,
    VolumesToBytesError,
};


#[derive(Debug, Clone)]
pub enum ValueParseResult {
    Parsed(DBEntry),
    UnrecognizedValue(DBKey),
}

#[derive(Debug, Clone)]
pub enum EntryParseResult {
    Parsed(DBEntry),
    UnrecognizedKey,
    UnrecognizedValue(DBKey),
}

impl From<ValueParseResult> for EntryParseResult {
    fn from(value: ValueParseResult) -> Self {
        match value {
            ValueParseResult::Parsed(parsed)         => Self::Parsed(parsed),
            ValueParseResult::UnrecognizedValue(key) => Self::UnrecognizedValue(key),
        }
    }
}

#[derive(Error, Debug)]
pub enum ValueToBytesError {
    #[error("error while writing NBT: {0}")]
    NbtIoError(#[from] NbtIoError),
    #[error("there were too many metadata entries in a LevelChunkMetaDataDictionary")]
    DictionaryLength,
    #[error("there were too many entries in a Checksums value")]
    ChecksumsLength,
    #[error("there were too many entries in a LegacyExtraBlockData value")]
    ExtraBlocksLength,
    #[error("there were too many entries in a HarcodedSpawners value")]
    SpawnersLength,
    #[error("there were too many entries in an AabbVolumes key-value map")]
    AabbMapLength,
    #[error("a string for a namespaced identifier in an AabbVolumes value was too long")]
    AabbStringLength,
}

impl From<ChecksumsToBytesError> for ValueToBytesError {
    fn from(value: ChecksumsToBytesError) -> Self {
        match value {
            ChecksumsToBytesError::ExcessiveLength => Self::ChecksumsLength,
        }
    }
}

impl From<ExtraBlocksToBytesError> for ValueToBytesError {
    fn from(value: ExtraBlocksToBytesError) -> Self {
        match value {
            ExtraBlocksToBytesError::ExcessiveLength => Self::ExtraBlocksLength,
        }
    }
}

impl From<SpawnersToBytesError> for ValueToBytesError {
    fn from(value: SpawnersToBytesError) -> Self {
        match value {
            SpawnersToBytesError::ExcessiveLength => Self::SpawnersLength,
        }
    }
}

impl From<MetaDictToBytesError> for ValueToBytesError {
    fn from(value: MetaDictToBytesError) -> Self {
        match value {
            MetaDictToBytesError::ExcessiveLength => Self::DictionaryLength,
            MetaDictToBytesError::NbtError(err)   => Self::NbtIoError(err),
        }
    }
}

impl From<VolumesToBytesError> for ValueToBytesError {
    fn from(value: VolumesToBytesError) -> Self {
        match value {
            VolumesToBytesError::ExcessiveMapLength    => Self::AabbMapLength,
            VolumesToBytesError::ExcessiveStringLength => Self::AabbStringLength,
        }
    }
}

#[derive(Error, Debug)]
#[error("error while converting a DBEntry's value to bytes: {value_error}")]
pub struct EntryToBytesError {
    pub key:         Vec<u8>,
    pub value_error: ValueToBytesError,
}
