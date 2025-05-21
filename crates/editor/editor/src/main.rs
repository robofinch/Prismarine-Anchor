#![allow(unused_imports)]
#![allow(clippy::all)]

// This is used for testing, at least for now. It's very hacky, but so be it.

use std::{mem::size_of, ops::RangeInclusive, path::Path};
use std::io::{Cursor, Read};

use rusty_leveldb::LdbIterator;
use subslice_to_array::SubsliceToArray as _;
#[cfg(not(target_arch = "wasm32"))]
use rusty_leveldb::PosixDiskEnv;

use prismarine_anchor_leveldb_entries::{
    DBEntry, DBKey, EntryParseOptions, EntryToBytesOptions, KeyToBytesOptions,
};
use prismarine_anchor_leveldb_values::{
    aabb_volumes::AabbVolumes,
    DataFidelity,
    dimensioned_chunk_pos::DimensionedChunkPos,
    metadata::LevelChunkMetaDataDictionary,
    palettized_storage::PalettizedStorage,
};
use prismarine_anchor_leveldb_values::{
    legacy_extra_block_data::{NbtPieces, SubchunkExtraBlockData, TerrainExtraBlockData},
    subchunk_blocks::{SubchunkBlocks, SubchunkBlocksV9},
};
use prismarine_anchor_mc_datatypes::version::{NumericVersion, VersionName};
use prismarine_anchor_nbt::{NbtList, NbtTag, snbt::VerifiedSnbt};
use prismarine_anchor_nbt::{
    io::{read_compound, write_compound},
    settings::{
        EnabledEscapeSequences, Endianness, IoOptions, NbtCompression,
        SnbtParseOptions, SnbtWriteOptions,
    },
};
use prismarine_anchor_util::print_debug;

// Unstable
use prismarine_anchor_world::bedrock::BedrockWorldFiles;



