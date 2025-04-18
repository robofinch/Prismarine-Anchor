use prismarine_anchor_leveldb_values::{
    actor_digest_version::ActorDigestVersion,
    chunk_position::DimensionedChunkPos,
    metadata::LevelChunkMetaDataDictionary,
};
use prismarine_anchor_nbt::NbtCompound;

use crate::{
    read_nbt_list, nbt_list_to_bytes, key::BedrockLevelDBKey,
    EntryParseResult, EntryToBytesOptions, ValueToBytesError, ValueToBytesOptions, ValueParseResult,
};

/// The entries in a world's LevelDB database used by Minecraft Bedrock,
/// parsed out of binary, but not necessary into the most useful completely-parsed format.
///
/// Based on information from [minecraft.wiki], [LeviLamina],
/// and data from iterating through an actual world's keys and values.
///
/// [minecraft.wiki]: https://minecraft.wiki/w/Bedrock_Edition_level_format#Chunk_key_format
/// [LeviLamina]: https://github.com/LiteLDev/LeviLamina
#[derive(Debug, Clone)]
pub enum BedrockLevelDBEntry {
    // ================================
    //  Chunk-specific data
    // ================================

    Version(DimensionedChunkPos, u8),
    LegacyVersion(DimensionedChunkPos, u8),
    ActorDigestVersion(DimensionedChunkPos, ActorDigestVersion),

    // Data3D(DimensionedChunkPos),
    // Data2D(DimensionedChunkPos),
    // LegacyData2D(DimensionedChunkPos),

    // SubchunkBlocks(DimensionedChunkPos, i8),
    // LegacyTerrain(DimensionedChunkPos),
    // LegacyExtraBlockData(DimensionedChunkPos),

    BlockEntities(DimensionedChunkPos, Vec<NbtCompound>),
    LegacyEntities(DimensionedChunkPos, Vec<NbtCompound>),
    PendingTicks(DimensionedChunkPos, Vec<NbtCompound>),
    RandomTicks(DimensionedChunkPos, Vec<NbtCompound>),

    // BorderBlocks(DimensionedChunkPos),
    // HardcodedSpawners(DimensionedChunkPos),
    // AabbVolumes(DimensionedChunkPos),

    // Checksums(DimensionedChunkPos),
    MetaDataHash(DimensionedChunkPos, u64),

    // GenerationSeed(DimensionedChunkPos),
    // FinalizedState(DimensionedChunkPos),
    // BiomeState(DimensionedChunkPos),

    // ConversionData(DimensionedChunkPos),

    // CavesAndCliffsBlending(DimensionedChunkPos),
    // BlendingBiomeHeight(DimensionedChunkPos),
    // BlendingData(DimensionedChunkPos),

    // ActorDigest(DimensionedChunkPos),

    // ================================
    //  Data not specific to a chunk
    // ================================

    // Actor(ActorID),

    LevelChunkMetaDataDictionary(LevelChunkMetaDataDictionary),

    // AutonomousEntities,

    // LocalPlayer,
    // Player(UUID),
    // LegacyPlayer(i64),
    // PlayerServer(UUID),

    // VillageDwellers(NamedDimension, UUID),
    // VillageInfo(NamedDimension, UUID),
    // VillagePOI(NamedDimension, UUID),
    // VillagePlayers(NamedDimension, UUID),

    // Map(i64),
    // Portals,

    // StructureTemplate(NamespacedIdentifier),
    // TickingArea(UUID),
    // Scoreboard,
    // WanderingTraderScheduler,

    // BiomeData,
    // MobEvents,

    // Overworld,
    // Nether,
    // TheEnd,

    // PositionTrackingDB(u32),
    // PositionTrackingLastId,

    // FlatWorldLayers,

    // TODO: other encountered keys from very old versions:
    // mVillages
    // villages
    // VillageManager <- I think I saw some library include this
    // dimension0
    // dimension1
    // dimension2 <- not sure if it exists, but dimension0 does.
    // idcounts

    RawEntry {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    RawValue {
        key: BedrockLevelDBKey,
        value: Vec<u8>,
    },
}

impl BedrockLevelDBEntry {
    pub fn parse_entry(key: &[u8], value: &[u8]) -> Self {
        match Self::parse_recognized_entry(key, value) {
            EntryParseResult::Parsed(entry) => entry,
            EntryParseResult::UnrecognizedKey => Self::RawEntry {
                key:   key.to_owned(),
                value: value.to_owned(),
            },
            EntryParseResult::UnrecognizedValue(parsed_key) => Self::RawValue {
                key:   parsed_key,
                value: value.to_owned(),
            }
        }
    }

    pub fn parse_entry_vec(key: Vec<u8>, value: Vec<u8>) -> Self {
        match Self::parse_recognized_entry(&key, &value) {
            EntryParseResult::Parsed(entry) => entry,
            EntryParseResult::UnrecognizedKey => Self::RawEntry {
                key,
                value,
            },
            EntryParseResult::UnrecognizedValue(parsed_key) => Self::RawValue {
                key: parsed_key,
                value,
            }
        }
    }

