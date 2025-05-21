use prismarine_anchor_leveldb_values::{
    aabb_volumes::AabbVolumes,
    actor_digest::ActorDigest,
    actor_digest_version::ActorDigestVersion,
    actor_id::ActorID,
    biome_state::BiomeState,
    border_blocks::BorderBlocks,
    blending_data::BlendingData,
    checksums::Checksums,
    dimensioned_chunk_pos::DimensionedChunkPos,
    chunk_version::ChunkVersion,
    concatenated_nbt_compounds::ConcatenatedNbtCompounds,
    data_2d::Data2D,
    data_3d::Data3D,
    finalized_state::FinalizedState,
    flat_world_layers::FlatWorldLayers,
    hardcoded_spawners::HardcodedSpawners,
    legacy_data_2d::LegacyData2D,
    legacy_extra_block_data::LegacyExtraBlockData,
    legacy_terrain::LegacyTerrain,
    level_spawn_was_fixed::LevelSpawnWasFixed,
    metadata::LevelChunkMetaDataDictionary,
    nbt_compound_conversion::NbtCompoundConversion as _,
    subchunk_blocks::SubchunkBlocks,
    uuid::UUID,
};
use prismarine_anchor_mc_datatypes::{dimensions::NamedDimension, identifier::NamespacedIdentifier};
use prismarine_anchor_nbt::NbtCompound;

