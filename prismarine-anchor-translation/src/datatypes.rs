use std::{array, fmt, mem};
use std::cmp::Ordering;
use std::{
    collections::{BTreeMap, VecDeque},
    fmt::{Display, Formatter},
};

use thiserror::Error;

use prismarine_anchor_nbt::{NbtCompound, NbtTag};


// A BTreeMap is used instead of HashMap in order to make caching of Blocks simpler
// (HashMaps do not implement Hash or Ord, BTreeMaps implement both).
pub type BlockProperties = BTreeMap<String, BlockProperty>;

pub const UNIVERSAL_NAMESPACE: &str = "universal_minecraft";
pub const MINECRAFT_NAMESPACE: &str = "minecraft";


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockPosition {
    pub x: i32,
    pub y: i32,
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
    #[inline]
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

    #[inline]
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
    #[inline]
    pub fn new(
        identifier: NamespacedIdentifier,
        properties: Option<BlockProperties>,
        extra_layers: Option<Vec<Block>>,
    ) -> Self {
        Self {
            identifier,
            properties: properties.unwrap_or_else(BlockProperties::new),
            // Vec::new() is cheap
            extra_layers: extra_layers.unwrap_or(Vec::new()),
        }
    }

    #[inline]
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
            let inner_layers = mem::take(&mut block.extra_layers);

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
    #[inline]
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

    #[inline]
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
    #[inline]
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
    #[inline]
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
            Some(colon_pos) => {
                let path = identifier.split_off(colon_pos + 1);
                // Pop the colon
                identifier.pop();
                path
            }
            None => if let Some(namespace) = opts.default_namespace {
                mem::replace(&mut identifier, namespace.to_owned())

            } else {
                let mut quoted = String::with_capacity(identifier.len() + 2);
                quoted.push('\"');
                quoted.push_str(&identifier);
                quoted.push('\"');
                return Err(IdentifierParseError::InvalidIdentifier(quoted));
            }
        };

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

impl Display for NamespacedIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.path)
    }
}

/// Parse options for [`NamespacedIdentifier`]s, also known as Resource Locations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdentifierParseOptions {
    /// If `Some`, if the `namespace:` part of `namespace:path` is missing, assume
    /// that the namespace is this string. If this is `None` and a namespace is missing,
    /// an error is returned from appropriate functions.
    pub default_namespace: Option<&'static str>,
    /// If true, use Java Edition's stricter restrictions for the characters
    /// which may appear in a [`NamespacedIdentifier`].
    pub java_character_constraints: bool,
}

impl Default for IdentifierParseOptions {
    /// Defaults to the strictest settings.
    #[inline]
    fn default() -> Self {
        Self {
            default_namespace: None,
            java_character_constraints: true,
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

/// Indicates a version of the game, which may have a different encoding for game data.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameVersion {
    Universal,
    Bedrock(VersionName),
    Java(VersionName),
    // TODO: add more versions
    Other(String, VersionName),
}

impl PartialOrd for GameVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::Universal, Self::Universal)
                => Some(Ordering::Equal),
            (Self::Bedrock(v), Self::Bedrock(other_v))
                => v.partial_cmp(other_v),
            (Self::Java(v), Self::Java(other_v))
                => v.partial_cmp(other_v),
            (Self::Other(name, v), Self::Other(other_name, other_v)) if name == other_name
                => v.partial_cmp(other_v),
            _ => None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VersionName {
    Numeric(NumericVersion),
    String(String),
}

impl VersionName {
    #[inline]
    pub fn parse_numeric(version: &str) -> Option<Self> {
        NumericVersion::parse(version).map(Self::Numeric)
    }

    #[inline]
    pub fn numeric(major: u32, minor: u32, patch: u32) -> Self {
        Self::Numeric(NumericVersion(major, minor, patch, 0, 0))
    }
}

impl PartialOrd for VersionName {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if let &Self::Numeric(version) = self {
            if let &Self::Numeric(other_version) = other {
                return Some(version.cmp(&other_version))
            }
        }
        None
    }
}

impl From<String> for VersionName {
    #[inline]
    fn from(version: String) -> Self {
        Self::parse_numeric(&version).unwrap_or(Self::String(version))
    }
}

impl Display for VersionName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            &Self::Numeric(numeric) => Display::fmt(&numeric, f),
            Self::String(string)    => Display::fmt(&string,  f),
        }
    }
}

/// A type that should be able to describe numeric versions of Minecraft, such as
/// 1.21.0 (stored here as 1.21.0.0.0), as well as edge cases like 1.16.100.56 in Bedrock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NumericVersion(pub u32, pub u32, pub u32, pub u32, pub u32);

impl NumericVersion {
    /// Parse a string into a numeric version.
    ///
    /// Note that the first component is always the major
    /// version, so "1.0" is parsed the same as "1.0.0" and not "0.1.0".
    /// A single component, such as "1", is assumed to be a mistake
    /// and returns `None`. Additionally, parsing `"1."` will return `None`,
    /// as it is split into `"1"` and `""`, and the latter cannot be parsed into a number.
    pub fn parse(version: &str) -> Option<Self> {
        let mut components = version.split('.');

        let nums: [Option<u32>; 5] = array::from_fn(|idx| {
            if let Some(next_component) = components.next() {
                u32::from_str_radix(next_component, 10).ok()
            } else if idx <= 1 {
                // This is the either the first or second component, so we either got
                // "" or something like "1", which is not allowed.
                None
            } else {
                Some(0)
            }
        });

        if components.next().is_some() {
            // There were more than 5 version components
            return None;
        }

        Some(Self(nums[0]?, nums[1]?, nums[2]?, nums[3]?, nums[4]?))
    }
}

impl Display for NumericVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Only show the fourth or fifth components if necessary.
        match (self.3 == 0, self.4 == 0) {
            (true,  true)  => write!(f, "{}.{}.{}",       self.0, self.1, self.2),
            (false, true)  => write!(f, "{}.{}.{}.{}",    self.0, self.1, self.2, self.3),
            (_,     false) => write!(f, "{}.{}.{}.{}.{}", self.0, self.1, self.2, self.3, self.4),
        }
    }
}

impl From<[u32; 5]> for NumericVersion {
    #[inline]
    fn from(value: [u32; 5]) -> Self {
        Self(value[0], value[1], value[2], value[3], value[4])
    }
}

impl From<(u32, u32, u32, u32, u32)> for NumericVersion {
    #[inline]
    fn from(value: (u32, u32, u32, u32, u32)) -> Self {
        Self(value.0, value.1, value.2, value.3, value.4)
    }
}

impl From<[u32; 3]> for NumericVersion {
    #[inline]
    fn from(value: [u32; 3]) -> Self {
        Self(value[0], value[1], value[2], 0, 0)
    }
}

impl From<(u32, u32, u32)> for NumericVersion {
    #[inline]
    fn from(value: (u32, u32, u32)) -> Self {
        Self(value.0, value.1, value.2, 0, 0)
    }
}

impl From<NumericVersion> for [u32; 5] {
    #[inline]
    fn from(value: NumericVersion) -> Self {
        [value.0, value.1, value.2, value.3, value.4]
    }
}

impl From<NumericVersion> for (u32, u32, u32, u32, u32) {
    #[inline]
    fn from(value: NumericVersion) -> Self {
        (value.0, value.1, value.2, value.3, value.4)
    }
}

impl TryFrom<&str> for NumericVersion {
    type Error = ();

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value).ok_or(())
    }
}
