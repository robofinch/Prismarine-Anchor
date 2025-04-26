use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use prismarine_anchor_nbt::snbt;
use prismarine_anchor_nbt::{
    comparable::ComparableNbtTag, NbtContainerType, NbtType, snbt::VerifiedSnbt,
};
use prismarine_anchor_translation::datatypes::{BlockPosition, BlockProperty};

use super::{
    block_property_from_str, code_functions::CodeFunction, container_type,
    MappingParseError, MappingParseOptions, NamespacedIdentifier,
    nbt_type, PropertyName, PropertyNameBoxed, Snbt,
};


// ================================================================
//   Mapping files
// ================================================================

#[derive(Debug)]
pub struct MappingFile {
    pub functions_and_options: Box<[MappingFunction]>,
}

impl MappingFile {
    /// Deserialize a JSON file of Minecraft to/from Universal mappings
    /// into a more workable Rust version.
    // Having this function be separate from parse_mapping_file could maybe reduce binary size
    // from monomorphization
    #[inline]
    pub fn from_json(json: &str, opts: MappingParseOptions) -> Result<Self, MappingParseError> {
        parse_mapping_file(serde_json::from_str(json)?, opts)
    }
}

// ================================================================
//   Final data structures
// ================================================================

#[derive(Debug, Clone)]
pub enum Index {
    Number(usize),
    String(Box<str>),
}

