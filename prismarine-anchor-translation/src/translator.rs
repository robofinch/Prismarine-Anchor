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
    Numeric(u32, u32, u32),
    String(String),
}

impl VersionName {
    pub fn parse_numeric(version: &str) -> Option<Self> {
        let mut version = version.split(".");
        let major = u32::from_str_radix(version.next()?, 10).ok()?;
        let minor = u32::from_str_radix(version.next()?, 10).ok()?;
        let patch = u32::from_str_radix(version.next()?, 10).ok()?;

        if version.next().is_none() {
            Some(Self::Numeric(major, minor, patch))
        } else {
            None
        }
    }
}

impl PartialOrd for VersionName {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if let &Self::Numeric(major, minor, patch) = self {
            if let &Self::Numeric(other_major, other_minor, other_patch) = other {
                return Some((major, minor, patch).cmp(&(other_major, other_minor, other_patch)))
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
            &Self::Numeric(major, minor, patch) => write!(f, "{major}.{minor}.{patch}"),
            Self::String(string) => write!(f, "{string}"),
        }
    }
}
