#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkVersion(pub u8);

impl ChunkVersion {
    /// The most recent `ChunkVersion`s are in the 40s.
    // TODO: figure out what the possible values are
    #[inline]
    pub fn parse(version: u8) -> Option<Self> {
        Some(Self(version))
    }
}

impl From<ChunkVersion> for u8 {
    #[inline]
    fn from(value: ChunkVersion) -> Self {
        value.0
    }
}

impl TryFrom<u8> for ChunkVersion {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::parse(value).ok_or(())
    }
}
