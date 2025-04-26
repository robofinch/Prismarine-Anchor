use std::fmt;
use std::fmt::{Display, Formatter};


/// A 128-bit UUID in the 8-4-4-4-12 hex digit format,
/// such as `002494ea-22dc-4fec-b590-4ea523338c20`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UUID(pub [u32; 4]);

impl UUID {
    /// Parse a 128-bit UUID in the 8-4-4-4-12 hex digit format,
    /// such as `002494ea-22dc-4fec-b590-4ea523338c20`.
    pub fn new(uuid: &str) -> Option<Self> {
        // Based on the slightly-more-complicated UUID implementation in
        // prismarine-anchor-nbt's lexer

        // Four hyphens, 32 hex digits which are ASCII and are one byte each
        if uuid.len() != 36 {
            return None;
        }

        let uuid_chars: Vec<char> = uuid.chars().collect();

        // The above check doesn't exclude the chance of multibyte chars
        if uuid_chars.len() != 36 {
            return None;
        }

        // Two utility functions

        fn chars_to_u32(chars: [char; 8]) -> Option<u32> {
            let nibbles = chars.map(|c| c.to_digit(16));

            let mut sum = 0;
            for nibble in nibbles {
                sum = (sum << 4) + nibble?;
            }

            Some(sum)
        }

        fn pair_to_u32(chars: ([char; 4], [char; 4])) -> Option<u32> {
            let upper = chars.0.map(|c| c.to_digit(16));
            let lower = chars.1.map(|c| c.to_digit(16));

            let mut sum = 0;

            for nibble in upper {
                sum = (sum << 4) + nibble?;
            }
            for nibble in lower {
                sum = (sum << 4) + nibble?;
            }

            Some(sum)
        }

        // Split the UUID into its parts
        let first:       [char; 8] = uuid_chars[ 0.. 8].try_into().unwrap();
        let second:      [char; 4] = uuid_chars[ 9..13].try_into().unwrap();
        let third:       [char; 4] = uuid_chars[14..18].try_into().unwrap();
        let fourth:      [char; 4] = uuid_chars[19..23].try_into().unwrap();
        let fifth_start: [char; 4] = uuid_chars[24..28].try_into().unwrap();
        let fifth_end:   [char; 8] = uuid_chars[28..36].try_into().unwrap();

        Some(Self([
            chars_to_u32(first)?,
            pair_to_u32((second, third))?,
            pair_to_u32((fourth, fifth_start))?,
            chars_to_u32(fifth_end)?,
        ]))
    }

    /// Extend the provided bytes with this UUID serialized into a byte string in the
    /// 8-4-4-4-12 UUID format.
    #[inline]
    pub fn extend_serialized(self, bytes: &mut Vec<u8>) {
        bytes.reserve(36);
        bytes.extend(self.to_string().as_bytes());
    }

    /// Write this UUID into bytes in the 8-4-4-4-12 hex digit UUID format.
    #[inline]
    pub fn to_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes);
        bytes
    }
}

impl TryFrom<&str> for UUID {
    type Error = ();

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(())
    }
}

impl From<UUID> for Vec<u8> {
    #[inline]
    fn from(value: UUID) -> Self {
        value.to_bytes()
    }
}

impl Display for UUID {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:08x}", self.0[0])?;
        write!(f, "-{:04x}", self.0[1] >> 16)?;
        write!(f, "-{:04x}", self.0[1] & 0xFFFF)?;
        write!(f, "-{:04x}", self.0[2] >> 16)?;
        write!(f, "-{:04x}", self.0[2] & 0xFFFF)?;
        write!(f, "{:08x}", self.0[3])
    }
}
