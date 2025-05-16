mod leveldb;


use std::{io, fmt, mem};
use std::{borrow::Cow, rc::Rc};
use std::{
    io::{Cursor, Read, Write},
    fmt::{Debug, Formatter},
    path::{Path, PathBuf},
};

use rusty_leveldb::{DB as LevelDB, env::Env, Status};
use thiserror::Error;

use prismarine_anchor_leveldb_entries::{
    DBEntry, DBKey, EntryBytes, EntryParseOptions, EntryToBytesOptions, KeyToBytesOptions, ValueToBytesError
};
use prismarine_anchor_nbt::io as nbt_io;
use prismarine_anchor_nbt::{NbtCompound, settings::IoOptions};
use prismarine_anchor_nbt::io::{NbtIoError, write_compound};

use self::leveldb::{DBCompressor, new_leveldb};


// other possible things to do in the future:

// support the oldest versions which didn't use LevelDB

// read resource packs folder and world_resource_packs.json
// (probably won't, easier to just provide some other API to provide resource packs to editor)

// pub struct WorldIcon();

/// Data associated with one Bedrock world's folder (or .mcworld file)
pub struct BedrockWorldFiles {
    level_dat:  LevelDatFile,
    level_name: String,
    db:         LevelDB,
    env:        Rc<Box<dyn Env>>,
    root_path:  PathBuf,
}

impl BedrockWorldFiles {
    /// Open a Bedrock world folder in the provided `Env` (with either the world folder or
    /// the world folder's contents at the `Env`'s root).
    ///
    /// Ideally, while this `BedrockWorld` is open,
    /// `level.dat`, `levelname.txt`, and the LevelDB database
    /// should not be edited by anything else.
    pub fn open_world(env: Box<dyn Env>) -> Result<Self, BedrockWorldFileError> {
        Self::open_world_from_path(env, Path::new(""))
    }

    /// Open a Bedrock world folder in the provided `Env` (with either the world folder or
    /// the world folder's contents at the indicated root path).
    ///
    /// Ideally, while this `BedrockWorld` is open,
    /// `level.dat`, `levelname.txt`, and the LevelDB database
    /// should not be edited by anything else.
    pub fn open_world_from_path(
        env:       Box<dyn Env>,
        root_path: &Path,
    ) -> Result<Self, BedrockWorldFileError> {
        let env_ref = env.as_ref();

        fn add_status_context(err: Status) -> BedrockWorldFileError {
            BedrockWorldFileError::StatusCode(Cow::Borrowed("trying to open a Bedrock world"), err)
        }
        fn add_io_context(err: io::Error) -> BedrockWorldFileError {
            BedrockWorldFileError::Io(Cow::Borrowed("trying to open a Bedrock world"), err)
        }

        let mut list = env
            .children(root_path)
            .map_err(add_status_context)?;

        // Find the common directory of all children of the root path
        let mut nested_root_path = PathBuf::new();
        if let Some(last) = list.pop() {
            nested_root_path = last;
        }
        for path in list {
            let old_root_path = mem::replace(&mut nested_root_path, PathBuf::new());

            let root_path_components = old_root_path.components();
            let path_components = path.components();

            for (step, other_step) in root_path_components.zip(path_components) {
                if step == other_step {
                    nested_root_path.push(step);
                }
            }
        }

        let root_path = root_path.join(nested_root_path);

        let level_dat = LevelDatFile::parse_from_env(env_ref, &root_path)?;

        let mut level_name = String::new();
        let _ = open_from_path(env_ref, &root_path, "levelname.txt")
            .map_err(add_status_context)?
            .read_to_string(&mut level_name)
            .map_err(add_io_context)?;

        let env = Rc::new(env);

        let db_path = root_path.join("db/");
        let db = new_leveldb(env.clone(), db_path, false, DBCompressor::default())
            .map_err(add_status_context)?;

        Ok(Self {
            level_dat,
            level_name,
            db,
            env,
            root_path,
        })
    }

