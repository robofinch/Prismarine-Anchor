use std::fmt;
use std::fmt::{Display, Formatter};

use subslice_to_array::SubsliceToArray as _;

use prismarine_anchor_util::{chars_to_u32, InspectNone as _, pair_to_u32};


/// A 128-bit UUID in the 8-4-4-4-12 hex digit format,
/// such as `002494ea-22dc-4fec-b590-4ea523338c20`.
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct UUID(pub [u32; 4]);

impl UUID {
    /// Parse a 128-bit UUID in the 8-4-4-4-12 hex digit format,
    /// such as `002494ea-22dc-4fec-b590-4ea523338c20`.
    pub fn parse(uuid: &str) -> Option<Self> {
        // Based on the slightly-more-complicated UUID implementation in
        // prismarine-anchor-nbt's lexer

        // Four hyphens, 32 hex digits which are ASCII and are one byte each
        if uuid.len() != 36 {
            log::warn!("A presumed 8-4-4-4-12 hex digit UUID was not the correct length");
            return None;
        }

        let uuid_chars: Vec<char> = uuid.chars().collect();

        // The above check doesn't exclude the chance of multibyte chars
        if uuid_chars.len() != 36 {
            log::warn!("A presumed 8-4-4-4-12 hex digit UUID was not the correct length");
            return None;
        }

        // Split the UUID into its parts
        let first:       [char; 8] = uuid_chars.subslice_to_array::< 0,  8>();
        let second:      [char; 4] = uuid_chars.subslice_to_array::< 9, 13>();
        let third:       [char; 4] = uuid_chars.subslice_to_array::<14, 18>();
        let fourth:      [char; 4] = uuid_chars.subslice_to_array::<19, 23>();
        let fifth_start: [char; 4] = uuid_chars.subslice_to_array::<24, 28>();
        let fifth_end:   [char; 8] = uuid_chars.subslice_to_array::<28, 36>();

        #[inline]
        fn convert(
            first:       [char; 8],
            second:      [char; 4],
            third:       [char; 4],
            fourth:      [char; 4],
            fifth_start: [char; 4],
            fifth_end:   [char; 8],
        ) -> Option<UUID> {
            Some(UUID([
                chars_to_u32(first)?,
                pair_to_u32((second, third))?,
                pair_to_u32((fourth, fifth_start))?,
                chars_to_u32(fifth_end)?,
            ]))
        }

        convert(first, second, third, fourth, fifth_start, fifth_end)
            .inspect_none(|| log::warn!("Failed to parse UUID: {uuid}"))
    }

    /// Extend the provided bytes with this UUID serialized into a byte string in the
    /// 8-4-4-4-12 UUID format.
    #[inline]
    pub fn extend_serialized(self, bytes: &mut Vec<u8>) {
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
        Self::parse(value).ok_or(())
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
