#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacyChunkVersion(pub u8);

impl LegacyChunkVersion {
    // TODO: figure out what the possible values are
    #[inline]
    pub fn parse(version: u8) -> Option<Self> {
        Some(Self(version))
    }
}

impl From<LegacyChunkVersion> for u8 {
    #[inline]
    fn from(value: LegacyChunkVersion) -> Self {
        value.0
    }
}

impl TryFrom<u8> for LegacyChunkVersion {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::parse(value).ok_or(())
    }
}
