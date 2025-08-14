use std::{array, thread};
use std::{path::Path, sync::Arc, thread::JoinHandle};

use crossbeam::channel;
use object_pool::{Pool, ReusableOwned};
use rusty_leveldb::{LdbIterator as _, PosixDiskEnv};

use prismarine_anchor_leveldb_entries::{
    DBEntry, DBKey, DataFidelity,
    EntryParseOptions, EntryToBytesOptions,
};
use prismarine_anchor_mc_datatypes::NumericVersion;
use prismarine_anchor_nbt::NbtList;
use prismarine_anchor_util::print_debug;

// Unstable
use prismarine_anchor_world::bedrock::BedrockWorldFiles;


// In total, these settings lead to using 7 threads (including the main thread).
const WORLD_THREADS: usize = 2;
const ENTRY_PARSING_THREADS_PER_WORLD: usize = 2;


/// Pass a list of directory paths to the program.
/// They will be interpreted as the world folders of Minecraft: Bedrock Edition worlds.
fn main() {
    env_logger::init();

    let (world_sender, world_receiver) = channel::bounded(WORLD_THREADS);

    let threads: [JoinHandle<_>; WORLD_THREADS] = array::from_fn(|_| {
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

fn parse_world(world_num: usize, world_path: String) {
    println!("Opening world {world_num} at {world_path}");

    let Ok(mut world) = BedrockWorldFiles::open_world_from_path(
        Box::new(PosixDiskEnv::new()),
        &Path::new(&world_path),
    ).inspect_err(|err| println!("Error in world {world_num}: {err}")) else {
        return;
    };

    let version = world
        .level_dat()
        .nbt
        .get("lastOpenedWithVersion")
        .map(|version: &NbtList| {
            let version = nbtlist_to_version(version);
            println!("In world {world_num}: Last opened with: {version}");
            version
        })
        .inspect_err(|_| println!("In world {world_num}: No last opened version"))
        .unwrap_or(NumericVersion::from((0, 0, 0)));

    let to_bytes_opts = EntryToBytesOptions {
        value_fidelity: DataFidelity::BitPerfect,
        ..EntryToBytesOptions::for_version(version)
    };
    let parse_opts = EntryParseOptions {
        value_fidelity: DataFidelity::BitPerfect,
    };

    let buffers = Arc::new(Pool::new(
        ENTRY_PARSING_THREADS_PER_WORLD,
        || (Vec::new(), Vec::new()),
    ));
    let (task_sender, task_receiver)   = channel::bounded(ENTRY_PARSING_THREADS_PER_WORLD);
    let (entry_sender, entry_reciever) = channel::bounded(ENTRY_PARSING_THREADS_PER_WORLD);

    for _ in 0..ENTRY_PARSING_THREADS_PER_WORLD {
        // Give type hint for the above channel creation
        let task_receiver: channel::Receiver<
            ReusableOwned<(Vec<u8>, Vec<u8>)>,
        > = task_receiver.clone();

        let entry_sender  = entry_sender.clone();

        thread::spawn(move || {
            while let Ok(entry_buffers) = task_receiver.recv() {
                let (key, value) = &*entry_buffers;

                let entry = DBEntry::parse_entry(
                    key,
                    value,
                    parse_opts,
                );
                let entry_bytes = entry
                    .to_bytes(to_bytes_opts)
                    .expect("converting a DBEntry to bytes failed");

                if key != &entry_bytes.key {
                    println!(
                        "Key roundtrip failed in world {} (key: {:?})",
                        world_num,
                        entry.to_key(),
                    );
                }
                if value != &entry_bytes.value {
                    println!(
                        "Value roundtrip failed in world {} (entry: {:?})",
                        world_num,
                        entry,
                    );

                    // println!("Side-by-side values:");
                    // for (roundtrip, old) in std::iter::zip(value, &entry_bytes.value) {
                    //     println!("{roundtrip:3} | {old:3}");
                    // }
                    // if value.len() > entry_bytes.value.len() {
                    //     println!("Roundtripped value is longer");
                    // } else if value.len() < entry_bytes.value.len() {
                    //     println!("Roundtripped value is shorter");
                    // }
                }

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
            react_to_entry(world_num, entry);
        }
    }

    while let Ok(entry) = entry_reciever.try_recv() {
        react_to_entry(world_num, entry);
    }
}

fn nbtlist_to_version(version: &NbtList) -> NumericVersion {
    NumericVersion(
        version.get(0).unwrap_or(0i32) as u32,
        version.get(1).unwrap_or(0i32) as u32,
        version.get(2).unwrap_or(0i32) as u32,
        version.get(3).unwrap_or(0i32) as u32,
        version.get(4).unwrap_or(0i32) as u32,
    )
}

fn react_to_entry(world_num: usize, entry: DBEntry) {
    let key = entry.to_key();

    fn print_if_raw(world_num: usize, name: &'static str, entry: &DBEntry) {
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
        DBKey::Version{..} => {
            print_if_raw(world_num, "Version", &entry);
        }
        DBKey::LegacyVersion{..} => {
            print_if_raw(world_num, "LegacyVersion", &entry);
        }
        DBKey::ActorDigestVersion{..} => {
            print_if_raw(world_num, "ActorDigestVersion", &entry);
        }
        DBKey::Data3D{..} => {
            print_if_raw(world_num, "Data3D", &entry);
        }
        DBKey::Data2D{..} => {
            print_if_raw(world_num, "Data2D", &entry);
            // println!("In world {world_num}: Data2D: {entry:?}");
        }
        DBKey::LegacyData2D{..} => {
            print_if_raw(world_num, "LegacyData2D", &entry);
        }
        DBKey::SubchunkBlocks{..} => {
            print_if_raw(world_num, "SubchunkBlocks", &entry);
        }
        DBKey::LegacyTerrain(_) => {
            print_if_raw(world_num, "LegacyTerrain", &entry);
            // check_parsed!("LegacyTerrain", &entry, LegacyTerrain);
        }
        DBKey::LegacyExtraBlockData(_) => {
            print_if_raw(world_num, "LegacyExtraBlockData", &entry);
            // check_parsed!("LegacyExtraBlockData", &entry, LegacyExtraBlockData);
            // println!("In world {world_num}: LegacyExtraBlockData: {entry:#?}");
        }
        DBKey::BlockEntities{..} => {
            print_if_raw(world_num, "BlockEntities", &entry);
        }
        DBKey::Entities{..} => {
            print_if_raw(world_num, "Entities", &entry);
            // check_parsed!("Entities", &entry, Entities);
        }
        DBKey::PendingTicks{..} => {
            print_if_raw(world_num, "PendingTicks", &entry);
        }
        DBKey::RandomTicks{..} => {
            print_if_raw(world_num, "RandomTicks", &entry);
        }
        DBKey::BorderBlocks(_) => {
            print_if_raw(world_num, "BorderBlocks", &entry);
            // println!("In world {world_num}: BorderBlocks: {entry:?}");
        }
        DBKey::HardcodedSpawners(_) => {
            print_if_raw(world_num, "HardcodedSpawners", &entry);
            // println!("In world {world_num}: HardcodedSpawners: {entry:?}");
        }
        DBKey::AabbVolumes(_) => {
            print_if_raw(world_num, "AabbVolumes", &entry);
            // check_parsed!("AabbVolumes", &entry, AabbVolumes);
        }
        DBKey::Checksums(_) => {
            print_if_raw(world_num, "Checksums", &entry);
            // println!("In world {world_num}: Checksums: {entry:?}");
        }
        DBKey::MetaDataHash{..} => {
            print_if_raw(world_num, "MetaDataHash", &entry);
        }
        DBKey::FinalizedState{..} => {
            print_if_raw(world_num, "FinalizedState", &entry);
        }
        DBKey::GenerationSeed(_) => {
            print_if_raw(world_num, "GenerationSeed", &entry);
            check_parsed!("GenerationSeed", &entry, GenerationSeed);
        }
        DBKey::BiomeState{..} => {
            print_if_raw(world_num, "BiomeState", &entry);
            // check_parsed!("BiomeState", &entry, BiomeState);
        }
        DBKey::CavesAndCliffsBlending{..} => {
            // I've seen `[0]` and `[1]` in some of my worlds, but there could be other stuff.
            print_if_raw(world_num, "CavesAndCliffsBlending", &entry);
        }
        DBKey::BlendingData{..} => {
            print_if_raw(world_num, "BlendingData", &entry);
        }
        DBKey::ActorDigest(_) => {
            print_if_raw(world_num, "ActorDigest", &entry);
            // println!("In world {world_num}: ActorDigest: {entry:?}");
        }
        DBKey::Actor(_) => {
            print_if_raw(world_num, "Actor", &entry);
            // println!("In world {world_num}: Actor info: {entry:?}");

            // if let DBEntry::Actor(pos, Actor::Multiple(actors)) = &entry {
            //     println!("Multiple actors with id {pos:?}: {actors:?}");
            // }
        }
        DBKey::LevelChunkMetaDataDictionary => {
            print_if_raw(world_num, "LevelChunkMetaDataDictionary", &entry);
        }
        DBKey::AutonomousEntities => {
            print_if_raw(world_num, "AutonomousEntities", &entry);
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
            print_if_raw(world_num, "VillageDwellers", &entry);
            // println!("In world {world_num}: VillageDwellers: {entry:?}");
        }
        DBKey::VillageInfo{..} => {
            print_if_raw(world_num, "VillageInfo", &entry);
            // println!("In world {world_num}: VillageInfo: {entry:?}");
        }
        DBKey::VillagePOI{..} => {
            print_if_raw(world_num, "VillagePOI", &entry);
            // println!("In world {world_num}: VillagePOI: {entry:?}");
        }
        DBKey::VillagePlayers{..} => {
            print_if_raw(world_num, "VillagePlayers", &entry);
            // println!("In world {world_num}: VillagePlayers: {entry:?}");
        }
        DBKey::VillageRaid{..} => {
            print_if_raw(world_num, "VillageRaid", &entry);
            // println!("In world {world_num}: VillageRaid: {entry:?}");
        }
        DBKey::Map{..} => {
            print_if_raw(world_num, "Map", &entry);
            // println!("In world {world_num}: Map: {entry:?}");
            // check_parsed!("Map", &entry, Map);
        }
        DBKey::Portals => {
            print_if_raw(world_num, "Portals", &entry);
            // println!("In world {world_num}: Portals: {entry:?}");
        }
        DBKey::StructureTemplate(..) => {
            print_if_raw(world_num, "StructureTemplate", &entry);
            // println!("In world {world_num}: StructureTemplate: {entry:?}");
            // check_parsed!("StructureTemplate", &entry, StructureTemplate);
        }
        DBKey::TickingArea(..) => {
            print_if_raw(world_num, "TickingArea", &entry);
            // println!("In world {world_num}: TickingArea: {entry:?}");
        }
        DBKey::Scoreboard => {
            print_if_raw(world_num, "Scoreboard", &entry);
            // println!("In world {world_num}: Scoreboard: {entry:?}");
        }
        DBKey::WanderingTraderScheduler => {
            // print_raw(world_num, "WanderingTraderScheduler", &entry);
            println!("In world {world_num}: WanderingTraderScheduler: {entry:?}");
        }
        DBKey::BiomeData => {
            print_if_raw(world_num, "BiomeData", &entry);
            // println!("In world {world_num}: BiomeData: {entry:?}");
        }
        DBKey::MobEvents => {
            print_if_raw(world_num, "MobEvents", &entry);
            // println!("In world {world_num}: MobEvents: {entry:?}");
        }
        DBKey::Overworld => {
            print_if_raw(world_num, "Overworld", &entry);
            // println!("In world {world_num}: Overworld: {entry:?}");
        }
        DBKey::Nether => {
            print_if_raw(world_num, "Nether", &entry);
            // println!("In world {world_num}: Nether: {entry:?}");
        }
        DBKey::TheEnd => {
            print_if_raw(world_num, "TheEnd", &entry);
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
            print_if_raw(world_num, "BiomeIdsTable", &entry);
            // println!("In world {world_num}: BiomeIdsTable: {entry:?}");
        }
        DBKey::FlatWorldLayers => {
            print_if_raw(world_num, "FlatWorldLayers", &entry);
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
            print_if_raw(world_num, "Dimension0", &entry);
            // println!("In world {world_num}: Dimension0: {entry:?}");
        }
        DBKey::Dimension1 => {
            print_if_raw(world_num, "Dimension1", &entry);
            // println!("In world {world_num}: Dimension1: {entry:?}");
        }
        DBKey::Dimension2 => {
            print_if_raw(world_num, "Dimension2", &entry);
            // println!("In world {world_num}: Dimension2: {entry:?}");
        }

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
    }
}
