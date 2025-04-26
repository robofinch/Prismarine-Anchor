mod entry;
mod key;


use thiserror::Error;

use prismarine_anchor_leveldb_values::metadata::MetaDataWriteError;
use prismarine_anchor_nbt::io::NbtIoError;


pub use self::{entry::DBEntry, key::DBKey};


// Note in case the LevelDB part didn't make it obvious: this is for Minecraft Bedrock.


/// Settings for converting a `DBKey` into raw key bytes for use in a LevelDB.
///
/// If `write_overworld_id` is false, then only non-Overworld dimensions will have their
/// numeric IDs written when a `NumericDimension` is serialized.
/// Likewise, if `write_overworld_name` is false, then only non-Overworld dimensions
/// will have their names written when a `NamedDimension` is serialized.
///
/// The best choice is
/// - `write_overworld_id = false` for all current versions (up to at least 1.21.51), and
/// - `write_overworld_name = false` for any version below 1.20.40, and conversely
/// - `write_overworld_name = true` for any version at or above 1.20.40.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyToBytesOptions {
    pub write_overworld_id:   bool,
    pub write_overworld_name: bool,
}

/// Settings for converting a `DBEntry` into raw value bytes for use in a LevelDB.
///
/// If `error_on_excessive_length` is true, then if a `LevelChunkMetaDataDictionary` with more
/// than 2^32 values is attempted to be written to bytes, an error is returned. If false,
/// the dictionary is silently truncated to 2^32 values if such a thing occurs.
///
/// It should probably be set to `true` unless you have cause to write weirdly massive data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueToBytesOptions {
    pub error_on_excessive_length: bool,
}

/// Settings for converting a `DBEntry`
/// into raw key and value bytes for use in a LevelDB.
///
/// If `write_overworld_id` is false, then only non-Overworld dimensions will have their
/// numeric IDs written when a `NumericDimension` is serialized.
/// Likewise, if `write_overworld_name` is false, then only non-Overworld dimensions
/// will have their names written when a `NamedDimension` is serialized.
///
/// If `error_on_excessive_length` is true, then if a `LevelChunkMetaDataDictionary` with more
/// than 2^32 values is attempted to be written to bytes, an error is returned. If false,
/// the dictionary is silently truncated to 2^32 values if such a thing occurs.
///
/// The best choice is
/// - `write_overworld_id = false` for all current versions (up to at least 1.21.51), and
/// - `write_overworld_name = false` for any version below 1.20.40, and conversely
/// - `write_overworld_name = true` for any version at or above 1.20.40.
/// - `error_on_excessive_length = true`, unless you have cause to write weirdly massive data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryToBytesOptions {
    pub write_overworld_id:        bool,
    pub write_overworld_name:      bool,
    pub error_on_excessive_length: bool,
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
            error_on_excessive_length: opts.error_on_excessive_length,
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
}

impl From<MetaDataWriteError> for ValueToBytesError {
    fn from(value: MetaDataWriteError) -> Self {
        match value {
            MetaDataWriteError::DictionaryLength => Self::DictionaryLength,
            MetaDataWriteError::NbtError(err)    => Self::NbtIoError(err),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EntryBytes {
    pub key:   Vec<u8>,
    pub value: Vec<u8>,
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
/// `various_text-characters[0, 1, 2, 3,]more_text[255, 255]`
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
