use prismarine_anchor_mc_datatypes::{dimensions::OverworldElision, version::NumericVersion};


// ================================
//  Traits
// ================================

pub trait DatabaseKey: Sized {
    /// Convert the `DatabaseKey` into bytes used in the LevelDB.
    fn to_bytes(&self, opts: KeyToBytesOptions) -> Vec<u8>;

    /// Convert the `DatabaseKey` into bytes used in the LevelDB.
    ///
    /// By default, this calls the implementation of `to_bytes`.
    #[inline]
    fn into_bytes(self, opts: KeyToBytesOptions) -> Vec<u8> {
        self.to_bytes(opts)
    }
}

pub trait DatabaseEntry: Sized {
    type Key: DatabaseKey;
    type ParseError;
    type ToBytesError;
    type ValueParseError;
    type ValueToBytesError;

    /// Convert the `DatabaseEntry` into bytes used in the LevelDB.
    fn to_bytes(
        &self,
        opts: EntryToBytesOptions,
    ) -> Result<EntryBytes, Self::ToBytesError>;

    /// Convert the `DatabaseEntry` into bytes used in the LevelDB.
    ///
    /// By default, this calls the implementation of `to_bytes`.
    #[inline]
    fn into_bytes(
        self,
        opts: EntryToBytesOptions,
    ) -> Result<EntryBytes, Self::ToBytesError> {
        self.to_bytes(opts)
    }

    /// Get the `DatabaseKey` corresponding to this `DatabaseEntry`.
    fn to_key(&self) -> Self::Key;

    /// Get the `DatabaseKey` corresponding to this `DatabaseEntry`, consuming the entry.
    ///
    /// By default, this calls the implementation of `to_key`.
    #[inline]
    fn into_key(self) -> Self::Key {
        self.to_key()
    }

    /// Attempt to convert the value of this `DatabaseEntry` into bytes.
    fn to_value_bytes(
        &self,
        opts: ValueToBytesOptions,
    ) -> Result<Vec<u8>, Self::ValueToBytesError>;

    /// Attempt to convert the value of this `DatabaseEntry` into bytes, consuming the entry.
    ///
    /// By default, this calls the implementation of `to_value_bytes`.
    #[inline]
    fn into_value_bytes(
        self,
        opts: ValueToBytesOptions,
    ) -> Result<Vec<u8>, Self::ValueToBytesError> {
        self.to_value_bytes(opts)
    }

    /// Attempt to parse value bytes from a LevelDB into this `DatabaseEntry`.
    fn parse_value(
        key:   Self::Key,
        value: &[u8],
        opts:  EntryParseOptions,
    ) -> Result<Self, Self::ValueParseError>;

    /// Attempt to parse value bytes from a LevelDB into this `DatabaseEntry`.
    ///
    /// By default, this calls the implementation of `parse_value`.
    #[inline]
    fn parse_owned_value(
        key:   Self::Key,
        value: Vec<u8>,
        opts:  EntryParseOptions,
    ) -> Result<Self, Self::ValueParseError> {
        Self::parse_value(key, &value, opts)
    }
}

// ================================
//  Options
// ================================

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

impl From<EntryToBytesOptions> for KeyToBytesOptions {
    fn from(opts: EntryToBytesOptions) -> Self {
        Self {
            write_overworld_id:   opts.write_overworld_id,
            write_overworld_name: opts.write_overworld_name,
        }
    }
}

#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct ValueToBytesOptions {
    pub data_fidelity:           DataFidelity,
    pub handle_excessive_length: HandleExcessiveLength,
}

impl From<EntryToBytesOptions> for ValueToBytesOptions {
    fn from(opts: EntryToBytesOptions) -> Self {
        Self {
            handle_excessive_length: opts.handle_excessive_length,
            data_fidelity:           opts.value_fidelity,
        }
    }
}

#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct EntryParseOptions {
    /// The data fidelity of entry values; does not affect keys.
    pub value_fidelity: DataFidelity,
}