/// A single mapping function (may be nested)
#[derive(Debug)]
pub enum MappingFunction {
    NewBlock(NamespacedIdentifier),
    // Not actually implemented
    // NewEntity(NamespacedIdentifier),
    NewNbt(Box<[NewNbtOptions]>),
    /// Add new Properties to the block with the indicated SNBT values.
    NewProperties(BTreeMap<PropertyNameBoxed, BlockProperty>),
    /// For something with one of the indicated `NamespacedIdentifier`s,
    /// apply the corresponding mapping functions to the block.
    MapBlockName(HashMap<NamespacedIdentifier, Box<[MappingFunction]>>),
    /// For something with an NBT value in the current path such that
    /// `Some(ComparableNbtTag)` is in the below map, apply the corresponding mapping functions.
    /// If the NBT value value in the current path doesn't satisfy that, apply the
    /// default `None` case (if it exists).
    MapNbt(BTreeMap<Option<ComparableNbtTag>, Box<[MappingFunction]>>),
    /// If something has a certain Property, and if its value for that Property is in
    /// the map with `BlockProperty` keys, then apply the corresponding mapping functions
    /// to the block.
    MapProperties(HashMap<PropertyNameBoxed, HashMap<BlockProperty, Box<[MappingFunction]>>>),
    CarryNbt {
        outer_name: Box<str>, // option in source JSON
        outer_type: NbtContainerType,
        /// `NbtContainerType` is the type of the thing whose index is Index,
        /// not the type being indexed into.
        path:       Option<Box<[(Index, NbtContainerType)]>>,
        /// None means unchanged
        key:        Option<Index>,
        /// None means unchanged
        value_type: Option<NbtType>,
    },
    /// The keys might be property names or NBT keys. Any property or NBT key
    /// which the block has is carried to the output if the block's value for that property
    /// or key is listed in the `Box<[T]>`.
    CarryProperties(HashMap<Box<str>, Box<[ComparableNbtTag]>>),
    Code(CodeFunction),
    Multiblock(Box<[(BlockPosition, Box<[MappingFunction]>)]>),
    WalkInputNbt {
        // Apparently there used to be "outer_name", probably that was replaced with "path"
        path:   Box<[(Index, NbtContainerType)]>,
        // This one thing nearly triples the size of MappingFunction, so it's in a Box instead
        // of just being directly included.
        // nested: NestedWalkInputNbt,
        nested: Box<NestedWalkInputNbt>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum MappingFunctionType {
    NewBlock,
    // Not actually implemented
    // NewEntity,
    NewNbt,
    NewProperties,
    MapBlockName,
    MapNbt,
    MapProperties,
    CarryNbt,
    CarryProperties,
    Code,
    Multiblock,
    WalkInputNbt,
}

impl MappingFunction {
    pub fn function_type(&self) -> MappingFunctionType {
        match self {
            Self::NewBlock { .. }        => MappingFunctionType::NewBlock,
            Self::NewNbt { .. }          => MappingFunctionType::NewNbt,
            Self::NewProperties { .. }   => MappingFunctionType::NewProperties,
            Self::MapBlockName { .. }    => MappingFunctionType::MapBlockName,
            Self::MapNbt { .. }          => MappingFunctionType::MapNbt,
            Self::MapProperties { .. }   => MappingFunctionType::MapProperties,
            Self::CarryNbt { .. }        => MappingFunctionType::CarryNbt,
            Self::CarryProperties { .. } => MappingFunctionType::CarryProperties,
            Self::Code { .. }            => MappingFunctionType::Code,
            Self::Multiblock { .. }      => MappingFunctionType::Multiblock,
            Self::WalkInputNbt { .. }    => MappingFunctionType::WalkInputNbt,
        }
    }
}

#[derive(Debug)]
pub struct NewNbtOptions {
    // Optional in source JSON, but given a default
    pub outer_name: Box<str>,
    // Optional in source JSON, but given a default
    pub outer_type: NbtContainerType,
    // Whether a nonexistent list is the same as empty list depends on whether it's
    // inside walk_input_nbt.
    // Note that the second entry is *not* the NbtContainerType of the thing which the first entry
    // indexes into; it's the NbtContainerType of the thing whose index is StringOrUsize.
    pub path:       Option<Box<[(Index, NbtContainerType)]>>,
    pub key:        Index, // string or int, for compounds or lists
    pub value:      VerifiedSnbt,
}

#[derive(Debug)]
pub struct NestedWalkInputNbt {
    pub functions:      Box<[MappingFunction]>,
    // Should only not be defined if this is a nested array type, probably.
    // I'm slightly more lenient, it can be missing wherever. But if so,
    // the only things which run are functions and/or self_default (TODO: not sure yet).
    pub self_type:      Option<NbtType>,
    pub self_default:   Box<[MappingFunction]>, // functions to run if self type is different
    // Nested is only for container NBT types, and only runs if self has the correct type
    pub nested:         Option<IndexedNested>,
    pub nested_default: Box<[MappingFunction]>, // function to run if not an index of nested
}

#[derive(Debug)]
pub enum IndexedNested {
    String(HashMap<Box<str>, NestedWalkInputNbt>),
    Number(HashMap<usize, NestedWalkInputNbt>),
}

// ================================================================
//   JSON structures
// ================================================================

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum IndexJson {
    Number(usize),
    String(String),
}

impl From<IndexJson> for Index {
    fn from(value: IndexJson) -> Self {
        match value {
            IndexJson::Number(n) => Self::Number(n),
            IndexJson::String(string) => Self::String(string.into_boxed_str()),
        }
    }
}

fn parse_mapping_file(
    json: MappingJson<'_>,
    opts: MappingParseOptions,
) -> Result<MappingFile, MappingParseError> {
    let functions_and_options = json
        .0
        .into_iter()
        .map(|function_json| function_json.parse(opts))
        .collect::<Result<_, MappingParseError>>()?;

    Ok(MappingFile {
        functions_and_options,
    })
}

/// A not-yet-validated mapping file
#[derive(Serialize, Deserialize)]
struct MappingJson<'a>(#[serde(borrow)] Vec<FunctionJson<'a>>);

#[derive(Serialize, Deserialize)]
struct FunctionJson<'a> {
    function: MappingFunctionType,
    #[serde(borrow)]
    path:     Option<&'a RawValue>, // Special case for one function
    #[serde(borrow)]
    options:  &'a RawValue,
}

impl FunctionJson<'_> {
    fn parse_multiple(
        function_vec: Vec<FunctionJson<'_>>,
        opts:         MappingParseOptions,
    ) -> Result<Box<[MappingFunction]>, MappingParseError> {
        function_vec
            .into_iter()
            .map(|function| function.parse(opts))
            .collect::<Result<_, _>>()
    }

    fn parse(self, opts: MappingParseOptions) -> Result<MappingFunction, MappingParseError> {
        // TODO: test whether serde_json's recursion limit protects us, or if we're
        // vulnerable to stack overflows.

        // More or less, the shorter functions are inlined below,
        // the longer ones get their own function called from here.
        let options_json = self.options.get();

        match self.function {
            MappingFunctionType::NewBlock => {
                let identifier: String = serde_json::from_str(options_json)?;

                Ok(MappingFunction::NewBlock(
                    NamespacedIdentifier::parse_string(identifier, opts.identifier_options)?,
                ))
            }
            MappingFunctionType::NewNbt          => parse_new_nbt(         options_json, opts),
            MappingFunctionType::NewProperties   => parse_new_properties(  options_json, opts),
            MappingFunctionType::MapBlockName    => parse_map_block_name(  options_json, opts),
            MappingFunctionType::MapNbt          => parse_map_nbt(         options_json, opts),
            MappingFunctionType::MapProperties   => parse_map_properties(  options_json, opts),
            MappingFunctionType::CarryNbt        => parse_carry_nbt(       options_json, opts),
            MappingFunctionType::CarryProperties => parse_carry_properties(options_json, opts),
            MappingFunctionType::Code => {
                CodeFunction::parse(options_json).map(MappingFunction::Code)
            }
            MappingFunctionType::Multiblock      => parse_multiblock(      options_json, opts),
            MappingFunctionType::WalkInputNbt => {
                parse_walk_nbt(options_json, self.path.map(RawValue::get), opts)
            }
        }
    }
}

