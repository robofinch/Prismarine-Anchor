mod entry;
mod key;


use thiserror::Error;

use prismarine_anchor_leveldb_values::{
    checksums::ChecksumsToBytesError, hardcoded_spawners::SpawnersToBytesError, metadata::MetaDictToBytesError, DataFidelity, HandleExcessiveLength, OverworldElision, ValueParseOptions, ValueToBytesOptions
};
use prismarine_anchor_nbt::io::NbtIoError;
use prismarine_anchor_translation::datatypes::NumericVersion;


pub use self::{entry::DBEntry, key::DBKey};

// Note in case the LevelDB part didn't make it obvious: this is for Minecraft Bedrock.


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryBytes {
    pub key:   Vec<u8>,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Error, Debug)]
#[error("error while converting a DBEntry's value to bytes: {value_error}")]
pub struct EntryToBytesError {
    pub key:         Vec<u8>,
    pub value_error: ValueToBytesError,
}

/// For use during development. Instead of printing binary data as entirely binary,
/// stretches of ASCII alphanumeric characters (plus `.`, `-`, `_`) are printed as text,
/// with binary data interspersed.
///
/// For example:
/// `various_text-characters[0,1,2,3,]more_text[255,255,]`
fn print_debug(value: &[u8]) {
    #![allow(dead_code)]
    #![allow(clippy::all)]
    // Apparently this wasn't covered.
    #![expect(clippy::cast_lossless)]

    let mut nums = value.iter().peekable();

    while nums.peek().is_some() {
        while let Some(&&num) = nums.peek() {
            if let Some(ch) = char::from_u32(num as u32) {
                if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
                    nums.next();
                    print!("{ch}");
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        print!("[");
        while let Some(&&num) = nums.peek() {
            if let Some(ch) = char::from_u32(num as u32) {
                if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
                    break;
                }
            }
            nums.next();
            print!("{num},");
        }
        print!("]");
    }
    println!();
}
