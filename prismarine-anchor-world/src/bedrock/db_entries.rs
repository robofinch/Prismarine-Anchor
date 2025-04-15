use std::str;
use std::io::{Cursor, Read, Write};

use indexmap::IndexMap;
use thiserror::Error;

use prismarine_anchor_nbt::{settings::IoOptions, NbtCompound};
use prismarine_anchor_nbt::io::{read_nbt, write_nbt, NbtIoError};
use prismarine_anchor_translation::datatypes::{IdentifierParseOptions, NamespacedIdentifier};

use crate::shared_types::{ChunkPosition, NamedDimension, NumericDimension, VanillaDimension, UUID};


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DimensionedChunkPos(ChunkPosition, NumericDimension);

impl DimensionedChunkPos {
    /// Attempt to parse the bytes as a `ChunkPosition` followed by an optional `NumericDimension`.
    /// The dimension defaults to the Overworld if not present.
    ///
    /// Warning: the `NumericDimension` might not be a vanilla dimension, which could indicate
    /// that a successful parse is unintended.
    pub fn new_raw(bytes: &[u8]) -> Option<Self> {
        if bytes.len() == 8 {
            Some(Self(
                ChunkPosition {
                    x: i32::from_le_bytes(bytes[0..4].try_into().unwrap()),
                    y: i32::from_le_bytes(bytes[4..8].try_into().unwrap()),
                },
                NumericDimension::OVERWORLD,
            ))

        } else if bytes.len() == 12 {

            let dimension_id = u32::from_le_bytes(bytes[8..12].try_into().unwrap());

            Some(Self(
                ChunkPosition {
                    x: i32::from_le_bytes(bytes[0..4].try_into().unwrap()),
                    y: i32::from_le_bytes(bytes[4..8].try_into().unwrap()),
                },
                NumericDimension::from_bedrock_numeric(dimension_id),
            ))

        } else {
            None
        }
    }

    /// Extend the provided bytes with the byte format of a `DimensionedChunkPos`, namely
    /// a `ChunkPosition` followed by a `NumericDimension`. If the dimension
    /// is the Overworld, its dimension ID doesn't need to be serialized, but if
    /// `write_overworld_id` is true, then it will be.
    pub fn extend_serialized(self, bytes: &mut Vec<u8>, write_overworld_id: bool) {
        bytes.reserve(12);
        bytes.extend(self.0.x.to_le_bytes());
        bytes.extend(self.0.y.to_le_bytes());
        if write_overworld_id || self.1.to_bedrock_numeric() != 0 {
            bytes.extend(self.1.to_bedrock_numeric().to_le_bytes());
        }
    }

    pub fn to_bytes(self, write_overworld_id: bool) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes, write_overworld_id);
        bytes
    }
}

impl TryFrom<&[u8]> for DimensionedChunkPos {
    type Error = ();

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::new_raw(value).ok_or(())
    }
}

