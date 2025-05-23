use std::str;

use subslice_to_array::SubsliceToArray as _;

use prismarine_anchor_mc_datatypes::{
    IdentifierParseOptions, NamedDimension, NamespacedIdentifier,
    OverworldElision, VanillaDimension,
};


use super::interface::KeyToBytesOptions;
use super::entries::helpers::{ActorID, DimensionedChunkPos, Uuid};


/// The keys in a world's LevelDB database used by Minecraft Bedrock.
/// Based on information from [minecraft.wiki]
/// and data from iterating through an actual world's keys.
///
/// [minecraft.wiki]: https://minecraft.wiki/w/Bedrock_Edition_level_format#Chunk_key_format
// TODO: are "since 1.18.0" and "since 1.0.0" the precise versions that something changed?
// TODO: improve documentation
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone)]
pub enum DBKey {
    // ================================
    //  Chunk-specific data
    // ================================

    Version(DimensionedChunkPos),
    /// The `Version` key started being used instead in 1.16.100.
    LegacyVersion(DimensionedChunkPos),
    ActorDigestVersion(DimensionedChunkPos),

    /// Biome IDs, and block heightmap
    Data3D(DimensionedChunkPos),
    /// Biome IDs, and block heightmap. Not written since 1.18.0.
    Data2D(DimensionedChunkPos),
    /// Biome IDs and colors, and block heightmap. Not written since 1.0.0.
    LegacyData2D(DimensionedChunkPos),

    /// Block data for one subchunk,
    /// and might also include blocklight and skylight, depending on version.
    SubchunkBlocks(DimensionedChunkPos, i8),
    /// Block data, blocklight and skylight, block heightmap, and biome IDs and colors.
    /// In other words, all terrain data for a chunk. Not written since 1.0.0.
    LegacyTerrain(DimensionedChunkPos),
    /// Data for extra block layers, such as grass inside a snow layer. Not written since 1.2.13.
    LegacyExtraBlockData(DimensionedChunkPos),

    /// NBT data of block entities.
    BlockEntities(DimensionedChunkPos),
    /// NBT data of entities. The newer format relies on `Actor` keys; this is no longer used.
    Entities(DimensionedChunkPos),
    /// NBT data of pending ticks.
    PendingTicks(DimensionedChunkPos),
    /// NBT data of random ticks
    RandomTicks(DimensionedChunkPos),

    /// Used in Education Edition
    BorderBlocks(DimensionedChunkPos),
    /// Bounding boxes for structure spawns, such as a Nether Fortress or Pillager Outpost.
    /// Replaced by `AabbVolumes` starting in 1.21.20 (or, in one of the previews for 1.21.20).
    HardcodedSpawners(DimensionedChunkPos),
    /// Bounding boxes for structures, including structure spawns (such as a Pillager Outpost)
    /// and volumes where mobs cannot spawn through the normal biome-based means (such as
    /// Trial Chambers).
    AabbVolumes(DimensionedChunkPos),

    /// xxHash64 checksums of `SubchunkBlocks`, `BlockEntities`, `Entities`, and `Data2D`
    /// values. Not written since 1.18.0.
    Checksums(DimensionedChunkPos),
    /// A hash which is the key in the `LevelChunkMetaDataDictionary` record
    /// for the NBT metadata of this chunk. Seems like it might default to something dependent
    /// on the current game or chunk version
    // TODO: figure out if it's allowed to be missing.
    MetaDataHash(DimensionedChunkPos),

    /// The seed which was used to generate this chunk.
    // TODO: what versions use this? probably stopped being used after it moved to metadata,
    // so knowing when metadata began being used might be enough
    GenerationSeed(DimensionedChunkPos),
    /// Indicates the state of world generation in this chunk, such as whether it
    /// has been fully generated.
    FinalizedState(DimensionedChunkPos),
    // Map from biomes to a byte indicating a snow level. Probably the maximum snow level
    // which snow can pile up to naturally in that chunk.
    // TODO: confirm this
    BiomeState(DimensionedChunkPos),