    pub fn parse_recognized_entry(key: &[u8], value: &[u8]) -> EntryParseResult {
        let Some(key) = BedrockLevelDBKey::parse_recognized_key(key) else {
            return EntryParseResult::UnrecognizedKey;
        };
        Self::parse_recognized_value(key, value).into()
    }

    pub fn parse_value(key: BedrockLevelDBKey, value: &[u8]) -> Self {
        match Self::parse_recognized_value(key, value) {
            ValueParseResult::Parsed(parsed) => parsed,
            ValueParseResult::UnrecognizedValue(key) => Self::RawValue {
                key,
                value: value.to_vec()
            },
        }
    }

    pub fn parse_value_vec(key: BedrockLevelDBKey, value: Vec<u8>) -> Self {
        match Self::parse_recognized_value(key, &value) {
            ValueParseResult::Parsed(parsed) => parsed,
            ValueParseResult::UnrecognizedValue(key) => Self::RawValue {
                key,
                value,
            },
        }
    }

    pub fn parse_recognized_value(key: BedrockLevelDBKey, value: &[u8]) -> ValueParseResult {
        match key {
            BedrockLevelDBKey::Version(chunk_pos) => {
                if value.len() == 1 {
                    println!("Chunk version: {}", value[0]);
                    return ValueParseResult::Parsed(Self::Version(chunk_pos, value[0]));
                }
            }
            BedrockLevelDBKey::LegacyVersion(chunk_pos) => {
                if value.len() == 1 {
                    println!("Legacy chunk version: {}", value[0]);
                    return ValueParseResult::Parsed(Self::LegacyVersion(chunk_pos, value[0]));
                }
            }
            BedrockLevelDBKey::ActorDigestVersion(chunk_pos) => {
                if value.len() == 1 {
                    if let Ok(digest_version) = ActorDigestVersion::try_from(value[0]) {
                        return ValueParseResult::Parsed(
                            Self::ActorDigestVersion(chunk_pos, digest_version),
                        );
                    }
                }
            }
            BedrockLevelDBKey::BlockEntities(chunk_pos) => {
                if let Some(compounds) = read_nbt_list(value) {
                    return ValueParseResult::Parsed(Self::BlockEntities(chunk_pos, compounds));
                }
            },
            BedrockLevelDBKey::LegacyEntities(chunk_pos) => {
                if let Some(compounds) = read_nbt_list(value) {
                    return ValueParseResult::Parsed(Self::LegacyEntities(chunk_pos, compounds));
                }
            },
            BedrockLevelDBKey::PendingTicks(chunk_pos) => {
                if let Some(compounds) = read_nbt_list(value) {
                    return ValueParseResult::Parsed(Self::PendingTicks(chunk_pos, compounds));
                }
            },
            BedrockLevelDBKey::RandomTicks(chunk_pos) => {
                if let Some(compounds) = read_nbt_list(value) {
                    return ValueParseResult::Parsed(Self::RandomTicks(chunk_pos, compounds));
                }
            },
            BedrockLevelDBKey::MetaDataHash(chunk_pos) => {
                if let Ok(bytes) = <[u8; 8]>::try_from(value) {
                    return ValueParseResult::Parsed(
                        Self::MetaDataHash(chunk_pos, u64::from_le_bytes(bytes))
                    );
                }
            }
            BedrockLevelDBKey::LevelChunkMetaDataDictionary => {
                if let Ok(dictionary) = LevelChunkMetaDataDictionary::parse(value) {
                    return ValueParseResult::Parsed(Self::LevelChunkMetaDataDictionary(dictionary));
                }
                // TODO: use the error value to log debug information
                // println!("error: {}", LevelChunkMetaDataDictionary::parse(value).unwrap_err());
            }
            BedrockLevelDBKey::RawKey(key) => {
                return ValueParseResult::Parsed(Self::RawEntry {
                    key,
                    value: value.to_vec(),
                });
            }
            // TODO: explicitly handle every case. This is just to make it compile.
            _ => return ValueParseResult::Parsed(BedrockLevelDBEntry::RawEntry {
                key: vec![],
                value: vec![],
            })
        }

        ValueParseResult::UnrecognizedValue(key)
    }

    pub fn to_key(&self) -> BedrockLevelDBKey {
        match self {
            Self::Version(chunk_pos, ..)        => BedrockLevelDBKey::Version(*chunk_pos),
            Self::LegacyVersion(chunk_pos, ..)  => BedrockLevelDBKey::LegacyVersion(*chunk_pos),
            Self::ActorDigestVersion(chunk_pos, ..)
                => BedrockLevelDBKey::ActorDigestVersion(*chunk_pos),
            Self::BlockEntities(chunk_pos, ..)  => BedrockLevelDBKey::BlockEntities(*chunk_pos),
            Self::LegacyEntities(chunk_pos, ..) => BedrockLevelDBKey::LegacyEntities(*chunk_pos),
            Self::PendingTicks(chunk_pos, ..)   => BedrockLevelDBKey::RandomTicks(*chunk_pos),
            Self::RandomTicks(chunk_pos, ..)    => BedrockLevelDBKey::RandomTicks(*chunk_pos),
            Self::MetaDataHash(chunk_pos, ..)   => BedrockLevelDBKey::MetaDataHash(*chunk_pos),
            Self::LevelChunkMetaDataDictionary(_)
                => BedrockLevelDBKey::LevelChunkMetaDataDictionary,
            Self::RawEntry { key, .. }          => BedrockLevelDBKey::RawKey(key.clone()),
            Self::RawValue { key, .. }          => key.clone(),
        }
    }

