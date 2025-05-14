mod entry;
mod key;


use thiserror::Error;

use prismarine_anchor_leveldb_values::{
    aabb_volumes::VolumesToBytesError,
    checksums::ChecksumsToBytesError,
    DataFidelity,
    HandleExcessiveLength,
    hardcoded_spawners::SpawnersToBytesError,
    metadata::MetaDictToBytesError,
    ValueParseOptions,
    ValueToBytesOptions,
};
use prismarine_anchor_mc_datatypes::{dimensions::OverworldElision, version::NumericVersion};
use prismarine_anchor_nbt::io::NbtIoError;


pub use self::{entry::DBEntry, key::DBKey};

// Note in case the LevelDB part didn't make it obvious: this is for Minecraft Bedrock.


#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone)]
pub struct EntryBytes {
    pub key:   Vec<u8>,
    pub value: Vec<u8>,
}

#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct EntryParseOptions {
    /// The data fidelity of entry values; does not affect keys.
    pub value_fidelity: DataFidelity,
}

impl From<EntryParseOptions> for ValueParseOptions {
    fn from(opts: EntryParseOptions) -> Self {
        Self {
            data_fidelity: opts.value_fidelity,
        }
    }
}

/// Settings for converting a `DBKey` into raw key bytes for use in a LevelDB.
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct KeyToBytesOptions {
    pub write_overworld_id:   OverworldElision,
    pub write_overworld_name: OverworldElision,
}

impl KeyToBytesOptions {
    /// This provides options corresponding to Minecraft's behavior up to at least 1.21.51
    // TODO: is this tied to something like chunk version?
    pub fn for_version(version: NumericVersion) -> Self {
        if version < NumericVersion::from([1, 20, 40]) {
            Self {
                write_overworld_id:   OverworldElision::AlwaysElide,
                write_overworld_name: OverworldElision::AlwaysElide,
            }
        } else {
            Self {
                write_overworld_id:   OverworldElision::AlwaysElide,
                write_overworld_name: OverworldElision::AlwaysWrite,
            }
        }
    }
}

/// Settings for converting a `DBEntry`
/// into raw key and value bytes for use in a LevelDB.
///
/// The best choice is
/// - `write_overworld_id = AlwaysElide` for all current versions (up to at least 1.21.51),
/// - `write_overworld_name = AlwaysElide` for any version below 1.20.40, and conversely
/// - `write_overworld_name = AlwaysWrite` for any version at or above 1.20.40.
/// - `handle_excessive_length = ReturnError`, unless you have cause to write weirdly massive data.
/// - `value_fidelity = DataFidelity::Semantic`, unless running tests.
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct EntryToBytesOptions {
    pub write_overworld_id:      OverworldElision,
    pub write_overworld_name:    OverworldElision,
    pub handle_excessive_length: HandleExcessiveLength,
    /// The data fidelity of entry values; does not affect keys.
    pub value_fidelity:          DataFidelity,
}

impl EntryToBytesOptions {
    pub fn for_version(version: NumericVersion) -> Self {
        let KeyToBytesOptions {
            write_overworld_id,
            write_overworld_name,
        } = KeyToBytesOptions::for_version(version);
        Self {
            write_overworld_id,
            write_overworld_name,
            handle_excessive_length: HandleExcessiveLength::ReturnError,
            value_fidelity:          DataFidelity::Semantic,
        }
    }
}

impl From<EntryToBytesOptions> for KeyToBytesOptions {
    fn from(opts: EntryToBytesOptions) -> Self {
        Self {
            write_overworld_id:   opts.write_overworld_id,
            write_overworld_name: opts.write_overworld_name,
        }
    }
}

impl From<EntryToBytesOptions> for ValueToBytesOptions {
    fn from(opts: EntryToBytesOptions) -> Self {
        Self {
            handle_excessive_length: opts.handle_excessive_length,
            data_fidelity:           opts.value_fidelity,
        }
    }
}

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
