#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct ActorDigestVersionDBValue(pub u8);

impl ActorDigestVersionDBValue {
    #[inline]
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() == 1 {
            Some(Self(value[0]))
        } else {
            None
        }
    }

    #[inline]
    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) {
        bytes.push(self.0);
    }

    #[inline]
    pub fn to_bytes(&self) -> Vec<u8> {
        vec![self.0]
    }
}
