use prismarine_anchor_leveldb_values::{
    actor::ActorID,
    actor_digest_version::ActorDigestVersion,
    biome_state::BiomeState,
    blending_data::BlendingData,
    chunk_position::DimensionedChunkPos,
    chunk_version::ChunkVersion,
    concatenated_nbt_compounds::ConcatenatedNbtCompounds,
    data_2d::Data2D,
    data_3d::Data3D,
    dimensions::NamedDimension,
    finalized_state::FinalizedState,
    legacy_data_2d::LegacyData2D,
    metadata::LevelChunkMetaDataDictionary,
    nbt_compound_conversion::NbtCompoundConversion as _,
    subchunk_blocks::SubchunkBlocks,
    uuid::UUID,
};
use prismarine_anchor_nbt::NbtCompound;
use prismarine_anchor_translation::datatypes::NamespacedIdentifier;

// Crazy luck with the alignment
use crate::{
    EntryBytes, EntryParseResult, EntryToBytesError, EntryToBytesOptions,
    key::DBKey, ValueParseResult, ValueToBytesError, ValueToBytesOptions,
};


/// The entries in a world's `LevelDB` database used by Minecraft Bedrock,
/// parsed out of binary, but not necessary into the most useful completely-parsed format.
///
/// Based on information from [minecraft.wiki], [LeviLamina],
/// and data from iterating through an actual world's keys and values.
///
/// [minecraft.wiki]: https://minecraft.wiki/w/Bedrock_Edition_level_format#Chunk_key_format
/// [LeviLamina]: https://github.com/LiteLDev/LeviLamina
#[derive(Debug, Clone)]
pub enum DBEntry {
    // ================================
    //  Chunk-specific data
    // ================================

    Version(DimensionedChunkPos, ChunkVersion),
    LegacyVersion(DimensionedChunkPos, ChunkVersion),
    ActorDigestVersion(DimensionedChunkPos, ActorDigestVersion),

    Data3D(DimensionedChunkPos, Box<Data3D>),
    Data2D(DimensionedChunkPos, Box<Data2D>),
    LegacyData2D(DimensionedChunkPos, Box<LegacyData2D>),

    SubchunkBlocks(DimensionedChunkPos, i8, SubchunkBlocks),
    // LegacyTerrain(DimensionedChunkPos),
    // LegacyExtraBlockData(DimensionedChunkPos),
    BlockEntities(DimensionedChunkPos, ConcatenatedNbtCompounds),
    // On a super old save, I saw this have the value [3] and fail to parse.
    LegacyEntities(DimensionedChunkPos, ConcatenatedNbtCompounds),
    PendingTicks(DimensionedChunkPos, ConcatenatedNbtCompounds),
    RandomTicks(DimensionedChunkPos, ConcatenatedNbtCompounds),

    // TODO: learn what the format of BorderBlocks data is.
    // BorderBlocks(DimensionedChunkPos),
    // HardcodedSpawners(DimensionedChunkPos), // Not NBT
    // AabbVolumes(DimensionedChunkPos), // Not NBT

    // Checksums(DimensionedChunkPos),
    MetaDataHash(DimensionedChunkPos, u64),

    // GenerationSeed(DimensionedChunkPos),
    FinalizedState(DimensionedChunkPos, FinalizedState),
    BiomeState(DimensionedChunkPos, BiomeState),

    // ConversionData(DimensionedChunkPos),

    // Not used, apparently, so Vec<u8> is the best we can do without more info.
    CavesAndCliffsBlending(DimensionedChunkPos, Vec<u8>),
    // Not used, apparently, so Vec<u8> is the best we can do without more info.
    BlendingBiomeHeight(DimensionedChunkPos, Vec<u8>),
    BlendingData(DimensionedChunkPos, BlendingData),

    // ActorDigest(DimensionedChunkPos, Vec<ActorID>),

    // ================================
    //  Data not specific to a chunk
    // ================================

    Actor(ActorID, NbtCompound),

