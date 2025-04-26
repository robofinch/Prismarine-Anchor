//! Modified from `mem_env.rs` in `rusty-leveldb`, a.k.a. `leveldb-rs`
// MemEnv didn't expose some of the stuff necessary for converting back to a ZIP archive,
// in particular an iterator over all files.

use std::io;
use std::borrow::Cow;
use std::{
    collections::{HashMap, hash_map::Entry},
    io::{Cursor, Read, Result as IoResult, Seek, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
#[cfg(not(target_arch = "wasm32"))]
use std::{thread, time::Duration};

use rusty_leveldb::{Result as StatusResult, Status, StatusCode};
use rusty_leveldb::env::{path_to_str, path_to_string, Env, FileLock, Logger, RandomAccess};
use thiserror::Error;
use web_time::{SystemTime, UNIX_EPOCH};
use zip::{result::ZipError, write::SimpleFileOptions, ZipArchive, ZipWriter};


/// `ZipEnv` supports writing or reading an in-memory virtual file system to or from a ZIP archive.
#[expect(missing_debug_implementations, reason = "contains too much data")]
pub struct ZipEnv(MemFS);

impl Default for ZipEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl ZipEnv {
    pub fn new() -> Self {
        Self(MemFS::new())
    }

    pub fn try_into_bytes(self) -> Result<Vec<u8>, ZipEnvError> {
        let mut writer: _ = ZipWriter::new(Cursor::new(Vec::new()));

        #[expect(
            clippy::iter_over_hash_type,
            reason = "unavoidable, and any order should work for zip",
        )]
        for (key, entry) in self.0.store.lock().unwrap().iter() {
            let file = &entry.f.0.lock().unwrap().0;
            let is_large = u32::try_from(file.len()).is_err();
            let opts = SimpleFileOptions::default().large_file(is_large);

            writer.start_file(key, opts)?;
            io::copy(&mut Cursor::new(file), &mut writer).map_err(ZipError::Io)?;
        }

        writer.finish().map(|c| c.into_inner()).map_err(ZipEnvError::Zip)
    }

    pub fn try_into_archive(self) -> Result<ZipArchive<impl Read>, ZipEnvError> {
        let mut writer: _ = ZipWriter::new(Cursor::new(Vec::new()));

        #[expect(
            clippy::iter_over_hash_type,
            reason = "unavoidable, and any order should work for zip",
        )]
        for (key, entry) in self.0.store.lock().unwrap().iter() {
            let file = &entry.f.0.lock().unwrap().0;
            let is_large = u32::try_from(file.len()).is_err();
            let opts = SimpleFileOptions::default().large_file(is_large);

            writer.start_file(key, opts)?;
            io::copy(&mut Cursor::new(file), &mut writer).map_err(ZipError::Io)?;
        }

        writer.finish_into_readable().map_err(ZipEnvError::Zip)
    }
}

impl<R: Read + Seek> TryFrom<ZipArchive<R>> for ZipEnv {
    type Error = ZipEnvError;

    fn try_from(mut archive: ZipArchive<R>) -> Result<Self, ZipEnvError> {

        let fs = MemFS::new();

        for idx in 0..archive.len() {
            let mut file = archive.by_index(idx)?;
            let name = file.enclosed_name().ok_or(
                ZipError::InvalidArchive(Cow::Borrowed("Invalid file path and/or name"))
            )?;
            let mut writer = fs.open_w(&name, false, true)?;

            io::copy(&mut file, writer.as_mut()).map_err(ZipError::Io)?;
        }

        Ok(Self(fs))
    }
}

impl Env for ZipEnv {
    fn open_sequential_file(&self, p: &Path) -> StatusResult<Box<dyn Read>> {
        let f = self.0.open(p, false)?;
        Ok(Box::new(MemFileReader::new(f, 0)))
    }
    fn open_random_access_file(&self, p: &Path) -> StatusResult<Box<dyn RandomAccess>> {
        self.0
            .open(p, false)
            .map(|m| Box::new(m) as Box<dyn RandomAccess>)
    }
    fn open_writable_file(&self, p: &Path) -> StatusResult<Box<dyn Write>> {
        self.0.open_w(p, true, true)
    }
    fn open_appendable_file(&self, p: &Path) -> StatusResult<Box<dyn Write>> {
        self.0.open_w(p, true, false)
    }

    fn exists(&self, p: &Path) -> StatusResult<bool> {
        self.0.exists(p)
    }
    fn children(&self, p: &Path) -> StatusResult<Vec<PathBuf>> {
        self.0.children_of(p)
    }
    fn size_of(&self, p: &Path) -> StatusResult<usize> {
        self.0.size_of(p)
    }

    fn delete(&self, p: &Path) -> StatusResult<()> {
        self.0.delete(p)
    }
    fn mkdir(&self, p: &Path) -> StatusResult<()> {
        if self.exists(p)? {
            Err(Status::new(StatusCode::AlreadyExists, ""))
        } else {
            Ok(())
        }
    }
    fn rmdir(&self, p: &Path) -> StatusResult<()> {
        if !self.exists(p)? {
            Err(Status::new(StatusCode::NotFound, ""))
        } else {
            Ok(())
        }
    }
    fn rename(&self, old: &Path, new: &Path) -> StatusResult<()> {
        self.0.rename(old, new)
    }

