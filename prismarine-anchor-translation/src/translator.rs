use std::array;
use std::{borrow::Cow, cmp::Ordering};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

use crate::datatypes::{Block, BlockEntity, BlockOrEntity, BlockPosition, Entity, Item};


/// Intended to translate granular game data from one version of the game to another.
/// Not everything, such as villages, can be translated individually; additional work
/// may be necessary for the `Translator`'s user in such a case.
///
/// `Error` is returned by any translation function that errors
/// (possibly in addition to other information).
///
/// `BlockMetadata` can be used to return additional information from the `translate_block`
/// function, such as whether `block_entity`, `position`, or `get_block` were actually used.
///
/// `EntityMetadata` and `ItemMetadata` provide the same ability for `translate_entity`
/// and `translate_item`.
pub trait Translator<Error, BlockMetadata = (), EntityMetadata = (), ItemMetadata = ()> {
    /// Used to display what the translator is.
    fn translator_name(&self) -> String;

    /// Attempts to translate the provided `block` (and possibly `block_entity`). On success,
    /// the translated `BlockOrEntity` is returned along with any extra information provided
    /// by the `Translator` in `BlockMetadata`.
    ///
    /// On error, the returned `Block` and `Option<BlockEntity>` should
    /// be the `block` and `block_entity` inputs; they can be used by the caller
    /// to avoid data being lost from a failed translation.
    fn translate_block(
        &self,
        block: Block,
        block_entity: Option<BlockEntity>,
        position: BlockPosition,
        get_block: &dyn Fn(BlockPosition) -> (Block, Option<BlockEntity>),
    ) -> Result<
        (BlockOrEntity, BlockMetadata),
        (Error, Block, Option<BlockEntity>, BlockMetadata)
    >;

    /// Attempts to translate the provided `entity`. On success,
    /// the translated `BlockOrEntity` is returned along with any extra information provided
    /// by the `Translator` in `EntityMetadata`.
    ///
    /// On error, the returned `Entity` should be the `entity` input;
    /// it can be used by the caller to avoid data being lost from a failed translation.
    fn translate_entity(
        &self,
        entity: Entity,
    ) -> Result<(BlockOrEntity, EntityMetadata), (Error, Entity, EntityMetadata)>;

    /// Attempts to translate the provided `item`. On success,
    /// the translated `Item` is returned along with any extra information provided
    /// by the `Translator` in `ItemMetadata`.
    ///
    /// On error, the returned `Item` should be the `item` input;
    /// it can be used by the caller to avoid data being lost from a failed translation.
    fn translate_item(
        &self,
        item: Item,
    ) -> Result<(Item, ItemMetadata), (Error, Item, ItemMetadata)>;

    /// Translate a biome's string identifier. If one of the
    fn translate_biome(&self, biome: &str) -> Result<Cow<'static, str>, Error>;
    /// Translate a biome's string identifier to a numeric
    fn biome_to_numeric(&self, biome: &str) -> Result<u32, Error>;
    fn biome_from_numeric(&self, num: u32) -> Result<Cow<'static, str>, Error>;
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
    pub fn parse_numeric(version: &str) -> Option<Self> {
        NumericVersion::parse(version).map(Self::Numeric)
    }

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
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
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
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        // Only show the fourth or fifth components if necessary.
        match (self.3 == 0, self.4 == 0) {
            (true,  true)  => write!(f, "{}.{}.{}",       self.0, self.1, self.2),
            (false, true)  => write!(f, "{}.{}.{}.{}",    self.0, self.1, self.2, self.3),
            (_,     false) => write!(f, "{}.{}.{}.{}.{}", self.0, self.1, self.2, self.3, self.4),
        }
    }
}

impl From<[u32; 5]> for NumericVersion {
    fn from(value: [u32; 5]) -> Self {
        Self(value[0], value[1], value[2], value[3], value[4])
    }
}

impl From<(u32, u32, u32, u32, u32)> for NumericVersion {
    fn from(value: (u32, u32, u32, u32, u32)) -> Self {
        Self(value.0, value.1, value.2, value.3, value.4)
    }
}

impl From<[u32; 3]> for NumericVersion {
    fn from(value: [u32; 3]) -> Self {
        Self(value[0], value[1], value[2], 0, 0)
    }
}

impl From<(u32, u32, u32)> for NumericVersion {
    fn from(value: (u32, u32, u32)) -> Self {
        Self(value.0, value.1, value.2, 0, 0)
    }
}

impl From<NumericVersion> for [u32; 5] {
    fn from(value: NumericVersion) -> Self {
        [value.0, value.1, value.2, value.3, value.4]
    }
}

impl From<NumericVersion> for (u32, u32, u32, u32, u32) {
    fn from(value: NumericVersion) -> Self {
        (value.0, value.1, value.2, value.3, value.4)
    }
}

impl TryFrom<&str> for NumericVersion {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value).ok_or(())
    }
}
