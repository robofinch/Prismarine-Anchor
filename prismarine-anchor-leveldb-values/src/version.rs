use crate::bijective_enum_map;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkVersion {
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
}

bijective_enum_map! {
    ChunkVersion, u8, u8,
}