fn main() -> anyhow::Result<()> {
    println!("Hello, world!");

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut args = std::env::args().skip(1);
        while let Some(path) = args.next() {

            println!("Opening world: {path}");

            let Ok(mut world) = BedrockWorldFiles::open_world_from_path(
                Box::new(PosixDiskEnv::new()),
                &Path::new(&path),
            ).inspect_err(|err| println!("{err}")) else {
                continue;
            };
            // LMAO, i forgot that super old worlds don't have icons at all
            // assert!(world.world_icon().is_ok());

            fn nbtlist_to_version(version: &NbtList) -> NumericVersion {
                NumericVersion(
                    version.get(0).unwrap_or(0i32) as u32,
                    version.get(1).unwrap_or(0i32) as u32,
                    version.get(2).unwrap_or(0i32) as u32,
                    version.get(3).unwrap_or(0i32) as u32,
                    version.get(4).unwrap_or(0i32) as u32,
                )
            }

            let empty = NbtList::new();

            let version: &NbtList = world.level_dat().nbt.get("lastOpenedWithVersion")
                // .expect("No game version in level.dat");
                .unwrap_or(&empty);

            let version = nbtlist_to_version(version);

            let opts = EntryToBytesOptions::for_version(version);
            let parse_opts = EntryParseOptions {
                value_fidelity: DataFidelity::BitPerfect,
            };

            println!("Last opened with: {version}");
            println!("Level dat: {:?}", world.level_dat());

            let dictionary = world.get(
                DBKey::LevelChunkMetaDataDictionary,
                opts.into(),
                EntryParseOptions { value_fidelity: DataFidelity::BitPerfect },
            );

            let _dictionary = dictionary.and_then(|dict| {
                if let DBEntry::LevelChunkMetaDataDictionary(dict) = dict {
                    Some(dict)
                } else {
                    None
                }
            });

            let mut iter = world.level_db().new_iter().unwrap();
            let mut key = Vec::new();
            let mut value = Vec::new();
            while iter.advance() {
                iter.current(&mut key, &mut value);

                let key = DBKey::parse_key(&key);

                println!("Key: {key:?}");

                match &key {
                    DBKey::LegacyExtraBlockData(pos) => {
                        let pos = pos.clone();

                        let entry = DBEntry::parse_value(key, &value, parse_opts);
                        let DBEntry::LegacyExtraBlockData(_, data) = &entry else {
                            println!("Couldn't parse: {entry:?}");
                            continue;
                        };

                        println!("At pos {pos:?}:");

                        if data.likely_nbt_pieces() {
                            let data = NbtPieces::from(data);

                            println!("{data}");
                        } else {
                            let terrain = world.get(
                                DBKey::LegacyTerrain(pos.clone()),
                                opts.into(),
                                parse_opts,
                            ).is_some();

                            if terrain {
                                let data = TerrainExtraBlockData::from(data);

                                println!("{data:?}");
                            } else {
                                let data = SubchunkExtraBlockData::from(data);

                                println!("{data:?}");
                            }
                        }

                        let block_entities = world.get(
                            DBKey::BlockEntities(pos),
                            opts.into(),
                            parse_opts,
                        );

                        if let Some(DBEntry::BlockEntities(_, block_entities)) = block_entities {
                            println!("Block entities: {block_entities:?}");
                        }

                        let terrain = world.get(
                            DBKey::LegacyTerrain(pos),
                            opts.into(),
                            parse_opts,
                        );

                        let Some(terrain) = terrain else {
                            println!("No associated terrain");
                            continue;
                        };

                        let DBEntry::LegacyTerrain(_, terrain) = terrain else {
                            println!("Associated terrain couldn't be parsed");
                            continue;
                        };

                        let mut unique_ids = Vec::new();
                        for id in terrain.block_ids {
                            if !unique_ids.contains(&id) {
                                unique_ids.push(id);
                            }
                        }

                        let chest_positions = terrain
                            .block_ids
                            .iter()
                            .enumerate()
                            .filter_map(|(idx, id)| {
                                // numerical ID of chest
                                if *id == 54 {
                                    Some((
                                        idx,
                                        (
                                            (idx >> 11) & 0b1111,
                                            idx & 0b111_1111,
                                            (idx >> 7) & 0b1111,
                                        ),
                                    ))
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>();

                        let nonzero_data = terrain
                            .block_data
                            .iter()
                            .enumerate()
                            .filter_map(|(idx, data)| {
                                if data != 0 {
                                    Some((
                                        idx,
                                        (
                                            (idx >> 11) & 0b1111,
                                            idx & 0b111_1111,
                                            (idx >> 7) & 0b1111,
                                        ),
                                        data,
                                        terrain.block_ids[idx],
                                    ))
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>();

                        println!("Associated terrain:");
                        println!("Unique block ids: {unique_ids:?}");
                        println!("Chest indices and positions: {chest_positions:?}");
                        println!("(index, (x, y, z), block data, block id): {nonzero_data:?}");

                        // println!("Raw value: {value:?}");
                        // print_debug(&value);
                    }
                    DBKey::LegacyTerrain(_) => {}
                    DBKey::RawKey(key) => {
                        println!("Raw Key! : {key:?}");
                        print_debug(key);
                    }
                    _ => {
                        // let entry = DBEntry::parse_value(key.clone(), &value, parse_opts);
                        // println!("{entry:?}");
                    }
                }

                continue;

                #[allow(unreachable_code)]
                match key {
                    // DBKey::ActorDigest(_) => {
                    //     let entry = DBEntry::parse_value(key, &value);
                    //     // print_raw("ActorDigest", &entry);
                    //     println!("ActorDigest: {entry:?}");
                    //     continue;
                    // }
                    // _ => continue,
                    DBKey::Data3D(_) => continue,
                    DBKey::SubchunkBlocks(_, _) => continue,
                    _ => {},
                }

                let entry = DBEntry::parse_value(key, &value, parse_opts);
                // if let DBEntry::RawValue { key, value } = entry {
                //     // panic!("Invalid value for {key:?}: {:?}", value.into_iter().take(20).collect::<Vec<_>>())
                //     panic!("Invalid value for {key:?}: {:?}", value)
                // }

                // let (round_trip_key, round_trip_value) = entry.to_bytes(opts).unwrap();

                // if round_trip_key != key {
                //     println!("unequal keys for key {key:?}, entry {entry:?}");
                // }

                // if round_trip_value != value {
                //     println!("unequal values for value {value:?}, entry {entry:?}");
                // }

                let key = entry.to_key();

                fn print_raw(name: &'static str, entry: &DBEntry) {
                    if let DBEntry::RawValue { .. } = entry {
                        println!("Could not parse {name}: {entry:?}");
                    }
                }

                macro_rules! check_parsed {
                    ($name:expr, $entry: expr, $variant:ident) => {
                        if let DBEntry::$variant{..} = $entry {
                            println!("{} parsed", $name);
                        } else {
                            println!("{} couldn't be parsed", $name);
                        }
                    };
                }

                match key {
                    // I don't know the format of these.
                    DBKey::ConversionData{..} => {
                        // print_raw("ConversionData", &entry);
                        println!("ConversionData: {entry:?}");
                    }
                    DBKey::CavesAndCliffsBlending{..} => {
                        // print_raw("CavesAndCliffsBlending", &entry);
                        println!("CavesAndCliffsBlending: {entry:?}");
                    }
                    DBKey::BlendingBiomeHeight{..} => {
                        // print_raw("BlendingBiomeHeight", &entry);
                        println!("BlendingBiomeHeight: {entry:?}");
                    }

                    DBKey::RawKey(key) => {
                        println!("Raw Key! : {key:?}");
                        print_debug(&key);
                    }


                    DBKey::Version{..} => {
                        print_raw("Version", &entry);
                    }
                    DBKey::LegacyVersion{..} => {
                        print_raw("LegacyVersion", &entry);
                    }
                    DBKey::ActorDigestVersion{..} => {
                        print_raw("ActorDigestVersion", &entry);
                    }
                    DBKey::Data3D{..} => {
                        print_raw("Data3D", &entry);
                    }
                    DBKey::Data2D{..} => {
                        print_raw("Data2D", &entry);
                        // println!("Data2D: {entry:?}");
                    }
                    DBKey::LegacyData2D{..} => {
                        print_raw("LegacyData2D", &entry);
                    }
                    DBKey::SubchunkBlocks{..} => {
                        print_raw("SubchunkBlocks", &entry);
                    }
                    DBKey::LegacyTerrain(_) => {
                        print_raw("LegacyTerrain", &entry);
                        check_parsed!("LegacyTerrain", &entry, LegacyTerrain);
                    }
                    DBKey::LegacyExtraBlockData(_) => {
                        print_raw("LegacyExtraBlockData", &entry);
                        check_parsed!("LegacyExtraBlockData", &entry, LegacyExtraBlockData);

                        // if let DBEntry::LegacyExtraBlockData(_, data) = &entry {
                        //     for extra_block in data.0.iter().cloned() {
                        //         let ExtraBlock {
                        //             location_index,
                        //             padding,
                        //             block_id,
                        //             block_data,
                        //         } = extra_block;

                        //         println!("    location_index: {location_index:016b}");
                        //         println!("    padding:        {padding:016b}");
                        //         println!("    block_id:   {block_id}");
                        //         println!("    block_data: {block_data}");
                        //     }
                        // }

                        // println!("LegacyExtraBlockData: {entry:#?}");
                    }
                    DBKey::BlockEntities{..} => {
                        print_raw("BlockEntities", &entry);
                    }
                    DBKey::Entities{..} => {
                        print_raw("Entities", &entry);
                        // check_parsed!("Entities", &entry, Entities);
                    }
                    DBKey::PendingTicks{..} => {
                        print_raw("PendingTicks", &entry);
                    }
                    DBKey::RandomTicks{..} => {
                        print_raw("RandomTicks", &entry);
                    }
                    DBKey::BorderBlocks(_) => {
                        print_raw("BorderBlocks", &entry);
                        // println!("BorderBlocks: {entry:?}");
                    }
                    DBKey::HardcodedSpawners(_) => {
                        print_raw("HardcodedSpawners", &entry);
                        // println!("HardcodedSpawners: {entry:?}");
                    }
                    DBKey::AabbVolumes(_) => {
                        print_raw("AabbVolumes", &entry);
                        // check_parsed!("AabbVolumes", &entry, AabbVolumes);
                    }
                    DBKey::Checksums(_) => {
                        print_raw("Checksums", &entry);
                        // println!("Checksums: {entry:?}");
                    }
                    DBKey::MetaDataHash{..} => {
                        print_raw("MetaDataHash", &entry);
                    }
                    DBKey::FinalizedState{..} => {
                        print_raw("FinalizedState", &entry);
                    }
                    DBKey::GenerationSeed(_) => {
                        print_raw("GenerationSeed", &entry);
                        check_parsed!("GenerationSeed", &entry, GenerationSeed);
                    }
                    DBKey::BiomeState{..} => {
                        print_raw("BiomeState", &entry);
                        // check_parsed!("BiomeState", &entry, BiomeState);
                    }
                    DBKey::BlendingData{..} => {
                        print_raw("BlendingData", &entry);
                    }
                    DBKey::ActorDigest(_) => {
                        print_raw("ActorDigest", &entry);
                        // println!("ActorDigest: {entry:?}");
                    }
                    DBKey::Actor(_) => {
                        print_raw("Actor", &entry);
                        // println!("Actor info: {entry:?}");
                    }
                    DBKey::LevelChunkMetaDataDictionary => {
                        print_raw("LevelChunkMetaDataDictionary", &entry);
                    }
                    DBKey::AutonomousEntities => {
                        // print_raw("AutonomousEntities", &entry);
                        println!("AutonomousEntities: {entry:?}")
                    }
                    DBKey::LocalPlayer => {
                        // print_raw("LocalPlayer", &entry);
                        check_parsed!("LocalPlayer", &entry, LocalPlayer);
                    }
                    DBKey::Player{..} => {
                        // print_raw("Player", &entry);
                        check_parsed!("Player", &entry, Player);
                    }
                    DBKey::LegacyPlayer{..} => {
                        // print_raw("LegacyPlayer", &entry);
                        check_parsed!("LegacyPlayer", &entry, LegacyPlayer);
                    }
                    DBKey::PlayerServer{..} => {
                        // print_raw("PlayerServer", &entry);
                        check_parsed!("PlayerServer", &entry, PlayerServer);
                    }
                    DBKey::VillageDwellers{..}=> {
                        print_raw("VillageDwellers", &entry);
                        // println!("VillageDwellers: {entry:?}");
                    }
                    DBKey::VillageInfo{..} => {
                        print_raw("VillageInfo", &entry);
                        // println!("VillageInfo: {entry:?}");
                    }
                    DBKey::VillagePOI{..} => {
                        print_raw("VillagePOI", &entry);
                        // println!("VillagePOI: {entry:?}");
                    }
                    DBKey::VillagePlayers{..} => {
                        print_raw("VillagePlayers", &entry);
                        // println!("VillagePlayers: {entry:?}");
                    }
                    DBKey::VillageRaid{..} => {
                        print_raw("VillageRaid", &entry);
                        // println!("VillageRaid: {entry:?}");
                    }
                    DBKey::Map{..} => {
                        print_raw("Map", &entry);
                        // println!("Map: {entry:?}");
                        // check_parsed!("Map", &entry, Map);
                    }
                    DBKey::Portals => {
                        print_raw("Portals", &entry);
                        // println!("Portals: {entry:?}");
                    }
                    DBKey::StructureTemplate(..) => {
                        print_raw("StructureTemplate", &entry);
                        // println!("StructureTemplate: {entry:?}");
                        // check_parsed!("StructureTemplate", &entry, StructureTemplate);
                    }
                    DBKey::TickingArea(..) => {
                        // print_raw("TickingArea", &entry);
                        println!("TickingArea: {entry:?}");
                    }
                    DBKey::Scoreboard => {
                        print_raw("Scoreboard", &entry);
                        // println!("Scoreboard: {entry:?}");
                    }
                    DBKey::WanderingTraderScheduler => {
                        // print_raw("WanderingTraderScheduler", &entry);
                        println!("WanderingTraderScheduler: {entry:?}");
                    }
                    DBKey::BiomeData => {
                        print_raw("BiomeData", &entry);
                        // println!("BiomeData: {entry:?}");
                    }
                    DBKey::MobEvents => {
                        print_raw("MobEvents", &entry);
                        // println!("MobEvents: {entry:?}");
                    }
                    DBKey::Overworld => {
                        print_raw("Overworld", &entry);
                        // println!("Overworld: {entry:?}");
                    }
                    DBKey::Nether => {
                        print_raw("Nether", &entry);
                        // println!("Nether: {entry:?}");
                    }
                    DBKey::TheEnd => {
                        print_raw("TheEnd", &entry);
                        // println!("TheEnd: {entry:?}");
                    }
                    DBKey::PositionTrackingDB{..} => {
                        // print_raw("PositionTrackingDB", &entry);
                        println!("PositionTrackingDB: {entry:?}");
                    }
                    DBKey::PositionTrackingLastId => {
                        // print_raw("PositionTrackingLastId", &entry);
                        println!("PositionTrackingLastId: {entry:?}");
                    }
                    DBKey::BiomeIdsTable => {
                        print_raw("BiomeIdsTable", &entry);
                        // println!("BiomeIdsTable: {entry:?}");
                    }
                    DBKey::FlatWorldLayers => {
                        print_raw("FlatWorldLayers", &entry);
                        // println!("FlatWorldLayers: {entry:?}");
                    }
                    DBKey::LevelSpawnWasFixed => {
                        // print_raw("LevelSpawnWasFixed", &entry);
                        println!("LevelSpawnWasFixed: {entry:?}");
                    }
                    DBKey::MVillages => {
                        // print_raw("MVillages", &entry);
                        println!("MVillages: {entry:?}");
                    }
                    DBKey::Villages => {
                        // print_raw("Villages", &entry);
                        // println!("Villages: {entry:?}");
                        check_parsed!("Villages", &entry, Villages);
                    }
                    DBKey::Dimension0 => {
                        print_raw("Dimension0", &entry);
                        // println!("Dimension0: {entry:?}");
                    }
                    DBKey::Dimension1 => {
                        print_raw("Dimension1", &entry);
                        // println!("Dimension1: {entry:?}");
                    }
                    DBKey::Dimension2 => {
                        print_raw("Dimension2", &entry);
                        // println!("Dimension2: {entry:?}");
                    }

                }

                // fn chunk_pos_to_version(
                //     chunk_pos: DimensionedChunkPos,
                //     world: &mut BedrockWorldFiles,
                //     opts: KeyToBytesOptions,
                //     dict: &LevelChunkMetaDataDictionary,
                // ) -> Option<(Option<String>, Option<String>)> {
                //     let Some(metadata_hash) = world.get(DBKey::MetaDataHash(chunk_pos), opts) else {
                //         println!("Chunks with no hash?? pos: {chunk_pos:?}");
                //         return None;
                //     };

                //     let DBEntry::MetaDataHash(_, metadata_hash) = metadata_hash else {
                //         return None;
                //     };

                //     let Some(metadata) = dict.get(metadata_hash) else {
                //         println!("Metadata not in dictionary?? pos: {chunk_pos:?}, hash: {metadata_hash}");
                //         return None;
                //     };

                //     Some((
                //         metadata.original_base_game_version.clone(),
                //         metadata.last_saved_base_game_version.clone(),
                //     ))
                // }


                // if let DBEntry::Version(chunk_pos, version) = entry {
                //     print!("Chunk version: {}", u8::from(version));
                //     if let Some(dict) = &dictionary {
                //         println!(", (Original, last opened): {:?}", chunk_pos_to_version(chunk_pos, &mut world, opts.into(), dict));
                //     } else {
                //         println!("")
                //     }
                // }

                // if let DBEntry::LegacyVersion(chunk_pos, version) = entry {
                //     print!("Legacy chunk version: {}", u8::from(version));
                //     if let Some(dict) = &dictionary {
                //         println!(", (Original, last opened): {:?}", chunk_pos_to_version(chunk_pos, &mut world, opts.into(), dict));
                //     } else {
                //         println!("")
                //     }
                // }

                // if let BedrockLevelDBKey::LevelChunkMetaDataDictionary = &parsed_key {

                //     println!("{:?}", entry);

                //     let value = world.level_db()
                //         .get(&parsed_key.clone().to_bytes(key_opts));

                //     if let Some(value) = value {
                //         let mut value = Cursor::new(value);
                //         let mut buf = [0; 4];
                //         value.read_exact(&mut buf).unwrap();

                //         loop {
                //             let mut buf: [u8; 8] = [0; 8];
                //             let Ok(()) = value.read_exact(&mut buf) else {
                //                 break;
                //             };

                //             let Ok((nbt, ..)) = read_compound(&mut value, IoOptions::bedrock_uncompressed()) else {
                //                 break;
                //             };

                //             let mut writer = Cursor::new(Vec::new());
                //             write_compound(
                //                 &mut writer,
                //                 IoOptions {
                //                     endianness: Endianness::NetworkLittleEndian,
                //                     ..IoOptions::bedrock_uncompressed()
                //                 },
                //                 None,
                //                 &nbt,
                //             ).unwrap();

                //             assert_eq!(buf, xxhash_rust::xxh64::xxh64(&writer.into_inner(), 0).to_le_bytes());
                //         }
                //     }
                // }



            }

        }
    }

    Ok(())
}
