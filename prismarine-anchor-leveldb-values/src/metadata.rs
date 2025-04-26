use std::io;
use std::{collections::BTreeMap, ops::Range};
use std::io::{Cursor, Read as _, Write as _};

use indexmap::IndexMap;
use thiserror::Error;
use xxhash_rust::xxh64;

use prismarine_anchor_nbt::{NbtCompound, NbtTag};
use prismarine_anchor_nbt::{
    io::{NbtIoError, read_compound, write_compound},
    settings::{Endianness, IoOptions},
};

use crate::bijective_enum_map;
use crate::{all_read, dimensions::NamedDimension};


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
            let (nbt, _) = read_compound(&mut reader, IoOptions::bedrock_uncompressed())?;
            let metadata = MetaData::from(nbt);

            // Check that the hash is correct
            let computed_hash = metadata.clone().xxhash64()?;

            if hash != computed_hash {
                return Err(MetaDataParseError::IncorrectHash {
                    computed: computed_hash,
                    received: hash,
                });
            }

            // println!("Checking for duplicate...");
            // Reject if there's a duplicate hash
            if map.insert(hash, metadata).is_some() {
                return Err(MetaDataParseError::DuplicateHash(hash));
            }
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
                        return Err(MetaDataWriteError::DictionaryLength);
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

    pub fn to_bytes(
        &self,
        error_on_excessive_length: bool,
    ) -> Result<Vec<u8>, MetaDataWriteError> {
        let (len, len_usize) = self.len(error_on_excessive_length)?;

        let mut writer = Cursor::new(Vec::new());
        writer
            .write_all(&len.to_le_bytes())
            .expect("Cursor IO doesn't fail");

        for (hash, nbt) in self.0.iter().take(len_usize) {
            let nbt = nbt.clone().into();
            writer
                .write_all(&hash.to_le_bytes())
                .expect("Cursor IO doesn't fail");

            // Could only fail on invalid NBT.
            write_compound(&mut writer, IoOptions::bedrock_uncompressed(), None, &nbt)?;
        }

        Ok(writer.into_inner())
    }

    pub fn into_bytes(
        self,
        error_on_excessive_length: bool,
    ) -> Result<Vec<u8>, MetaDataWriteError> {
        let (len, len_usize) = self.len(error_on_excessive_length)?;

        let mut writer = Cursor::new(Vec::new());
        writer
            .write_all(&len.to_le_bytes())
            .expect("Cursor IO doesn't fail");

        for (hash, nbt) in self.0.into_iter().take(len_usize) {
            let nbt = nbt.into();
            writer
                .write_all(&hash.to_le_bytes())
                .expect("Cursor IO doesn't fail");

            // Could only fail on invalid NBT.
            write_compound(&mut writer, IoOptions::bedrock_uncompressed(), None, &nbt)?;
        }

        Ok(writer.into_inner())
    }
}

impl Default for LevelChunkMetaDataDictionary {
    fn default() -> Self {
        Self::new()
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
        write_compound(&mut writer, network_little_endian, None, &nbt)?;

        Ok(xxh64::xxh64(&writer.into_inner(), 0))
    }
}

impl From<NbtCompound> for MetaData {
    #[expect(
        clippy::too_many_lines,
        reason = "it's well-organized into helper macros",
    )]
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
                    None => None,
                }
            };
        }

        macro_rules! try_from_take_meta {
            ($meta:ident, $tag:ident) => {
                take_meta!($meta, $tag).and_then(|tag_inner| match $meta::try_from(tag_inner) {
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
            ($meta:ident) => {{
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
            }};
        }

        macro_rules! take_range {
            ($meta:ident) => {{
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
            }};
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
            ($val:expr, $meta:ident, $tag:ident $(,)?) => {
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
            DimensionName, String,
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
        Self::from_iter(compound)
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

bijective_enum_map! {
    MetaDataType, u8, u8,
    LastSavedBaseGameVersion        <=> 0,
    OriginalBaseGameVersion         <=> 1,
    BiomeBaseGameVersion            <=> 2,
    DimensionName                   <=> 3,
    GenerationSeed                  <=> 4,
    GeneratorType                   <=> 5,
    WorldGen1_18AppliedBelow0       <=> 6,
    Overworld1_18HeightExtended     <=> 7,
    BlendingVersion                 <=> 8,
    OriginalDimensionHeightRange    <=> 9,
    LastSavedDimensionHeightRange   <=> 10,
    UnderwaterLavaLakeFixed         <=> 11,
    WorldGenBelowZeroFixed          <=> 12,
    SkullFlatteningPerformed        <=> 13,
}

bijective_enum_map! {
    MetaDataType, &'static str, &str,
    LastSavedBaseGameVersion        <=> "LastSavedBaseGameVersion",
    OriginalBaseGameVersion         <=> "OriginalBaseGameVersion",
    BiomeBaseGameVersion            <=> "BiomeBaseGameVersion",
    DimensionName                   <=> "DimensionName",
    GenerationSeed                  <=> "GenerationSeed",
    GeneratorType                   <=> "GeneratorType",
    WorldGen1_18AppliedBelow0       <=> "WorldGen1_18AppliedBelow0",
    Overworld1_18HeightExtended     <=> "Overworld1_18HeightExtended",
    BlendingVersion                 <=> "BlendingVersion",
    OriginalDimensionHeightRange    <=> "OriginalDimensionHeightRange",
    LastSavedDimensionHeightRange   <=> "LastSavedDimensionHeightRange",
    UnderwaterLavaLakeFixed         <=> "UnderwaterLavaLakeFixed",
    WorldGenBelowZeroFixed          <=> "WorldGenBelowZeroFixed",
    SkullFlatteningPerformed        <=> "SkullFlatteningPerformed",
}

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

bijective_enum_map! {
    BlendingVersion, i16, i16,
    V1_19_0   <=> 0,
    V1_19_0_1 <=> 1,
    V1_19_0_2 <=> 2,
    V1_19_0_3 <=> 3,
    V1_20_0   <=> 4,
    V1_20_0_1 <=> 5,
    V1_21_50  <=> 6,
    V1_21_60  <=> 7,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeneratorType {
    Old,
    Infinite,
    Flat,
}

bijective_enum_map! {
    GeneratorType, i32, i32,
    Old      <=> 0,
    Infinite <=> 1,
    Flat     <=> 2,
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
    IncorrectHash { computed: u64, received: u64 },
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
    NbtError(#[from] NbtIoError),
}