fn parse_new_nbt(
    options_json: &str,
    opts:         MappingParseOptions,
) -> Result<MappingFunction, MappingParseError> {

    #[derive(Serialize, Deserialize)]
    struct NewNbtOptionJson {
        outer_name: Option<String>,
        outer_type: Option<String>,
        path:       Option<Vec<(IndexJson, String)>>,
        key:        IndexJson,
        value:      String,
    }

    let json_vec: Vec<NewNbtOptionJson> = serde_json::from_str(options_json)?;

    // Try to avoid needing to downsize when we call .into_boxed_slice() later
    let mut new_nbt_options = Vec::new();
    new_nbt_options.reserve_exact(json_vec.len());

    for json in json_vec {

        let outer_name = match json.outer_name {
            Some(string) => string.into_boxed_str(),
            None => "".into(),
        };
        let outer_type = if let Some(outer_type) = json.outer_type {
            container_type(&outer_type)?
        } else {
            NbtContainerType::Compound
        };

        let path = if let Some(path) = json.path {
            let path_steps = path
                .into_iter()
                .map(|(index, next_container_type)| {
                    Ok((index.into(), container_type(&next_container_type)?))
                })
                .collect::<Result<_, MappingParseError>>()?;

            Some(path_steps)
        } else {
            None
        };

        let key = json.key.into();
        let value = VerifiedSnbt::new(json.value, opts.snbt_options)
            .map_err(MappingParseError::InvalidSnbt)?;

        new_nbt_options.push(NewNbtOptions {
            outer_name,
            outer_type,
            path,
            key,
            value,
        });
    }

    Ok(MappingFunction::NewNbt(new_nbt_options.into_boxed_slice()))
}

fn parse_new_properties(
    options_json: &str,
    opts:         MappingParseOptions,
) -> Result<MappingFunction, MappingParseError> {
    // There might be some clever way to avoid the extra allocations
    // from remapping everything, but whatever.
    let new_properties: HashMap<String, &RawValue> = serde_json::from_str(options_json)?;

    let new_properties = new_properties
        .into_iter()
        .map(|(property_name, value)| {
            let property_value: String = serde_json::from_str(value.get())?;

            let property_value = block_property_from_str(&property_value, &property_name, opts)?;

            Ok((property_name.into_boxed_str(), property_value))
        })
        .collect::<Result<_, MappingParseError>>()?;

    Ok(MappingFunction::NewProperties(new_properties))
}

fn parse_map_block_name(
    options_json: &str,
    opts:         MappingParseOptions,
) -> Result<MappingFunction, MappingParseError> {

    type BlockNameMapJson<'a> = HashMap<String, Vec<FunctionJson<'a>>>;

    let blockname_map: BlockNameMapJson<'_> = serde_json::from_str(options_json)?;

    let blockname_map = blockname_map
        .into_iter()
        .map(|(key, function_vec)| {
            Ok((
                NamespacedIdentifier::parse_string(key, opts.identifier_options)?,
                FunctionJson::parse_multiple(function_vec, opts)?,
            ))
        })
        .collect::<Result<_, MappingParseError>>()?;

    Ok(MappingFunction::MapBlockName(blockname_map))
}

fn parse_map_nbt(
    options_json: &str,
    opts:         MappingParseOptions,
) -> Result<MappingFunction, MappingParseError> {

    #[derive(Serialize, Deserialize)]
    struct MapNbtJson<'a> {
        #[serde(borrow)]
        cases:   Option<BTreeMap<String, Vec<FunctionJson<'a>>>>,
        #[serde(borrow)]
        default: Option<Vec<FunctionJson<'a>>>,
    }

    let json: MapNbtJson<'_> = serde_json::from_str(options_json)?;

    let mut nbt_map = if let Some(cases) = json.cases {
        cases
            .into_iter()
            .map(|(key, function_vec)| {
                let key = snbt::parse_any(&key, opts.snbt_options)
                    .map_err(MappingParseError::InvalidSnbtKey)?;
                let key = ComparableNbtTag::new(key);

                Ok((Some(key), FunctionJson::parse_multiple(function_vec, opts)?))
            })
            .collect::<Result<_, MappingParseError>>()?
    } else {
        BTreeMap::new()
    };

    if let Some(function_vec) = json.default {
        nbt_map.insert(None, FunctionJson::parse_multiple(function_vec, opts)?);
    }

    Ok(MappingFunction::MapNbt(nbt_map))
}

