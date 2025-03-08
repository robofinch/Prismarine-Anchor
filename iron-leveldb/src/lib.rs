use std::{io::Read, path::Path, rc::Rc};
use flate2::{Compress, Compression, Decompress};
use flate2::bufread::{ZlibDecoder, ZlibEncoder};
use rusty_leveldb::compressor::NoneCompressor;
use rusty_leveldb::{BloomPolicy, Compressor, CompressorId, CompressorList, Options, Status, StatusCode, DB};


// may need to implement some more helper functions for handling the DB, or a wrapper class


/// Create a new LevelDB with settings that should be compatible with Minecraft
pub fn new_leveldb(
    name: impl AsRef<Path>, create_if_missing: bool, compressor: DBCompressor
) -> Result<DB, Status> {
    let kilobyte = 1024;
    let megabyte = 1_048_576;

    // These compressor settings are based off of rusty-leveldb's MCPE example
    let mut compressors = CompressorList::new();
    compressors.set_with_id(0, NoneCompressor);
    compressors.set_with_id(2, ZlibCompressor::new(true, Compression::default()));
    compressors.set_with_id(4, ZlibCompressor::new(false, Compression::default()));

    let compressor = match compressor {
        DBCompressor::None              => 0,
        DBCompressor::ZlibWithHeader    => 2,
        DBCompressor::ZlibWithoutHeader => 4
    };

    // These settings are what Amulet Editor uses, for the most part.
    let options = Options {
        create_if_missing,
        filter_policy: Rc::new(Box::new(BloomPolicy::new(10))),
        block_cache_capacity_bytes: 40 * megabyte,
        write_buffer_size: 4 * megabyte,
        log: None,
        compressor,
        compressor_list: Rc::new(compressors),
        block_size: 16 * kilobyte,
        ..Options::default()
    };

    DB::open(name, options)
}

/// Indicates whether world data should be read and written as compressed,
/// and whether the Zlib header should be present in the data read and written.
#[derive(Debug, Copy, Clone)]
pub enum DBCompressor {
    None,
    ZlibWithHeader,
    ZlibWithoutHeader
}

impl Default for DBCompressor {
    fn default() -> Self {
        DBCompressor::ZlibWithoutHeader
    }
}

struct ZlibCompressor {
    include_zlib_header: bool,
    compression_level: Compression
}

impl ZlibCompressor {
    pub fn new(include_zlib_header: bool, compression_level: Compression) -> Self {
        Self {
            include_zlib_header,
            compression_level
        }
    }
}

impl CompressorId for ZlibCompressor {
    const ID: u8 = 2;
}

impl Compressor for ZlibCompressor {
    fn encode(&self, block: Vec<u8>) -> Result<Vec<u8>, Status> {
        // I don't like how it looks to be allocating a large vec, but oh well.
        let mut encoder = ZlibEncoder::new_with_compress(
            block.as_slice(),
            Compress::new(self.compression_level, self.include_zlib_header)
        );
        let mut buf = Vec::new();
        // There really shouldn't be any IO error while reading/writing a Vec,
        // bar out-of-memory maybe? so it's probably a compression error.
        encoder.read_to_end(&mut buf).map_err(|e| {
            Status::new(
                StatusCode::CompressionError,
                &format!("Compression or IO error while compressing data: {e}")
            )
        })?;
        Ok(buf)
    }

    fn decode(&self, block: Vec<u8>) -> Result<Vec<u8>, Status> {
        let mut decoder = ZlibDecoder::new_with_decompress(
            block.as_slice(),
            Decompress::new(self.include_zlib_header)
        );
        let mut buf = Vec::new();
        // There really shouldn't be any IO error while reading/writing a Vec,
        // bar out-of-memory maybe? so it's probably a compression error.
        decoder.read_to_end(&mut buf).map_err(|e| {
            Status::new(
                StatusCode::CompressionError,
                &format!("Compression or IO error while decompressing data: {e}")
            )
        })?;
        Ok(buf)
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn leveldb() {

    }
}
