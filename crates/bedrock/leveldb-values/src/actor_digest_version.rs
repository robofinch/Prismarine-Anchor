use bijective_enum_map::injective_enum_map;

use prismarine_anchor_mc_datatypes::version::NumericVersion;


#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub enum ActorDigestVersion {
    V1_18_30,
}

impl ActorDigestVersion {
    #[inline]
    pub fn parse(version: u8) -> Option<Self> {
        Self::try_from(version).ok()
    }

    #[inline]
    pub fn to_byte(self) -> u8 {
        u8::from(self)
    }

    /// Returns an `ActorDigestVersion` chunk value which was used during the given game version,
    /// assuming no experiments are enabled, which could be used to write new chunks to a world in
    /// that version of the game. If the game version is too old to use such a value,
    /// then `None` is returned. Note that an excessively high game version will simply return the
    /// newest `ActorDigestVersion`, not `None`.
    pub fn version_for(game_version: NumericVersion) -> Option<Self> {
        if game_version < NumericVersion::from([1, 18, 30]) {
            None
        } else {
            Some(Self::V1_18_30)
        }
    }

    /// Get the lowest game version in which this `ChunkActorDigestVersionVersion` was used.
    #[inline]
    pub fn lowest_game_version(self) -> NumericVersion {
        match self {
            Self::V1_18_30 => NumericVersion::from([1, 18, 30]),
        }
    }
}

injective_enum_map! {
    ActorDigestVersion, u8,
    V1_18_30 <=> 0,
}
