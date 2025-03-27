use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

use rusty_leveldb::Result as StatusResult;
use rusty_leveldb::env::{Env, FileLock, Logger, RandomAccess};
use zip::ZipArchive;


#[derive(Debug)]
pub struct ZipEnv<R> {
    archive: ZipArchive<R>,
}

impl<R> Env for ZipEnv<R> {
    fn open_sequential_file(&self, _: &Path) -> StatusResult<Box<dyn Read>> {
        todo!()
    }
    fn open_random_access_file(&self, _: &Path) -> StatusResult<Box<dyn RandomAccess>> {
        todo!()
    }
    fn open_writable_file(&self, _: &Path) -> StatusResult<Box<dyn Write>> {
        todo!()
    }
    fn open_appendable_file(&self, _: &Path) -> StatusResult<Box<dyn Write>> {
        todo!()
    }

    fn exists(&self, _: &Path) -> StatusResult<bool> {
        todo!()
    }
    fn children(&self, _: &Path) -> StatusResult<Vec<PathBuf>> {
        todo!()
    }
    fn size_of(&self, _: &Path) -> StatusResult<usize> {
        todo!()
    }

    fn delete(&self, _: &Path) -> StatusResult<()> {
        todo!()
    }
    fn mkdir(&self, _: &Path) -> StatusResult<()> {
        todo!()
    }
    fn rmdir(&self, _: &Path) -> StatusResult<()> {
        todo!()
    }
    fn rename(&self, _: &Path, _: &Path) -> StatusResult<()> {
        todo!()
    }

    fn lock(&self, _: &Path) -> StatusResult<FileLock> {
        todo!()
    }
    fn unlock(&self, l: FileLock) -> StatusResult<()> {
        todo!()
    }

    fn new_logger(&self, _: &Path) -> StatusResult<Logger> {
        todo!()
    }

    fn micros(&self) -> u64 {
        todo!()
    }
    fn sleep_for(&self, micros: u32) {
        todo!()
    }
}
