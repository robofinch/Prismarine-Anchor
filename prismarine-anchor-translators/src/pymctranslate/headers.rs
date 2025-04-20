use std::collections::{HashMap, HashSet};

use serde::{Serialize, Deserialize};

use prismarine_anchor_translation::datatypes::{MINECRAFT_NAMESPACE, GameVersion, VersionName};

use super::{MappingParseError, MappingParseOptions, NamespacedIdentifier};


// ================================================================
//  Final data structures
// ================================================================

pub struct VersionMetadata {
    pub block_format:               BlockFormat,
    pub block_entity_format:        BlockEntityFormat,
    pub block_entity_coord_format:  BlockEntityCoordFormat,
    pub entity_format:              EntityFormat,
    pub entity_coord_format:        EntityCoordFormat,
    pub data_version:               Option<u64>,
    pub version:                    GameVersion,
}

pub struct BiomeMap {
    pub biome_to_number: HashMap<NamespacedIdentifier, Option<u16>>,
    pub number_to_biome: HashMap<u16, NamespacedIdentifier>,
    /// Plains if present, else 0
    pub default_biome_number: u16,
    pub to_universal:    HashMap<NamespacedIdentifier, NamespacedIdentifier>,
    pub from_universal:  HashMap<NamespacedIdentifier, NamespacedIdentifier>,
}

pub struct NumericalBlockMap {
    pub to_number:     HashMap<NamespacedIdentifier, u16>,
    pub to_identifier: Vec<Option<NamespacedIdentifier>>,
    /// Air if present, else 0
    pub default_block_number: u16,
}

