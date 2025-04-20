use prismarine_anchor_translation::datatypes::NumericVersion;

use crate::bijective_enum_map;


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChunkVersion {
    V0,  V1,  V2,  V3,  V4,  V5,  V6,  V7,  V8,  V9,
    V10, V11, V12, V13, V14, V15, V16, V17, V18, V19,
    V20, V21, V22, V23, V24, V25, V26, V27, V28, V29,
    V30, V31, V32, V33, V34, V35, V36, V37, V38, V39,
    V40, V41,
}

impl ChunkVersion {
    #[inline]
    pub fn parse(version: u8) -> Option<Self> {
        Self::try_from(version).ok()
    }

    #[inline]
    pub fn to_byte(self) -> u8 {
        u8::from(self)
    }

    /// Returns whether this value should be stored in the `LegacyVersion` key
    /// of a chunk instead of the `Version` key.
    #[inline]
    pub fn should_be_in_legacy_version(self) -> bool {
        self < Self::V20
    }

    /// Returns whether, in the provided version of Bedrock, chunk versions should be stored in
    /// the `LegacyVersion` key of a chunk instead of the `Version` key.
    #[inline]
    pub fn game_uses_legacy_version(game_version: NumericVersion) -> bool {
        // The wiki says that it moved "in" 1.16.100,
        // so I assume the change occurred along with the shift from 19 to 20.
        game_version < Self::V20.lowest_game_version()
    }

    /// Returns a `Version` or `LegacyVersion` value which was used during the given game version,
    /// assuming no experiments are enabled, which could be used to write new chunks to a world in
    /// that version of the game. If the game version is too old to use such a value,
    /// then `None` is returned. Note that an excessively high game version will simply return the
    /// newest `ChunkVersion`, not `None`.
    ///
    /// If multiple values are possible, the lowest is chosen, with the exception of 16 and 17,
    /// whose usage is uncertain; they are not returned at all.
    /// If you know when those chunk versions were used, please reach out!
    /// Note that some chunk versions are not returned as they were only used during 1.17
    /// if the caves and cliffs experimental feature was enabled.
    pub fn chunk_version_for(game_version: NumericVersion) -> Option<Self> {

        // TODO: this doesn't seem to cover everything, I have an alpha save whose
        // last saved version *seems* to be plain 0's, which uses chunk version 4 -
        // maybe i'm not looking at quite the right data for game version,
        // but I wouldn't trust this for the oldest versions.

        let versions = [
            Self::V0, Self::V1, Self::V2, Self::V3, Self::V4, Self::V5, Self::V6, Self::V7,
            Self::V8, Self::V9, Self::V10, Self::V11, Self::V12, Self::V13, Self::V14, Self::V15,
            // Skip 16 and 17
            Self::V18, Self::V19, Self::V20, Self::V21, Self::V22,
            // Skip experimental versions
            Self::V39, Self::V40, Self::V41,
        ];
        let game_versions = versions.map(Self::lowest_game_version);

        match game_versions.binary_search(&game_version) {
            Ok(idx) => Some(versions[idx]),
            // The `game_version` is strictly greater than `game_versions[idx-1]`,
            // and strictly less than `game_versions[idx]` (or `idx == game_versions.len()`)
            Err(idx @ 1..) => Some(versions[idx - 1]),
            // This indicates that the `game_version` is strictly less
            // than the first game version in the array; this is too old.
            Err(0) => None,
        }
    }

