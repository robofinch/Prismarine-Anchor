use crate::bijective_enum_map;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyChunkVersion {
}

impl LegacyChunkVersion {
    #[inline]
    pub fn parse(version: u8) -> Option<Self> {
        Self::try_from(version).ok()
    }

    #[inline]
    pub fn to_byte(self) -> u8 {
        u8::from(self)
    }
}

// TODO: figure out which version values are only allowed for `ChunkVersion`,
// not `LegacyChunkVersion`, and vice-versa.
bijective_enum_map! {
    LegacyChunkVersion, u8, u8,
}