    pub fn into_key(self) -> BedrockLevelDBKey {
        match self {
            Self::Version(chunk_pos, ..)        => BedrockLevelDBKey::Version(chunk_pos),
            Self::LegacyVersion(chunk_pos, ..)  => BedrockLevelDBKey::LegacyVersion(chunk_pos),
            Self::ActorDigestVersion(chunk_pos, ..)
                => BedrockLevelDBKey::ActorDigestVersion(chunk_pos),
            Self::BlockEntities(chunk_pos, ..)  => BedrockLevelDBKey::BlockEntities(chunk_pos),
            Self::LegacyEntities(chunk_pos, ..) => BedrockLevelDBKey::LegacyEntities(chunk_pos),
            Self::PendingTicks(chunk_pos, ..)   => BedrockLevelDBKey::PendingTicks(chunk_pos),
            Self::RandomTicks(chunk_pos, ..)    => BedrockLevelDBKey::RandomTicks(chunk_pos),
            Self::MetaDataHash(chunk_pos, ..)   => BedrockLevelDBKey::MetaDataHash(chunk_pos),
            Self::LevelChunkMetaDataDictionary(_)
                => BedrockLevelDBKey::LevelChunkMetaDataDictionary,
            Self::RawEntry { key, .. }          => BedrockLevelDBKey::RawKey(key),
            Self::RawValue { key, .. }          => key,
        }
    }

    /// If `error_on_excessive_length` is true and this is a LevelChunkMetaDataDictionary
    /// entry whose number of values is too large to fit in a u32, then an error is returned.
    pub fn to_value_bytes(
        &self,
        opts: ValueToBytesOptions,
    ) -> Result<Vec<u8>, ValueToBytesError> {

        Ok(match self {
            Self::Version(.., version)                => vec![*version],
            Self::LegacyVersion(.., version)          => vec![*version],
            Self::ActorDigestVersion(.., version)     => vec![u8::from(*version)],
            Self::BlockEntities(.., compounds)        => nbt_list_to_bytes(compounds)?,
            Self::LegacyEntities(.., compounds)       => nbt_list_to_bytes(compounds)?,
            Self::PendingTicks(.., compounds)         => nbt_list_to_bytes(compounds)?,
            Self::RandomTicks(.., compounds)          => nbt_list_to_bytes(compounds)?,
            Self::MetaDataHash(.., hash)              => hash.to_le_bytes().to_vec(),
            Self::LevelChunkMetaDataDictionary(dict)  => {
                dict.to_bytes(opts.error_on_excessive_length)?
            }
            Self::RawEntry { value, .. }              => value.clone(),
            Self::RawValue { value, .. }              => value.clone(),
        })
    }

    pub fn to_bytes(
        &self,
        opts: EntryToBytesOptions,
    ) -> Result<(Vec<u8>, Vec<u8>), (Vec<u8>, ValueToBytesError)> {

        let key = self.to_key().to_bytes(opts.into());

        match self.to_value_bytes(opts.into()) {
            Ok(value) => Ok((key, value)),
            Err(err)  => Err((key, err))
        }
    }

    pub fn into_bytes(
        self,
        opts: EntryToBytesOptions,
    ) -> Result<(Vec<u8>, Vec<u8>), (Vec<u8>, ValueToBytesError)> {

        match self {
            Self::RawEntry { key, value } => Ok((key, value)),
            Self::RawValue { key, value } => {
                let key_bytes = key.to_bytes(opts.into());
                Ok((key_bytes, value))
            }
            // TODO: maybe some other entries could also be more memory efficient, too.
            _ => {
                let value_bytes = self.to_value_bytes(opts.into());
                let key_bytes = self
                    .into_key()
                    .to_bytes(opts.into());

                match value_bytes {
                    Ok(value) => Ok((key_bytes, value)),
                    Err(err)  => Err((key_bytes, err)),
                }
            }
        }
    }
}

impl From<(&[u8], &[u8])> for BedrockLevelDBEntry {
    fn from(raw_entry: (&[u8], &[u8])) -> Self {
        Self::parse_entry(raw_entry.0, raw_entry.1)
    }
}

impl From<(Vec<u8>, Vec<u8>)> for BedrockLevelDBEntry {
    fn from(raw_entry: (Vec<u8>, Vec<u8>)) -> Self {
        Self::parse_entry_vec(raw_entry.0, raw_entry.1)
    }
}
