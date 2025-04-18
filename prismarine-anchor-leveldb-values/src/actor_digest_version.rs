#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorDigestVersion {
    V1_18_30,
}

impl From<ActorDigestVersion> for u8 {
    #[inline]
    fn from(value: ActorDigestVersion) -> Self {
        match value {
            ActorDigestVersion::V1_18_30 => 0,
        }
    }
}

impl TryFrom<u8> for ActorDigestVersion {
    type Error = ();

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::V1_18_30),
            _ => Err(()),
        }
    }
}
