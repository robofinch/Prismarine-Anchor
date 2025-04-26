use std::{cmp::Ordering, collections::HashMap, fmt::Debug};

use thiserror::Error;

use prismarine_anchor_translation::{datatypes::GameVersion, translator::Translator};


type InternalTranslator = dyn Translator<anyhow::Error, BlockMetadata, (), ()>;


pub struct Translators {
    translators: HashMap<(GameVersion, GameVersion), Box<InternalTranslator>>,
}

impl Translators {
    pub fn new() -> Self {
        Self {
            translators: HashMap::new(),
        }
    }

    /// Add a custom translator. The translator's error type must be [`anyhow::Error`]
    /// and its block metadata must be this crate's [`BlockMetadata`] type.
    ///
    /// If you use `prismarine-anchor-derive`, you can use `#[derive(CustomTranslator)]`
    /// on your translator, provided that your block metadata can be converted to `BlockMetadata`
    /// and your error type can be converted to `anyhow::Error`.
    ///
    /// You should implement `From<YourBlockMetadataType>` for [`BlockMetadata`], and if your
    /// error type impements `std::error::Error` and `Send + Sync + 'static`,
    /// then it aleady can be converted to [`anyhow::Error`].
    /// (Most types are `Send + Sync + 'static`.)
    pub fn add_custom_translator<T: Translator<anyhow::Error, BlockMetadata, (), ()> + 'static> (
        &mut self, source: GameVersion, target: GameVersion, translator: T
    ) {
        self.translators.insert((source, target), Box::new(translator));
    }

    pub fn load_translator(
        &mut self, source: GameVersion, target: GameVersion,
    ) -> Result<(), TranslatorLoadError> {
        #[cfg(feature = "py_mc_translate")]
        {
            self.load_pymc_translator(source, target)
        }
        #[cfg(all(not(feature = "py_mc_translate"), feature = "minecraft_data"))]
        {
            self.load_mc_data_translator(source, target)
        }
        #[cfg(all(not(feature = "py_mc_translate"), not(feature = "minecraft_data")))]
        {
            let _ = source;
            let _ = target;
            Err(TranslatorLoadError::NotSupported)
        }
    }

    #[cfg(feature = "py_mc_translate")]
    pub fn load_pymc_translator(
        &mut self, _source: GameVersion, _target: GameVersion,
    ) -> Result<(), TranslatorLoadError> {
        todo!()
    }

    #[cfg(feature = "minecraft_data")]
    pub fn load_mc_data_translator(
        &mut self, _source: GameVersion, _target: GameVersion,
    ) -> Result<(), TranslatorLoadError> {
        todo!()
    }

    pub fn get_translator(
        &self, source: GameVersion, target: GameVersion
    ) -> Option<&InternalTranslator> {
        self.translators.get(&(source, target)).map(Box::as_ref)
    }

    /// Finds a game version pair similar to `(source, target)` such that a translator
    /// is currently available for that pair. Specifically, one element of the found pair is either
    /// `source` or `target`, and the other element (which is returned) is either the next
    /// or previous `GameVersion` that works (compared to the `source` or `target` it replaces).
    ///
    /// If `searching_for_source` and `searching_for_next`,
    /// then a pair of the form `(v, target)` is searched for,
    /// where `v` is **greater** than or equal to `source`; `v` is returned,
    /// and a translator for converting from version `v` to version `target` is loaded.
    ///
    /// If not `searching_for_source` and `searching_for_next`,
    /// then a pair of the form `(source, v)` is searched for,
    /// where `v` is **greater** than or equal to `target`.
    ///
    /// If `searching_for_source` and not `searching_for_next`,
    /// then a pair of the form `(v, target)` is searched for,
    /// where `v` is **less** than or equal to `source`.
    ///
    /// If not `searching_for_source` and not `searching_for_next`,
    /// then a pair of the form `(source, v)` is searched for,
    /// where `v` is **less** than or equal to `target`.
    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "This is temporary. This code needs to be improved."
        // TODO: revamp universal translation stuff
    )]
    pub fn find_available_translator(
        &self, source: &GameVersion, target: &GameVersion,
        searching_for_source: bool, searching_for_next: bool
    ) -> Option<&InternalTranslator> {

        let mut possibilities = self.translators.keys()
            .filter(|key| {
                if searching_for_source && target == &key.1 {
                    if let Some(ordering) = source.partial_cmp(&key.0) {
                        // I think the match is more readable
                        #[expect(clippy::match_like_matches_macro)]
                        return match (searching_for_next, ordering) {
                            (true,  Ordering::Greater) => true,
                            (_,     Ordering::Equal)   => true,
                            (false, Ordering::Less)    => true,
                            _ => false
                        }
                    }

                } else if !searching_for_source && source == &key.0 {
                    if let Some(ordering) = target.partial_cmp(&key.1) {
                        // I think the match is more readable
                        #[expect(clippy::match_like_matches_macro)]
                        return match (searching_for_next, ordering) {
                            (true,  Ordering::Greater) => true,
                            (_,     Ordering::Equal)   => true,
                            (false, Ordering::Less)    => true,
                            _ => false
                        }
                    }
                }

                false
            })
            .collect::<Vec<_>>();

        possibilities.sort_by(|key, other| {
            match (searching_for_source, searching_for_next) {
                (true,  true)  => key.0.partial_cmp(&other.0),
                (true,  false) => other.0.partial_cmp(&key.0),
                (false, true)  => key.1.partial_cmp(&other.1),
                (false, false) => other.1.partial_cmp(&key.1),
            }.expect("Anything in possibilities is comparable")
        });

        self.translators.get(possibilities.first()?).map(Box::as_ref)
    }
}

impl Default for Translators {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct BlockMetadata {
    pub depends_on_block_entity: bool,
    pub depends_on_position:     bool,
    pub depends_on_get_block:    bool,
    pub depends_on_other_state:  bool,
}

#[derive(Error, Debug)]
pub enum TranslatorLoadError {
    #[error("No translator could be found for the indicated (source, target) GameVersion pair")]
    NotSupported,
    // other errors will be added
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