fn parse_map_properties(
    options_json: &str,
    opts:         MappingParseOptions,
) -> Result<MappingFunction, MappingParseError> {

    type PropertyMapJson<'a> = HashMap<PropertyName, HashMap<String, Vec<FunctionJson<'a>>>>;

    let json:PropertyMapJson<'_> = serde_json::from_str(options_json)?;

    // Most normal nested iteration
    let property_map = json
        .into_iter()
        .map(|(property_name, value_map)| {
            let value_map = value_map
                .into_iter()
                .map(|(key, function_vec)| {
                    Ok((
                        block_property_from_str(&key, &property_name, opts)?,
                        FunctionJson::parse_multiple(function_vec, opts)?,
                    ))
                })
                .collect::<Result<_, MappingParseError>>()?;

            Ok((property_name.into_boxed_str(), value_map))
        })
        .collect::<Result<_, MappingParseError>>()?;

    Ok(MappingFunction::MapProperties(property_map))
}

fn parse_carry_nbt(
    options_json: &str,
    _opts:        MappingParseOptions,
) -> Result<MappingFunction, MappingParseError> {

    #[derive(Serialize, Deserialize)]
    struct CarryNbtJson {
        outer_name: Option<String>,
        outer_type: Option<String>,
        path:       Option<Vec<(IndexJson, String)>>,
        key:        Option<IndexJson>,
        r#type:     Option<String>,
    }

    let json: CarryNbtJson = serde_json::from_str(options_json)?;

    let outer_name = match json.outer_name {
        Some(string) => string.into_boxed_str(),
        None => "".into(),
    };
    let outer_type = if let Some(outer_type) = json.outer_type {
        container_type(&outer_type)?
    } else {
        NbtContainerType::Compound
    };

    // If not None, apply the conversion function (which may return an Err), and propagate any Err
    let key = json.key.map(Index::from);
    let value_type = json.r#type.map(|s| nbt_type(&s)).transpose()?;

    let path = if let Some(path) = json.path {
        let path_steps = path
            .into_iter()
            .map(|(index, next_container_type)| {
                Ok((index.into(), container_type(&next_container_type)?))
            })
            .collect::<Result<_, MappingParseError>>()?;

        Some(path_steps)
    } else {
        None
    };

    Ok(MappingFunction::CarryNbt {
        outer_name,
        outer_type,
        path,
        key,
        value_type,
    })
}

fn parse_carry_properties(
    options_json: &str,
    opts:         MappingParseOptions,
) -> Result<MappingFunction, MappingParseError> {

    let carry_properties_json: HashMap<String, Vec<Snbt>> = serde_json::from_str(options_json)?;

    let carry_properties = carry_properties_json
        .into_iter()
        .map(|(key, snbt_vec)| {
            let nbt_vec = snbt_vec
                .into_iter()
                .map(|snbt| {
                    let tag = snbt::parse_any(&snbt, opts.snbt_options)
                        .map_err(MappingParseError::InvalidSnbt)?;

                    Ok(ComparableNbtTag::new(tag))
                })
                .collect::<Result<_, MappingParseError>>()?;

            Ok((key.into_boxed_str(), nbt_vec))
        })
        .collect::<Result<_, MappingParseError>>()?;

    Ok(MappingFunction::CarryProperties(carry_properties))
}

fn parse_multiblock(
    options_json: &str,
    opts:         MappingParseOptions,
) -> Result<MappingFunction, MappingParseError> {

    #[derive(Serialize, Deserialize)]
    struct MultiblockEntry<'a> {
        coords:    Vec<i32>,
        #[serde(borrow)]
        functions: Vec<FunctionJson<'a>>,
    }

    let json_vec: Vec<MultiblockEntry<'_>> = serde_json::from_str(options_json)?;

    let multiblock_entries = json_vec
        .into_iter()
        .map(|json| {
            if json.coords.len() != 3 {
                return Err(MappingParseError::MultiblockCoordLen(json.coords.len()));
            }
            let coords = BlockPosition {
                x: json.coords[0],
                y: json.coords[1],
                z: json.coords[2],
            };

            let functions = FunctionJson::parse_multiple(json.functions, opts)?;

            Ok((coords, functions))
        })
        .collect::<Result<_, MappingParseError>>()?;

    Ok(MappingFunction::Multiblock(multiblock_entries))
}

