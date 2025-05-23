#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct ActorID {
    pub upper: u32,
    pub lower: u32,
}

impl ActorID {
    #[inline]
    pub fn parse(bytes: [u8; 8]) -> Self {
        let upper = [bytes[0], bytes[1], bytes[2], bytes[3]];
        let lower = [bytes[4], bytes[5], bytes[6], bytes[7]];

        // Yes, big-endian bytes. Yes, this is weird for MCBE.
        Self {
            upper: u32::from_be_bytes(upper),
            lower: u32::from_be_bytes(lower),
        }
    }

    #[inline]
    pub fn extend_serialized(self, bytes: &mut Vec<u8>) {
        bytes.extend(self.to_bytes());
    }

    #[inline]
    pub fn to_bytes(self) -> [u8; 8] {
        let upper = self.upper.to_be_bytes();
        let lower = self.lower.to_be_bytes();
        [
            upper[0], upper[1], upper[2], upper[3],
            lower[0], lower[1], lower[2], lower[3],
        ]
    }
}
