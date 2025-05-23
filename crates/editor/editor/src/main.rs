#![allow(unused_imports)]
#![allow(clippy::all)]

// This is used for testing, at least for now. It's very hacky, but so be it.

use std::{array, thread};
use std::{io::Cursor, path::Path, sync::Arc, thread::JoinHandle};

use crossbeam::channel;
use object_pool::{Pool, ReusableOwned};
use rusty_leveldb::LdbIterator;

#[cfg(not(target_arch = "wasm32"))]
use rusty_leveldb::PosixDiskEnv;

use prismarine_anchor_leveldb_entries::{
    DBEntry, DBKey, EntryParseOptions, EntryToBytesOptions,
};
use prismarine_anchor_leveldb_values::{
    aabb_volumes::AabbVolumes,
    actor::Actor,
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
use prismarine_anchor_nbt::{io::read_compound, NbtList, settings::IoOptions};
use prismarine_anchor_util::print_debug;

// Unstable
use prismarine_anchor_world::bedrock::BedrockWorldFiles;


fn main() {
    println!("Hello, world!");

    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();

        // In total this will use 6 threads, exluding the main thread.
        const NUM_CONCURRENT_WORLDS: usize = 2;
        let (world_sender, world_receiver) = channel::bounded(NUM_CONCURRENT_WORLDS);

        let threads: [JoinHandle<_>; NUM_CONCURRENT_WORLDS] = array::from_fn(|_| {
            let world_receiver = world_receiver.clone();

            thread::spawn(move || {
                while let Ok((world_num, world_path)) = world_receiver.recv() {
                    parse_world(world_num, world_path);
                }
            })
        });

        for (world_num, world_path) in std::env::args().skip(1).enumerate() {
            world_sender.send((world_num, world_path)).expect("receiver is not dropped");
        }

        drop(world_sender);

        for thread in threads {
            thread.join().unwrap();
        }
    }
}