    /// Returns a `Version` or `LegacyVersion` value which was used during the given game version,
    /// assuming that the caves and cliffs experimental feature is enabled (which was in 1.17),
    /// which could be used to write new chunks to a world in that version of the game.
    /// If the game version is too old to use such a value, then `None` is returned.
    /// Note that an excessively high game version will simply return the newest `ChunkVersion`,
    /// not `None`, and that for non-1.17 game versions, the output is identical to
    /// `chunk_version_for`.
    ///
    /// If multiple values are possible, the lowest is chosen, with the exception of chunk versions
    /// 16-17, 23-24, 26-28, 30, and 32-38, which do not have precisely known
    /// corresponding game versions; they are not returned at all.
    /// If you know when those chunk versions were used, please reach out!
    pub fn chunk_version_for_caves_and_cliffs(game_version: NumericVersion) -> Option<Self> {
        let versions = [
            Self::V0, Self::V1, Self::V2, Self::V3, Self::V4, Self::V5, Self::V6, Self::V7,
            Self::V8, Self::V9, Self::V10, Self::V11, Self::V12, Self::V13, Self::V14, Self::V15,
            // Skip 16 and 17
            Self::V18, Self::V19, Self::V20, Self::V21, Self::V22,
            Self::V25, Self::V29, Self::V31, // Skip 23-24, 26-28, 30, and 32-38
            Self::V39, Self::V40, Self::V41,
        ];
        let game_versions = versions.map(Self::lowest_game_version);

        match game_versions.binary_search(&game_version) {
            Ok(idx) => Some(versions[idx]),
            // The `game_version` is strictly greater than `game_versions[idx-1]`,
            // and strictly less than `game_versions[idx]` (or `idx == game_versions.len()`)
            Err(idx @ 1..) => Some(versions[idx - 1]),
            // This indicates that the `game_version` is strictly less
            // than the first game version in the array; this is too old.
            Err(0) => None,
        }
    }