    /// No longer used.
    // TODO: figure out what it was.. Maybe an NBT compound?
    // Presumably something to do with converting a world from one version to another.
    // Maybe LCE to Bedrock, maybe it *had* been used for Old -> Infinite
    // or pre-Caves and Cliffs to post-Caves and cliffs.
    // It seems likely that it's been moved to the MetaData dictionary, just like GenerationSeed.
    ConversionData(DimensionedChunkPos),

    // I might never know what `CavesAndCliffsBlending` and `BlendingBiomeHeight` are/do.
    /// Full internal name is `GeneratedPreCavesAndCliffsBlending`
    CavesAndCliffsBlending(DimensionedChunkPos),
    BlendingBiomeHeight(DimensionedChunkPos),
    // I've managed to parse this.... sort of. Still don't know the details of what it does.
    BlendingData(DimensionedChunkPos),

    /// List of the `ActorID`s of actors in this chunk.
    ActorDigest(DimensionedChunkPos),

    // ================================
    //  Data not specific to a chunk
    // ================================

    Actor(ActorID),
    /// Stores the NBT metadata of all chunks. Maps the xxHash64 hash of NBT data
    /// to that NBT data, so that each chunk need only store 8 bytes instead of the entire
    /// NBT; most chunks have the same metadata.
    // TODO: if a chunk doesn't have a MetaDataHash, what metadata does it use? the metadata
    // associated with the current BaseGameVersion, maybe?
    LevelChunkMetaDataDictionary,

    // It seems that this is at least used for the Ender Dragon, dunno what else
    AutonomousEntities,

    /// NBT data of the world's local player entity.
    LocalPlayer,
    /// NBT data of a player with the indicated UUID
    Player(Uuid),
    /// NBT data of a player with the indicted numeric ID, which comes
    /// from `clientid.txt`. No longer used.
    // (based on a number stored in `clientid.txt`, which seems to fit in 64 bits or 63 unsigned).
    // Found a value around 1.7 * 2^63. Maybe this is unsigned, let's try that.
    LegacyPlayer(u64),
    // This is NBT data, but I haven't looked closely at it.
    // Could be an alternative version of ~local_player?
    PlayerServer(Uuid),

    /// Key has the dimension which the village is in, and the name of the village.
    // TODO: should have entity IDs of village dwellers. Verify.
    VillageDwellers(Option<NamedDimension>, Uuid),
    /// Key has the dimension which the village is in, and the name of the village.
    // Village bounding box, and possibly other information TODO: figure out what
    VillageInfo(Option<NamedDimension>, Uuid),
    /// Key has the dimension which the village is in, and the name of the village.
    // villager - workstation mappings. Bed mappings aren't stored, apparently.
    VillagePOI(Option<NamedDimension>, Uuid),
    /// Key has the dimension which the village is in, and the name of the village.
    // Probably tracking player reputation or something? Idk. TODO: figure it out
    VillagePlayers(Option<NamedDimension>, Uuid),
    /// Key has the dimension which the village is in, and the name of the village.
    VillageRaid(Option<NamedDimension>, Uuid),

    Map(i64),
    StructureTemplate(NamespacedIdentifier),

    Scoreboard,
    TickingArea(Uuid),

    // NBT data which effectively maps numeric biome IDs to floats indicating that
    // biome's snow accumulation (the maximum height of snow layers that can naturally
    // accumulate during snowfall), or so I think.
    // TODO: confirm this
    BiomeData,
    BiomeIdsTable,
    // NBT with a few binary flags
    MobEvents,
    Portals,
    PositionTrackingDB(u32),
    PositionTrackingLastId,
    WanderingTraderScheduler,

    // data:
    // LimboEntities
    Overworld,
    Nether,
    TheEnd, // This one also has DragonFight

    // Other encountered keys from very old versions:

    /// No longer used
    FlatWorldLayers,
    /// No longer used
    LevelSpawnWasFixed, // I've only ever seen this with the string value "True"
    // idcounts   <- I've only heard of this, not seen this as a key.