    /// Read the in-memory `level.dat` information.
    pub fn level_dat(&self) -> &LevelDatFile {
        &self.level_dat
    }

    /// Read or write the in-memory `level.dat` information.
    pub fn level_dat_mut(&mut self) -> &mut LevelDatFile {
        &mut self.level_dat
    }

    /// Write the in-memory `level.dat` information to this world's `Env`.
    pub fn save_level_dat(&self) -> Result<(), BedrockWorldFileError> {
        self.level_dat
            .write_to_env(self.env.as_ref().as_ref(), &self.root_path)
    }

    /// Read the in-memory `levelname.txt` information.
    pub fn level_name(&self) -> &String {
        &self.level_name
    }

    /// Read or write the in-memory `levelname.txt` information.
    pub fn level_name_mut(&mut self) -> &mut String {
        &mut self.level_name
    }

    /// Write the in-memory `levelname.txt` information to this world's `Env`.
    pub fn save_level_name(&self) -> Result<(), BedrockWorldFileError> {
        //
        fn add_status_context(err: Status) -> BedrockWorldFileError {
            BedrockWorldFileError::StatusCode(Cow::Borrowed("writing to levelname.txt"), err)
        }
        fn add_io_context(err: io::Error) -> BedrockWorldFileError {
            BedrockWorldFileError::Io(Cow::Borrowed("writing to levelname.txt"), err)
        }

        let env = self.env.as_ref().as_ref();

        let mut file = write_to_path(env, &self.root_path, "levelname.txt")
            .map_err(add_status_context)?;

        file.write_all(self.level_name.as_bytes())
            .map_err(add_io_context)?;

        Ok(())
    }

    /// Get access to the `Env`-based LevelDB of this world.
    pub fn level_db(&mut self) -> &mut LevelDB {
        &mut self.db
    }

    /// Read the entry in the LevelDB with the provided key and serialization options,
    /// and parse it into a `DBEntry` if present.
    pub fn get(
        &mut self,
        key: DBKey,
        opts: KeyToBytesOptions,
        parse_opts: EntryParseOptions,
    ) -> Option<DBEntry> {
        self.db
            .get(&key.to_bytes(opts))
            .map(|value| DBEntry::parse_value_vec(key, value, parse_opts))
    }

    /// Write the provided entry into the LevelDB using the provided serialization options.
    pub fn put(
        &mut self,
        entry: DBEntry,
        opts:  EntryToBytesOptions,
    ) -> Result<(), BedrockWorldFileError> {
        let EntryBytes { key, value } = entry
            .into_bytes(opts)
            .map_err(|err| err.value_error)?;

        self.db.put(&key, &value).map_err(|err| {
            BedrockWorldFileError::StatusCode(
                Cow::Borrowed("writing an entry to the LevelDB"),
                err,
            )
        })
    }

    /// Read the world's icon from this world's `Env`.
    pub fn world_icon(&self) -> Result<Box<dyn Read>, BedrockWorldFileError> {
        open_from_path(
            self.env.as_ref().as_ref(),
            &self.root_path,
            "world_icon.jpeg",
        )
        .map_err(|err| {
            BedrockWorldFileError::StatusCode(
                Cow::Borrowed("opening a world's icon image"),
                err,
            )
        })
    }
}

impl Debug for BedrockWorldFiles {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "BedrockWorld named \"{}\"", self.level_name)
    }
}

/// Partially parsed `level.dat` file
#[derive(Debug)]
pub struct LevelDatFile {
    pub version: i32,
    pub nbt:     NbtCompound,
}

