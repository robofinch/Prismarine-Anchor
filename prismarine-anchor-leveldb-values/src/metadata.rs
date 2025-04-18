use std::io;
use std::{collections::BTreeMap, ops::Range};
use std::io::{Cursor, Read, Write};

use indexmap::IndexMap;
use thiserror::Error;
use xxhash_rust::xxh64;

use prismarine_anchor_nbt::{NbtCompound, NbtTag};
use prismarine_anchor_nbt::{
    io::{read_nbt, write_nbt, NbtIoError},
    settings::{Endianness, IoOptions},
};

use crate::dimensions::NamedDimension;


#[derive(Debug, Clone)]
pub struct LevelChunkMetaDataDictionary(IndexMap<u64, MetaData>);

impl LevelChunkMetaDataDictionary {
    #[inline]
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    #[inline]
    pub fn get(&self, metadata_hash: u64) -> Option<&MetaData> {
        self.0.get(&metadata_hash)
    }

    /// Inserts the provided metadata into the dictionary, and returns the hash key used.
    /// If an error occurs, the dictionary is not modified.
    #[inline]
    pub fn insert(&mut self, metadata: MetaData) -> Result<u64, MetaDataHashError> {
        let hash = metadata.clone().xxhash64()?;
        self.0.insert(hash, metadata);
        Ok(hash)
    }

    #[inline]
    pub fn contains_hash(&self, metadata_hash: u64) -> bool {
        self.0.contains_key(&metadata_hash)
    }

    #[inline]
    pub fn contains_metadata(&self, metadata: MetaData) -> Result<bool, MetaDataHashError> {
        let hash = metadata.xxhash64()?;
        Ok(self.0.contains_key(&hash))
    }

    pub fn parse(value: &[u8]) -> Result<Self, MetaDataParseError> {

        if value.len() < 4 {
            return Err(MetaDataParseError::NoHeader);
        }

        let num_entries = u32::from_le_bytes(value[0..4].try_into().unwrap());

        let mut reader = Cursor::new(&value[4..]);
        let mut map = IndexMap::new();

        // Read each of the dictionary's entries
        for _ in 0..num_entries {

            // The hash is the key
            let mut hash = [0; 8];
            reader.read_exact(&mut hash)?;
            let hash = u64::from_le_bytes(hash);

            // MetaData stored as an NBT is the value
            let (nbt, _) = read_nbt(
                &mut reader,
                IoOptions::bedrock_uncompressed(),
            )?;
            let metadata = MetaData::from(nbt);

            // Check that the hash is correct
            let computed_hash = metadata.clone().xxhash64()?;

            if hash != computed_hash {
                return Err(MetaDataParseError::IncorrectHash {
                    computed: computed_hash,
                    received: hash,
                })
            }

            // println!("Checking for duplicate...");
            // Reject if there's a duplicate hash
            let None = map.insert(hash, metadata) else {
                return Err(MetaDataParseError::DuplicateHash(hash))
            };

        }

        // println!("Checking for excess...");
        // Reject if there was excess data
        if !all_read(reader.position(), reader.into_inner().len()) {
            return Err(MetaDataParseError::ExcessData);
        }

        Ok(Self(map))
    }

    fn len(&self, error_on_excessive_length: bool) -> Result<(u32, usize), MetaDataWriteError> {
        if size_of::<usize>() >= size_of::<u32>() {

            let len = match u32::try_from(self.0.len()) {
                Ok(len) => len,
                Err(_) => {
                    if error_on_excessive_length {
                        return Err(MetaDataWriteError::DictionaryLength)
                    } else {
                        u32::MAX
                    }
                }
            };

            // This cast from u32 to usize won't overflow
            Ok((len, len as usize))
        } else {
            // This cast from usize to u32 won't overflow
            Ok((self.0.len() as u32, self.0.len()))
        }
    }

    pub fn to_bytes(&self, error_on_excessive_length: bool) -> Result<Vec<u8>, MetaDataWriteError> {
        let (len, len_usize) = self.len(error_on_excessive_length)?;

        let mut writer = Cursor::new(Vec::new());
        writer.write_all(&len.to_le_bytes()).expect("Cursor IO doesn't fail");

        for (hash, nbt) in self.0.iter().take(len_usize) {
            writer.write_all(&hash.to_le_bytes()).expect("Cursor IO doesn't fail");

            // Could only fail on invalid NBT.
            write_nbt(&mut writer, IoOptions::bedrock_uncompressed(), None, &nbt.clone().into())?;
        }

        Ok(writer.into_inner())
    }

