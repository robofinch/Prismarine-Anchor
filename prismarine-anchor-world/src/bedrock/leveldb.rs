use std::{io::Read as _, path::Path, rc::Rc};

use flate2::{Compress, Compression, Decompress};
use flate2::bufread::{ZlibDecoder, ZlibEncoder};
use rusty_leveldb::{
    compressor::NoneCompressor, Compressor, CompressorId, CompressorList,
    DB, env::Env, Options, Status, StatusCode,
};


/// Initialize a LevelDB with settings that should be compatible with Minecraft
pub(super) fn new_leveldb<P: AsRef<Path>>(
    env:               Rc<Box<dyn Env>>,
    db_path:           P,
    create_if_missing: bool,
    compressor:        DBCompressor,
) -> Result<DB, Status> {
    // These compressor settings are based off of rusty-leveldb's MCPE example
    let mut compressors = CompressorList::new();
    compressors.set_with_id(0, NoneCompressor);
    compressors.set_with_id(2, ZlibCompressor::new(true,  Compression::default()));
    compressors.set_with_id(4, ZlibCompressor::new(false, Compression::default()));

    let compressor = match compressor {
        DBCompressor::None              => 0,
        DBCompressor::ZlibWithHeader    => 2,
        DBCompressor::ZlibWithoutHeader => 4,
    };

    // Larger values are apparently better for bulk scans.
    // The default is 4096, and the cache is (by default) 1024 times 4096.
    // I'm multiplying it by 4.
    // TODO: would even larger values be more performant?
    let block_size = 4 * 4096;

    let options = Options {
        block_size,
        block_cache_capacity_bytes: block_size * 1024,
        create_if_missing,
        compressor,
        compressor_list:            Rc::new(compressors),
        env,
        write_buffer_size:          block_size * 1024,
        ..Options::default()
    };

    DB::open(db_path, options)
}

/// Indicates whether world data should be read and written as compressed,
/// and whether the Zlib header should be present in the data read and written.
#[expect(unused)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DBCompressor {
    None,
    ZlibWithHeader,
    /// Also known by terms like `ZlibRaw`
    #[default]
    ZlibWithoutHeader,
}

#[derive(Debug)]
struct ZlibCompressor {
    include_zlib_header: bool,
    compression_level:   Compression,
}

impl ZlibCompressor {
    #[inline]
    fn new(include_zlib_header: bool, compression_level: Compression) -> Self {
        Self {
            include_zlib_header,
            compression_level,
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
            Compress::new(self.compression_level, self.include_zlib_header),
        );
        let mut buf = Vec::new();
        // There really shouldn't be any IO error while reading/writing a Vec,
        // bar out-of-memory maybe? so it's probably a compression error.
        encoder.read_to_end(&mut buf).map_err(|e| {
            Status::new(
                StatusCode::CompressionError,
                &format!("Compression or IO error while compressing data: {e}"),
            )
        })?;
        Ok(buf)
    }

    fn decode(&self, block: Vec<u8>) -> Result<Vec<u8>, Status> {
        let mut decoder = ZlibDecoder::new_with_decompress(
            block.as_slice(),
            Decompress::new(self.include_zlib_header),
        );
        let mut buf = Vec::new();
        // There really shouldn't be any IO error while reading/writing a Vec,
        // bar out-of-memory maybe? so it's probably a compression error.
        decoder.read_to_end(&mut buf).map_err(|e| {
            Status::new(
                StatusCode::CompressionError,
                &format!("Compression or IO error while decompressing data: {e}"),
            )
        })?;
        Ok(buf)
    }
}