impl LevelDatFile {
    /// Read the `level.dat` file for a world whose folder is located at `root_path`
    /// inside the provided `Env`.
    pub fn parse_from_env(env: &dyn Env, root_path: &Path) -> Result<Self, BedrockWorldFileError> {

        fn add_status_context(err: Status) -> BedrockWorldFileError {
            BedrockWorldFileError::StatusCode(Cow::Borrowed("trying to read level.dat"), err)
        }
        fn add_nbt_context(err: NbtIoError) -> BedrockWorldFileError {
            BedrockWorldFileError::NbtError(Cow::Borrowed("reading level.dat"), err)
        }

        let mut file = open_from_path(env, root_path, "level.dat")
            .map_err(add_status_context)?;

        let opts = IoOptions::bedrock_uncompressed();

        let (version,  _) = nbt_io::read_bedrock_header(&mut file, opts)
            .map_err(add_nbt_context)?;
        let (compound, _) = nbt_io::read_compound(&mut file, opts)
            .map_err(add_nbt_context)?;

        Ok(Self {
            version,
            nbt: compound,
        })
    }

    /// Write this `level.dat` file to a world whose folder is located at `root_path`
    /// inside the provided `Env`.
    pub fn write_to_env(
        &self,
        env:       &dyn Env,
        root_path: &Path,
    ) -> Result<(), BedrockWorldFileError> {

        fn add_status_context(err: Status) -> BedrockWorldFileError {
            BedrockWorldFileError::StatusCode(Cow::Borrowed("writing to level.dat"), err)
        }
        fn add_io_context(err: io::Error) -> BedrockWorldFileError {
            BedrockWorldFileError::Io(Cow::Borrowed("writing to level.dat"), err)
        }
        fn add_nbt_context(err: NbtIoError) -> BedrockWorldFileError {
            BedrockWorldFileError::NbtError(Cow::Borrowed("writing to level.dat"), err)
        }

        let dat_path = root_path.join("level.dat");
        let new_dat_path = root_path.join("level.dat_new");

        let mut nbt_buffer = Cursor::new(Vec::new());

        write_compound(
            &mut nbt_buffer,
            IoOptions::bedrock_uncompressed(),
            None,
            &self.nbt,
        )
        .map_err(add_nbt_context)?;

        let nbt_buffer = nbt_buffer.into_inner();

        // Try to limit any issues with errors mid-write by writing to level.dat_new
        // and then renaming it to level.dat
        let mut file = env
            .open_writable_file(&new_dat_path)
            .map_err(add_status_context)?;

        nbt_io::write_bedrock_header(
            &mut file,
            IoOptions::bedrock_uncompressed(),
            self.version,
            nbt_buffer.len(),
        )
        .map_err(add_nbt_context)?;

        file.write_all(nbt_buffer.as_slice())
            .map_err(add_io_context)?;

        env.rename(&new_dat_path, &dat_path)
            .map_err(add_status_context)?;

        Ok(())
    }
}

/// Errors that may occur while reading or writing `BedrockWorldFiles`.
#[derive(Error, Debug)]
pub enum BedrockWorldFileError {
    // Error message should be a present participle, e.g. "trying to [do something]"
    #[error("error while {0}: {1}")]
    StatusCode(Cow<'static, str>, Status),
    #[error("error while {0}: {1}")]
    NbtError(Cow<'static, str>, NbtIoError),
    #[error("error while writing a LevelDB entry: {0}")]
    LevelDBValue(#[from] ValueToBytesError),
    #[error("error while {0}: {1}")]
    Io(Cow<'static, str>, io::Error),
}

#[inline]
fn open_from_path<P: AsRef<Path>>(
    env:       &dyn Env,
    root_path: &Path,
    rel_path:  P,
) -> Result<Box<dyn Read>, Status> {
    env.open_sequential_file(&root_path.join(rel_path))
}

#[inline]
fn write_to_path<P: AsRef<Path>>(
    env:       &dyn Env,
    root_path: &Path,
    rel_path:  P,
) -> Result<Box<dyn Write>, Status> {
    env.open_writable_file(&root_path.join(rel_path))
}