    pub fn into_bytes(self, error_on_excessive_length: bool) -> Result<Vec<u8>, MetaDataWriteError> {
        let (len, len_usize) = self.len(error_on_excessive_length)?;

        let mut writer = Cursor::new(Vec::new());
        writer.write_all(&len.to_le_bytes()).expect("Cursor IO doesn't fail");

        for (hash, nbt) in self.0.into_iter().take(len_usize) {
            writer.write_all(&hash.to_le_bytes()).expect("Cursor IO doesn't fail");

            // Could only fail on invalid NBT.
            write_nbt(&mut writer, IoOptions::bedrock_uncompressed(), None, &nbt.into())?;
        }

        Ok(writer.into_inner())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetaData {
    pub last_saved_base_game_version:       Option<String>,
    pub original_base_game_version:         Option<String>,
    pub biome_base_game_version:            Option<String>,
    pub dimension_name:                     Option<NamedDimension>,
    pub generation_seed:                    Option<u64>,
    pub generator_type:                     Option<GeneratorType>,
    pub world_gen_1_18_applied_below_0:     Option<bool>,
    pub overworld_1_18_height_extended:     Option<bool>,
    pub blending_version:                   Option<BlendingVersion>,
    pub original_dimension_height_range:    Option<Range<i16>>,
    pub last_saved_dimension_height_range:  Option<Range<i16>>,
    pub underwater_lava_lake_fixed:         Option<bool>,
    pub world_gen_below_zero_fixed:         Option<bool>,
    pub skull_flattening_performed:         Option<bool>,
    pub unrecognized:                       BTreeMap<String, NbtTag>,
}

impl MetaData {
    pub fn xxhash64(self) -> Result<u64, MetaDataHashError> {
        let nbt = NbtCompound::from(self);
        let network_little_endian = IoOptions {
            endianness: Endianness::NetworkLittleEndian,
            ..IoOptions::bedrock_uncompressed()
        };

        let mut writer = Cursor::new(Vec::new());
        write_nbt(&mut writer, network_little_endian, None,&nbt)?;

        Ok(xxh64::xxh64(&writer.into_inner(), 0))
    }
}

impl From<NbtCompound> for MetaData {
    fn from(mut value: NbtCompound) -> Self {

        let mut unrecognized: BTreeMap<String, NbtTag> = BTreeMap::new();

        fn parse_range(range: &NbtCompound) -> Option<(i16, i16)> {
            if range.len() != 2 {
                return None;
            }

            let Some(&NbtTag::Short(max)) = range.get_tag("max") else {
                return None;
            };
            let Some(&NbtTag::Short(min)) = range.get_tag("min") else {
                return None;
            };

            Some((min, max))
        }

        macro_rules! take_meta {
            ($meta:ident, $tag:ident) => {
                match value.remove_tag(<&str>::from(MetaDataType::$meta)) {
                    Some(NbtTag::$tag(inner)) => Some(inner),
                    Some(other_tag) => {
                        unrecognized.insert(
                            <&str>::from(MetaDataType::$meta).to_owned(),
                            other_tag,
                        );
                        None
                    }
                    None => None
                }
            };
        }

        macro_rules! try_from_take_meta {
            ($meta:ident, $tag:ident) => {
                take_meta!($meta, $tag)
                    .and_then(|tag_inner| match $meta::try_from(tag_inner) {
                        Ok(meta) => Some(meta),
                        Err(_) => {
                            unrecognized.insert(
                                <&str>::from(MetaDataType::$meta).to_owned(),
                                NbtTag::$tag(tag_inner),
                            );
                            None
                        }
                    })
            };
        }

        macro_rules! take_short_flag {
            ($meta:ident) => {
                {
                    let flag = take_meta!($meta, Short);
                    if let Some(flag) = flag {
                        if flag == 0 {
                            Some(false)
                        } else if flag == 1 {
                            Some(true)
                        } else {
                            unrecognized.insert(
                                <&str>::from(MetaDataType::$meta).to_owned(),
                                NbtTag::Short(flag),
                            );
                            None
                        }
                    } else {
                        None
                    }
                }
            };
        }

        macro_rules! take_range {
            ($meta:ident) => {
                {
                    let range = take_meta!($meta, Compound);
                    if let Some(compound) = range {
                        if let Some((min, max)) = parse_range(&compound) {
                            Some(min..max)
                        } else {
                            unrecognized.insert(
                                <&str>::from(MetaDataType::$meta).to_owned(),
                                NbtTag::Compound(compound),
                            );
                            None
                        }
                    } else {
                        None
                    }
                }
            };
        }

        let last_saved_base_game_version        = take_meta!(LastSavedBaseGameVersion, String);
        let original_base_game_version          = take_meta!(OriginalBaseGameVersion, String);
        let biome_base_game_version             = take_meta!(BiomeBaseGameVersion, String);
        let dimension_name                      = take_meta!(DimensionName, String)
            .map(|name| NamedDimension::from_bedrock_name(&name));
        let generation_seed                     = take_meta!(GenerationSeed, Long)
            .map(|seed| seed as u64);
        let generator_type                      = try_from_take_meta!(GeneratorType, Int);
        let world_gen_1_18_applied_below_0      = take_short_flag!(WorldGen1_18AppliedBelow0);
        let overworld_1_18_height_extended      = take_short_flag!(Overworld1_18HeightExtended);
        let blending_version                    = try_from_take_meta!(BlendingVersion, Short);
        let original_dimension_height_range     = take_range!(OriginalDimensionHeightRange);
        let last_saved_dimension_height_range   = take_range!(LastSavedDimensionHeightRange);
        let underwater_lava_lake_fixed          = take_short_flag!(UnderwaterLavaLakeFixed);
        let world_gen_below_zero_fixed          = take_short_flag!(WorldGenBelowZeroFixed);
        let skull_flattening_performed          = take_short_flag!(SkullFlatteningPerformed);

        Self {
            last_saved_base_game_version,
            original_base_game_version,
            biome_base_game_version,
            dimension_name,
            generation_seed,
            generator_type,
            world_gen_1_18_applied_below_0,
            overworld_1_18_height_extended,
            blending_version,
            original_dimension_height_range,
            last_saved_dimension_height_range,
            underwater_lava_lake_fixed,
            world_gen_below_zero_fixed,
            skull_flattening_performed,
            unrecognized,
        }
    }
}

impl From<MetaData> for NbtCompound {
    fn from(value: MetaData) -> Self {

        let mut compound = value.unrecognized;

        macro_rules! add_meta {
            ($val:expr, $meta:ident, $tag:ident) => {
                if let Some(tag_inner) = $val {
                    compound.insert(
                        <&str>::from(MetaDataType::$meta).to_owned(),
                        NbtTag::$tag(tag_inner),
                    );
                }
            };
        }

        macro_rules! add_meta_from {
            ($val:expr, $meta:ident, $tag:ident) => {
                if let Some(tag_inner) = $val {
                    compound.insert(
                        <&str>::from(MetaDataType::$meta).to_owned(),
                        NbtTag::$tag(tag_inner.into()),
                    );
                }
            };
        }

        macro_rules! add_short_flag {
            ($val:expr, $meta:ident) => {
                if let Some(flag) = $val {
                    compound.insert(
                        <&str>::from(MetaDataType::$meta).to_owned(),
                        NbtTag::Short(if flag { 1 } else { 0 }),
                    );
                }
            };
        }

        // Note: it is vital that "max" is inserted before "min", to be alphabetical.
        macro_rules! add_range {
            ($val:expr, $meta:ident) => {
                if let Some(range) = $val {
                    let mut range_compound = NbtCompound::new();
                    range_compound.insert("max".to_owned(), range.end);
                    range_compound.insert("min".to_owned(), range.start);
                    compound.insert(
                        <&str>::from(MetaDataType::$meta).to_owned(),
                        NbtTag::Compound(range_compound),
                    );
                }
            };
        }

        add_meta!(value.last_saved_base_game_version, LastSavedBaseGameVersion, String);
        add_meta!(value.original_base_game_version, OriginalBaseGameVersion, String);
        add_meta!(value.biome_base_game_version, BiomeBaseGameVersion, String);
        add_meta!(
            value.dimension_name.map(|dimension| dimension.as_bedrock_name().to_owned()),
            DimensionName, String
        );
        add_meta!(value.generation_seed.map(|seed| seed as i64), GenerationSeed, Long);
        add_meta_from!(value.generator_type, GeneratorType, Int);
        add_short_flag!(value.world_gen_1_18_applied_below_0, WorldGen1_18AppliedBelow0);
        add_short_flag!(value.overworld_1_18_height_extended, Overworld1_18HeightExtended);
        add_meta_from!(value.blending_version, BlendingVersion, Short);
        add_range!(value.original_dimension_height_range, OriginalDimensionHeightRange);
        add_range!(value.last_saved_dimension_height_range, LastSavedDimensionHeightRange);
        add_short_flag!(value.underwater_lava_lake_fixed, UnderwaterLavaLakeFixed);
        add_short_flag!(value.world_gen_below_zero_fixed, WorldGenBelowZeroFixed);
        add_short_flag!(value.skull_flattening_performed, SkullFlatteningPerformed);

        // preserve_order is enabled in order to ensure that the compound remains sorted.
        NbtCompound::from_iter(compound.into_iter())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MetaDataType {
    LastSavedBaseGameVersion,
    OriginalBaseGameVersion,
    BiomeBaseGameVersion,
    DimensionName,
    GenerationSeed,
    GeneratorType,
    WorldGen1_18AppliedBelow0,
    Overworld1_18HeightExtended,
    BlendingVersion,
    OriginalDimensionHeightRange,
    LastSavedDimensionHeightRange,
    UnderwaterLavaLakeFixed,
    WorldGenBelowZeroFixed,
    SkullFlatteningPerformed,
}

// Two-way map with From and TryFrom, for mapping MetaDataTypes to and from u8's and strings.
macro_rules! map_meta_data_type {
    (
        $other_type:ty, $similar_type:ty,
        $zero:expr, $one:expr, $two:expr, $three:expr, $four:expr, $five:expr, $six:expr,
        $seven:expr, $eight:expr, $nine:expr, $ten:expr, $eleven:expr, $twelve:expr, $thirteen:expr,
    ) => {
        impl From<MetaDataType> for $other_type {
            #[inline]
            fn from(value: MetaDataType) -> Self {
                match value {
                    MetaDataType::LastSavedBaseGameVersion        => $zero,
                    MetaDataType::OriginalBaseGameVersion         => $one,
                    MetaDataType::BiomeBaseGameVersion            => $two,
                    MetaDataType::DimensionName                   => $three,
                    MetaDataType::GenerationSeed                  => $four,
                    MetaDataType::GeneratorType                   => $five,
                    MetaDataType::WorldGen1_18AppliedBelow0       => $six,
                    MetaDataType::Overworld1_18HeightExtended     => $seven,
                    MetaDataType::BlendingVersion                 => $eight,
                    MetaDataType::OriginalDimensionHeightRange    => $nine,
                    MetaDataType::LastSavedDimensionHeightRange   => $ten,
                    MetaDataType::UnderwaterLavaLakeFixed         => $eleven,
                    MetaDataType::WorldGenBelowZeroFixed          => $twelve,
                    MetaDataType::SkullFlatteningPerformed        => $thirteen,
                }
            }
        }

        impl TryFrom<$similar_type> for MetaDataType {
            type Error = ();

            #[inline]
            fn try_from(value: $similar_type) -> Result<Self, Self::Error> {
                Ok(match value {
                    $zero     => Self::LastSavedBaseGameVersion,
                    $one      => Self::OriginalBaseGameVersion,
                    $two      => Self::BiomeBaseGameVersion,
                    $three    => Self::DimensionName,
                    $four     => Self::GenerationSeed,
                    $five     => Self::GeneratorType,
                    $six      => Self::WorldGen1_18AppliedBelow0,
                    $seven    => Self::Overworld1_18HeightExtended,
                    $eight    => Self::BlendingVersion,
                    $nine     => Self::OriginalDimensionHeightRange,
                    $ten      => Self::LastSavedDimensionHeightRange,
                    $eleven   => Self::UnderwaterLavaLakeFixed,
                    $twelve   => Self::WorldGenBelowZeroFixed,
                    $thirteen => Self::SkullFlatteningPerformed,
                    _ => return Err(()),
                })
            }
        }
    };
}

// Map the MetaDataTypes to and from u8's and strings.
map_meta_data_type!(u8, u8, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13,);
map_meta_data_type!(
    &'static str, &str,
    "LastSavedBaseGameVersion",
    "OriginalBaseGameVersion",
    "BiomeBaseGameVersion",
    "DimensionName",
    "GenerationSeed",
    "GeneratorType",
    "WorldGen1_18AppliedBelow0",
    "Overworld1_18HeightExtended",
    "BlendingVersion",
    "OriginalDimensionHeightRange",
    "LastSavedDimensionHeightRange",
    "UnderwaterLavaLakeFixed",
    "WorldGenBelowZeroFixed",
    "SkullFlatteningPerformed",
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlendingVersion {
    V1_19_0,
    V1_19_0_1,
    V1_19_0_2,
    V1_19_0_3,
    V1_20_0,
    V1_20_0_1,
    V1_21_50,
    V1_21_60,
}

impl From<BlendingVersion> for i16 {
    #[inline]
    fn from(value: BlendingVersion) -> Self {
        match value {
            BlendingVersion::V1_19_0   => 0,
            BlendingVersion::V1_19_0_1 => 1,
            BlendingVersion::V1_19_0_2 => 2,
            BlendingVersion::V1_19_0_3 => 3,
            BlendingVersion::V1_20_0   => 4,
            BlendingVersion::V1_20_0_1 => 5,
            BlendingVersion::V1_21_50  => 6,
            BlendingVersion::V1_21_60  => 7,
        }
    }
}

impl TryFrom<i16> for BlendingVersion {
    type Error = ();

    #[inline]
    fn try_from(value: i16) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => BlendingVersion::V1_19_0,
            1 => BlendingVersion::V1_19_0_1,
            2 => BlendingVersion::V1_19_0_2,
            3 => BlendingVersion::V1_19_0_3,
            4 => BlendingVersion::V1_20_0,
            5 => BlendingVersion::V1_20_0_1,
            6 => BlendingVersion::V1_21_50,
            7 => BlendingVersion::V1_21_60,
            _ => return Err(())
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeneratorType {
    Old,
    Infinite,
    Flat,
}

impl From<GeneratorType> for i32 {
    #[inline]
    fn from(value: GeneratorType) -> Self {
        match value {
            GeneratorType::Old      => 0,
            GeneratorType::Infinite => 1,
            GeneratorType::Flat     => 2,
        }
    }
}

impl TryFrom<i32> for GeneratorType {
    type Error = ();

    #[inline]
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => GeneratorType::Old,
            1 => GeneratorType::Infinite,
            2 => GeneratorType::Flat,
            _ => return Err(()),
        })
    }
}

#[derive(Error, Debug)]
pub enum MetaDataParseError {
    #[error("the metadata dictionary was shorter than the required 4 byte header")]
    NoHeader,
    #[error("the number of metadata entries could not fit in a u32")]
    DictionaryLength,
    #[error("the hash value {0} appeared twice in a metadata dictionary")]
    DuplicateHash(u64),
    #[error("all entries of a metadata dictionary were parsed, but excess data was provided")]
    ExcessData,
    #[error(
        "a metadata entry with hash key {} was received, but its hash was computed as {}",
        received, computed,
    )]
    IncorrectHash {
        computed: u64,
        received: u64,
    },
    #[error(transparent)]
    HashError(#[from] MetaDataHashError),
    #[error("NBT error while parsing metadata dictionary: {0}")]
    NbtError(#[from] NbtIoError),
    #[error("IO error while parsing metadata dictionary: {0}")]
    IoError(#[from] io::Error),
}

#[derive(Error, Debug)]
pub enum MetaDataWriteError {
    #[error("the number of metadata entries could not fit in a u32")]
    DictionaryLength,
    #[error("NBT error while writing metadata dictionary to NBT: {0}")]
    NbtError(#[from] NbtIoError),
}

#[derive(Error, Debug)]
pub enum MetaDataHashError {
    #[error("error while writing metadata to NBT to compute its xxhash64 hash: {0}")]
    NbtError(#[from] NbtIoError)
}

/// Compare a reader's position to the total length of data that was expected to be read,
/// to check if everything was read.
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
