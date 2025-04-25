#[cfg(feature = "dimensions")]
pub mod dimensions;
#[cfg(feature = "chunk_position")]
pub mod chunk_position;
#[cfg(feature = "uuid")]
pub mod uuid;

#[cfg(feature = "concatenated_nbt_compounds")]
pub mod concatenated_nbt_compounds; // For multiple sorts of values
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
#[cfg(feature = "autonomous_entities")]
pub mod autonomous_entities;
#[cfg(feature = "player")]
pub mod player; // Does this include PlayerServer? No clue!
#[cfg(feature = "village")]
pub mod village;
#[cfg(feature = "map")]
pub mod map;
#[cfg(feature = "portals")]
pub mod portals;
#[cfg(feature = "structure_template")]
pub mod structure_template;
#[cfg(feature = "ticking_area")]
pub mod ticking_area;
#[cfg(feature = "scoreboard")]
pub mod scoreboard;
#[cfg(feature = "wandering_trader_scheduler")]
pub mod wandering_trader_scheduler;
#[cfg(feature = "biome_data")]
pub mod biome_data;
#[cfg(feature = "mob_events")]
pub mod mob_events;
#[cfg(feature = "overworld")]
pub mod overworld;
#[cfg(feature = "nether")]
pub mod nether;
#[cfg(feature = "the_end")]
pub mod the_end;
#[cfg(feature = "position_tracking")]
pub mod position_tracking;
#[cfg(feature = "flat_world_layers")]
pub mod flat_world_layers;

/// Compare a reader's position to the total length of data that was expected to be read,
/// to check if everything was read.
#[cfg(any(
    feature = "data_3d",
    feature = "concatenated_nbt_compounds",
    feature = "metadata",
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

/// Map an enum into and from two other types using `From` and `TryFrom` (with unit error).
///
/// The two other types should be similar enough that the same value expression works for either;
/// in practice, they should usually be the same, but it is useful for converting
/// into `&'static str` and trying to convert from `&str`, for example.
///
/// # Examples:
/// ```
/// # use prismarine_anchor_leveldb_values::bijective_enum_map;
///
/// #[derive(Debug, PartialEq, Eq)]
/// enum AtMostTwo {
///     Zero,
///     One,
///     Two,
/// }
///
/// #[derive(Debug, PartialEq, Eq)]
/// enum Empty {}
///
/// bijective_enum_map! {
///     AtMostTwo, u8, u8,
///     Zero <=> 0,
///     One  <=> 1,
///     Two  <=> 2,
/// }
///
/// bijective_enum_map! {
///     Empty, &'static str, &str,
/// }
///
/// assert_eq!(u8::from(AtMostTwo::One), 1u8);
/// assert_eq!(AtMostTwo::try_from(2u8), Ok(AtMostTwo::Two));
/// assert_eq!(AtMostTwo::try_from(4u8), Err(()));
/// assert_eq!(Empty::try_from("42"), Err(()))
/// ```
#[cfg(any(
    doc,
    feature = "actor_digest_version",
    feature = "metadata",
    feature = "chunk_version",
))]
#[macro_export]
macro_rules! bijective_enum_map {
    { $enum_name:ty, $into:ty, $try_from:ty, $($enum_variant:ident <=> $value:expr),+ $(,)? } => {
        impl From<$enum_name> for $into {
            #[inline]
            fn from(value: $enum_name) -> Self {
                match value {
                    $( <$enum_name>::$enum_variant => $value ),+
                }
            }
        }
        impl TryFrom<$try_from> for $enum_name {
            type Error = ();

            #[inline]
            fn try_from(value: $try_from) -> Result<Self, Self::Error> {
                Ok(match value {
                    $( $value => Self::$enum_variant ),+,
                    _ => return Err(())
                })
            }
        }
    };

    { $enum_name:ty, $into:ty, $try_from:ty, $($enum_variant:ident : $value:expr),+ $(,)? } => {
        bijective_map_enum!($enum_name:ty, $into:ty, $try_from:ty, $($enum_variant <=> $value),+)
    };

    { $enum_name:ty, $into:ty, $try_from:ty $(,)? } => {
        impl From<$enum_name> for $into {
            #[inline]
            fn from(value: $enum_name) -> Self {
                match value {}
            }
        }
        impl TryFrom<$try_from> for $enum_name {
            type Error = ();

            #[inline]
            fn try_from(_value: $try_from) -> Result<Self, Self::Error> {
                Err(())
            }
        }
    };
}

/// For use during development. Instead of printing binary data as entirely binary,
/// stretches of ASCII alphanumeric characters (plus `.`, `-`, `_`) are printed as text,
/// with binary data interspersed.
///
/// For example:
/// `various_text-characters[0, 1, 2, 3,]more_text[255, 255]`
#[allow(dead_code)]
fn print_debug(value: &[u8]) {
    let mut nums = value.iter().peekable();

    while let Some(_) = nums.peek() {
        while let Some(&&num) = nums.peek() {
            if let Some(ch) = char::from_u32(num as u32) {
                if ch.is_ascii_alphanumeric()
                    || ch == '.' || ch == '-' || ch == '_'
                {
                    nums.next();
                    print!("{ch}");
                } else {
                    break;
                }
            } else {
                break
            }
        }
        print!("[");
        while let Some(&&num) = nums.peek() {
            if let Some(ch) = char::from_u32(num as u32) {
                if ch.is_ascii_alphanumeric()
                    || ch == '.' || ch == '-' || ch == '_'
                {
                    break;
                }
            }
            nums.next();
            print!("{num},");
        }
        print!("]");
    }
    println!("")
}
