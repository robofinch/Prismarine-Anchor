#![allow(clippy::len_zero)]

use std::array;

use nonmax::NonMaxI16;


#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub enum BlendingData {
    Zero,
    Version {
        version: u8,
    },
    VersionAndData {
        version:  u8,
        i16_data: [Option<NonMaxI16>; 16],
        i8_data:  i8,
    },
}

impl BlendingData {
    pub fn parse(value: &[u8]) -> Option<Self> {
        // Need at least one byte
        if value.len() < 1 {
            return None;
        }

        if value[0] == 0 {
            match value.len() {
                1 => Some(Self::Zero),
                2 => Some(Self::Version { version: value[1] }),
                _ => None,
            }
        } else if value[0] == 1 {
            if value.len() == 2 + 32 + 1 {
                let version   = value[1];
                let i16_bytes = &value[2..34];
                let i8_data   = value[34] as i8;

                let mut i16_bytes = i16_bytes.iter();
                let i16_data: [_; 16] = array::from_fn(|_| {
                    // This doesn't panic because the i16_bytes iter contains
                    // exactly 32 u8's.
                    #[expect(
                        clippy::unwrap_used,
                        reason = "we call `.next().unwrap()` exactly `32 == 16 * 2` times",
                    )]
                    let entry = i16::from_le_bytes([
                        *i16_bytes.next().unwrap(),
                        *i16_bytes.next().unwrap(),
                    ]);

                    NonMaxI16::new(entry)
                });

                Some(Self::VersionAndData {
                    version,
                    i16_data,
                    i8_data,
                })
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) {
        match self {
            Self::Zero => bytes.push(0),
            Self::Version { version } => {
                bytes.extend([0, *version]);
            }
            Self::VersionAndData {
                version,
                i16_data,
                i8_data,
            } => {
                bytes.reserve(35);
                bytes.extend([1, *version]);
                for &entry in i16_data {
                    let entry_repr = if let Some(non_max) = entry {
                        non_max.get()
                    } else {
                        i16::MAX
                    };
                    bytes.extend(entry_repr.to_le_bytes());
                }
                bytes.push(*i8_data as u8);
            }
        }
    }

    #[inline]
    pub fn to_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes);
        bytes
    }
}