pub struct WaterloggingInfo {
    pub waterloggable:      HashSet<NamespacedIdentifier>,
    pub always_waterlogged: HashSet<NamespacedIdentifier>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockFormat {
    #[serde(rename = "numerical")]
    Numerical,
    #[serde(rename = "pseudo-numerical")]
    PseudoNumerical,
    #[serde(rename = "blockstate")]
    Blockstate,
    #[serde(rename = "nbt-blockstate")]
    NbtBlockstate,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockEntityFormat {
    #[serde(rename = "namespace-str-id")]
    NamespaceStrId,
    #[serde(rename = "str-id")]
    StrId,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockEntityCoordFormat {
    #[serde(rename = "xyz-int")]
    IntXYZ,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityFormat {
    #[serde(rename = "namespace-str-id")]
    NamespaceStrId,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityCoordFormat {
    #[serde(rename = "Pos-list-float")]
    PosListFloat
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    #[serde(rename = "bedrock")]
    Bedrock,
    #[serde(rename = "java")]
    Java,
    #[serde(rename = "universal")]
    Universal,
}

// ================================================================
//  JSON parsing
// ================================================================

#[derive(Serialize, Deserialize, Debug)]
struct InitJson {
    block_format:               BlockFormat,
    block_entity_format:        BlockEntityFormat,
    block_entity_coord_format:  BlockEntityCoordFormat,
    entity_format:              EntityFormat,
    entity_coord_format:        EntityCoordFormat,
    platform:                   Platform,
    version:                    Vec<u32>,
    #[serde(rename = "version_max")]
    _version_max:               Vec<i32>,
    data_version:               Option<u64>,
    #[serde(rename = "data_version_max")]
    _data_version_max:          Option<u64>,
}

impl VersionMetadata {
    pub fn from_json(json: &str, _opts: MappingParseOptions) -> Result<Self, MappingParseError> {
        let init_json: InitJson = serde_json::from_str(json)?;

        let mut version = init_json.version.into_iter();
        let major = version.next().unwrap_or(0);
        let minor = version.next().unwrap_or(0);
        let patch = version.next().unwrap_or(0);

        let version_name = VersionName::numeric(major, minor, patch);

        let game_version = match init_json.platform {
            Platform::Universal => GameVersion::Universal,
            Platform::Bedrock   => GameVersion::Bedrock(version_name),
            Platform::Java      => GameVersion::Java(version_name),
        };

        Ok(Self {
            block_format:               init_json.block_format,
            block_entity_format:        init_json.block_entity_format,
            block_entity_coord_format:  init_json.block_entity_coord_format,
            entity_format:              init_json.entity_format,
            entity_coord_format:        init_json.entity_coord_format,
            data_version:               init_json.data_version,
            version:                    game_version,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct BiomeMapJson {
    int_map: HashMap<String, Option<u16>>,
    #[serde(rename = "version2universal")]
    to_universal: HashMap<String, String>,
    #[serde(rename = "universal2version")]
    from_universal: HashMap<String, String>
}

impl BiomeMap {
    pub fn from_json(json: &str, opts: MappingParseOptions) -> Result<Self, MappingParseError> {
        let json: BiomeMapJson = serde_json::from_str(json)?;

        let id_opts = opts.identifier_options;

        let biome_to_number: HashMap<NamespacedIdentifier, Option<u16>> = json.int_map.into_iter()
            .map(|(key, value)| {
                Ok((
                    NamespacedIdentifier::parse_string(key, id_opts)?,
                    value,
                ))
            })
            .collect::<Result<_, MappingParseError>>()?;

        let number_to_biome = biome_to_number.iter().filter_map(|(key, value)| {
            Some((value.clone()?, key.clone()))
        }).collect();

        let plains = NamespacedIdentifier {
            namespace: MINECRAFT_NAMESPACE.into(),
            path: "plains".into(),
        };

        let default_biome_number = biome_to_number.get(&plains).cloned().flatten().unwrap_or(0);

        let to_universal = json.to_universal.into_iter().map(|(key, value)| {
            Ok((
                NamespacedIdentifier::parse_string(key, id_opts)?,
                NamespacedIdentifier::parse_string(value, id_opts)?,
            ))
        }).collect::<Result<_, MappingParseError>>()?;

        let from_universal = json.from_universal.into_iter().map(|(key, value)| {
            Ok((
                NamespacedIdentifier::parse_string(key, id_opts)?,
                NamespacedIdentifier::parse_string(value, id_opts)?,
            ))
        }).collect::<Result<_, MappingParseError>>()?;

        Ok(Self {
            biome_to_number,
            number_to_biome,
            default_biome_number,
            to_universal,
            from_universal,
        })
    }
}

impl NumericalBlockMap {
    pub fn from_json(json: &str, opts: MappingParseOptions) -> Result<Self, MappingParseError> {
        let to_number_map: HashMap<String, u16> = serde_json::from_str(json)?;

        let to_number: HashMap<NamespacedIdentifier, u16> = to_number_map.into_iter()
            .map(|(identifier, num)| {
                Ok((NamespacedIdentifier::parse_string(identifier, opts.identifier_options)?, num))
            })
            .collect::<Result<_, MappingParseError>>()?;

        // A value such that the range (0..max_num_plus_one) contains all the identifiers' numbers
        let max_num_plus_one = to_number.values().max().map(|&m| usize::from(m)+1).unwrap_or(0);

        let mut to_identifier = vec![None; max_num_plus_one];
        for (key, value) in to_number.iter() {
            to_identifier[usize::from(*value)] = Some(key.clone());
        }

        let air = NamespacedIdentifier {
            namespace: MINECRAFT_NAMESPACE.into(),
            path: "air".into(),
        };
        let default_block_number = *to_number.get(&air).unwrap_or(&0);

        Ok(Self {
            to_number,
            to_identifier,
            default_block_number,
        })
    }
}

impl WaterloggingInfo {
    pub fn from_json(
        waterloggable_json: &str,
        always_waterlogged_json: &str,
        opts: MappingParseOptions,
    ) -> Result<Self, MappingParseError> {

        let waterloggable: Vec<String> = serde_json::from_str(waterloggable_json)?;
        let always_waterlogged: Vec<String> = serde_json::from_str(always_waterlogged_json)?;

        let waterloggable = waterloggable.into_iter()
            .map(|s| Ok(
                NamespacedIdentifier::parse_string(s, opts.identifier_options)?
            ))
            .collect::<Result<_, MappingParseError>>()?;

        let always_waterlogged = always_waterlogged.into_iter()
            .map(|s| Ok(
                NamespacedIdentifier::parse_string(s, opts.identifier_options)?
            ))
            .collect::<Result<_, MappingParseError>>()?;

        Ok(Self { waterloggable, always_waterlogged })
    }
}
