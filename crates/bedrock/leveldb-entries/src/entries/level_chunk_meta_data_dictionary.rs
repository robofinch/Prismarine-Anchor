use std::io;
use std::collections::VecDeque;
use std::io::{Cursor, Read as _};

use indexmap::IndexMap;
use subslice_to_array::SubsliceToArray as _;
use thiserror::Error;
use xxhash_rust::xxh64;

use prismarine_anchor_nbt::{Endianness, IoOptions, NbtCompound, NbtTag};
use prismarine_anchor_nbt::io::{NbtIoError, read_compound, write_compound};
use prismarine_anchor_util::u64_equals_usize;

use crate::interface::ValueToBytesOptions;


#[derive(Debug, Clone)]
pub struct LevelChunkMetaDataDictionary(IndexMap<u64, MetaData>);

impl LevelChunkMetaDataDictionary {
    #[inline]
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    #[inline]
    pub fn get(&self, metadata_hash: u64) -> Option<&MetaData> {
        self.0.get(&metadata_hash)
    }

    /// Inserts the provided metadata into the dictionary, and returns the hash key used.
    /// If an error occurs, the dictionary is not modified.
    #[inline]
    pub fn insert(&mut self, metadata: MetaData) -> Result<u64, MetaDataHashError> {
        let hash = metadata.clone().xxhash64()?;
        self.0.insert(hash, metadata);
        Ok(hash)
    }

    #[inline]
    pub fn contains_hash(&self, metadata_hash: u64) -> bool {
        self.0.contains_key(&metadata_hash)
    }

    /// Determines whether the dictionary contains the hash of the provided `MetaData`.
    /// This requires normalizing the `MetaData` by sorting its keys.
    #[inline]
    pub fn contains_metadata(&self, metadata: &mut MetaData) -> Result<bool, MetaDataHashError> {
        let hash = metadata.xxhash64()?;
        Ok(self.0.contains_key(&hash))
    }

    pub fn parse(value: &[u8]) -> Result<Self, MetaDataParseError> {
        if value.len() < 4 {
            return Err(MetaDataParseError::NoHeader);
        }

        let num_entries = u32::from_le_bytes(value.subslice_to_array::<0, 4>());

        let mut reader = Cursor::new(&value[4..]);
        let mut map = IndexMap::new();

        // Read each of the dictionary's entries
        for _ in 0..num_entries {

            // The hash is the key
            let mut hash = [0; 8];
            reader.read_exact(&mut hash)?;
            let hash = u64::from_le_bytes(hash);

            // MetaData stored as an NBT is the value
            let (nbt, _) = read_compound(&mut reader, IoOptions::bedrock_uncompressed())?;
            let mut metadata = MetaData(nbt);

            // Check that the hash is correct
            let computed_hash = metadata.xxhash64()?;

            if hash != computed_hash {
                return Err(MetaDataParseError::IncorrectHash {
                    computed: computed_hash,
                    received: hash,
                });
            }

            // Reject if there's a duplicate hash
            if map.insert(hash, metadata).is_some() {
                return Err(MetaDataParseError::DuplicateHash(hash));
            }
        }

        // Reject if there was excess data
        if !u64_equals_usize(reader.position(), reader.into_inner().len()) {
            return Err(MetaDataParseError::ExcessData);
        }

        Ok(Self(map))
    }

    pub fn extend_serialized(
        &self,
        bytes: &mut Vec<u8>,
        opts:  ValueToBytesOptions,
    ) -> Result<(), MetaDictToBytesError> {
        let (len, len_usize) = opts
            .handle_excessive_length
            .length_to_u32(self.0.len())
            .ok_or(MetaDictToBytesError::ExcessiveLength)?;

        bytes.extend(len.to_le_bytes());

        for (hash, metadata) in self.0.iter().take(len_usize) {
            bytes.extend(hash.to_le_bytes());

            // Could only fail on invalid NBT.
            write_compound(
                bytes,
                IoOptions::bedrock_uncompressed(),
                None,
                &metadata.0,
            )?;
        }

        Ok(())
    }

    pub fn to_bytes(&self, opts: ValueToBytesOptions) -> Result<Vec<u8>, MetaDictToBytesError> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes, opts)?;
        Ok(bytes)
    }
}

impl Default for LevelChunkMetaDataDictionary {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Note that Minecraft calls it `MetaData` instead of `Metadata`
#[cfg_attr(feature = "derive_standard", derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct MetaData(pub NbtCompound);

impl MetaData {
    /// This method recursively sorts the inner NBT compound, and then hashes the result in the
    /// same way that Minecraft does.
    ///
    /// It is possible that Minecraft might sort NBT lists; the order of NBT lists is not touched
    /// here, so, if Minecraft ever adds an NBT list to the metadata of a chunk,
    /// incompatibility could arise.
    pub fn xxhash64(&mut self) -> Result<u64, MetaDataHashError> {

        let mut to_sort = VecDeque::new();
        to_sort.push_back(&mut self.0);

        while let Some(nbt) = to_sort.pop_front() {
            let inner = nbt.inner_mut();
            inner.sort_unstable_keys();

            for tag in inner.values_mut() {
                if let NbtTag::Compound(nbt) = tag {
                    to_sort.push_back(nbt);
                }
            }
        }
        let network_little_endian = IoOptions {
            endianness: Endianness::NetworkLittleEndian,
            ..IoOptions::bedrock_uncompressed()
        };

        let mut writer = Cursor::new(Vec::new());
        write_compound(&mut writer, network_little_endian, None, &self.0)?;

        Ok(xxh64::xxh64(&writer.into_inner(), 0))
    }
}

#[derive(Error, Debug)]
pub enum MetaDataParseError {
    #[error("the metadata dictionary was shorter than the required 4 byte header")]
    NoHeader,
    #[error("the hash value {0} appeared twice in a metadata dictionary")]
    DuplicateHash(u64),
    #[error("all entries of a metadata dictionary were parsed, but excess data was provided")]
    ExcessData,
    #[error(
        "a metadata entry with hash key {} was received, but its hash was computed as {}",
        received, computed,
    )]
    IncorrectHash { computed: u64, received: u64 },
    #[error(transparent)]
    HashError(#[from] MetaDataHashError),
    #[error("NBT error while parsing metadata dictionary: {0}")]
    NbtError(#[from] NbtIoError),
    #[error("IO error while parsing metadata dictionary: {0}")]
    IoError(#[from] io::Error),
}

#[derive(Error, Debug)]
pub enum MetaDictToBytesError {
    #[error("the number of metadata entries could not fit in a u32")]
    ExcessiveLength,
    #[error("NBT error while writing metadata dictionary to NBT: {0}")]
    NbtError(#[from] NbtIoError),
}

#[derive(Error, Debug)]
pub enum MetaDataHashError {
    #[error("error while writing metadata NBT to bytes to compute its xxhash64 hash: {0}")]
    NbtError(#[from] NbtIoError),
}
