/// A no-longer-used value whose semantic meaning likely moved to somewhere in `MetaData`.
/// The meaning of this entry is currently not known.
///
/// Its full name is `GeneratedPreCavesAndCliffsBlending`, as per LeviLamina. Observed values so
/// far are `[0]` (presumably `false`) and `[1]` (presumably `true`), though if `[2]` is ever
/// observed, then it may be an enum rather than a boolean.
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Copy, Clone)]
pub struct CavesAndCliffsBlending(pub bool);

impl CavesAndCliffsBlending {
    #[inline]
    pub fn parse(value: &[u8]) -> Option<Self> {
        match value {
            [0] => Some(Self(false)),
            [1] => Some(Self(true)),
            _   => None,
        }
    }

    #[inline]
    pub fn extend_serialized(self, bytes: &mut Vec<u8>) {
        bytes.push(if self.0 { 1 } else { 0 });
    }

    #[inline]
    pub fn to_bytes(self) -> Vec<u8> {
        if self.0 {
            vec![1]
        } else {
            vec![0]
        }
    }
}
