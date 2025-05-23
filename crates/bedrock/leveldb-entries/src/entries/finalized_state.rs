#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct FinalizedStateDbValue(pub u32);

impl FinalizedStateDbValue {
    #[inline]
    pub fn parse(value: &[u8]) -> Option<Self> {
        let four_bytes = value.try_into().ok()?;
        Some(Self(u32::from_le_bytes(four_bytes)))
    }

    #[inline]
    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) {
        bytes.extend(self.0.to_le_bytes());
    }

    #[inline]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_le_bytes().to_vec()
    }
}
