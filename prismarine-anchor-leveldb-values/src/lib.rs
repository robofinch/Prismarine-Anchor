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


/// Compare a reader's position to the total length of data that was expected to be read,
/// to check if everything was read.
#[cfg(any(
    feature = "concatenated_nbt_compounds",
    feature = "data_3d",
    feature = "metadata",
    feature = "subchunk_blocks",
))]
#[inline]
fn all_read(read_position: u64, total_len: usize) -> bool {
    // The as casts don't overflow because we check the size.
    if size_of::<usize>() <= size_of::<u64>() {
        let total_len = total_len as u64;
        read_position == total_len

    } else {
        let read_len = read_position as usize;
        read_len == total_len
    }
}

/// For use during development. Instead of printing binary data as entirely binary,
/// stretches of ASCII alphanumeric characters (plus `.`, `-`, `_`) are printed as text,
/// with binary data interspersed.
///
/// For example:
/// `various_text-characters[0, 1, 2, 3,]more_text[255, 255]`
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