    fn lock(&self, p: &Path) -> StatusResult<FileLock> {
        self.0.lock(p)
    }
    fn unlock(&self, p: FileLock) -> StatusResult<()> {
        self.0.unlock(p)
    }

    fn micros(&self) -> u64 {
        // Having an unbounded loop for this would feel weird to me, even though
        // that's what leveldb-rs does
        for _ in 0..1_000_000 {
            if let Ok(dur) = SystemTime::now().duration_since(UNIX_EPOCH) {
                // In theory, this could overflow.
                return dur.as_micros() as u64;
            }
        }
        0
    }
    fn sleep_for(&self, _micros: u32) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            #[expect(clippy::used_underscore_binding)]
            thread::sleep(Duration::new(0, _micros * 1000));
        }
    }

    fn new_logger(&self, p: &Path) -> StatusResult<Logger> {
        self.open_appendable_file(p)
            .map(|dst| Logger::new(Box::new(dst)))
    }
}

#[derive(Error, Debug)]
pub enum ZipEnvError {
    #[error(transparent)]
    Zip(#[from] ZipError),
    #[error(transparent)]
    Status(#[from] Status),
}

// ================================
// Mostly existing code
// ================================

#[derive(Debug, Clone)]
struct BufferBackedFile(Vec<u8>);

impl RandomAccess for BufferBackedFile {
    fn read_at(&self, off: usize, dst: &mut [u8]) -> StatusResult<usize> {
        if off > self.0.len() {
            return Ok(0);
        }
        let remaining = self.0.len() - off;
        let to_read = if dst.len() > remaining {
            remaining
        } else {
            dst.len()
        };
        dst[0..to_read].copy_from_slice(&self.0[off..off + to_read]);
        Ok(to_read)
    }
}

/// A `MemFile` holds a shared, concurrency-safe buffer. It can be shared among several
/// `MemFileReader`s and `MemFileWriter`s, each with an independent offset.
#[derive(Debug, Clone)]
struct MemFile(Arc<Mutex<BufferBackedFile>>);

impl MemFile {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(BufferBackedFile(Vec::new()))))
    }
}

/// A `MemFileReader` holds a reference to a `MemFile` and a read offset.
struct MemFileReader(MemFile, usize);

impl MemFileReader {
    fn new(f: MemFile, from: usize) -> Self {
        Self(f, from)
    }
}

// We need Read/Write/Seek implementations for our MemFile in order to work well with the
// concurrency requirements. It's very hard or even impossible to implement those traits just by
// wrapping MemFile in other types.
impl Read for MemFileReader {
    fn read(&mut self, dst: &mut [u8]) -> IoResult<usize> {
        let buf = (self.0).0.lock().unwrap();
        if self.1 >= buf.0.len() {
            // EOF
            return Ok(0);
        }
        let remaining = buf.0.len() - self.1;
        let to_read = if dst.len() > remaining {
            remaining
        } else {
            dst.len()
        };

        dst[0..to_read].copy_from_slice(&buf.0[self.1..self.1 + to_read]);
        self.1 += to_read;
        Ok(to_read)
    }
}

/// A `MemFileWriter` holds a reference to a `MemFile` and a write offset.
struct MemFileWriter(MemFile, usize);

impl MemFileWriter {
    fn new(f: MemFile, append: bool) -> Self {
        let len = f.0.lock().unwrap().0.len();
        Self(f, if append { len } else { 0 })
    }
}

impl Write for MemFileWriter {
    fn write(&mut self, src: &[u8]) -> IoResult<usize> {
        let mut buf = (self.0).0.lock().unwrap();
        // Write is append.
        if self.1 == buf.0.len() {
            buf.0.extend_from_slice(src);
        } else {
            // Write in the middle, possibly appending.
            let remaining = buf.0.len() - self.1;
            if src.len() <= remaining {
                // src fits into buffer.
                buf.0[self.1..self.1 + src.len()].copy_from_slice(src);
            } else {
                // src doesn't fit; first copy what fits, then append the rest/
                buf.0[self.1..self.1 + remaining].copy_from_slice(&src[0..remaining]);
                buf.0.extend_from_slice(&src[remaining..src.len()]);
            }
        }
        self.1 += src.len();
        Ok(src.len())
    }
    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }
}

impl RandomAccess for MemFile {
    fn read_at(&self, off: usize, dst: &mut [u8]) -> StatusResult<usize> {
        let guard = self.0.lock().unwrap();
        let buf: &BufferBackedFile = &guard;
        buf.read_at(off, dst)
    }
}

struct MemFSEntry {
    f: MemFile,
    locked: bool,
}

