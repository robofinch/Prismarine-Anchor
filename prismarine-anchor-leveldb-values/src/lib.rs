#[cfg(feature = "dimensions")]
pub mod dimensions;
#[cfg(feature = "chunk_position")]
pub mod chunk_position;
#[cfg(feature = "uuid")]
pub mod uuid;

#[cfg(feature = "concatenated_nbt_compounds")]
pub mod concatenated_nbt_compounds; // For multiple sorts of values
#[cfg(feature = "nbt_compound_conversion")]
pub mod nbt_compound_conversion;
#[cfg(feature = "palettized_storage")]
pub mod palettized_storage;

#[cfg(feature = "chunk_version")]
pub mod chunk_version;
#[cfg(feature = "actor_digest_version")]
pub mod actor_digest_version;
#[cfg(feature = "data_3d")]
pub mod data_3d;
#[cfg(feature = "data_2d")]
pub mod data_2d;
#[cfg(feature = "legacy_data_2d")]
pub mod legacy_data_2d;
#[cfg(feature = "subchunk_blocks")]
pub mod subchunk_blocks;
#[cfg(feature = "legacy_terrain")]
pub mod legacy_terrain;
#[cfg(feature = "legacy_extra_block_data")]
pub mod legacy_extra_block_data;
#[cfg(feature = "border_blocks")]
pub mod border_blocks;
#[cfg(feature = "hardcoded_spawners")]
pub mod hardcoded_spawners;
#[cfg(feature = "aabb_volumes")]
pub mod aabb_volumes;
#[cfg(feature = "checksums")]
pub mod checksums;
#[cfg(feature = "metadata")]
pub mod metadata; // for both MetaDataHash and LevelChunkMetaDataDictionary
#[cfg(feature = "finalized_state")]
pub mod finalized_state;
#[cfg(feature = "biome_state")]
pub mod biome_state;
#[cfg(feature = "conversion_data")]
pub mod conversion_data;
#[cfg(feature = "blending_data")]
pub mod blending_data;
#[cfg(feature = "actor_digest")]
pub mod actor_digest;
#[cfg(feature = "actor")]
pub mod actor;
#[cfg(feature = "flat_world_layers")]
pub mod flat_world_layers;
#[cfg(feature = "level_spawn_was_fixed")]
pub mod level_spawn_was_fixed;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueParseOptions {
    pub data_fidelity: DataFidelity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueToBytesOptions {
    pub data_fidelity:           DataFidelity,
    pub handle_excessive_length: HandleExcessiveLength,
}

// TODO: implement BitPerfect

/// Control whether non-semantically-important data is parsed or serialized to bytes.
/// (Semantically-important data is always parsed and serialized.)
///
/// NOTE: you may also need to enable the `preserve_order` feature of `prismarine-anchor-nbt`
/// for `BitPerfect` to fully function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataFidelity {
    /// Preserve all data, including semantically unimportant data like padding bits.
    /// Also preserves the order of all entries in likely-unordered key-value maps; currently,
    /// this only affects `AabbVolumes`, which in the `Semantic` fidelity is sorted (in line
    /// with observed data from Minecraft saves).
    ///
    /// NOTE: you may also need to enable the `preserve_order` feature of `prismarine-anchor-nbt`
    /// for this option to fully function.
    BitPerfect,
    /// Preserve all semantically important data. This does not read padding bits, and writes
    /// zeroes as padding bits. The order of entries in most key-value maps is preserved,
    /// except the entries in the maps of `AabbVolumes` are sorted by key (in line with observed
    /// data from Minecraft saves).
    Semantic,
}

/// How to handle lists or maps whose number of entries is too large to fit in a u32.
///
/// If set to `ReturnError`, then if a list with a length that needs to be
/// written into a `u32` in the byte representation (e.g. `Checksums` or
/// `LevelChunkMetaDataDictionary` data) with more than 2^32 values is attempted to be written
/// to bytes, an error is returned.
/// If `SilentlyTruncate`, the list is silently truncated to 2^32 values if such a thing occurs.
///
/// Note that this does *not* affect `SubchunkBlocks` data; if there are more than 255
/// block layers in `SubchunkBlocks` data, then only the first 255 layers will be written;
/// no error is ever returned (and to begin with, there should never be anywhere near that many
/// layers).
///
/// It should probably be set to `ReturnError` unless you have cause to write weirdly massive data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleExcessiveLength {
    ReturnError,
    SilentlyTruncate,
}