    LevelChunkMetaDataDictionary(LevelChunkMetaDataDictionary),

    AutonomousEntities(NbtCompound),

    LocalPlayer(NbtCompound),
    Player(UUID, NbtCompound),
    LegacyPlayer(i64, NbtCompound),
    PlayerServer(UUID, NbtCompound),

    VillageDwellers(NamedDimension, UUID, NbtCompound),
    VillageInfo(NamedDimension, UUID, NbtCompound),
    VillagePOI(NamedDimension, UUID, NbtCompound),
    VillagePlayers(NamedDimension, UUID, NbtCompound),

    Map(i64, NbtCompound),
    Portals(NbtCompound),

    StructureTemplate(NamespacedIdentifier, NbtCompound),
    TickingArea(UUID, NbtCompound),
    Scoreboard(NbtCompound),
    WanderingTraderScheduler(NbtCompound),

    BiomeData(NbtCompound),
    MobEvents(NbtCompound),

    Overworld(NbtCompound),
    Nether(NbtCompound),
    TheEnd(NbtCompound),

    PositionTrackingDB(u32, NbtCompound),
    PositionTrackingLastId(NbtCompound),
    // BiomeIdsTable

    // FlatWorldLayers,

    // TODO: other encountered keys from very old versions:
    LegacyMVillages(NbtCompound),
    LegacyVillages(NbtCompound),
    // LegacyVillageManager <- I think I saw some library include this. Probably NBT?
    // note that the raw key is, allegedly, "VillageManager"

    LegacyDimension0(NbtCompound),
    LegacyDimension1(NbtCompound),
    // dimension2 <- not sure if it exists, since the end probably didn't have structures
    // idcounts   <- I've only heard of this, not seen this as a key.

    RawEntry {
        key:   Vec<u8>,
        value: Vec<u8>,
    },
    RawValue {
        key:   DBKey,
        value: Vec<u8>,
    },
}