#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct ValueParseOptions {
    pub data_fidelity: DataFidelity,
}

impl From<EntryParseOptions> for ValueParseOptions {
    fn from(opts: EntryParseOptions) -> Self {
        Self {
            data_fidelity: opts.value_fidelity,
        }
    }
}

// ================================
//  Other
// ================================

#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone)]
pub struct EntryBytes {
    pub key:   Vec<u8>,
    pub value: Vec<u8>,
}

/// Control whether semantically-unimportant data is parsed or serialized to bytes.
/// (Semantically-important data is always parsed and serialized.)
///
/// NOTE: you may also need to enable the `preserve_order` feature of `prismarine-anchor-nbt`
/// for `BitPerfect` to fully function.
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub enum DataFidelity {
    /// Preserve all data, including semantically unimportant data like padding bits, and preserves
    /// the order of all entries in likely-unordered key-value maps.
    ///
    /// NOTE: you may also need to enable the `preserve_order` feature of `prismarine-anchor-nbt`
    /// for this option to fully function.
    BitPerfect,
    /// Preserve all semantically important data. Currently, padding bits in `PalettizedStorage`
    /// (when read/written from/to the packed index representation) and the order of entries
    /// in most key-value maps is still preserved, but when convenient, such information may
    /// be ignored.
    Semantic,
}

/// How to handle lists or maps whose number of entries is too large to fit in a u32, or strings
/// whose length does not fit in a u16.
///
/// If set to `ReturnError`, then if a list with a length that needs to be
/// written into a `u32` or `u16` in the byte representation (e.g. `Checksums` or
/// `LevelChunkMetaDataDictionary` data, or a `NamespacedIdentifier` string) with more than
/// 2^32 or 2^16 values is attempted to be written to bytes, an error is returned.
/// If `SilentlyTruncate`, the list or string is silently truncated to the maximum length if such
/// an event occurs.
///
/// Note that this does *not* affect `SubchunkBlocks` data; if there are more than 255
/// block layers in `SubchunkBlocks` data, then only the first 255 layers will be written;
/// no error is ever returned (and to begin with, there should never be anywhere near that many
/// layers).
///
/// It should probably be set to `ReturnError` unless you have cause to write weirdly massive data.
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub enum HandleExcessiveLength {
    ReturnError,
    SilentlyTruncate,
}

impl HandleExcessiveLength {
    /// Given a `usize` length, attempts to cast it to a `u32`. If `self` is `ReturnError`
    /// and the conversion fails, then an error is returned; otherwise, the value is saturated
    /// to `u32::MAX` instead.
    ///
    /// Both a `u32` and `usize` are returned, to handle the case that the `usize` length
    /// must be truncated.
    pub fn length_to_u32(self, len: usize) -> Option<(u32, usize)> {
        if size_of::<usize>() >= size_of::<u32>() {
            let len = match u32::try_from(len) {
                Ok(len) => len,
                Err(_) => match self {
                    Self::ReturnError      => return None,
                    Self::SilentlyTruncate => u32::MAX,
                }
            };

            // This cast from u32 to usize won't overflow
            Some((len, len as usize))
        } else {
            // This cast from usize to u32 won't overflow
            Some((len as u32, len))
        }
    }

    /// Given a `usize` length, attempts to cast it to a `u16`. If `self` is `ReturnError`
    /// and the conversion fails, then an error is returned; otherwise, the value is saturated
    /// to `u16::MAX` instead.
    ///
    /// Both a `u16` and `usize` are returned, to handle the case that the `usize` length
    /// must be truncated.
    pub fn length_to_u16(self, len: usize) -> Option<(u16, usize)> {
        let len = match u16::try_from(len) {
            Ok(len) => len,
            Err(_) => match self {
                Self::ReturnError      => return None,
                Self::SilentlyTruncate => u16::MAX,
            }
        };

        Some((len, usize::from(len)))
    }
}