// Parsing `walk_nbt` is more complicated
#[derive(Serialize, Deserialize)]
struct WalkInputOptionsJson<'a> {
    #[serde(borrow)]
    functions:      Option<Vec<FunctionJson<'a>>>,

    r#type:         Option<String>,
    #[serde(borrow)]
    self_default:   Option<Vec<FunctionJson<'a>>>,

    #[serde(borrow)]
    keys:           Option<HashMap<String, WalkInputOptionsJson<'a>>>,
    #[serde(borrow)]
    index:          Option<HashMap<String, WalkInputOptionsJson<'a>>>,
    #[serde(borrow)]
    nested_default: Option<Vec<FunctionJson<'a>>>,
}

fn parse_walk_input_options(
    json: WalkInputOptionsJson<'_>,
    opts: MappingParseOptions,
) -> Result<NestedWalkInputNbt, MappingParseError> {
    let carry_default = MappingFunction::CarryNbt {
        outer_name: "".into(),
        outer_type: NbtContainerType::Compound,
        path:       None,
        key:        None,
        value_type: None,
    };

    let self_type = json.r#type.map(|s| nbt_type(&s)).transpose()?;

    let self_default = json
        .self_default
        .map(|functions| FunctionJson::parse_multiple(functions, opts))
        .transpose()?
        .unwrap_or(Box::new([carry_default]));

    let functions = json
        .functions
        .map(|functions| FunctionJson::parse_multiple(functions, opts))
        .transpose()?
        .unwrap_or(Box::new([]));

    let (nested, parse_nested_default) = match self_type {
        Some(NbtType::Compound) => (
            if let Some(keys) = json.keys {
                let nested = keys
                    .into_iter()
                    .map(|(key, options)| {
                        Ok((
                            key.into_boxed_str(),
                            parse_walk_input_options(options, opts)?,
                        ))
                    })
                    .collect::<Result<_, MappingParseError>>()?;

                Some(IndexedNested::String(nested))
            } else {
                None
            },
            true,
        ),
        Some(NbtType::List | NbtType::ByteArray | NbtType::IntArray | NbtType::LongArray) => (
            if let Some(index) = json.index {
                let nested = index
                    .into_iter()
                    .map(|(index, options)| {
                        #[expect(clippy::map_err_ignore)]
                        let index = usize::from_str_radix(&index, 10)
                            .map_err(|_| MappingParseError::InvalidIndex(index))?;

                        Ok((index, parse_walk_input_options(options, opts)?))
                    })
                    .collect::<Result<_, MappingParseError>>()?;

                Some(IndexedNested::Number(nested))
            } else {
                None
            },
            true,
        ),
        _ => (None, false),
    };

    let carry_default = MappingFunction::CarryNbt {
        outer_name: "".into(),
        outer_type: NbtContainerType::Compound,
        path:       None,
        key:        None,
        value_type: None,
    };

    let nested_default = if parse_nested_default {
        json.nested_default
            .map(|functions| FunctionJson::parse_multiple(functions, opts))
            .transpose()?
            .unwrap_or(Box::new([carry_default]))
    } else {
        Box::new([carry_default])
    };

    Ok(NestedWalkInputNbt {
        functions,
        self_type,
        self_default,
        nested,
        nested_default,
    })
}


fn parse_walk_nbt(
    options_json: &str,
    path_json:    Option<&str>,
    opts:         MappingParseOptions,
) -> Result<MappingFunction, MappingParseError> {
    let path: Box<[(Index, NbtContainerType)]> = if let Some(path_json) = path_json {
        let path: Vec<(IndexJson, String)> = serde_json::from_str(path_json)?;

        path.into_iter()
            .map(|(index, next_container_type)| {
                Ok((index.into(), container_type(&next_container_type)?))
            })
            .collect::<Result<_, MappingParseError>>()?
    } else {
        Box::new([])
    };

    let json: WalkInputOptionsJson<'_> = serde_json::from_str(options_json)?;

    Ok(MappingFunction::WalkInputNbt {
        path,
        nested: Box::new(parse_walk_input_options(json, opts)?),
    })
}