impl DBEntry {
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
            },
        }
    }

    pub fn parse_entry_vec(key: Vec<u8>, value: Vec<u8>) -> Self {
        match Self::parse_recognized_entry(&key, &value) {
            EntryParseResult::Parsed(entry) => entry,
            EntryParseResult::UnrecognizedKey => Self::RawEntry { key, value },
            EntryParseResult::UnrecognizedValue(parsed_key) => Self::RawValue {
                key: parsed_key,
                value,
            },
        }
    }

    pub fn parse_recognized_entry(key: &[u8], value: &[u8]) -> EntryParseResult {
        let Some(key) = DBKey::parse_recognized_key(key) else {
            return EntryParseResult::UnrecognizedKey;
        };
        Self::parse_recognized_value(key, value).into()
    }

    pub fn parse_value(key: DBKey, value: &[u8]) -> Self {
        match Self::parse_recognized_value(key, value) {
            ValueParseResult::Parsed(parsed) => parsed,
            ValueParseResult::UnrecognizedValue(key) => Self::RawValue {
                key,
                value: value.to_vec(),
            },
        }
    }

    pub fn parse_value_vec(key: DBKey, value: Vec<u8>) -> Self {
        match Self::parse_recognized_value(key, &value) {
            ValueParseResult::Parsed(parsed) => parsed,
            ValueParseResult::UnrecognizedValue(key) => Self::RawValue { key, value },
        }
    }

    #[expect(clippy::too_many_lines, reason = "it's a giant match, which uses helper functions")]
    pub fn parse_recognized_value(key: DBKey, value: &[u8]) -> ValueParseResult {
        use ValueParseResult as V;

        match key {
            DBKey::Version(chunk_pos) => {
                if value.len() == 1 {
                    if let Some(chunk_version) = ChunkVersion::parse(value[0]) {
                        return V::Parsed(Self::Version(chunk_pos, chunk_version));
                    }
                }
            }
            DBKey::LegacyVersion(chunk_pos) => {
                if value.len() == 1 {
                    if let Some(chunk_version) = ChunkVersion::parse(value[0]) {
                        return V::Parsed(Self::LegacyVersion(chunk_pos, chunk_version));
                    }
                }
            }
            DBKey::ActorDigestVersion(chunk_pos) => {
                if value.len() == 1 {
                    if let Ok(digest_version) = ActorDigestVersion::try_from(value[0]) {
                        return V::Parsed(Self::ActorDigestVersion(chunk_pos, digest_version));
                    }
                }
            }
            DBKey::Data3D(chunk_pos) => {
                if let Some(data_3d) = Data3D::parse(value) {
                    return V::Parsed(Self::Data3D(chunk_pos, Box::new(data_3d)));
                }
            }
            DBKey::Data2D(chunk_pos) => {
                if let Some(data_2d) = Data2D::parse(value) {
                    return V::Parsed(Self::Data2D(chunk_pos, Box::new(data_2d)));
                }
            }
            DBKey::LegacyData2D(chunk_pos) => {
                if let Some(legacy_data_2d) = LegacyData2D::parse(value) {
                    return V::Parsed(Self::LegacyData2D(chunk_pos, Box::new(legacy_data_2d)));
                }
            }
            DBKey::SubchunkBlocks(chunk_pos, y_index) => {
                if let Some(subchunk_blocks) = SubchunkBlocks::parse(value) {
                    return V::Parsed(Self::SubchunkBlocks(chunk_pos, y_index, subchunk_blocks));
                }
            }
            DBKey::BlockEntities(chunk_pos) => {
                // The `true` is definitely needed.
                if let Ok(compounds) = ConcatenatedNbtCompounds::parse(value, true) {
                    return V::Parsed(Self::BlockEntities(chunk_pos, compounds));
                }
            }
            DBKey::LegacyEntities(chunk_pos) => {
                // TODO: Not sure if `true` is needed.
                if let Ok(compounds) = ConcatenatedNbtCompounds::parse(value, true) {
                    return V::Parsed(Self::LegacyEntities(chunk_pos, compounds));
                }
            }
            DBKey::PendingTicks(chunk_pos) => {
                // TODO: Not sure if `true` is needed.
                if let Ok(compounds) = ConcatenatedNbtCompounds::parse(value, true) {
                    return V::Parsed(Self::PendingTicks(chunk_pos, compounds));
                }
            }
            DBKey::RandomTicks(chunk_pos) => {
                // TODO: Not sure if `true` is needed.
                if let Ok(compounds) = ConcatenatedNbtCompounds::parse(value, true) {
                    return V::Parsed(Self::RandomTicks(chunk_pos, compounds));
                }
            }
            DBKey::MetaDataHash(chunk_pos) => {
                if let Ok(bytes) = <[u8; 8]>::try_from(value) {
                    return V::Parsed(Self::MetaDataHash(chunk_pos, u64::from_le_bytes(bytes)));
                }
            }
            DBKey::FinalizedState(chunk_pos) => {
                if let Some(finalized_state) = FinalizedState::parse(value) {
                    return V::Parsed(Self::FinalizedState(chunk_pos, finalized_state));
                }
            }
            DBKey::BiomeState(chunk_pos) => {
                if let Some(biome_state) = BiomeState::parse(value) {
                    return V::Parsed(Self::BiomeState(chunk_pos, biome_state));
                }
            }
            DBKey::CavesAndCliffsBlending(chunk_pos) => {
                return V::Parsed(Self::CavesAndCliffsBlending(chunk_pos, value.to_vec()));
            }
            DBKey::BlendingBiomeHeight(chunk_pos) => {
                return V::Parsed(Self::BlendingBiomeHeight(chunk_pos, value.to_vec()));
            }
            DBKey::BlendingData(chunk_pos) => {
                if let Some(blending_data) = BlendingData::parse(value) {
                    return V::Parsed(Self::BlendingData(chunk_pos, blending_data));
                }
            }
            DBKey::Actor(actor_id) => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::Actor(actor_id, nbt));
                }
            }
            DBKey::LevelChunkMetaDataDictionary => {
                if let Ok(dictionary) = LevelChunkMetaDataDictionary::parse(value) {
                    return V::Parsed(Self::LevelChunkMetaDataDictionary(dictionary));
                }
                // TODO: use the error value to log debug information
                // println!("error: {}", LevelChunkMetaDataDictionary::parse(value).unwrap_err());
            }
            DBKey::AutonomousEntities => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::AutonomousEntities(nbt));
                }
            }
            DBKey::LocalPlayer => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::LocalPlayer(nbt));
                }
            }
            DBKey::Player(uuid) => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::Player(uuid, nbt));
                }
            }
            DBKey::LegacyPlayer(client_id) => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::LegacyPlayer(client_id, nbt));
                }
            }
            DBKey::PlayerServer(uuid) => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::PlayerServer(uuid, nbt));
                }
            }
            DBKey::VillageDwellers(dim, uuid) => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::VillageDwellers(dim, uuid, nbt));
                } else {
                    // Note that `dim` is not Copy, so we can't rely on falling through
                    // to the end of the function
                    return V::UnrecognizedValue(DBKey::VillageDwellers(dim, uuid));
                }
            }
            DBKey::VillageInfo(dim, uuid) => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::VillageInfo(dim, uuid, nbt));
                } else {
                    // Note that `dim` is not Copy, so we can't rely on falling through
                    // to the end of the function
                    return V::UnrecognizedValue(DBKey::VillageInfo(dim, uuid));
                }
            }
            DBKey::VillagePOI(dim, uuid) => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::VillagePOI(dim, uuid, nbt));
                } else {
                    // Note that `dim` is not Copy, so we can't rely on falling through
                    // to the end of the function
                    return V::UnrecognizedValue(DBKey::VillagePOI(dim, uuid));
                }
            }
            DBKey::VillagePlayers(dim, uuid) => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::VillagePlayers(dim, uuid, nbt));
                } else {
                    // Note that `dim` is not Copy, so we can't rely on falling through
                    // to the end of the function
                    return V::UnrecognizedValue(DBKey::VillagePlayers(dim, uuid));
                }
            }
            DBKey::Map(map_id) => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::Map(map_id, nbt));
                }
            }
            DBKey::Portals => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::Portals(nbt));
                }
            }
            DBKey::StructureTemplate(identifier) => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::StructureTemplate(identifier, nbt));
                } else {
                    // Note that `identifer` is not Copy, so we can't rely on falling through
                    // to the end of the function
                    return V::UnrecognizedValue(DBKey::StructureTemplate(identifier));
                }
            }
            DBKey::TickingArea(uuid) => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::TickingArea(uuid, nbt));
                }
            }
            DBKey::Scoreboard => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::Scoreboard(nbt));
                }
            }
            DBKey::WanderingTraderScheduler => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::WanderingTraderScheduler(nbt));
                }
            }
            DBKey::BiomeData => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::BiomeData(nbt));
                }
            }
            DBKey::MobEvents => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::MobEvents(nbt));
                }
            }
            DBKey::Overworld => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::Overworld(nbt));
                }
            }
            DBKey::Nether => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::Nether(nbt));
                }
            }
            DBKey::TheEnd => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::TheEnd(nbt));
                }
            }
            DBKey::PositionTrackingDB(id) => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::PositionTrackingDB(id, nbt));
                }
            }
            DBKey::PositionTrackingLastId => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::PositionTrackingLastId(nbt));
                }
            }
            DBKey::LegacyMVillages => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::LegacyMVillages(nbt));
                }
            }
            DBKey::LegacyVillages => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::LegacyVillages(nbt));
                }
            }
            DBKey::LegacyDimension0 => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::LegacyDimension0(nbt));
                }
            }
            DBKey::LegacyDimension1 => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::LegacyDimension1(nbt));
                }
            }
            DBKey::RawKey(key) => {
                return V::Parsed(Self::RawEntry {
                    key,
                    value: value.to_vec(),
                });
            }
            // TODO: explicitly handle every case. This is just to make it compile.
            // _ => return ValueParseResult::Parsed(DBEntry::RawEntry {
            //     key: vec![],
            //     value: vec![],
            // })
            _ => {}
        }

        ValueParseResult::UnrecognizedValue(key)
    }

    pub fn to_key(&self) -> DBKey {
        match self {
            Self::Version(chunk_pos, ..)            => DBKey::Version(*chunk_pos),
            Self::LegacyVersion(chunk_pos, ..)      => DBKey::LegacyVersion(*chunk_pos),
            Self::ActorDigestVersion(chunk_pos, ..) => DBKey::ActorDigestVersion(*chunk_pos),
            Self::Data3D(chunk_pos, ..)             => DBKey::Data3D(*chunk_pos),
            Self::Data2D(chunk_pos, ..)             => DBKey::Data2D(*chunk_pos),
            Self::LegacyData2D(chunk_pos, ..)       => DBKey::LegacyData2D(*chunk_pos),
            Self::SubchunkBlocks(c_pos, y, ..)      => DBKey::SubchunkBlocks(*c_pos, *y),
            Self::BlockEntities(chunk_pos, ..)      => DBKey::BlockEntities(*chunk_pos),
            Self::LegacyEntities(chunk_pos, ..)     => DBKey::LegacyEntities(*chunk_pos),
            Self::PendingTicks(chunk_pos, ..)       => DBKey::PendingTicks(*chunk_pos),
            Self::RandomTicks(chunk_pos, ..)        => DBKey::RandomTicks(*chunk_pos),
            Self::MetaDataHash(chunk_pos, ..)       => DBKey::MetaDataHash(*chunk_pos),
            Self::FinalizedState(chunk_pos, ..)     => DBKey::FinalizedState(*chunk_pos),
            Self::BiomeState(chunk_pos, ..)         => DBKey::BiomeState(*chunk_pos),
            Self::CavesAndCliffsBlending(c_pos, ..) => DBKey::CavesAndCliffsBlending(*c_pos),
            Self::BlendingBiomeHeight(c_pos, ..)    => DBKey::BlendingBiomeHeight(*c_pos),
            Self::BlendingData(chunk_pos, ..)       => DBKey::BlendingData(*chunk_pos),
            Self::Actor(actor_id, ..)               => DBKey::Actor(*actor_id),
            Self::LevelChunkMetaDataDictionary(_)   => DBKey::LevelChunkMetaDataDictionary,
            Self::AutonomousEntities(_)             => DBKey::AutonomousEntities,
            Self::LocalPlayer(_)                    => DBKey::LocalPlayer,
            Self::Player(uuid, _)                   => DBKey::Player(*uuid),
            Self::LegacyPlayer(id, _)               => DBKey::LegacyPlayer(*id),
            Self::PlayerServer(uuid, _)             => DBKey::PlayerServer(*uuid),
            Self::VillageDwellers(dim, uuid, _)     => DBKey::VillageDwellers(dim.clone(), *uuid),
            Self::VillageInfo(dim, uuid, _)         => DBKey::VillageInfo(dim.clone(), *uuid),
            Self::VillagePOI(dim, uuid, _)          => DBKey::VillagePOI(dim.clone(), *uuid),
            Self::VillagePlayers(dim, uuid, _)      => DBKey::VillagePlayers(dim.clone(), *uuid),
            Self::Map(map_id, _)                    => DBKey::Map(*map_id),
            Self::Portals(_)                        => DBKey::Portals,
            Self::StructureTemplate(name, _)        => DBKey::StructureTemplate(name.clone()),
            Self::TickingArea(uuid, _)              => DBKey::TickingArea(*uuid),
            Self::Scoreboard(_)                     => DBKey::Scoreboard,
            Self::WanderingTraderScheduler(_)       => DBKey::WanderingTraderScheduler,
            Self::BiomeData(_)                      => DBKey::BiomeData,
            Self::MobEvents(_)                      => DBKey::MobEvents,
            Self::Overworld(_)                      => DBKey::Overworld,
            Self::Nether(_)                         => DBKey::Nether,
            Self::TheEnd(_)                         => DBKey::TheEnd,
            Self::PositionTrackingDB(id, _)         => DBKey::PositionTrackingDB(*id),
            Self::PositionTrackingLastId(_)         => DBKey::PositionTrackingLastId,
            Self::LegacyMVillages(_)                => DBKey::LegacyMVillages,
            Self::LegacyVillages(_)                 => DBKey::LegacyVillages,
            Self::LegacyDimension0(_)               => DBKey::LegacyDimension0,
            Self::LegacyDimension1(_)               => DBKey::LegacyDimension1,
            Self::RawEntry { key, .. }              => DBKey::RawKey(key.clone()),
            Self::RawValue { key, .. }              => key.clone(),
        }
    }

    pub fn into_key(self) -> DBKey {
        match self {
            Self::Version(chunk_pos, ..)            => DBKey::Version(chunk_pos),
            Self::LegacyVersion(chunk_pos, ..)      => DBKey::LegacyVersion(chunk_pos),
            Self::ActorDigestVersion(chunk_pos, ..) => DBKey::ActorDigestVersion(chunk_pos),
            Self::Data3D(chunk_pos, ..)             => DBKey::Data3D(chunk_pos),
            Self::Data2D(chunk_pos, ..)             => DBKey::Data2D(chunk_pos),
            Self::LegacyData2D(chunk_pos, ..)       => DBKey::LegacyData2D(chunk_pos),
            Self::SubchunkBlocks(c_pos, y, ..)      => DBKey::SubchunkBlocks(c_pos, y),
            Self::BlockEntities(chunk_pos, ..)      => DBKey::BlockEntities(chunk_pos),
            Self::LegacyEntities(chunk_pos, ..)     => DBKey::LegacyEntities(chunk_pos),
            Self::PendingTicks(chunk_pos, ..)       => DBKey::PendingTicks(chunk_pos),
            Self::RandomTicks(chunk_pos, ..)        => DBKey::RandomTicks(chunk_pos),
            Self::MetaDataHash(chunk_pos, ..)       => DBKey::MetaDataHash(chunk_pos),
            Self::FinalizedState(chunk_pos, ..)     => DBKey::FinalizedState(chunk_pos),
            Self::BiomeState(chunk_pos, ..)         => DBKey::BiomeState(chunk_pos),
            Self::CavesAndCliffsBlending(c_pos, ..) => DBKey::CavesAndCliffsBlending(c_pos),
            Self::BlendingBiomeHeight(c_pos, ..)    => DBKey::BlendingBiomeHeight(c_pos),
            Self::BlendingData(chunk_pos, ..)       => DBKey::BlendingData(chunk_pos),
            Self::Actor(actor_id, ..)               => DBKey::Actor(actor_id),
            Self::LevelChunkMetaDataDictionary(_)   => DBKey::LevelChunkMetaDataDictionary,
            Self::AutonomousEntities(_)             => DBKey::AutonomousEntities,
            Self::LocalPlayer(_)                    => DBKey::LocalPlayer,
            Self::Player(uuid, _)                   => DBKey::Player(uuid),
            Self::LegacyPlayer(id, _)               => DBKey::LegacyPlayer(id),
            Self::PlayerServer(uuid, _)             => DBKey::PlayerServer(uuid),
            Self::VillageDwellers(dim, uuid, _)     => DBKey::VillageDwellers(dim, uuid),
            Self::VillageInfo(dim, uuid, _)         => DBKey::VillageInfo(dim, uuid),
            Self::VillagePOI(dim, uuid, _)          => DBKey::VillagePOI(dim, uuid),
            Self::VillagePlayers(dim, uuid, _)      => DBKey::VillagePlayers(dim, uuid),
            Self::Map(map_id, _)                    => DBKey::Map(map_id),
            Self::Portals(_)                        => DBKey::Portals,
            Self::StructureTemplate(name, _)        => DBKey::StructureTemplate(name),
            Self::TickingArea(uuid, _)              => DBKey::TickingArea(uuid),
            Self::Scoreboard(_)                     => DBKey::Scoreboard,
            Self::WanderingTraderScheduler(_)       => DBKey::WanderingTraderScheduler,
            Self::BiomeData(_)                      => DBKey::BiomeData,
            Self::MobEvents(_)                      => DBKey::MobEvents,
            Self::Overworld(_)                      => DBKey::Overworld,
            Self::Nether(_)                         => DBKey::Nether,
            Self::TheEnd(_)                         => DBKey::TheEnd,
            Self::PositionTrackingDB(id, _)         => DBKey::PositionTrackingDB(id),
            Self::PositionTrackingLastId(_)         => DBKey::PositionTrackingLastId,
            Self::LegacyMVillages(_)                => DBKey::LegacyMVillages,
            Self::LegacyVillages(_)                 => DBKey::LegacyVillages,
            Self::LegacyDimension0(_)               => DBKey::LegacyDimension0,
            Self::LegacyDimension1(_)               => DBKey::LegacyDimension1,
            Self::RawEntry { key, .. }              => DBKey::RawKey(key),
            Self::RawValue { key, .. }              => key,
        }
    }

    /// If `error_on_excessive_length` is true and this is a `LevelChunkMetaDataDictionary`
    /// entry whose number of values is too large to fit in a u32, then an error is returned.
    pub fn to_value_bytes(&self, opts: ValueToBytesOptions) -> Result<Vec<u8>, ValueToBytesError> {
        #[expect(clippy::match_same_arms, reason = "clarity")]
        Ok(match self {
            Self::Version(.., version)                  => vec![u8::from(*version)],
            Self::LegacyVersion(.., version)            => vec![u8::from(*version)],
            Self::ActorDigestVersion(.., version)       => vec![u8::from(*version)],
            Self::Data3D(.., data)                      => data.to_bytes(),
            Self::Data2D(.., data)                      => data.to_bytes(),
            Self::LegacyData2D(.., data)                => data.to_bytes(),
            Self::SubchunkBlocks(.., blocks)            => blocks.to_bytes()?,
            Self::BlockEntities(.., compounds)          => compounds.to_bytes(true)?,
            Self::LegacyEntities(.., compounds)         => compounds.to_bytes(true)?,
            Self::PendingTicks(.., compounds)           => compounds.to_bytes(true)?,
            Self::RandomTicks(.., compounds)            => compounds.to_bytes(true)?,
            Self::MetaDataHash(.., hash)                => hash.to_le_bytes().to_vec(),
            Self::FinalizedState(.., state)             => state.to_bytes(),
            Self::BiomeState(.., state)                 => state.to_bytes(),
            Self::CavesAndCliffsBlending(.., raw)       => raw.clone(),
            Self::BlendingBiomeHeight(.., raw)          => raw.clone(),
            Self::BlendingData(.., blending_data)       => blending_data.to_bytes(),
            Self::Actor(.., nbt)                        => nbt.to_bytes()?,
            Self::LevelChunkMetaDataDictionary(dict) => {
                dict.to_bytes(opts.error_on_excessive_length)?
            }
            Self::AutonomousEntities(nbt)               => nbt.to_bytes()?,
            Self::LocalPlayer(nbt)                      => nbt.to_bytes()?,
            Self::Player(_, nbt)                        => nbt.to_bytes()?,
            Self::LegacyPlayer(_, nbt)                  => nbt.to_bytes()?,
            Self::PlayerServer(_, nbt)                  => nbt.to_bytes()?,
            Self::VillageDwellers(.., nbt)              => nbt.to_bytes()?,
            Self::VillageInfo(.., nbt)                  => nbt.to_bytes()?,
            Self::VillagePOI(.., nbt)                   => nbt.to_bytes()?,
            Self::VillagePlayers(.., nbt)               => nbt.to_bytes()?,
            Self::Map(_, nbt)                           => nbt.to_bytes()?,
            Self::Portals(nbt)                          => nbt.to_bytes()?,
            Self::StructureTemplate(_, nbt)             => nbt.to_bytes()?,
            Self::TickingArea(_, nbt)                   => nbt.to_bytes()?,
            Self::Scoreboard(nbt)                       => nbt.to_bytes()?,
            Self::WanderingTraderScheduler(nbt)         => nbt.to_bytes()?,
            Self::BiomeData(nbt)                        => nbt.to_bytes()?,
            Self::MobEvents(nbt)                        => nbt.to_bytes()?,
            Self::Overworld(nbt)                        => nbt.to_bytes()?,
            Self::Nether(nbt)                           => nbt.to_bytes()?,
            Self::TheEnd(nbt)                           => nbt.to_bytes()?,
            Self::PositionTrackingDB(_, nbt)            => nbt.to_bytes()?,
            Self::PositionTrackingLastId(nbt)           => nbt.to_bytes()?,
            Self::LegacyMVillages(nbt)                  => nbt.to_bytes()?,
            Self::LegacyVillages(nbt)                   => nbt.to_bytes()?,
            Self::LegacyDimension0(nbt)                 => nbt.to_bytes()?,
            Self::LegacyDimension1(nbt)                 => nbt.to_bytes()?,
            Self::RawEntry { value, .. }                => value.clone(),
            Self::RawValue { value, .. }                => value.clone(),
        })
    }

    pub fn to_bytes(&self, opts: EntryToBytesOptions) -> Result<EntryBytes, EntryToBytesError> {
        let key = self.to_key().to_bytes(opts.into());

        match self.to_value_bytes(opts.into()) {
            Ok(value) => Ok(EntryBytes {
                key,
                value,
            }),
            Err(value_error) => Err(EntryToBytesError {
                key,
                value_error,
            }),
        }
    }

    pub fn into_bytes(self, opts: EntryToBytesOptions) -> Result<EntryBytes, EntryToBytesError> {
        match self {
            Self::RawEntry { key, value } => Ok(EntryBytes { key, value }),
            Self::RawValue { key, value } => {
                let key_bytes = key.to_bytes(opts.into());

                Ok(EntryBytes {
                    key: key_bytes,
                    value,
                })
            }
            Self::CavesAndCliffsBlending(chunk_pos, raw) => {
                let key = DBKey::CavesAndCliffsBlending(chunk_pos);
                let key_bytes = key.to_bytes(opts.into());

                Ok(EntryBytes {
                    key:   key_bytes,
                    value: raw,
                })
            }
            Self::BlendingBiomeHeight(chunk_pos, raw) => {
                let key = DBKey::BlendingBiomeHeight(chunk_pos);
                let key_bytes = key.to_bytes(opts.into());

                Ok(EntryBytes {
                    key:   key_bytes,
                    value: raw,
                })
            }
            // TODO: maybe some other entries could also be more memory efficient, too.
            _ => {
                let value_bytes = self.to_value_bytes(opts.into());
                let key_bytes = self.into_key().to_bytes(opts.into());

                match value_bytes {
                    Ok(value) => Ok(EntryBytes {
                        key: key_bytes,
                        value,
                    }),
                    Err(err) => Err(EntryToBytesError {
                        key:         key_bytes,
                        value_error: err,
                    }),
                }
            }
        }
    }
}

impl From<(&[u8], &[u8])> for DBEntry {
    fn from(raw_entry: (&[u8], &[u8])) -> Self {
        Self::parse_entry(raw_entry.0, raw_entry.1)
    }
}

impl From<(Vec<u8>, Vec<u8>)> for DBEntry {
    fn from(raw_entry: (Vec<u8>, Vec<u8>)) -> Self {
        Self::parse_entry_vec(raw_entry.0, raw_entry.1)
    }
}