impl HandleExcessiveLength {
    /// Given a `usize` length, attempts to cast it to a `u32`. If `self` is `ReturnError`
    /// and the conversion fails, then an error is returned; otherwise, the value is saturated
    /// to `u32::MAX` instead.
    ///
    /// Both a `u32` and `usize` are returned, to handle the case that the `usize` length
    /// must be truncated.
    pub fn length_to_u32(self, len: usize) -> Option<(u32, usize)> {
        if size_of::<usize>() >= size_of::<u32>() {
            let len = match u32::try_from(len) {
                Ok(len) => len,
                Err(_) => match self {
                    Self::ReturnError      => return None,
                    Self::SilentlyTruncate => u32::MAX,
                }
            };

            // This cast from u32 to usize won't overflow
            Some((len, len as usize))
        } else {
            // This cast from usize to u32 won't overflow
            Some((len as u32, len))
        }
    }
}

/// Some versions of Bedrock elide the numeric ID or name of the Overworld,
/// and only serialize the IDs or names of non-Overworld dimensions.
///
/// Dimension IDs and names are read as `Option<NumericDimension>` or `Option<NamedDimension>`,
/// with `None` indicating an implicit Overworld value.
///
/// These options indicate how a `Option<NumericDimension>` or `Option<NamedDimension>`
/// should be serialized: either
/// - never elide the value and always write it,
/// - always elide the Overworld value and only write the ID or name of a non-Overworld
///   dimension, or
/// - elide the Overworld value if the option is `None`.
///
/// The best choices (aside from testing, where `MatchElision` may be useful) are
/// - numeric dimension IDs for all current versions (up to at least 1.21.51): `AlwaysElide`
/// - dimension names for any version below 1.20.40: `AlwaysElide`
/// - dimension names for any version at or above 1.20.40: `AlwaysWrite`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverworldElision {
    /// Write the IDs and names of all dimensions.
    AlwaysWrite,
    /// Always write the ID or name of the Overworld, and only write the IDs and names of all
    /// non-Overworld dimensions.
    AlwaysElide,
    /// Elide the ID or name of the Overworld if and only if a `Option<NumericDimension>`
    /// or `Option<NamedDimension>` is `None`. The IDs and names of all non-Overworld dimensions
    /// are always written.
    MatchElision,
}

// Note that the `dimensions` module implements two functions for `OverworldElision`.


// Throughout this crate, pretend we're implementing something like the following trait:
/*
trait DBValue {
    // Could be `u8`, `[u8; N]`, or `&[u8]`; may or may not accept `opts`
    fn parse(value: &[u8], opts: ValueParseOptions) -> Option<Self>;

    // Could allow for either `self.to_bytes(opts)` or `self.to_bytes(opts)?`,
    // or `self.to_le_bytes().to_vec()` or `vec![u8::from(self)]`.
    // So, this is optional, and could be `-> Vec<u8>` instead of `-> Result<Vec<u8>, E>`.
    // (Any nontrivial value should implement this.)
    // Requiring `opts` is optional.
    type E;
    fn to_bytes(&self, opts: ValueToBytesOptions) -> Result<Vec<u8>, E>;
}
*/
// Exceptions (which don't follow the above pattern) are
// `ChunkPosition`, `Dimension`, `DimensionedChunkPos`,
// and structs in `palettized_storage`.


/// For use during development. Instead of printing binary data as entirely binary,
/// stretches of ASCII alphanumeric characters (plus `.`, `-`, `_`) are printed as text,
/// with binary data interspersed.
///
/// For example:
/// `various_text-characters[0,1,2,3,]more_text[255,255,]`
fn print_debug(value: &[u8]) {
    #![allow(dead_code)]
    #![allow(clippy::all)]
    // Apparently this wasn't covered.
    #![expect(clippy::cast_lossless)]

    let mut nums = value.iter().peekable();

    while nums.peek().is_some() {
        while let Some(&&num) = nums.peek() {
            if let Some(ch) = char::from_u32(num as u32) {
                if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
                    nums.next();
                    print!("{ch}");
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        print!("[");
        while let Some(&&num) = nums.peek() {
            if let Some(ch) = char::from_u32(num as u32) {
                if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
                    break;
                }
            }
            nums.next();
            print!("{num},");
        }
        print!("]");
    }
    println!();
}