    /// No longer used
    MVillages,
    /// No longer used
    Villages,
    // does this actually exist? did someone use it as a name for mVillages?
    // note that the raw key is, allegedly, "VillageManager"
    // LegacyVillageManager,

    /// No longer used
    Dimension0,
    /// No longer used
    Dimension1,
    /// No longer used
    Dimension2,

    RawKey(Vec<u8>),
}

impl DBKey {
    pub fn parse_key(raw_key: &[u8]) -> Self {
        Self::parse_recognized_key(raw_key).unwrap_or_else(|| Self::RawKey(raw_key.to_owned()))
    }

    pub fn parse_key_vec(raw_key: Vec<u8>) -> Self {
        Self::parse_recognized_key(&raw_key).unwrap_or(Self::RawKey(raw_key))
    }

    #[expect(
        clippy::too_many_lines,
        reason = "best to contain where raw keys are handled",
    )]
    pub fn parse_recognized_key(raw_key: &[u8]) -> Option<Self> {
        // Try some common prefixes first
        if (raw_key.len() == 12 || raw_key.len() == 16) && raw_key.starts_with(b"digp") {
            if let Some(dimensioned_pos) = DimensionedChunkPos::parse(&raw_key[4..]) {
                return Some(Self::ActorDigest(dimensioned_pos));
            }
        } else if raw_key.len() == 19 && raw_key.starts_with(b"actorprefix") {
            let actorid_bytes: [u8; 8] = raw_key.subslice_to_array::<11, 19>();

            return Some(Self::Actor(ActorID::parse(actorid_bytes)));
        }

        // Next, most data is chunk data, so we want to match against that before rarer keys.
        // AFAIK only "map_######" can collide with these keys (and would end up interpreted as a
        // chunk in an impossibly far-from-origin position), and a legacy "player_[ID]" plausibly
        // could have the same issue, too.
        let not_a_chunk_key = raw_key.starts_with(b"map") || raw_key.starts_with(b"player_");

        if (raw_key.len() == 9 || raw_key.len() == 13) && !not_a_chunk_key {
            // b'd' (Overworld), b'a' (BiomeData), b's' (mobevents / mVillages),
            // b'r' (~local_player), and b'e' (BiomeIdsTable)
            // should never be allowed as tags here, to avoid a collision. We don't check for
            // them above, since they're a colder path, and we don't strictly need to.
            // Note that:
            //    b'a' == 97
            //    b'd' == 100
            //    b'e' == 101
            //    b'r' == 114
            //    b's' == 115
            let tag = raw_key[raw_key.len() - 1];
            if ((43 <= tag && tag <= 65) && tag != 47) || tag == 118 || tag == 119 {
                if let Some(dimensioned_pos) =
                    DimensionedChunkPos::parse(&raw_key[..raw_key.len() - 1])
                {
                    // These chunk key numeric values are hardcoded twice in this file,
                    // and a few are also in prismarine-anchor-leveldb-values/src/checksums.rs
                    return Some(match tag {
                        43  => Self::Data3D                 (dimensioned_pos),
                        44  => Self::Version                (dimensioned_pos),
                        45  => Self::Data2D                 (dimensioned_pos),
                        46  => Self::LegacyData2D           (dimensioned_pos),
                        // 47 is subchunk block data, handled below
                        48  => Self::LegacyTerrain          (dimensioned_pos),
                        49  => Self::BlockEntities          (dimensioned_pos),
                        50  => Self::Entities               (dimensioned_pos),
                        51  => Self::PendingTicks           (dimensioned_pos),
                        52  => Self::LegacyExtraBlockData   (dimensioned_pos),
                        53  => Self::BiomeState             (dimensioned_pos),
                        54  => Self::FinalizedState         (dimensioned_pos),
                        55  => Self::ConversionData         (dimensioned_pos),
                        56  => Self::BorderBlocks           (dimensioned_pos),
                        57  => Self::HardcodedSpawners      (dimensioned_pos),
                        58  => Self::RandomTicks            (dimensioned_pos),
                        59  => Self::Checksums              (dimensioned_pos),
                        60  => Self::GenerationSeed         (dimensioned_pos),
                        61  => Self::CavesAndCliffsBlending (dimensioned_pos),
                        62  => Self::BlendingBiomeHeight    (dimensioned_pos),
                        63  => Self::MetaDataHash           (dimensioned_pos),
                        64  => Self::BlendingData           (dimensioned_pos),
                        65  => Self::ActorDigestVersion     (dimensioned_pos),
                        118 => Self::LegacyVersion          (dimensioned_pos),
                        119 => Self::AabbVolumes            (dimensioned_pos),
                        _ => unreachable!()
                    });
                }
            }

        } else if (raw_key.len() == 10 || raw_key.len() == 14) && !not_a_chunk_key {
            // Subchunk keys are slightly different from the others

            // Note that 47 is b'/', and that `scoreboard`, `dimension0`, and `dimension1`
            // would enter this `else if` block before failing this check.
            if raw_key[raw_key.len() - 2] == 47 {
                if let Some(dimensioned_pos) =
                    DimensionedChunkPos::parse(&raw_key[..raw_key.len() - 2])
                {
                    return Some(Self::SubchunkBlocks(
                        dimensioned_pos,
                        raw_key[raw_key.len() - 1] as i8,
                    ));
                }
            }
        }

        // NOT else if: some of these keys may be 9, 10, 13, or 14 bytes long.
        if let Ok(key_string) = str::from_utf8(raw_key) {
            let parts: Vec<&str> = key_string.split('_').collect();

            // The majority of the rest of the keys will likely be villages and maps

            // VILLAGE_[DIMENSION]?_[UUID]_[VARIANT]
            if (parts.len() == 3 || parts.len() == 4)
                && parts[0] == "VILLAGE"
                && ["DWELLERS", "INFO", "PLAYERS", "POI", "RAID"]
                    .contains(&parts[parts.len() - 1])
            {
                // Note that len is 3 or 4, so this doesn't overflow or panic
                if let Some(uuid) = Uuid::parse(parts[parts.len() - 2]) {
                    let dimension = if parts.len() == 4 {
                        // Dimension included
                        Some(NamedDimension::from_bedrock_name(parts[1]))
                    } else {
                        None
                    };

                    return Some(match parts[parts.len() - 1] {
                        "DWELLERS" => Self::VillageDwellers(dimension, uuid),
                        "INFO"     => Self::VillageInfo(dimension, uuid),
                        "PLAYERS"  => Self::VillagePlayers(dimension, uuid),
                        "POI"      => Self::VillagePOI(dimension, uuid),
                        "RAID"     => Self::VillageRaid(dimension, uuid),
                        _ => unreachable!(),
                    });
                }

            } else if parts.len() == 2 && parts[0] == "map" {
                // Maps
                if let Ok(map_id) = i64::from_str_radix(parts[1], 10) {
                    return Some(Self::Map(map_id));
                }

            } else if parts.len() == 2 && parts[0] == "player" {
                // A remote player
                if let Some(uuid) = Uuid::parse(parts[1]) {
                    return Some(Self::Player(uuid));

                } else if let Ok(id) = u64::from_str_radix(parts[1], 10) {
                    return Some(Self::LegacyPlayer(id));
                }

            } else if parts.len() == 3 && parts[0] == "player" && parts[1] == "server" {
                // A player, probably the local player?
                if let Some(uuid) = Uuid::parse(parts[2]) {
                    return Some(Self::PlayerServer(uuid));
                }

            } else if parts.len() == 2 && parts[0] == "tickingarea" {
                // A ticking area, could be in any dimension.
                if let Some(uuid) = Uuid::parse(parts[1]) {
                    return Some(Self::TickingArea(uuid));
                }
            }

            if let Some(structure_identifier) = key_string.strip_prefix("structuretemplate_") {
                if let Ok(identifier) = NamespacedIdentifier::parse_string(
                    structure_identifier.to_owned(),
                    IdentifierParseOptions {
                        default_namespace:          None,
                        java_character_constraints: false,
                    },
                ) {
                    return Some(Self::StructureTemplate(identifier));
                }
            }

            // Next, try the new tag with a weird format
            if let Some(num) = key_string.strip_prefix("PosTrackDB-0x") {
                if let Ok(num) = u32::from_str_radix(num, 16) {
                    return Some(Self::PositionTrackingDB(num));
                }
            }

            // Dimensions
            if let Some(dimension) = VanillaDimension::try_from_bedrock_name(key_string) {
                return Some(match dimension {
                    VanillaDimension::Overworld => Self::Overworld,
                    VanillaDimension::Nether    => Self::Nether,
                    VanillaDimension::End       => Self::TheEnd,
                });
            }

            // Lastly, there's odds-and-ends which only have one key per world
            return Some(match key_string {
                "~local_player"                 => Self::LocalPlayer,
                "LevelChunkMetaDataDictionary"  => Self::LevelChunkMetaDataDictionary,
                "AutonomousEntities"            => Self::AutonomousEntities,
                "scoreboard"                    => Self::Scoreboard,
                "BiomeData"                     => Self::BiomeData,
                "BiomeIdsTable"                 => Self::BiomeIdsTable,
                "mobevents"                     => Self::MobEvents,
                "portals"                       => Self::Portals,
                "PositionTrackDB-LastId"        => Self::PositionTrackingLastId,
                "schedulerWT"                   => Self::WanderingTraderScheduler,
                "game_flatworldlayers"          => Self::FlatWorldLayers,
                "LevelSpawnWasFixed"            => Self::LevelSpawnWasFixed,
                "mVillages"                     => Self::MVillages,
                "villages"                      => Self::Villages,
                "dimension0"                    => Self::Dimension0,
                "dimension1"                    => Self::Dimension1,
                "dimension2"                    => Self::Dimension2,
                _ => return None,
            });
        }

        if raw_key.len() <= 100 {
            log::warn!("Could not parse DBKey: {raw_key:?}");
        } else {
            log::warn!("Could not parse DBKey. First 100 bytes: {:?}", &raw_key[..100]);
            log::trace!("Remainder of unparsed DBKey: {:?}", &raw_key[100..]);
        }

        None
    }

    /// Extend the provided `Vec` with the raw key bytes of a `DBKey`,
    /// using the provided serialization settings.
    #[expect(
        clippy::too_many_lines,
        reason = "best to contain where raw keys are handled",
    )]
    pub fn extend_serialized(&self, bytes: &mut Vec<u8>, opts: KeyToBytesOptions) {

        fn extend_village(
            bytes:                &mut Vec<u8>,
            dimension:            &Option<NamedDimension>,
            uuid:                 &Uuid,
            write_overworld_name: OverworldElision,
            variant:              &'static [u8],
        ) {
            let dimension_name = write_overworld_name
                .maybe_elide_name(dimension.as_ref());

            if let Some(dimension_name) = dimension_name {
                let dimension = dimension_name
                    .as_bedrock_name()
                    .as_bytes();

                bytes.reserve(b"VILLAGE".len() + 3 + variant.len() + dimension.len() + 36);
                bytes.extend(b"VILLAGE_");
                bytes.extend(dimension);
                bytes.push(b'_');
                uuid.extend_serialized(bytes);
                bytes.push(b'_');
                bytes.extend(variant);

            } else {
                bytes.reserve(b"VILLAGE".len() + 2 + variant.len() + 36);
                bytes.extend(b"VILLAGE_");
                uuid.extend_serialized(bytes);
                bytes.push(b'_');
                bytes.extend(variant);
            }
        }

        let (dimensioned_pos, key_tag) = match self {
            // These chunk key numeric values are hardcoded twice in this file,
            // and a few are also in prismarine-anchor-leveldb-values/src/checksums.rs
            &Self::Data3D                 (d_pos) => (d_pos, 43),
            &Self::Version                (d_pos) => (d_pos, 44),
            &Self::Data2D                 (d_pos) => (d_pos, 45),
            &Self::LegacyData2D           (d_pos) => (d_pos, 46),
            // 47 is handled below
            &Self::LegacyTerrain          (d_pos) => (d_pos, 48),
            &Self::BlockEntities          (d_pos) => (d_pos, 49),
            &Self::Entities               (d_pos) => (d_pos, 50),
            &Self::PendingTicks           (d_pos) => (d_pos, 51),
            &Self::LegacyExtraBlockData   (d_pos) => (d_pos, 52),
            &Self::BiomeState             (d_pos) => (d_pos, 53),
            &Self::FinalizedState         (d_pos) => (d_pos, 54),
            &Self::ConversionData         (d_pos) => (d_pos, 55),
            &Self::BorderBlocks           (d_pos) => (d_pos, 56),
            &Self::HardcodedSpawners      (d_pos) => (d_pos, 57),
            &Self::RandomTicks            (d_pos) => (d_pos, 58),
            &Self::Checksums              (d_pos) => (d_pos, 59),
            &Self::GenerationSeed         (d_pos) => (d_pos, 60),
            &Self::CavesAndCliffsBlending (d_pos) => (d_pos, 61),
            &Self::BlendingBiomeHeight    (d_pos) => (d_pos, 62),
            &Self::MetaDataHash           (d_pos) => (d_pos, 63),
            &Self::BlendingData           (d_pos) => (d_pos, 64),
            &Self::ActorDigestVersion     (d_pos) => (d_pos, 65),
            &Self::LegacyVersion          (d_pos) => (d_pos, 118),
            &Self::AabbVolumes            (d_pos) => (d_pos, 119),

            // Time for a "little" side trip
            &Self::SubchunkBlocks(d_pos, subchunk) => {
                bytes.reserve(14);
                d_pos.extend_serialized(bytes, opts.write_overworld_id);
                bytes.push(47);
                bytes.push(subchunk as u8);
                return;
            }
            &Self::ActorDigest(dimensioned_pos) => {
                bytes.reserve(16);
                bytes.extend(b"digp");
                dimensioned_pos.extend_serialized(bytes, opts.write_overworld_id);
                return;
            }
            &Self::Actor(actor_id) => {
                bytes.reserve(19);
                bytes.extend(b"actorprefix");
                actor_id.extend_serialized(bytes);
                return;
            }
            &Self::LevelChunkMetaDataDictionary => {
                bytes.extend(b"LevelChunkMetaDataDictionary");
                return;
            }
            &Self::AutonomousEntities => {
                bytes.extend(b"AutonomousEntities");
                return;
            }
            &Self::LocalPlayer => {
                bytes.extend(b"~local_player");
                return;
            }
            &Self::Player(uuid) => {
                bytes.reserve(b"player_".len() + 36);
                bytes.extend(b"player_");
                uuid.extend_serialized(bytes);
                return;
            }
            &Self::LegacyPlayer(id) => {
                let id_str = format!("{id}");

                bytes.reserve(b"player_".len() + id_str.len());
                bytes.extend(b"player_");
                bytes.extend(id_str.as_bytes());
                return;
            }
            &Self::PlayerServer(uuid) => {
                bytes.reserve(b"player_server_".len() + 36);
                bytes.extend(b"player_server_");
                uuid.extend_serialized(bytes);
                return;
            }
            Self::VillageDwellers(dimension, uuid) => {
                extend_village(bytes, dimension, uuid, opts.write_overworld_name, b"DWELLERS");
                return;
            }
            Self::VillageInfo(dimension, uuid) => {
                extend_village(bytes, dimension, uuid, opts.write_overworld_name, b"INFO");
                return;
            }
            Self::VillagePOI(dimension, uuid) => {
                extend_village(bytes, dimension, uuid, opts.write_overworld_name, b"POI");
                return;
            }
            Self::VillagePlayers(dimension, uuid) => {
                extend_village(bytes, dimension, uuid, opts.write_overworld_name, b"PLAYERS");
                return;
            }
            Self::VillageRaid(dimension, uuid) => {
                extend_village(bytes, dimension, uuid, opts.write_overworld_name, b"RAID");
                return;
            }
            &Self::Map(map_id) => {
                bytes.extend(b"map_");
                bytes.extend(format!("{map_id}").as_bytes());
                return;
            }
            Self::StructureTemplate(identifier) => {
                let identifier_len = identifier.namespace.len() + identifier.path.len() + 1;

                bytes.reserve(b"structuretemplate_".len() + identifier_len);
                bytes.extend(b"structuretemplate_");
                bytes.extend(identifier.namespace.as_bytes());
                bytes.push(b':');
                bytes.extend(identifier.path.as_bytes());
                return;
            }
            &Self::Scoreboard => {
                bytes.extend(b"scoreboard");
                return;
            }
            &Self::TickingArea(uuid) => {
                bytes.reserve(b"tickingarea_".len() + 36);
                bytes.extend(b"tickingarea_");
                uuid.extend_serialized(bytes);
                return;
            }
            &Self::BiomeData => {
                bytes.extend(b"BiomeData");
                return;
            }
            &Self::BiomeIdsTable => {
                bytes.extend(b"BiomeIdsTable");
                return;
            }
            &Self::MobEvents => {
                bytes.extend(b"mobevents");
                return;
            }
            &Self::Portals => {
                bytes.extend(b"portals");
                return;
            }
            &Self::PositionTrackingDB(id) => {
                let id = format!("{id:08x}");

                bytes.reserve(b"PosTrackDB-0x".len() + id.len());
                bytes.extend(b"PosTrackDB-0x");
                bytes.extend(id.as_bytes());
                return;
            }
            &Self::PositionTrackingLastId => {
                bytes.extend(b"PositionTrackDB-LastId");
                return;
            }
            &Self::WanderingTraderScheduler => {
                bytes.extend(b"schedulerWT");
                return;
            }
            &Self::Overworld => {
                bytes.extend(VanillaDimension::Overworld.to_bedrock_name().as_bytes());
                return;
            }
            &Self::Nether => {
                bytes.extend(VanillaDimension::Nether.to_bedrock_name().as_bytes());
                return;
            }
            &Self::TheEnd => {
                bytes.extend(VanillaDimension::End.to_bedrock_name().as_bytes());
                return;
            }
            &Self::FlatWorldLayers => {
                bytes.extend(b"game_flatworldlayers");
                return;
            }
            &Self::LevelSpawnWasFixed => {
                bytes.extend(b"LevelSpawnWasFixed");
                return;
            }
            &Self::MVillages => {
                bytes.extend(b"mVillages");
                return;
            }
            &Self::Villages => {
                bytes.extend(b"villages");
                return;
            }
            &Self::Dimension0 => {
                bytes.extend(b"dimension0");
                return;
            }
            &Self::Dimension1 => {
                bytes.extend(b"dimension1");
                return;
            }
            &Self::Dimension2 => {
                bytes.extend(b"dimension2");
                return;
            }
            Self::RawKey(raw_key) => {
                bytes.extend(raw_key);
                return;
            }
        };

        // Look back at the top of the match for context;
        // a lot of cases would have these lines in common.
        bytes.reserve(13);
        dimensioned_pos.extend_serialized(bytes, opts.write_overworld_id);
        bytes.push(key_tag);
    }

    /// Get the raw key bytes of a `DBKey` with the provided serialization options.
    pub fn to_bytes(&self, opts: KeyToBytesOptions) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes, opts);
        bytes
    }
}

impl From<&[u8]> for DBKey {
    fn from(raw_key: &[u8]) -> Self {
        Self::parse_key(raw_key)
    }
}

impl From<Vec<u8>> for DBKey {
    fn from(raw_key: Vec<u8>) -> Self {
        Self::parse_key_vec(raw_key)
    }
}
