// TODO / FIXME: the whole translation stuff probably needs to be organized better.
// Need a better planned structure, and Result types here that aren't solely targeted
// at Amulet. Note that right now, the massive results and tuples below make clippy mad.
#![allow(clippy::type_complexity)]
#![allow(clippy::result_large_err)]


use std::borrow::Cow;

use crate::datatypes::{Block, BlockEntity, BlockOrEntity, BlockPosition, Entity, Item};


/// Intended to translate granular game data from one version of the game to another.
///
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
        block:        Block,
        block_entity: Option<BlockEntity>,
        position:     BlockPosition,
        get_block:    &dyn Fn(BlockPosition) -> (Block, Option<BlockEntity>),
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