/// The keys in a world's LevelDB database used by Minecraft Bedrock.
/// Based on information from [minecraft.wiki]
/// and data from iterating through an actual world's keys.
///
/// [minecraft.wiki]: https://minecraft.wiki/w/Bedrock_Edition_level_format#Chunk_key_format
// TODO: are "since 1.18.0" and "since 1.0.0" the precise versions that something changed?
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BedrockLevelDBKey {
    // ================================
    //  Chunk-specific data
    // ================================

    // TODO: figure out what this is.
    // Is this the same as the data version? surely not game version?
    // Is it a different number I'll need mappings for?
    Version(DimensionedChunkPos),
    /// The `Version` key started being used instead in 1.16.100.
    LegacyVersion(DimensionedChunkPos),
    // TODO: figure out the possible values and what it does, if it's used.
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
    /// NBT data of entities.
    Entities(DimensionedChunkPos),
    /// NBT data of pending ticks.
    PendingTicks(DimensionedChunkPos),
    /// NBT data of random ticks
    RandomTicks(DimensionedChunkPos),

    /// Used in Education Edition
    BorderBlocks(DimensionedChunkPos),
    /// Bounding boxes for structure spawns, such as a Nether Fortress or Pillager Outpost.
    /// No longer used; structure spawns are no longer saved to the world file at all.
    // TODO: no longer used as of when?
    HardcodedSpawners(DimensionedChunkPos),
    // Found in levilamina, key tag value 119 (b'w'). Related to hardcoded spawners, maybe?
    // TODO: what is this?
    AabbVolumes(DimensionedChunkPos),

    /// xxHash64 checksums of `SubchunkBlocks`, `BlockEntities`, `Entities`, and `Data2D`
    /// values. Not written since 1.18.0.
    Checksums(DimensionedChunkPos),
    /// A hash which is the key in the `LevelChunkMetaDataDictionary` record
    /// for the NBT metadata of this chunk. ...Probably. Seems to default to something dependent
    /// on the current game version, presumably the version in level.dat?
    /// TODO: figure out what's going on with metadata.
    MetaDataHash(DimensionedChunkPos),

    /// The seed which was used to generate this chunk.
    // TODO: what versions use this?
    GenerationSeed(DimensionedChunkPos),
    /// Indicates the state of world generation in this chunk, such as whether it
    /// has been fully generated.
    FinalizedState(DimensionedChunkPos),
    // Map from biomes to a byte indicating a snow level. Probably the maximum snow level
    // which snow can pile up to naturally in that chunk.
    // TODO: confirm this
    BiomeState(DimensionedChunkPos),

    /// No longer used.
    // TODO: figure out what it was
    ConversionData(DimensionedChunkPos),

    // TODO: figure out what these three are
    CavesAndCliffsBlending(DimensionedChunkPos),
    BlendingBiomeHeight(DimensionedChunkPos),
    // Is this still used?
    BlendingData(DimensionedChunkPos),

    /// Actor Digest data
    // TODO: is this key format actually correct?
    ActorDigest(DimensionedChunkPos),

    // ================================
    //  Data not specific to a chunk
    // ================================

    // TODO: figure out if this key format is correct
    Actor(u64),
    /// Stores the NBT metadata of all chunks. Maps the xxHash64 hash of NBT data
    /// to that NBT data, so that each chunk need only store 8 bytes instead of the entire
    /// NBT; most chunks have the same metadata.
    // TODO: validate that the 64-bit hashes are indeed xxHash64
    // TODO: if a chunk doesn't have a MetaDataHash, what metadata does it use? the metadata
    // associated with the current BaseGameVersion, probably?
    LevelChunkMetaDataDictionary,

    // TODO: this actually exists. Figure out what it is.
    AutonomousEntities,

    /// NBT data of the world's local player entity.
    LocalPlayer,
    /// NBT data of a player with the indicated UUID
    Player(UUID),
    /// NBT data (TODO: confirm) of a player with the indicted numeric ID, which comes
    /// from `clientid.txt`.
    // (based on a number stored in `clientid.txt`, which seems to fit in 64 bits or 63 unsigned).
    LegacyPlayer(i64),
    // TODO: apparently this exists. What is it? is it an alternative version of ~local_player?
    PlayerServer(UUID),

    /// Key has the dimension which the village is in, and the name of the village.
    // TODO: should have entity IDs of village dwellers. Verify.
    VillageDwellers(NamedDimension, UUID),
    /// Key has the dimension which the village is in, and the name of the village.
    // Village bounding box, and possibly other information TODO: figure out what
    VillageInfo(NamedDimension, UUID),
    /// Key has the dimension which the village is in, and the name of the village.
    // villager - workstation mappings. Bed mappings aren't stored, apparently.
    VillagePOI(NamedDimension, UUID),
    /// Key has the dimension which the village is in, and the name of the village.
    // Probably tracking player reputation or something? Idk. TODO: figure it out
    VillagePlayers(NamedDimension, UUID),

    Map(i64),
    Portals,

    StructureTemplate(NamespacedIdentifier),
    // TODO: make a world with a tickingarea to figure out the key format
    TickingArea(UUID),
    Scoreboard,
    // TODO: rename to WanderingTraderScheduler if this is what I think it is
    WanderingTraderScheduler,

    // TODO: these seem to have small amounts of metadata. Are they still used?
    // NBT data which effectively maps numeric biome IDs to floats indicating that
    // biome's snow accumulation (the maximum height of snow layers that can naturally
    // accumulate during snowfall).
    // TODO: confirm this
    BiomeData,
    // NBT with a few binary flags
    MobEvents,

    // TODO: what do these do? Are they still used? - probably Limbo data, and who knows what else.
    Overworld,
    Nether,
    TheEnd,

    // New and exciting tags, with very little information about them
    PositionTrackingDB(u32),
    PositionTrackingLastId,

    /// This key, `game_flatworldlayers`, only seems to be used in very old versions.
    FlatWorldLayers,

    // TODO: other encountered keys from very old versions:
    // mVillages
    // villages
    // dimension0 <- presumably dimension1 and dimension2 then, as well?
    // idcounts   <- I've only heard of this, not seen this as a key.

    RawKey(Vec<u8>),
}

