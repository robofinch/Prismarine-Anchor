use std::{fmt, mem};
use std::{
    collections::{BTreeMap, VecDeque},
    fmt::{Display, Formatter},
};

use thiserror::Error;

use prismarine_anchor_nbt::{NbtCompound, NbtTag};


// A BTreeMap is used instead of HashMap in order to make caching of Blocks simpler
// (HashMaps do not implement Hash or Ord, BTreeMaps implement both).
pub type BlockProperties = BTreeMap<String, BlockProperty>;

pub const UNIVERSAL_NAMESPACE: &'static str = "universal_minecraft";
pub const MINECRAFT_NAMESPACE: &'static str = "minecraft";


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockPosition {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChunkPosition {
    pub x: i32,
    pub z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatingWorldPos {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// The possible variants of block properties in the Universal format
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BlockProperty {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    String(String),
}

impl Display for BlockProperty {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Byte(n)   => write!(f, "{n}"),
            Self::Short(n)  => write!(f, "{n}"),
            Self::Int(n)    => write!(f, "{n}"),
            Self::Long(n)   => write!(f, "{n}"),
            Self::String(s) => write!(f, "{s}"),
        }
    }
}

impl From<BlockProperty> for NbtTag {
    fn from(value: BlockProperty) -> Self {
        match value {
            BlockProperty::Byte(n)   => Self::Byte(n),
            BlockProperty::Short(n)  => Self::Short(n),
            BlockProperty::Int(n)    => Self::Int(n),
            BlockProperty::Long(n)   => Self::Long(n),
            BlockProperty::String(s) => Self::String(s),
        }
    }
}

impl TryFrom<NbtTag> for BlockProperty {
    type Error = NbtTag;

    fn try_from(tag: NbtTag) -> Result<Self, Self::Error> {
        match tag {
            NbtTag::Byte(n)   => Ok(Self::Byte(n)),
            NbtTag::Short(n)  => Ok(Self::Short(n)),
            NbtTag::Int(n)    => Ok(Self::Int(n)),
            NbtTag::Long(n)   => Ok(Self::Long(n)),
            NbtTag::String(s) => Ok(Self::String(s)),
            _ => Err(tag),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Block {
    pub identifier: NamespacedIdentifier,
    pub properties: BlockProperties,
    pub extra_layers: Vec<Block>,
}

impl Block {
    pub fn new(
        identifier: NamespacedIdentifier,
        properties: Option<BlockProperties>,
        extra_layers: Option<Vec<Block>>,
    ) -> Self {
        Self {
            identifier,
            properties: properties.unwrap_or_else(|| BlockProperties::new()),
            // Vec::new() is cheap
            extra_layers: extra_layers.unwrap_or(Vec::new()),
        }
    }

    pub fn new_air() -> Self {
        Self {
            identifier: NamespacedIdentifier {
                namespace: UNIVERSAL_NAMESPACE.into(),
                path: "air".into(),
            },
            properties: BlockProperties::new(),
            extra_layers: Vec::new(),
        }
    }

    /// Recursively flattens the `extra_layers` of this block
    /// into a single vector (this block's own `extra_layers`), so that afterwards,
    /// the blocks in `extra_layers` have no extra layers of their own. The order of extra
    /// layers is preserved
    /// (a block's extra layers are after that block, and before its next sibling block).
    pub fn flatten_layers(&mut self) {
        let new_vec = Vec::with_capacity(self.extra_layers.len());
        // mem::replace returns the value of self.extra_layers *and* gives us ownership of it,
        // and sets self.extra_layers to new_vec. (that's why it can give us ownership)
        let extra_layers = mem::replace(&mut self.extra_layers, new_vec);

        let mut layer_queue = VecDeque::from(extra_layers);

        while let Some(mut block) = layer_queue.pop_front() {
            let inner_layers = mem::replace(&mut block.extra_layers, Vec::new());

            // There's no method like .extend_front(), we have to manually push each element in
            // the vec to the front of the queue (in the correct order)
            layer_queue.reserve(inner_layers.len());
            for block in inner_layers.into_iter().rev() {
                layer_queue.push_front(block);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockEntity {
    pub identifier: NamespacedIdentifier,
    pub position: BlockPosition,
    pub nbt: NbtCompound,
    pub nbt_root_name: Box<str>,
}

impl BlockEntity {
    pub fn new(
        identifier: NamespacedIdentifier,
        position: BlockPosition,
    ) -> Self {
        Self {
            identifier,
            position,
            nbt: NbtCompound::new(),
            nbt_root_name: "".into(),
        }
    }

    pub fn new_with_nbt(
        identifier: NamespacedIdentifier,
        position: BlockPosition,
        nbt: NbtCompound,
        nbt_root_name: Box<str>,
    ) -> Self {
        Self {
            identifier,
            position,
            nbt,
            nbt_root_name,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    pub identifier: NamespacedIdentifier,
    pub position: FloatingWorldPos,
    pub nbt: NbtCompound,
    pub nbt_root_name: Box<str>,
}

impl Entity {
    pub fn new(
        identifier: NamespacedIdentifier,
        position: FloatingWorldPos,
        nbt: NbtCompound,
        nbt_root_name: Box<str>,
    ) -> Self {
        Self {
            identifier,
            position,
            nbt,
            nbt_root_name,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockOrEntity {
    Block(Block, Option<BlockEntity>),
    Entity(Entity),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub identifier: NamespacedIdentifier,
    pub nbt: NbtCompound,
}

impl Item {
    pub fn new(identifier: NamespacedIdentifier, nbt: NbtCompound) -> Self {
        Self { identifier, nbt }
    }
}

/// Namespaced identifiers are also known as resource locations.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NamespacedIdentifier {
    pub namespace: Box<str>,
    pub path: Box<str>,
}

impl NamespacedIdentifier {
    pub fn parse_string(
        mut identifier: String, opts: IdentifierParseOptions,
    ) -> Result<Self, IdentifierParseError> {

        let path = match identifier.find(':') {
            // "+ 1" because the UTF-8 byte length of ':' is 1
            Some(colon_pos) => identifier.split_off(colon_pos + 1),
            None => if opts.assume_empty_namespace {
                mem::replace(&mut identifier, String::new())

            } else {
                let mut quoted = String::with_capacity(identifier.len() + 2);
                quoted.push_str("\"");
                quoted.push_str(&identifier);
                quoted.push_str("\"");
                return Err(IdentifierParseError::InvalidIdentifier(quoted));
            }
        };
        // Either the namespace is empty or has a colon at the end. This pops the colon,
        // or leaves namespace unchanged.
        identifier.pop();
        let namespace = identifier;

        // Validate the namespace and path
        if opts.java_character_constraints {
            // If we can find a character which is not allowed, return an error.
            if let Some(ch) = namespace.chars().find(|&ch| {
                let allowed = ch.is_ascii_digit()
                    || ch.is_ascii_lowercase()
                    || ['_', '-', '.'].contains(&ch);
                !allowed
            }) {
                return Err(IdentifierParseError::InvalidNamespaceCharacter(path, ch));
            }

            if let Some(ch) = path.chars().find(|&ch| {
                let allowed = ch.is_ascii_digit()
                    || ch.is_ascii_lowercase()
                    || ['_', '-', '.', '/'].contains(&ch);
                !allowed
            }) {
                return Err(IdentifierParseError::InvalidPathCharacter(path, ch));
            }

        } else {
            // The character constraints used by Bedrock are a lot looser
            if namespace.find(':').is_some() {
                return Err(IdentifierParseError::InvalidNamespaceCharacter(path, ':'))
            }
            if namespace.find('/').is_some() {
                return Err(IdentifierParseError::InvalidNamespaceCharacter(path, '/'))
            }
            if path.find(':').is_some() {
                return Err(IdentifierParseError::InvalidPathCharacter(path, ':'))
            }
        }

        Ok(Self {
            namespace: namespace.into_boxed_str(),
            path: path.into_boxed_str(),
        })
    }
}

/// Parse options for [`NamespacedIdentifier`]s, also known as Resource Locations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdentifierParseOptions {
    /// If true, if the `namespace:` part of `namespace:path` is missing, assume
    /// that the namespace is the empty string.
    pub assume_empty_namespace: bool,
    /// If true, use Java Edition's stricter restrictions for the characters
    /// which may appear in a [`NamespacedIdentifier`].
    pub java_character_constraints: bool,
}

impl Default for IdentifierParseOptions {
    fn default() -> Self {
        Self {
            assume_empty_namespace:     false,
            java_character_constraints: false,
        }
    }
}

#[derive(Error, Debug, Clone)]
pub enum IdentifierParseError {
    #[error("expected a string identifier in the form \"namespace:path\", but receieved {0}")]
    InvalidIdentifier(String),
    #[error("invalid character '{1}' in the namespace of \"{0}\"")]
    InvalidNamespaceCharacter(String, char),
    #[error("invalid character '{1}' in the path of \"{0}\"")]
    InvalidPathCharacter(String, char),
}
