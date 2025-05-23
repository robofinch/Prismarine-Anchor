//! Translator corresponding to PyMCTranslate data.
//!
//! Currently, PyMCTranslate only handles blocks, not entities or items.
//! For instance, item frame blocks from Bedrock are set to air when converted to Java,
//! without adding a new entity.

mod specifications;
mod mappings;
// Files which occur once for a version
mod headers;
mod code_functions;


use serde_json::Error as JsonError;
use thiserror::Error;

use prismarine_anchor_nbt::snbt;
use prismarine_anchor_nbt::{
    NbtContainerType, NbtTag, NbtType,
    SnbtParseOptions, snbt::SnbtError,
};
use prismarine_anchor_translation::datatypes::{
    BlockProperty, IdentifierParseError, IdentifierParseOptions, NamespacedIdentifier,
};

// make unused warnings go away for now
// TODO: do something about them for real
pub use self::{mappings::MappingFile, specifications::SpecificationFile};
pub use self::headers::{BiomeMap, NumericalBlockMap, Platform, VersionMetadata, WaterloggingInfo};


/// Useful for annotating types more precisely.
/// A string which should be valid SNBT, but which hasn't yet been validated.
pub type Snbt = String;
/// Useful for annotating types more precisely.
/// The name of a property.
pub type PropertyName = String;
/// Useful for annotating types more precisely.
/// The name of a property.
pub type PropertyNameStr<'a> = &'a str;
/// Useful for annotating types more precisely.
/// The name of a property.
pub type PropertyNameBoxed = Box<str>;


#[derive(Debug)]
pub struct PyMcMappings;
// {
// blockstate, option numeric, biome data, metadata in version folder
// }

// ================================================================
//  Options and Error
// ================================================================

#[derive(Debug, Clone, Copy)]
pub struct MappingParseOptions {
    pub identifier_options: IdentifierParseOptions,
    pub snbt_options:       SnbtParseOptions,
}

// This is incredibly messy, but whatever.
#[derive(Error, Debug)]
pub enum MappingParseError {
    #[error(transparent)]
    Json(#[from] JsonError),
    #[error(transparent)]
    Identifier(#[from] IdentifierParseError),
    #[error("specification is missing the default for property '{0}'")]
    MissingDefault(PropertyName),
    #[error("specification had a default for '{0}', which is not a property")]
    ExtraDefault(PropertyName),
    #[error("the default for property '{property}' was '{invalid_value}', which is not an option")]
    InvalidDefault {
        property:      PropertyName,
        invalid_value: BlockProperty,
    },
    #[error("specification had one of 'snbt' or 'nbt_identifier', but should have both or neither")]
    SnbtXorIdentifier,
    #[error("the value for 'nbt_identifier' had length {0}, but should be length 2")]
    IdentifierLength(usize),
    #[error["the value for property '{property}' was invalid SNBT: {error}"]]
    InvalidPropertySnbt {
        property: PropertyName,
        error:    SnbtError,
    },
    #[error("a key had invalid SNBT data: {0}")]
    InvalidSnbtKey(SnbtError),
    #[error("invalid SNBT data: {0}")]
    InvalidSnbt(SnbtError),
    #[error("a property must be a Byte, Short, Int, Long, or String tag, but was {0}")]
    InvalidProperty(&'static str),
    #[error("a code function, '{0}', had unexpected inputs specified")]
    IncorrectInput(&'static str),
    #[error("a code function, '{0}', had unexpected outputs specified")]
    IncorrectOutput(&'static str),
    #[error(
        "expected the name of an NBT container type, like \"compound\" or \"int_array\", \
         but received \"{0}\"",
    )]
    InvalidContainerType(String),
    #[error(
        "expected the name of an NBT type, like \"int\" or \"byte_array\", but received \"{0}\"",
    )]
    InvalidNbtType(String),
    #[error("expected a string parsable as an integer index (usize), but received \"{0}\"")]
    InvalidIndex(String),
    #[error("expected multiblock coords to have 3 integer components, but receieved {0}")]
    MultiblockCoordLen(usize),
}

impl MappingParseError {
    #[inline]
    pub fn invalid_property(tag: &NbtTag) -> Self {
        Self::InvalidProperty(tag_description(&tag.tag_type()))
    }
}

// ================================================================
//  Utilities used in various parts of this module
// ================================================================

pub fn block_property_from_str(
    property:      &str,
    property_name: &str,
    opts:          MappingParseOptions,
) -> Result<BlockProperty, MappingParseError> {
    #[rustfmt::skip]
    let tag: NbtTag = snbt::parse_any(property, opts.snbt_options)
        .map_err(|error| {
            MappingParseError::InvalidPropertySnbt {
                property: property_name.to_owned(),
                error,
            }
        })?;

    tag.try_into()
        .map_err(|tag| MappingParseError::invalid_property(&tag))
}

#[inline]
pub fn tag_description(tag: &NbtType) -> &'static str {
    match tag {
        NbtType::Byte      => "a Byte",
        NbtType::Short     => "a Short",
        NbtType::Int       => "an Int",
        NbtType::Long      => "a Long",
        NbtType::Float     => "a Float",
        NbtType::Double    => "a Double",
        NbtType::String    => "a String",
        NbtType::ByteArray => "a ByteArray",
        NbtType::IntArray  => "an IntArray",
        NbtType::LongArray => "a LongArray",
        NbtType::Compound  => "a Compound",
        NbtType::List      => "a List",
    }
}

#[inline]
pub fn container_type(name: &str) -> Result<NbtContainerType, MappingParseError> {
    match name {
        "compound"   => Ok(NbtContainerType::Compound),
        "list"       => Ok(NbtContainerType::List),
        "byte_array" => Ok(NbtContainerType::ByteArray),
        "int_array"  => Ok(NbtContainerType::IntArray),
        "long_array" => Ok(NbtContainerType::LongArray),
        _ => Err(MappingParseError::InvalidContainerType(name.to_owned())),
    }
}

#[inline]
pub fn nbt_type(name: &str) -> Result<NbtType, MappingParseError> {
    match name {
        "byte"       => Ok(NbtType::Byte),
        "short"      => Ok(NbtType::Short),
        "int"        => Ok(NbtType::Int),
        "long"       => Ok(NbtType::Long),
        "float"      => Ok(NbtType::Float),
        "double"     => Ok(NbtType::Double),
        "byte_array" => Ok(NbtType::ByteArray),
        "string"     => Ok(NbtType::String),
        "list"       => Ok(NbtType::List),
        "compound"   => Ok(NbtType::Compound),
        "int_array"  => Ok(NbtType::IntArray),
        "long_array" => Ok(NbtType::LongArray),
        _ => Err(MappingParseError::InvalidNbtType(name.to_owned())),
    }
}