    /// Get the lowest game version in which this `ChunkVersion` was used.
    /// Note that versions 16-17, 23-24, 26-28, 30, and 32-38 do not have precisely known
    /// corresponding game versions, so they try to err on the side of being higher
    /// (in order to at least be valid).
    /// If you know when those chunk versions were used, please reach out!
    #[inline]
    pub fn lowest_game_version(self) -> NumericVersion {
        match self {
            Self::V0  => NumericVersion::from([0, 9, 0]),
            Self::V1  => NumericVersion::from([0, 9, 2]),
            Self::V2  => NumericVersion::from([0, 9, 5]),
            Self::V3  => NumericVersion::from([0, 17, 0]),
            Self::V4  => NumericVersion::from([0, 18, 0]),
            // V5 seems to be internally called VConsole1ToV18_0
            Self::V5  => NumericVersion::from([0, 18, 0]),
            Self::V6  => NumericVersion::from([1, 2, 0]),
            // V7 seems to be internally called V1_2_0Bis
            Self::V7  => NumericVersion::from([1, 2, 0]),
            Self::V8  => NumericVersion::from([1, 3, 0]),
            Self::V9  => NumericVersion::from([1, 8, 0]),
            Self::V10 => NumericVersion::from([1, 9, 0]),
            Self::V11 => NumericVersion::from([1, 10, 0]),
            Self::V12 => NumericVersion::from([1, 11, 0]),
            // TODO: Maybe 1.11.10 instead of 1.11.1?
            Self::V13 => NumericVersion::from([1, 11, 1]),
            // TODO: Maybe 1.11.20 instead of 1.11.2?
            Self::V14 => NumericVersion::from([1, 11, 2]),
            Self::V15 => NumericVersion::from([1, 12, 0]),
            Self::V16 => NumericVersion::from([1, 14, 0]),
            Self::V17 => NumericVersion::from([1, 15, 0]),
            Self::V18 => NumericVersion::from([1, 16, 0]),
            // V19 seems to be internally called V1_16_0Bis
            Self::V19 => NumericVersion::from([1, 16, 0]),
            // Is .56 overly precise? Might be 1.16.100
            Self::V20 => NumericVersion::from([1, 16, 100, 56, 0]),
            // V21 seems to be internally called V1_16_100Bis
            Self::V21 => NumericVersion::from([1, 16, 100, 58, 0]),
            Self::V22 => NumericVersion::from([1, 16, 210]),

            // Experimental stuff. Also, note that 1.16.300 does not seem to exist.
            // TODO: all these experimental version values. are probably inaccurate.

            // UNKNOWN: presumably somewhere around 1.17
            // V23 seems to be internally called V1_16_300CavesCliffsPart1
            Self::V23 => NumericVersion::from([1, 17, 0]),
            // UNKNOWN: presumably somewhere around 1.17
            // V24 seems to be internally called V1_16_300CavesCliffsInternalV1
            Self::V24 => NumericVersion::from([1, 17, 0]),
            // Note: probably used from 1.17.0 up to 1.17.30.
            // This is returned for 1.17.0 when the caves and cliffs experiment is enabled.
            // V25 seems to be internally called V1_16_300CavesCliffsPart2
            Self::V25 => NumericVersion::from([1, 17, 0]),
            // UNKNOWN: before 1.17.30
            // V26 seems to be internally called V1_16_300CavesCliffsInternalV2
            Self::V26 => NumericVersion::from([1, 17, 30]),
            // UNKNOWN: before 1.17.30
            // V27 seems to be internally called V1_16_300CavesCliffsPart3
            Self::V27 => NumericVersion::from([1, 17, 30]),
            // UNKNOWN: before 1.17.30
            // V28 seems to be internally called V1_16_300CavesCliffsInternalV3
            Self::V28 => NumericVersion::from([1, 17, 30]),
            // This is returned for 1.17.30 when the caves and cliffs experiment is enabled.
            // V29 seems to be internally called V1_16_300CavesCliffsPart4
            Self::V29 => NumericVersion::from([1, 17, 30]),
            // UNKNOWN: probably between 1.7.30 and 1.17.40
            // V30 seems to be internally called V1_16_300CavesCliffsInternalV4
            Self::V30 => NumericVersion::from([1, 17, 40]),
            // This is returned for 1.17.40 when the caves and cliffs experiment is enabled.
            // V31 seems to be internally called V1_16_300CavesCliffsPart5
            Self::V31 => NumericVersion::from([1, 17, 40]),
            // 32-38: UNKNOWN: probably after 1.17.40, definitely before 1.18.0.
            // AFAIK the last pre-1.18.0 version is 1.17.41, so I've put that here.
            // V32 seems to be internally called V1_16_300CavesCliffsInternalV5
            Self::V32 => NumericVersion::from([1, 17, 41]),
            // V33 seems to be internally called V1_18_0
            // ....interesting. I guess it *is* preparation for 1.18.0 features
            Self::V33 => NumericVersion::from([1, 17, 41]),
            // V34 seems to be internally called V1_18_0Internal
            Self::V34 => NumericVersion::from([1, 17, 41]),
            // V35 seems to be internally called V1_18_1
            Self::V35 => NumericVersion::from([1, 17, 41]),
            // V36 seems to be internally called V1_18_1Internal
            Self::V36 => NumericVersion::from([1, 17, 41]),
            // V37 seems to be internally called V1_18_2
            Self::V37 => NumericVersion::from([1, 17, 41]),
            // V38 seems to be internally called V1_18_2Internal
            Self::V38 => NumericVersion::from([1, 17, 41]),

            // Non-experimental stuff

            Self::V39 => NumericVersion::from([1, 18, 0]),
            Self::V40 => NumericVersion::from([1, 18, 30]),
            Self::V41 => NumericVersion::from([1, 21, 40]),
        }
    }
}

bijective_enum_map! {
    ChunkVersion, u8, u8,
    V0  <=> 0,    V1  <=> 1,    V2  <=> 2,    V3  <=> 3,    V4  <=> 4,
    V5  <=> 5,    V6  <=> 6,    V7  <=> 7,    V8  <=> 8,    V9  <=> 9,
    V10 <=> 10,   V11 <=> 11,   V12 <=> 12,   V13 <=> 13,   V14 <=> 14,
    V15 <=> 15,   V16 <=> 16,   V17 <=> 17,   V18 <=> 18,   V19 <=> 19,
    V20 <=> 20,   V21 <=> 21,   V22 <=> 22,   V23 <=> 23,   V24 <=> 24,
    V25 <=> 25,   V26 <=> 26,   V27 <=> 27,   V28 <=> 28,   V29 <=> 29,
    V30 <=> 30,   V31 <=> 31,   V32 <=> 32,   V33 <=> 33,   V34 <=> 34,
    V35 <=> 35,   V36 <=> 36,   V37 <=> 37,   V38 <=> 38,   V39 <=> 39,
    V40 <=> 40,   V41 <=> 41,
}