impl BedrockLevelDBKey {
    pub fn parse_key(raw_key: &[u8]) -> Self {
        Self::parse_recognized_key(raw_key).unwrap_or_else(|| Self::RawKey(raw_key.to_owned()))
    }

    pub fn parse_key_vec(raw_key: Vec<u8>) -> Self {
        Self::parse_recognized_key(&raw_key).unwrap_or(Self::RawKey(raw_key))
    }

    pub fn parse_recognized_key(raw_key: &[u8]) -> Option<Self> {
        // Try some common prefixes first
        if (raw_key.len() == 12 || raw_key.len() == 16) && raw_key.starts_with(b"digp") {
            if let Ok(dimensioned_pos) = DimensionedChunkPos::try_from(&raw_key[4..]) {
                return Some(Self::ActorDigest(dimensioned_pos));
            }
        } else if raw_key.len() == 19 && raw_key.starts_with(b"actorprefix") {
            // The unwrap is turning a slice of length 8 into an array of length 8
            let actor_id = u64::from_be_bytes(raw_key[11..19].try_into().unwrap());
            return Some(Self::Actor(actor_id));
        }

        // Next, most data is chunk data.
        // AFAIK only "map_######" can collide with these keys (with a custom dimension number),
        // or a legacy "player_[ID]" plausibly could, too
        if (raw_key.len() == 9 || raw_key.len() == 13)
            && !raw_key.starts_with(b"map")
            && !raw_key.starts_with(b"player_")
        {

            // b'd' (Overworld), b'a' (BiomeData), and b's' (mobevents)
            // should never be allowed as tags here, to avoid a collision.
            let tag = raw_key[raw_key.len() - 1];
            if (43 <= tag && tag <= 65 && tag != 47) || tag == 118 || tag == 119 {

                if let Ok(dimensioned_pos) = DimensionedChunkPos::try_from(
                    &raw_key[..raw_key.len() - 1]
                ) {
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

        } else if (raw_key.len() == 10 || raw_key.len() == 14) && !raw_key.starts_with(b"map") {
            // Subchunk keys are slightly different from the others

            if raw_key[raw_key.len() - 2] == 47 {
                if let Ok(dimensioned_pos) = DimensionedChunkPos::try_from(
                    &raw_key[..raw_key.len() - 2]
                ) {
                    return Some(Self::SubchunkBlocks(
                        dimensioned_pos,
                        raw_key[raw_key.len() - 1] as i8,
                    ));
                }
            }
        }

        // NOT else if: some of these keys may be 9, 10, 13, or 14 bytes long.
        if let Ok(key_string) = str::from_utf8(&raw_key) {
            let parts: Vec<&str> = key_string.split('_').collect();

            // The majority of the rest of the keys will likely be villages and maps

            if (parts.len() == 3 || parts.len() == 4)
                && parts[0] == "VILLAGE"
                && ["DWELLERS", "INFO", "PLAYERS", "POI"].contains(&parts[parts.len() - 1])
            {
                // Villages
                if let Some(uuid) =  UUID::new(parts[parts.len() - 2]) {

                    let dimension = if parts.len() == 4 {
                        // Dimension included
                        NamedDimension::from_bedrock_name(parts[1])
                    } else {
                        NamedDimension::OVERWORLD
                    };

                    return Some(match parts[parts.len() - 1] {
                        "DWELLERS" => Self::VillageDwellers(dimension, uuid),
                        "INFO"     => Self::VillageInfo(dimension, uuid),
                        "PLAYERS"  => Self::VillagePlayers(dimension, uuid),
                        "POI"      => Self::VillagePOI(dimension, uuid),
                        _ => unreachable!(),
                    });
                }

            } else if parts.len() == 2 && parts[0] == "map" {
                // Maps
                if let Ok(map_id) = i64::from_str_radix(parts[1], 10) {
                    return Some(Self::Map(map_id));
                }

            } else if parts.len() == 2 && parts[0] == "structuretemplate" {
                // Structure templates
                if let Ok(identifier) = NamespacedIdentifier::parse_string(
                    parts[1].to_owned(),
                    IdentifierParseOptions {
                        default_namespace: None,
                        java_character_constraints: false,
                    },
                ) {
                    return Some(Self::StructureTemplate(identifier));
                }

            } else if parts.len() == 2 && parts[0] == "player" {
                // A remote player
                if let Some(uuid) = UUID::new(parts[1]) {
                    return Some(Self::Player(uuid));

                } else if let Ok(id) = i64::from_str_radix(parts[1], 10) {
                    return Some(Self::LegacyPlayer(id))
                }

            } else if parts.len() == 3 && parts[0] == "player" && parts[1] == "server" {
                // A player, probably the local player?
                if let Some(uuid) = UUID::new(parts[2]) {
                    return Some(Self::PlayerServer(uuid));
                }

            } else if parts.len() == 2 && parts[0] == "tickingarea" {
                // A ticking area, could be in any dimension.
                if let Some(uuid) = UUID::new(parts[1]) {
                    return Some(Self::TickingArea(uuid));
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
                "portals"                       => Self::Portals,
                "scoreboard"                    => Self::Scoreboard,
                "schedulerWT"                   => Self::WanderingTraderScheduler,
                "BiomeData"                     => Self::BiomeData,
                "mobevents"                     => Self::MobEvents,
                "PositionTrackDB-LastId"        => Self::PositionTrackingLastId,
                "game_flatworldlayers"          => Self::FlatWorldLayers,
                _ => return None,
            });
        }

        None
    }

    /// Extend the provided `Vec` with the raw key bytes of a `BedrockLevelDBKey`.
    ///
    /// If `write_overworld_id` is false, then only non-Overworld dimensions will have their
    /// numeric IDs written when a `NumericDimension` is serialized.
    /// Likewise, if `write_overworld_name` is false, then only non-Overworld dimensions
    /// will have their names written when a `NamedDimension` is serialized.
    ///
    /// The best choice is `write_overworld_id = false` for all current versions
    /// (up to at least 1.21.51), `write_overworld_name = true` for any version at or above
    /// 1.20.40, and `write_overworld_name = false` for any version below 1.20.40.
    pub fn extend_serialized(
        &self, bytes: &mut Vec<u8>,
        write_overworld_id: bool, write_overworld_name: bool,
    ) {

        fn extend_village(
            bytes: &mut Vec<u8>,
            dimension: &NamedDimension,
            uuid: &UUID,
            variant: &'static [u8],
            write_overworld_name: bool,
        ) {
            if write_overworld_name || dimension != &NamedDimension::OVERWORLD {

                let dimension = dimension.as_bedrock_name().as_bytes();

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
                d_pos.extend_serialized(bytes, write_overworld_id);
                bytes.push(47);
                bytes.push(subchunk as u8);
                return
            }
            &Self::ActorDigest(dimensioned_pos) => {
                bytes.reserve(16);
                bytes.extend(b"digp");
                dimensioned_pos.extend_serialized(bytes, write_overworld_id);
                return
            }
            &Self::Actor(actor_id) => {
                bytes.reserve(19);
                bytes.extend(b"actorprefix");
                bytes.extend(actor_id.to_be_bytes());
                return
            }
            &Self::LevelChunkMetaDataDictionary => {
                bytes.extend(b"LevelChunkMetaDataDictionary");
                return
            }
            &Self::AutonomousEntities => {
                bytes.extend(b"AutonomousEntities");
                return
            }
            &Self::LocalPlayer => {
                bytes.extend(b"~local_player");
                return
            }
            &Self::Player(uuid) => {
                bytes.reserve(b"player_".len() + 36);
                bytes.extend(b"player_");
                uuid.extend_serialized(bytes);
                return
            }
            &Self::LegacyPlayer(id) => {
                let id_str = format!("{id}");

                bytes.reserve(b"player_".len() + id_str.len());
                bytes.extend(b"player_");
                bytes.extend(id_str.as_bytes());
                return
            }
            &Self::PlayerServer(uuid) => {
                bytes.reserve(b"player_server_".len() + 36);
                bytes.extend(b"player_server_");
                uuid.extend_serialized(bytes);
                return
            }
            Self::VillageDwellers(dimension, uuid) => {
                extend_village(bytes, dimension, uuid, b"DWELLERS", write_overworld_name);
                return
            }
            Self::VillageInfo(dimension, uuid) => {
                extend_village(bytes, dimension, uuid, b"INFO", write_overworld_name);
                return
            }
            Self::VillagePOI(dimension, uuid) => {
                extend_village(bytes, dimension, uuid, b"POI", write_overworld_name);
                return
            }
            Self::VillagePlayers(dimension, uuid) => {
                extend_village(bytes, dimension, uuid, b"PLAYERS", write_overworld_name);
                return
            }
            &Self::Map(map_id) => {
                bytes.extend(b"map_");
                bytes.extend(format!("{}", map_id).as_bytes());
                return
            }
            &Self::Portals => {
                bytes.extend(b"portals");
                return
            }
            Self::StructureTemplate(identifier) => {
                let identifier_len = identifier.namespace.len() + identifier.path.len() + 1;

                bytes.reserve(b"structuretemplate_".len() + identifier_len);
                bytes.extend(b"structuretemplate_");
                bytes.extend(identifier.namespace.as_bytes());
                bytes.push(b':');
                bytes.extend(identifier.path.as_bytes());
                return
            }
            &Self::TickingArea(uuid) => {
                bytes.reserve(b"tickingarea_".len() + 36);
                bytes.extend(b"tickingarea_");
                uuid.extend_serialized(bytes);
                return
            }
            &Self::Scoreboard => {
                bytes.extend(b"scoreboard");
                return
            }
            &Self::WanderingTraderScheduler => {
                bytes.extend(b"schedulerWT");
                return
            }
            &Self::BiomeData => {
                bytes.extend(b"BiomeData");
                return
            }
            &Self::MobEvents => {
                bytes.extend(b"mobevents");
                return
            }
            &Self::Overworld => {
                bytes.extend(VanillaDimension::Overworld.to_bedrock_name().as_bytes());
                return
            }
            &Self::Nether => {
                bytes.extend(VanillaDimension::Nether.to_bedrock_name().as_bytes());
                return
            }
            &Self::TheEnd => {
                bytes.extend(VanillaDimension::End.to_bedrock_name().as_bytes());
                return
            }
            &Self::PositionTrackingDB(id) => {
                let id = format!("{:08x}", id);

                bytes.reserve(b"PosTrackDB-0x".len() + id.len());
                bytes.extend(b"PosTrackDB-0x");
                bytes.extend(id.as_bytes());
                return
            }
            &Self::PositionTrackingLastId => {
                bytes.extend(b"PositionTrackDB-LastId");
                return
            }
            &Self::FlatWorldLayers => {
                bytes.extend(b"game_flatworldlayers");
                return
            }
            Self::RawKey(raw_key) => {
                bytes.extend(raw_key);
                return
            }
        };

        // Look back at the top of the function for context
        bytes.reserve(13);
        dimensioned_pos.extend_serialized(bytes, write_overworld_id);
        bytes.push(key_tag);
    }

    /// Get the raw key bytes of a `BedrockLevelDBKey`.
    ///
    /// If `write_overworld_id` is false, then only non-Overworld dimensions will have their
    /// numeric IDs written when a `NumericDimension` is serialized.
    /// Likewise, if `write_overworld_name` is false, then only non-Overworld dimensions
    /// will have their names written when a `NamedDimension` is serialized.
    ///
    /// The best choice is `write_overworld_id = false` for all current versions
    /// (up to at least 1.21.51), `write_overworld_name = true` for any version at or above
    /// 1.20.40, and `write_overworld_name = false` for any version below 1.20.40.
    pub fn into_bytes(self, write_overworld_id: bool, write_overworld_name: bool) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes, write_overworld_id, write_overworld_name);
        bytes
    }
}