fn parse_world(world_num: usize, world_path: String) {
    println!("Opening world {world_num} at {world_path}");

    let Ok(mut world) = BedrockWorldFiles::open_world_from_path(
        Box::new(PosixDiskEnv::new()),
        &Path::new(&world_path),
    ).inspect_err(|err| println!("In world {world_num}: {err}")) else {
        return;
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
        .inspect(|version: &&NbtList| {
            let version = nbtlist_to_version(version);
            println!("In world {world_num}: Last opened with: {version}");
        })
        .inspect_err(|_| println!("In world {world_num}: No last opened version"))
        // .expect("No game version in level.dat");
        .unwrap_or(&empty);

    let version = nbtlist_to_version(version);

    let opts = EntryToBytesOptions::for_version(version);
    #[allow(unused_variables)]
    let parse_opts = EntryParseOptions {
        value_fidelity: DataFidelity::BitPerfect,
    };

    // println!("Level dat: {:?}", world.level_dat());

    // let dictionary = world.get(
    //     DBKey::LevelChunkMetaDataDictionary,
    //     opts.into(),
    //     EntryParseOptions { value_fidelity: DataFidelity::BitPerfect },
    // );

    // let _dictionary = dictionary.and_then(|dict| {
    //     if let DBEntry::LevelChunkMetaDataDictionary(dict) = dict {
    //         Some(dict)
    //     } else {
    //         None
    //     }
    // });

    let entry_parsing_threads_per_world = 2;

    let buffers = Arc::new(Pool::new(
        entry_parsing_threads_per_world,
        || (Vec::new(), Vec::new()),
    ));
    let (task_sender, task_receiver)   = channel::bounded(entry_parsing_threads_per_world);
    let (entry_sender, entry_reciever) = channel::bounded(entry_parsing_threads_per_world);

    for _ in 0..entry_parsing_threads_per_world {
        // Give type hint for the above channel creation
        let task_receiver: channel::Receiver<
            ReusableOwned<(Vec<u8>, Vec<u8>)>,
        > = task_receiver.clone();

        let entry_sender  = entry_sender.clone();

        thread::spawn(move || {
            while let Ok(entry_buffers) = task_receiver.recv() {
                let entry = DBEntry::parse_entry(
                    &entry_buffers.0,
                    &entry_buffers.1,
                    parse_opts,
                );
                if entry_sender.send(entry).is_err() {
                    break;
                }
            }
        });
    }

    let mut iter = world.level_db().new_iter().unwrap();

    'outer: loop {
        while let Some(mut entry_buffers) = buffers.try_pull_owned() {
            if !iter.advance() {
                break 'outer;
            }

            let (ref mut key, ref mut value) = *entry_buffers;
            iter.current(key, value);

            // buffers.try_pull_* should limit use to num_threads
            task_sender
                .try_send(entry_buffers)
                .expect("Receiver should not be disconnected or full");
        }

        while let Ok(entry) = entry_reciever.try_recv() {
            react_to_entry(world_num, entry, opts);
        }
    }

    while let Ok(entry) = entry_reciever.try_recv() {
        react_to_entry(world_num, entry, opts);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn react_to_entry(world_num: usize, entry: DBEntry, _opts: EntryToBytesOptions) {
    // let EntryBytes {
    //     key: round_trip_key,
    //     value: round_trip_value,
    //  } = entry.to_bytes(opts).unwrap();

    // if round_trip_key != key {
    //     println!("unequal keys, for type {:?}", entry.to_key());
    // }

    // if round_trip_value != value {
    //     println!("unequal values, for type {:?}", entry.to_key());
    // }

    let key = entry.to_key();

    fn print_raw(world_num: usize, name: &'static str, entry: &DBEntry) {
        if let DBEntry::RawValue { .. } = entry {
            println!("In world {world_num}: Could not parse {name}: {entry:?}");
        }
    }

    macro_rules! check_parsed {
        ($name:expr, $entry: expr, $variant:ident) => {
            if let DBEntry::$variant{..} = $entry {
                println!("In world {world_num}: {} parsed", $name);
            } else {
                println!("In world {world_num}: {} couldn't be parsed", $name);
            }
        };
    }

    match key {
        // I don't know the format of these.
        DBKey::ConversionData{..} => {
            // print_raw(world_num, "ConversionData", &entry);
            println!("In world {world_num}: ConversionData: {entry:?}");
        }
        DBKey::BlendingBiomeHeight{..} => {
            // print_raw(world_num, "BlendingBiomeHeight", &entry);
            println!("In world {world_num}: BlendingBiomeHeight: {entry:?}");
        }

        DBKey::RawKey(key) => {
            println!("In world {world_num}: Raw Key! : {key:?}");
            print_debug(&key);
        }


        DBKey::Version{..} => {
            print_raw(world_num, "Version", &entry);
        }
        DBKey::LegacyVersion{..} => {
            print_raw(world_num, "LegacyVersion", &entry);
        }
        DBKey::ActorDigestVersion{..} => {
            print_raw(world_num, "ActorDigestVersion", &entry);
        }
        DBKey::Data3D{..} => {
            print_raw(world_num, "Data3D", &entry);
        }
        DBKey::Data2D{..} => {
            print_raw(world_num, "Data2D", &entry);
            // println!("In world {world_num}: Data2D: {entry:?}");
        }
        DBKey::LegacyData2D{..} => {
            print_raw(world_num, "LegacyData2D", &entry);
        }
        DBKey::SubchunkBlocks{..} => {
            print_raw(world_num, "SubchunkBlocks", &entry);
        }
        DBKey::LegacyTerrain(_) => {
            print_raw(world_num, "LegacyTerrain", &entry);
            // check_parsed!("LegacyTerrain", &entry, LegacyTerrain);
        }
        DBKey::LegacyExtraBlockData(_) => {
            print_raw(world_num, "LegacyExtraBlockData", &entry);
            // check_parsed!("LegacyExtraBlockData", &entry, LegacyExtraBlockData);
            // println!("In world {world_num}: LegacyExtraBlockData: {entry:#?}");
        }
        DBKey::BlockEntities{..} => {
            print_raw(world_num, "BlockEntities", &entry);
        }
        DBKey::Entities{..} => {
            print_raw(world_num, "Entities", &entry);
            // check_parsed!("Entities", &entry, Entities);
        }
        DBKey::PendingTicks{..} => {
            print_raw(world_num, "PendingTicks", &entry);
        }
        DBKey::RandomTicks{..} => {
            print_raw(world_num, "RandomTicks", &entry);
        }
        DBKey::BorderBlocks(_) => {
            print_raw(world_num, "BorderBlocks", &entry);
            // println!("In world {world_num}: BorderBlocks: {entry:?}");
        }
        DBKey::HardcodedSpawners(_) => {
            print_raw(world_num, "HardcodedSpawners", &entry);
            // println!("In world {world_num}: HardcodedSpawners: {entry:?}");
        }
        DBKey::AabbVolumes(_) => {
            print_raw(world_num, "AabbVolumes", &entry);
            // check_parsed!("AabbVolumes", &entry, AabbVolumes);
        }
        DBKey::Checksums(_) => {
            print_raw(world_num, "Checksums", &entry);
            // println!("In world {world_num}: Checksums: {entry:?}");
        }
        DBKey::MetaDataHash{..} => {
            print_raw(world_num, "MetaDataHash", &entry);
        }
        DBKey::FinalizedState{..} => {
            print_raw(world_num, "FinalizedState", &entry);
        }
        DBKey::GenerationSeed(_) => {
            print_raw(world_num, "GenerationSeed", &entry);
            check_parsed!("GenerationSeed", &entry, GenerationSeed);
        }
        DBKey::BiomeState{..} => {
            print_raw(world_num, "BiomeState", &entry);
            // check_parsed!("BiomeState", &entry, BiomeState);
        }
        DBKey::CavesAndCliffsBlending{..} => {
            // I've seen `[0]` and `[1]` in some of my worlds, but there could be other stuff.
            print_raw(world_num, "CavesAndCliffsBlending", &entry);
        }
        DBKey::BlendingData{..} => {
            print_raw(world_num, "BlendingData", &entry);
        }
        DBKey::ActorDigest(_) => {
            print_raw(world_num, "ActorDigest", &entry);
            // println!("In world {world_num}: ActorDigest: {entry:?}");
        }
        DBKey::Actor(_) => {
            print_raw(world_num, "Actor", &entry);
            // println!("In world {world_num}: Actor info: {entry:?}");

            // if let DBEntry::Actor(pos, Actor::Multiple(actors)) = &entry {
            //     println!("Multiple actors with id {pos:?}: {actors:?}");
            // }
        }
        DBKey::LevelChunkMetaDataDictionary => {
            print_raw(world_num, "LevelChunkMetaDataDictionary", &entry);
        }
        DBKey::AutonomousEntities => {
            print_raw(world_num, "AutonomousEntities", &entry);
            // println!("In world {world_num}: AutonomousEntities: {entry:?}")
        }
        DBKey::LocalPlayer => {
            // print_raw(world_num, "LocalPlayer", &entry);
            check_parsed!("LocalPlayer", &entry, LocalPlayer);
        }
        DBKey::Player{..} => {
            // print_raw(world_num, "Player", &entry);
            check_parsed!("Player", &entry, Player);
        }
        DBKey::LegacyPlayer{..} => {
            // print_raw(world_num, "LegacyPlayer", &entry);
            check_parsed!("LegacyPlayer", &entry, LegacyPlayer);
        }
        DBKey::PlayerServer{..} => {
            // print_raw(world_num, "PlayerServer", &entry);
            check_parsed!("PlayerServer", &entry, PlayerServer);
        }
        DBKey::VillageDwellers{..}=> {
            print_raw(world_num, "VillageDwellers", &entry);
            // println!("In world {world_num}: VillageDwellers: {entry:?}");
        }
        DBKey::VillageInfo{..} => {
            print_raw(world_num, "VillageInfo", &entry);
            // println!("In world {world_num}: VillageInfo: {entry:?}");
        }
        DBKey::VillagePOI{..} => {
            print_raw(world_num, "VillagePOI", &entry);
            // println!("In world {world_num}: VillagePOI: {entry:?}");
        }
        DBKey::VillagePlayers{..} => {
            print_raw(world_num, "VillagePlayers", &entry);
            // println!("In world {world_num}: VillagePlayers: {entry:?}");
        }
        DBKey::VillageRaid{..} => {
            print_raw(world_num, "VillageRaid", &entry);
            // println!("In world {world_num}: VillageRaid: {entry:?}");
        }
        DBKey::Map{..} => {
            print_raw(world_num, "Map", &entry);
            // println!("In world {world_num}: Map: {entry:?}");
            // check_parsed!("Map", &entry, Map);
        }
        DBKey::Portals => {
            print_raw(world_num, "Portals", &entry);
            // println!("In world {world_num}: Portals: {entry:?}");
        }
        DBKey::StructureTemplate(..) => {
            print_raw(world_num, "StructureTemplate", &entry);
            // println!("In world {world_num}: StructureTemplate: {entry:?}");
            // check_parsed!("StructureTemplate", &entry, StructureTemplate);
        }
        DBKey::TickingArea(..) => {
            print_raw(world_num, "TickingArea", &entry);
            // println!("In world {world_num}: TickingArea: {entry:?}");
        }
        DBKey::Scoreboard => {
            print_raw(world_num, "Scoreboard", &entry);
            // println!("In world {world_num}: Scoreboard: {entry:?}");
        }
        DBKey::WanderingTraderScheduler => {
            // print_raw(world_num, "WanderingTraderScheduler", &entry);
            println!("In world {world_num}: WanderingTraderScheduler: {entry:?}");
        }
        DBKey::BiomeData => {
            print_raw(world_num, "BiomeData", &entry);
            // println!("In world {world_num}: BiomeData: {entry:?}");
        }
        DBKey::MobEvents => {
            print_raw(world_num, "MobEvents", &entry);
            // println!("In world {world_num}: MobEvents: {entry:?}");
        }
        DBKey::Overworld => {
            print_raw(world_num, "Overworld", &entry);
            // println!("In world {world_num}: Overworld: {entry:?}");
        }
        DBKey::Nether => {
            print_raw(world_num, "Nether", &entry);
            // println!("In world {world_num}: Nether: {entry:?}");
        }
        DBKey::TheEnd => {
            print_raw(world_num, "TheEnd", &entry);
            // println!("In world {world_num}: TheEnd: {entry:?}");
        }
        DBKey::PositionTrackingDB{..} => {
            // print_raw(world_num, "PositionTrackingDB", &entry);
            println!("In world {world_num}: PositionTrackingDB: {entry:?}");
        }
        DBKey::PositionTrackingLastId => {
            // print_raw(world_num, "PositionTrackingLastId", &entry);
            println!("In world {world_num}: PositionTrackingLastId: {entry:?}");
        }
        DBKey::BiomeIdsTable => {
            print_raw(world_num, "BiomeIdsTable", &entry);
            // println!("In world {world_num}: BiomeIdsTable: {entry:?}");
        }
        DBKey::FlatWorldLayers => {
            print_raw(world_num, "FlatWorldLayers", &entry);
            // println!("In world {world_num}: FlatWorldLayers: {entry:?}");
        }
        DBKey::LevelSpawnWasFixed => {
            // print_raw(world_num, "LevelSpawnWasFixed", &entry);
            println!("In world {world_num}: LevelSpawnWasFixed: {entry:?}");
        }
        DBKey::MVillages => {
            // print_raw(world_num, "MVillages", &entry);
            println!("In world {world_num}: MVillages: {entry:?}");
        }
        DBKey::Villages => {
            // print_raw(world_num, "Villages", &entry);
            // println!("In world {world_num}: Villages: {entry:?}");
            check_parsed!("Villages", &entry, Villages);
        }
        DBKey::Dimension0 => {
            print_raw(world_num, "Dimension0", &entry);
            // println!("In world {world_num}: Dimension0: {entry:?}");
        }
        DBKey::Dimension1 => {
            print_raw(world_num, "Dimension1", &entry);
            // println!("In world {world_num}: Dimension1: {entry:?}");
        }
        DBKey::Dimension2 => {
            print_raw(world_num, "Dimension2", &entry);
            // println!("In world {world_num}: Dimension2: {entry:?}");
        }
    }
}