/// `MemFS` implements a completely in-memory file system, both for testing and temporary in-memory
/// databases. It supports full concurrency.
#[expect(missing_debug_implementations, reason = "contains too much data")]
pub struct MemFS {
    store: Arc<Mutex<HashMap<String, MemFSEntry>>>,
}

impl MemFS {
    fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Open a file. The caller can use the `MemFile` either inside a `MemFileReader` or as
    /// `RandomAccess`.
    fn open(&self, p: &Path, create: bool) -> StatusResult<MemFile> {
        let mut fs = self.store.lock().unwrap();
        match fs.entry(path_to_string(p)) {
            Entry::Occupied(o) => Ok(o.get().f.clone()),
            Entry::Vacant(v) => {
                if !create {
                    return Err(Status::new(
                        StatusCode::NotFound,
                        &format!("open: file not found: {}", path_to_str(p)),
                    ));
                }
                let f = MemFile::new();
                v.insert(MemFSEntry {
                    f: f.clone(),
                    locked: false,
                });
                Ok(f)
            }
        }
    }
    /// Open a file for writing.
    #[expect(clippy::fn_params_excessive_bools, reason = "originated from rusty-leveldb's code")]
    fn open_w(&self, p: &Path, append: bool, truncate: bool) -> StatusResult<Box<dyn Write>> {
        let f = self.open(p, true)?;
        if truncate {
            f.0.lock().unwrap().0.clear();
        }
        Ok(Box::new(MemFileWriter::new(f, append)))
    }
    fn exists(&self, p: &Path) -> StatusResult<bool> {
        let fs = self.store.lock()?;
        Ok(fs.contains_key(path_to_str(p)))
    }
    fn children_of(&self, p: &Path) -> StatusResult<Vec<PathBuf>> {
        let fs = self.store.lock()?;
        let mut prefix = path_to_string(p);
        let main_separator_str = std::path::MAIN_SEPARATOR.to_string();
        if !prefix.ends_with(&main_separator_str) {
            prefix.push(std::path::MAIN_SEPARATOR);
        }

        let mut children = Vec::new();
        #[expect(
            clippy::iter_over_hash_type,
            reason = "unavoidable, and thus order of `children_of` is unspecified",
        )]
        for k in fs.keys() {
            if k.starts_with(&prefix) {
                children.push(Path::new(k.strip_prefix(&prefix).unwrap_or(k)).to_owned());
            }
        }
        Ok(children)
    }
    fn size_of(&self, p: &Path) -> StatusResult<usize> {
        let mut fs = self.store.lock()?;
        match fs.entry(path_to_string(p)) {
            Entry::Occupied(o) => Ok(o.get().f.0.lock()?.0.len()),
            Entry::Vacant(_) => Err(Status::new(
                StatusCode::NotFound,
                &format!("size_of: file not found: {}", path_to_str(p)),
            )),
        }
    }
    fn delete(&self, p: &Path) -> StatusResult<()> {
        let mut fs = self.store.lock()?;
        match fs.entry(path_to_string(p)) {
            Entry::Occupied(o) => {
                o.remove_entry();
                Ok(())
            }
            Entry::Vacant(_) => Err(Status::new(
                StatusCode::NotFound,
                &format!("delete: file not found: {}", path_to_str(p)),
            )),
        }
    }
    fn rename(&self, from: &Path, to: &Path) -> StatusResult<()> {
        let mut fs = self.store.lock()?;
        match fs.remove(path_to_str(from)) {
            Some(v) => {
                fs.insert(path_to_string(to), v);
                Ok(())
            }
            None => Err(Status::new(
                StatusCode::NotFound,
                &format!("rename: file not found: {}", path_to_str(from)),
            )),
        }
    }
    fn lock(&self, p: &Path) -> StatusResult<FileLock> {
        let mut fs = self.store.lock()?;
        match fs.entry(path_to_string(p)) {
            Entry::Occupied(mut o) => {
                if o.get().locked {
                    Err(Status::new(
                        StatusCode::LockError,
                        &format!("already locked: {}", path_to_str(p)),
                    ))
                } else {
                    o.get_mut().locked = true;
                    Ok(FileLock {
                        id: path_to_string(p),
                    })
                }
            }
            Entry::Vacant(v) => {
                let f = MemFile::new();
                v.insert(MemFSEntry {
                    f: f.clone(),
                    locked: true,
                });
                Ok(FileLock {
                    id: path_to_string(p),
                })
            }
        }
    }
    fn unlock(&self, l: FileLock) -> StatusResult<()> {
        let mut fs = self.store.lock()?;
        let id = l.id.clone();
        match fs.entry(l.id) {
            Entry::Occupied(mut o) => {
                if !o.get().locked {
                    Err(Status::new(
                        StatusCode::LockError,
                        &format!("unlocking unlocked file: {id}"),
                    ))
                } else {
                    o.get_mut().locked = false;
                    Ok(())
                }
            }
            Entry::Vacant(_) => Err(Status::new(
                StatusCode::NotFound,
                &format!("unlock: file not found: {id}"),
            )),
        }
    }
}