impl From<&[u8]> for BedrockLevelDBKey {
    fn from(raw_key: &[u8]) -> Self {
        Self::parse_key(raw_key)
    }
}

impl From<Vec<u8>> for BedrockLevelDBKey {
    fn from(raw_key: Vec<u8>) -> Self {
        Self::parse_key_vec(raw_key)
    }
}

/// The entries in a world's LevelDB database used by Minecraft Bedrock.
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
    /// Always zero. Presumably here for future compatibility.
    ActorDigestVersion(DimensionedChunkPos, u8),

    // Data3D(DimensionedChunkPos),
    // Data2D(DimensionedChunkPos),
    // LegacyData2D(DimensionedChunkPos),

    // SubchunkBlocks(DimensionedChunkPos, i8),
    // LegacyTerrain(DimensionedChunkPos),
    // LegacyExtraBlockData(DimensionedChunkPos),

    // BlockEntities(DimensionedChunkPos),
    // Entities(DimensionedChunkPos),
    // PendingTicks(DimensionedChunkPos),
    // RandomTicks(DimensionedChunkPos),

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

    // Actor(u64),

    LevelChunkMetaDataDictionary(IndexMap<u64, NbtCompound>),

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
    // dimension1 <- not sure if it exists, but dimension0 does.
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
                    return ValueParseResult::Parsed(Self::Version(chunk_pos, value[0]));
                }
            }
            BedrockLevelDBKey::LegacyVersion(chunk_pos) => {
                if value.len() == 1 {
                    return ValueParseResult::Parsed(Self::LegacyVersion(chunk_pos, value[0]));
                }
            }
            BedrockLevelDBKey::ActorDigestVersion(chunk_pos) => {
                if value.len() == 1 {
                    return ValueParseResult::Parsed(Self::ActorDigestVersion(chunk_pos, value[0]));
                }
            }
            BedrockLevelDBKey::MetaDataHash(chunk_pos) => {
                if let Ok(bytes) = <[u8; 8]>::try_from(value) {
                    return ValueParseResult::Parsed(
                        Self::MetaDataHash(chunk_pos, u64::from_le_bytes(bytes))
                    );
                }
            }
            BedrockLevelDBKey::LevelChunkMetaDataDictionary => {
                if value.len() >= 4 {
                    let num_entries = u32::from_le_bytes(value[0..4].try_into().unwrap());

                    let mut reader = Cursor::new(&value[4..]);
                    let mut map = IndexMap::new();

                    // println!("Expected num entries: {num_entries}");

                    for _ in 0..num_entries {

                        // println!("Reading hash...");
                        let mut hash = [0; 8];
                        let Ok(()) = reader.read_exact(&mut hash) else {
                            return ValueParseResult::UnrecognizedValue(key);
                        };

                        // println!("Reading NBT...");

                        let nbt_result = read_nbt(
                            &mut reader,
                            IoOptions::bedrock_uncompressed(),
                        );
                        let nbt = match nbt_result {
                            Ok((nbt, _)) => nbt,
                            Err(_) => {
                                // println!("Error: {err}");
                                return ValueParseResult::UnrecognizedValue(key)
                            }
                        };

                        // println!("Checking for duplicate...");
                        // Reject if there's a duplicate hash
                        let None = map.insert(u64::from_le_bytes(hash), nbt) else {
                            return ValueParseResult::UnrecognizedValue(key);
                        };

                    }

                    // println!("Checking for excess...");
                    // Reject if there was excess data
                    let read_len = reader.position();
                    let total_len = reader.into_inner().len();
                    // println!("Read len: {read_len}, total len: {total_len}");

                    if let Ok(total_len) = u64::try_from(total_len) {
                        if read_len != total_len {
                            return ValueParseResult::UnrecognizedValue(key);
                        }
                    } else if let Ok(read_len) = usize::try_from(read_len) {
                        if read_len != total_len {
                            return ValueParseResult::UnrecognizedValue(key);
                        }
                    } else {
                        // This should be impossible.
                        // How can usize be both bigger and smaller than u64?
                        return ValueParseResult::UnrecognizedValue(key);
                    }

                    return ValueParseResult::Parsed(Self::LevelChunkMetaDataDictionary(map));
                }
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
        error_on_excessive_length: bool,
    ) -> Result<Vec<u8>, ValueToBytesError> {

        Ok(match self {
            Self::Version(.., version)            => vec![*version],
            Self::LegacyVersion(.., version)      => vec![*version],
            Self::ActorDigestVersion(.., version) => vec![*version],
            Self::MetaDataHash(.., hash)          => hash.to_le_bytes().to_vec(),
            Self::LevelChunkMetaDataDictionary(map) => {

                let (len, len_usize) = if size_of::<usize>() >= size_of::<u32>() {

                    let len = match u32::try_from(map.len()) {
                        Ok(len) => len,
                        Err(_) => {
                            if error_on_excessive_length {
                                return Err(ValueToBytesError::DictionaryLength)
                            } else {
                                u32::MAX
                            }
                        }
                    };

                    // This cast from u32 to usize won't overflow
                    (len, len as usize)
                } else {
                    // This cast from usize to u32 won't overflow
                    (map.len() as u32, map.len())
                };

                let mut writer = Cursor::new(Vec::new());
                writer.write_all(&len.to_le_bytes()).expect("Cursor IO doesn't fail");

                for (hash, nbt) in map.iter().take(len_usize) {
                    writer.write_all(&hash.to_le_bytes()).expect("Cursor IO doesn't fail");

                    // Could only fail on invalid NBT.
                    write_nbt(&mut writer, IoOptions::bedrock_uncompressed(), None, nbt)
                        .map_err(ValueToBytesError::NbtIoError)?;
                }

                writer.into_inner()
            }
            Self::RawEntry { value, .. }          => value.clone(),
            Self::RawValue { value, .. }          => value.clone(),
        })
    }

    pub fn to_bytes(
        &self,
        write_overworld_id: bool,
        write_overworld_name: bool,
        error_on_excessive_length: bool,
    ) -> (Vec<u8>, Result<Vec<u8>, ValueToBytesError>) {

        (
            self.to_key().into_bytes(write_overworld_id, write_overworld_name),
            self.to_value_bytes(error_on_excessive_length),
        )
    }

    pub fn into_bytes(
        self,
        write_overworld_id: bool,
        write_overworld_name: bool,
        error_on_excessive_length: bool,
    ) -> (Vec<u8>, Result<Vec<u8>, ValueToBytesError>) {

        match self {
            Self::RawEntry { key, value } => (key, Ok(value)),
            Self::RawValue { key, value } => {
                let key_bytes = key.into_bytes(write_overworld_id, write_overworld_name);
                (key_bytes, Ok(value))
            }
            // TODO: maybe some other entries could also be more memory efficient, too.
            _ => {
                let value_bytes = self.to_value_bytes(error_on_excessive_length);
                let key_bytes = self
                    .into_key()
                    .into_bytes(write_overworld_id, write_overworld_name);

                (key_bytes, value_bytes)
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

#[derive(Debug, Clone)]
pub enum EntryParseResult {
    Parsed(BedrockLevelDBEntry),
    UnrecognizedKey,
    UnrecognizedValue(BedrockLevelDBKey),
}

impl From<ValueParseResult> for EntryParseResult {
    fn from(value: ValueParseResult) -> Self {
        match value {
            ValueParseResult::Parsed(parsed)         => Self::Parsed(parsed),
            ValueParseResult::UnrecognizedValue(key) => Self::UnrecognizedValue(key),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ValueParseResult {
    Parsed(BedrockLevelDBEntry),
    UnrecognizedValue(BedrockLevelDBKey),
}

#[derive(Error, Debug)]
pub enum ValueToBytesError {
    #[error("error while writing NBT: {0}")]
    NbtIoError(#[from] NbtIoError),
    #[error("there were too many metadata entries in a LevelChunkMetaDataDictionary")]
    DictionaryLength,
}