// Crazy luck with the alignment
use crate::{
    key::DBKey, EntryBytes, EntryParseOptions, EntryParseResult, EntryToBytesError,
    EntryToBytesOptions, ValueParseResult, ValueToBytesError, ValueToBytesOptions,
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
    LegacyTerrain(DimensionedChunkPos, Box<LegacyTerrain>),
    LegacyExtraBlockData(DimensionedChunkPos, LegacyExtraBlockData),
    BlockEntities(DimensionedChunkPos, ConcatenatedNbtCompounds),
    // On a super old save, I saw Entities have the value [3] and fail to parse.
    // It was in the End dimension. Until I understand what that is, I'm just
    // going to let it stay as a `RawValue` instead of an `Entities` entry.
    /// No longer used
    Entities(DimensionedChunkPos, ConcatenatedNbtCompounds),
    PendingTicks(DimensionedChunkPos, ConcatenatedNbtCompounds),
    RandomTicks(DimensionedChunkPos, ConcatenatedNbtCompounds),

    BorderBlocks(DimensionedChunkPos, BorderBlocks),
    /// No longer used
    HardcodedSpawners(DimensionedChunkPos, HardcodedSpawners),
    AabbVolumes(DimensionedChunkPos, AabbVolumes),

    Checksums(DimensionedChunkPos, Checksums),
    MetaDataHash(DimensionedChunkPos, u64),

    GenerationSeed(DimensionedChunkPos, u64),
    FinalizedState(DimensionedChunkPos, FinalizedState),
    BiomeState(DimensionedChunkPos, BiomeState),

    // Haven't managed to find a save file with this yet. Without more info, Vec<u8>
    // is the best we can do.
    ConversionData(DimensionedChunkPos, Vec<u8>),

    // Not used, apparently, so Vec<u8> is the best we can do without more info.
    CavesAndCliffsBlending(DimensionedChunkPos, Vec<u8>),
    // Not used, apparently, so Vec<u8> is the best we can do without more info.
    BlendingBiomeHeight(DimensionedChunkPos, Vec<u8>),
    BlendingData(DimensionedChunkPos, BlendingData),

    ActorDigest(DimensionedChunkPos, ActorDigest),

    // ================================
    //  Data not specific to a chunk
    // ================================

    Actor(ActorID, NbtCompound),

    LevelChunkMetaDataDictionary(LevelChunkMetaDataDictionary),

    AutonomousEntities(NbtCompound),

    LocalPlayer(NbtCompound),
    Player(UUID, NbtCompound),
    /// No longer used
    LegacyPlayer(u64, NbtCompound),
    PlayerServer(UUID, NbtCompound),

    VillageDwellers(Option<NamedDimension>, UUID, NbtCompound),
    VillageInfo(    Option<NamedDimension>, UUID, NbtCompound),
    VillagePOI(     Option<NamedDimension>, UUID, NbtCompound),
    VillagePlayers( Option<NamedDimension>, UUID, NbtCompound),
    VillageRaid(    Option<NamedDimension>, UUID, NbtCompound),

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
    BiomeIdsTable(NbtCompound),

    // Other encountered keys from old versions:

    // No longer used
    FlatWorldLayers(FlatWorldLayers),
    // No longer used
    LevelSpawnWasFixed(LevelSpawnWasFixed),
    // No longer used
    // idcounts   <- I've only heard of this, not seen this as a key.

    /// No longer used
    MVillages(NbtCompound),
    /// No longer used
    Villages(NbtCompound),
    // LegacyVillageManager <- I think I saw some library include this. Probably NBT?
    // note that the raw key is, allegedly, "VillageManager"

    /// No longer used
    Dimension0(NbtCompound),
    /// No longer used
    Dimension1(NbtCompound),
    /// No longer used
    Dimension2(NbtCompound),

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
    pub fn parse_entry(key: &[u8], value: &[u8], opts: EntryParseOptions) -> Self {
        match Self::parse_recognized_entry(key, value, opts) {
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

    pub fn parse_entry_vec(key: Vec<u8>, value: Vec<u8>, opts: EntryParseOptions) -> Self {
        match Self::parse_recognized_entry(&key, &value, opts) {
            EntryParseResult::Parsed(entry) => entry,
            EntryParseResult::UnrecognizedKey => Self::RawEntry { key, value },
            EntryParseResult::UnrecognizedValue(parsed_key) => Self::RawValue {
                key: parsed_key,
                value,
            },
        }
    }

    pub fn parse_recognized_entry(
        key: &[u8],
        value: &[u8],
        opts: EntryParseOptions,
    ) -> EntryParseResult {
        let Some(key) = DBKey::parse_recognized_key(key) else {
            return EntryParseResult::UnrecognizedKey;
        };
        Self::parse_recognized_value(key, value, opts).into()
    }

    pub fn parse_value(key: DBKey, value: &[u8], opts: EntryParseOptions) -> Self {
        match Self::parse_recognized_value(key, value, opts) {
            ValueParseResult::Parsed(parsed) => parsed,
            ValueParseResult::UnrecognizedValue(key) => Self::RawValue {
                key,
                value: value.to_vec(),
            },
        }
    }

    pub fn parse_value_vec(key: DBKey, value: Vec<u8>, opts: EntryParseOptions) -> Self {
        match Self::parse_recognized_value(key, &value, opts) {
            ValueParseResult::Parsed(parsed) => parsed,
            ValueParseResult::UnrecognizedValue(key) => Self::RawValue { key, value },
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "it's a giant match, and at least uses helper functions",
    )]
    pub fn parse_recognized_value(
        key: DBKey,
        value: &[u8],
        opts: EntryParseOptions,
    ) -> ValueParseResult {
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
                let subchunk_blocks = SubchunkBlocks::parse(value)
                    .inspect_err(|err| log::warn!("Error parsing SubchunkBlocks data: {err}"));
                if let Ok(subchunk_blocks) = subchunk_blocks {
                    return V::Parsed(Self::SubchunkBlocks(chunk_pos, y_index, subchunk_blocks));
                }
            }
            DBKey::LegacyTerrain(chunk_pos) => {
                if let Some(terrain) = LegacyTerrain::parse(value) {
                    return V::Parsed(Self::LegacyTerrain(chunk_pos, Box::new(terrain)));
                }
            }
            DBKey::LegacyExtraBlockData(chunk_pos) => {
                if let Some(extra_blocks) = LegacyExtraBlockData::parse(value) {
                    return V::Parsed(Self::LegacyExtraBlockData(chunk_pos, extra_blocks));
                }
            }
            DBKey::BlockEntities(chunk_pos) => {
                // Note that block entities definitely have some `ByteString`s
                let compounds = ConcatenatedNbtCompounds::parse(value)
                    .inspect_err(|err| log::warn!("Error parsing BlockEntities: {err}"));
                if let Ok(compounds) = compounds {
                    return V::Parsed(Self::BlockEntities(chunk_pos, compounds));
                }
            }
            DBKey::Entities(chunk_pos) => {
                let compounds = ConcatenatedNbtCompounds::parse(value)
                    .inspect_err(|err| log::warn!("Error parsing Entities: {err}"));
                if let Ok(compounds) = compounds {
                    return V::Parsed(Self::Entities(chunk_pos, compounds));
                }
            }
            DBKey::PendingTicks(chunk_pos) => {
                let compounds = ConcatenatedNbtCompounds::parse(value)
                    .inspect_err(|err| log::warn!("Error parsing PendingTicks: {err}"));
                if let Ok(compounds) = compounds {
                    return V::Parsed(Self::PendingTicks(chunk_pos, compounds));
                }
            }
            DBKey::RandomTicks(chunk_pos) => {
                let compounds = ConcatenatedNbtCompounds::parse(value)
                    .inspect_err(|err| log::warn!("Error parsing RandomTicks: {err}"));
                if let Ok(compounds) = compounds {
                    return V::Parsed(Self::RandomTicks(chunk_pos, compounds));
                }
            }
            DBKey::BorderBlocks(chunk_pos) => {
                if let Some(border_blocks) = BorderBlocks::parse(value, opts.into()) {
                    return V::Parsed(Self::BorderBlocks(chunk_pos, border_blocks))
                }
            }
            DBKey::HardcodedSpawners(chunk_pos) => {
                if let Some(spawners) = HardcodedSpawners::parse(value) {
                    return V::Parsed(Self::HardcodedSpawners(chunk_pos, spawners));
                }
            }
            DBKey::AabbVolumes(chunk_pos) => {
                if let Some(volumes) = AabbVolumes::parse(value) {
                    return V::Parsed(Self::AabbVolumes(chunk_pos, volumes));
                }
            }
            DBKey::Checksums(chunk_pos) => {
                if let Some(checksums) = Checksums::parse(value) {
                    return V::Parsed(Self::Checksums(chunk_pos, checksums));
                }
            }
            DBKey::MetaDataHash(chunk_pos) => {
                if let Ok(bytes) = <[u8; 8]>::try_from(value) {
                    return V::Parsed(Self::MetaDataHash(chunk_pos, u64::from_le_bytes(bytes)));
                }
            }
            DBKey::GenerationSeed(chunk_pos) => {
                if let Ok(bytes) = <[u8; 8]>::try_from(value) {
                    return V::Parsed(Self::GenerationSeed(chunk_pos, u64::from_le_bytes(bytes)));
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
            DBKey::ConversionData(chunk_pos) => {
                log::warn!("Encountered ConversionData value: {value:?}");
                return V::Parsed(Self::ConversionData(chunk_pos, value.to_vec()));
            }
            DBKey::CavesAndCliffsBlending(chunk_pos) => {
                log::warn!("Encountered CavesAndCliffsBlending value: {value:?}");
                return V::Parsed(Self::CavesAndCliffsBlending(chunk_pos, value.to_vec()));
            }
            DBKey::BlendingBiomeHeight(chunk_pos) => {
                log::warn!("Encountered BlendingBiomeHeight value: {value:?}");
                return V::Parsed(Self::BlendingBiomeHeight(chunk_pos, value.to_vec()));
            }
            DBKey::BlendingData(chunk_pos) => {
                if let Some(blending_data) = BlendingData::parse(value) {
                    return V::Parsed(Self::BlendingData(chunk_pos, blending_data));
                }
            }
            DBKey::ActorDigest(chunk_pos) => {
                if let Some(digest) = ActorDigest::parse(value) {
                    return V::Parsed(Self::ActorDigest(chunk_pos, digest));
                }
            }
            DBKey::Actor(actor_id) => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::Actor(actor_id, nbt));
                }
            }
            DBKey::LevelChunkMetaDataDictionary => {
                let dictionary = LevelChunkMetaDataDictionary::parse(value)
                    .inspect_err(|err| log::warn!(
                        "Error parsing LevelChunkMetaDataDictionary: {err}",
                    ));
                if let Ok(dictionary) = dictionary {
                    return V::Parsed(Self::LevelChunkMetaDataDictionary(dictionary));
                }
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
            DBKey::VillageRaid(dim, uuid) => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::VillageRaid(dim, uuid, nbt));
                } else {
                    // Note that `dim` is not Copy, so we can't rely on falling through
                    // to the end of the function
                    return V::UnrecognizedValue(DBKey::VillageRaid(dim, uuid));
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
            DBKey::BiomeIdsTable => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::BiomeIdsTable(nbt));
                }
            }
            DBKey::FlatWorldLayers => {
                if let Some(layers) = FlatWorldLayers::parse(value) {
                    return V::Parsed(Self::FlatWorldLayers(layers));
                }
            }
            DBKey::LevelSpawnWasFixed => {
                if let Some(fixed) = LevelSpawnWasFixed::parse(value) {
                    return V::Parsed(Self::LevelSpawnWasFixed(fixed));
                }
            }
            DBKey::MVillages => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::MVillages(nbt));
                }
            }
            DBKey::Villages => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::Villages(nbt));
                }
            }
            DBKey::Dimension0 => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::Dimension0(nbt));
                }
            }
            DBKey::Dimension1 => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::Dimension1(nbt));
                }
            }
            DBKey::Dimension2 => {
                if let Some(nbt) = NbtCompound::parse(value) {
                    return V::Parsed(Self::Dimension2(nbt));
                }
            }
            DBKey::RawKey(key) => {
                log::warn!(
                    "Not parsing value bytes associated with a DBKey that could not be parsed",
                );
                return V::Parsed(Self::RawEntry {
                    key,
                    value: value.to_vec(),
                });
            }
        }

        log::warn!("Could not parse DBEntry value corresponding to key {key:?}.");
        if value.len() <= 100 {
            log::warn!("Unparsed DBEntry value bytes: {value:?}");
        } else {
            log::warn!("First 100 bytes of unparsed DBEntry value: {:?}", &value[..100]);
            log::trace!("Remainder of unparsed DBEntry value: {:?}", &value[100..]);
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
            Self::LegacyTerrain(chunk_pos, ..)      => DBKey::LegacyTerrain(*chunk_pos),
            Self::LegacyExtraBlockData(c_pos, ..)   => DBKey::LegacyExtraBlockData(*c_pos),
            Self::BlockEntities(chunk_pos, ..)      => DBKey::BlockEntities(*chunk_pos),
            Self::Entities(chunk_pos, ..)           => DBKey::Entities(*chunk_pos),
            Self::PendingTicks(chunk_pos, ..)       => DBKey::PendingTicks(*chunk_pos),
            Self::RandomTicks(chunk_pos, ..)        => DBKey::RandomTicks(*chunk_pos),
            Self::BorderBlocks(chunk_pos, ..)       => DBKey::BorderBlocks(*chunk_pos),
            Self::HardcodedSpawners(chunk_pos, ..)  => DBKey::HardcodedSpawners(*chunk_pos),
            Self::AabbVolumes(chunk_pos, ..)        => DBKey::AabbVolumes(*chunk_pos),
            Self::Checksums(chunk_pos, ..)          => DBKey::Checksums(*chunk_pos),
            Self::MetaDataHash(chunk_pos, ..)       => DBKey::MetaDataHash(*chunk_pos),
            Self::GenerationSeed(chunk_pos, ..)     => DBKey::GenerationSeed(*chunk_pos),
            Self::FinalizedState(chunk_pos, ..)     => DBKey::FinalizedState(*chunk_pos),
            Self::BiomeState(chunk_pos, ..)         => DBKey::BiomeState(*chunk_pos),
            Self::ConversionData(chunk_pos, ..)     => DBKey::ConversionData(*chunk_pos),
            Self::CavesAndCliffsBlending(c_pos, ..) => DBKey::CavesAndCliffsBlending(*c_pos),
            Self::BlendingBiomeHeight(c_pos, ..)    => DBKey::BlendingBiomeHeight(*c_pos),
            Self::BlendingData(chunk_pos, ..)       => DBKey::BlendingData(*chunk_pos),
            Self::ActorDigest(chunk_pos, ..)        => DBKey::ActorDigest(*chunk_pos),
            Self::Actor(actor_id, ..)               => DBKey::Actor(*actor_id),
            Self::LevelChunkMetaDataDictionary(..)  => DBKey::LevelChunkMetaDataDictionary,
            Self::AutonomousEntities(..)            => DBKey::AutonomousEntities,
            Self::LocalPlayer(..)                   => DBKey::LocalPlayer,
            Self::Player(uuid, ..)                  => DBKey::Player(*uuid),
            Self::LegacyPlayer(id, ..)              => DBKey::LegacyPlayer(*id),
            Self::PlayerServer(uuid, ..)            => DBKey::PlayerServer(*uuid),
            Self::VillageDwellers(dim, uuid, ..)    => DBKey::VillageDwellers(dim.clone(), *uuid),
            Self::VillageInfo(dim, uuid, ..)        => DBKey::VillageInfo(dim.clone(), *uuid),
            Self::VillagePOI(dim, uuid, ..)         => DBKey::VillagePOI(dim.clone(), *uuid),
            Self::VillagePlayers(dim, uuid, ..)     => DBKey::VillagePlayers(dim.clone(), *uuid),
            Self::VillageRaid(dim, uuid, ..)        => DBKey::VillageRaid(dim.clone(), *uuid),
            Self::Map(map_id, ..)                   => DBKey::Map(*map_id),
            Self::Portals(..)                       => DBKey::Portals,
            Self::StructureTemplate(name, ..)       => DBKey::StructureTemplate(name.clone()),
            Self::TickingArea(uuid, ..)             => DBKey::TickingArea(*uuid),
            Self::Scoreboard(..)                    => DBKey::Scoreboard,
            Self::WanderingTraderScheduler(..)      => DBKey::WanderingTraderScheduler,
            Self::BiomeData(..)                     => DBKey::BiomeData,
            Self::MobEvents(..)                     => DBKey::MobEvents,
            Self::Overworld(..)                     => DBKey::Overworld,
            Self::Nether(..)                        => DBKey::Nether,
            Self::TheEnd(..)                        => DBKey::TheEnd,
            Self::PositionTrackingDB(id, ..)        => DBKey::PositionTrackingDB(*id),
            Self::PositionTrackingLastId(..)        => DBKey::PositionTrackingLastId,
            Self::BiomeIdsTable(..)                 => DBKey::BiomeIdsTable,
            Self::FlatWorldLayers(..)               => DBKey::FlatWorldLayers,
            Self::LevelSpawnWasFixed(..)            => DBKey::LevelSpawnWasFixed,
            Self::MVillages(..)                     => DBKey::MVillages,
            Self::Villages(..)                      => DBKey::Villages,
            Self::Dimension0(..)                    => DBKey::Dimension0,
            Self::Dimension1(..)                    => DBKey::Dimension1,
            Self::Dimension2(..)                    => DBKey::Dimension2,
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
            Self::LegacyTerrain(chunk_pos, ..)      => DBKey::LegacyTerrain(chunk_pos),
            Self::LegacyExtraBlockData(c_pos, ..)   => DBKey::LegacyExtraBlockData(c_pos),
            Self::BlockEntities(chunk_pos, ..)      => DBKey::BlockEntities(chunk_pos),
            Self::Entities(chunk_pos, ..)           => DBKey::Entities(chunk_pos),
            Self::PendingTicks(chunk_pos, ..)       => DBKey::PendingTicks(chunk_pos),
            Self::RandomTicks(chunk_pos, ..)        => DBKey::RandomTicks(chunk_pos),
            Self::BorderBlocks(chunk_pos, ..)       => DBKey::BorderBlocks(chunk_pos),
            Self::HardcodedSpawners(chunk_pos, ..)  => DBKey::HardcodedSpawners(chunk_pos),
            Self::AabbVolumes(chunk_pos, ..)        => DBKey::AabbVolumes(chunk_pos),
            Self::Checksums(chunk_pos, ..)          => DBKey::Checksums(chunk_pos),
            Self::MetaDataHash(chunk_pos, ..)       => DBKey::MetaDataHash(chunk_pos),
            Self::GenerationSeed(chunk_pos, ..)     => DBKey::GenerationSeed(chunk_pos),
            Self::FinalizedState(chunk_pos, ..)     => DBKey::FinalizedState(chunk_pos),
            Self::BiomeState(chunk_pos, ..)         => DBKey::BiomeState(chunk_pos),
            Self::ConversionData(chunk_pos, ..)     => DBKey::ConversionData(chunk_pos),
            Self::CavesAndCliffsBlending(c_pos, ..) => DBKey::CavesAndCliffsBlending(c_pos),
            Self::BlendingBiomeHeight(c_pos, ..)    => DBKey::BlendingBiomeHeight(c_pos),
            Self::BlendingData(chunk_pos, ..)       => DBKey::BlendingData(chunk_pos),
            Self::ActorDigest(chunk_pos, ..)        => DBKey::ActorDigest(chunk_pos),
            Self::Actor(actor_id, ..)               => DBKey::Actor(actor_id),
            Self::LevelChunkMetaDataDictionary(..)  => DBKey::LevelChunkMetaDataDictionary,
            Self::AutonomousEntities(..)            => DBKey::AutonomousEntities,
            Self::LocalPlayer(..)                   => DBKey::LocalPlayer,
            Self::Player(uuid, ..)                  => DBKey::Player(uuid),
            Self::LegacyPlayer(id, ..)              => DBKey::LegacyPlayer(id),
            Self::PlayerServer(uuid, ..)            => DBKey::PlayerServer(uuid),
            Self::VillageDwellers(dim, uuid, ..)    => DBKey::VillageDwellers(dim, uuid),
            Self::VillageInfo(dim, uuid, ..)        => DBKey::VillageInfo(dim, uuid),
            Self::VillagePOI(dim, uuid, ..)         => DBKey::VillagePOI(dim, uuid),
            Self::VillagePlayers(dim, uuid, ..)     => DBKey::VillagePlayers(dim, uuid),
            Self::VillageRaid(dim, uuid, ..)        => DBKey::VillageRaid(dim, uuid),
            Self::Map(map_id, ..)                   => DBKey::Map(map_id),
            Self::Portals(..)                       => DBKey::Portals,
            Self::StructureTemplate(name, ..)       => DBKey::StructureTemplate(name),
            Self::TickingArea(uuid, ..)             => DBKey::TickingArea(uuid),
            Self::Scoreboard(..)                    => DBKey::Scoreboard,
            Self::WanderingTraderScheduler(..)      => DBKey::WanderingTraderScheduler,
            Self::BiomeData(..)                     => DBKey::BiomeData,
            Self::MobEvents(..)                     => DBKey::MobEvents,
            Self::Overworld(..)                     => DBKey::Overworld,
            Self::Nether(..)                        => DBKey::Nether,
            Self::TheEnd(..)                        => DBKey::TheEnd,
            Self::PositionTrackingDB(id, ..)        => DBKey::PositionTrackingDB(id),
            Self::PositionTrackingLastId(..)        => DBKey::PositionTrackingLastId,
            Self::BiomeIdsTable(..)                 => DBKey::BiomeIdsTable,
            Self::FlatWorldLayers(..)               => DBKey::FlatWorldLayers,
            Self::LevelSpawnWasFixed(..)            => DBKey::LevelSpawnWasFixed,
            Self::MVillages(..)                     => DBKey::MVillages,
            Self::Villages(..)                      => DBKey::Villages,
            Self::Dimension0(..)                    => DBKey::Dimension0,
            Self::Dimension1(..)                    => DBKey::Dimension1,
            Self::Dimension2(..)                    => DBKey::Dimension2,
            Self::RawEntry { key, .. }              => DBKey::RawKey(key),
            Self::RawValue { key, .. }              => key,
        }
    }

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
            Self::LegacyTerrain(.., terrain)            => terrain.to_bytes(),
            Self::LegacyExtraBlockData(.., blocks)      => blocks.to_bytes(opts)?,
            Self::BlockEntities(.., compounds)          => compounds.to_bytes()?,
            Self::Entities(.., compounds)               => compounds.to_bytes()?,
            Self::PendingTicks(.., compounds)           => compounds.to_bytes()?,
            Self::RandomTicks(.., compounds)            => compounds.to_bytes()?,
            Self::BorderBlocks(.., blocks)              => blocks.to_bytes(opts),
            Self::HardcodedSpawners(.., spawners)       => spawners.to_bytes(opts)?,
            Self::AabbVolumes(.., volumes)              => volumes.to_bytes(opts)?,
            Self::Checksums(.., checksums)              => checksums.to_bytes(opts)?,
            Self::MetaDataHash(.., hash)                => hash.to_le_bytes().to_vec(),
            Self::GenerationSeed(.., seed)              => seed.to_le_bytes().to_vec(),
            Self::FinalizedState(.., state)             => state.to_bytes(),
            Self::BiomeState(.., state)                 => state.to_bytes(),
            Self::ConversionData(.., raw)               => raw.clone(),
            Self::CavesAndCliffsBlending(.., raw)       => raw.clone(),
            Self::BlendingBiomeHeight(.., raw)          => raw.clone(),
            Self::BlendingData(.., blending_data)       => blending_data.to_bytes(),
            Self::ActorDigest(.., digest)               => digest.to_bytes(),
            Self::Actor(.., nbt)                        => nbt.to_bytes()?,
            Self::LevelChunkMetaDataDictionary(dict)    => dict.to_bytes(opts)?,
            Self::AutonomousEntities(nbt)               => nbt.to_bytes()?,
            Self::LocalPlayer(nbt)                      => nbt.to_bytes()?,
            Self::Player(.., nbt)                       => nbt.to_bytes()?,
            Self::LegacyPlayer(.., nbt)                 => nbt.to_bytes()?,
            Self::PlayerServer(.., nbt)                 => nbt.to_bytes()?,
            Self::VillageDwellers(.., nbt)              => nbt.to_bytes()?,
            Self::VillageInfo(.., nbt)                  => nbt.to_bytes()?,
            Self::VillagePOI(.., nbt)                   => nbt.to_bytes()?,
            Self::VillagePlayers(.., nbt)               => nbt.to_bytes()?,
            Self::VillageRaid(.., nbt)                  => nbt.to_bytes()?,
            Self::Map(.., nbt)                          => nbt.to_bytes()?,
            Self::Portals(nbt)                          => nbt.to_bytes()?,
            Self::StructureTemplate(.., nbt)            => nbt.to_bytes()?,
            Self::TickingArea(.., nbt)                  => nbt.to_bytes()?,
            Self::Scoreboard(nbt)                       => nbt.to_bytes()?,
            Self::WanderingTraderScheduler(nbt)         => nbt.to_bytes()?,
            Self::BiomeData(nbt)                        => nbt.to_bytes()?,
            Self::MobEvents(nbt)                        => nbt.to_bytes()?,
            Self::Overworld(nbt)                        => nbt.to_bytes()?,
            Self::Nether(nbt)                           => nbt.to_bytes()?,
            Self::TheEnd(nbt)                           => nbt.to_bytes()?,
            Self::PositionTrackingDB(.., nbt)           => nbt.to_bytes()?,
            Self::PositionTrackingLastId(nbt)           => nbt.to_bytes()?,
            Self::BiomeIdsTable(nbt)                    => nbt.to_bytes()?,
            Self::FlatWorldLayers(layers)               => layers.to_bytes(),
            Self::LevelSpawnWasFixed(fixed)             => fixed.to_bytes(),
            Self::MVillages(nbt)                        => nbt.to_bytes()?,
            Self::Villages(nbt)                         => nbt.to_bytes()?,
            Self::Dimension0(nbt)                       => nbt.to_bytes()?,
            Self::Dimension1(nbt)                       => nbt.to_bytes()?,
            Self::Dimension2(nbt)                       => nbt.to_bytes()?,
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
            Self::ConversionData(chunk_pos, raw) => {
                let key = DBKey::ConversionData(chunk_pos);
                let key_bytes = key.to_bytes(opts.into());

                Ok(EntryBytes {
                    key:   key_bytes,
                    value: raw,
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
