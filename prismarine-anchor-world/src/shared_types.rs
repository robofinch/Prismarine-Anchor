use std::fmt;
use std::fmt::{Display, Formatter};


/// A dimension of a Minecraft world, such as the Overworld.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Dimension {
    Vanilla(VanillaDimension),
    CustomNumeric(CustomDimensionNumber),
    CustomNamed(CustomDimensionName),
}

impl Dimension {
    pub const OVERWORLD: Self = Self::Vanilla(VanillaDimension::Overworld);
    pub const NETHER: Self    = Self::Vanilla(VanillaDimension::Nether);
    pub const END: Self       = Self::Vanilla(VanillaDimension::End);

    /// Get a `NumericDimension` if this dimension's numeric representation is known.
    #[inline]
    pub fn numeric(&self) -> Option<NumericDimension> {
        match self {
            Self::Vanilla(v)       => Some(NumericDimension::Vanilla(*v)),
            Self::CustomNumeric(n) => Some(NumericDimension::CustomNumeric(*n)),
            _ => None,
        }
    }

    /// Get a `NamedDimension` if this dimension's string representation is known.
    #[inline]
    pub fn named(self) -> Option<NamedDimension> {
        match self {
            Self::Vanilla(v)     => Some(NamedDimension::Vanilla(v)),
            Self::CustomNamed(n) => Some(NamedDimension::CustomNamed(n)),
            _ => None,
        }
    }
}

/// A dimension of a Minecraft world with a numeric identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NumericDimension {
    Vanilla(VanillaDimension),
    CustomNumeric(CustomDimensionNumber),
}

impl NumericDimension {
    pub const OVERWORLD: Self = Self::Vanilla(VanillaDimension::Overworld);
    pub const NETHER: Self    = Self::Vanilla(VanillaDimension::Nether);
    pub const END: Self       = Self::Vanilla(VanillaDimension::End);

    pub fn from_java_numeric(id: i32) -> Self {
        VanillaDimension::try_from_java_numeric(id)
            .map(Self::Vanilla)
            .unwrap_or(Self::CustomNumeric(CustomDimensionNumber(id)))
    }

    pub fn from_bedrock_numeric(id: u32) -> Self {
        VanillaDimension::try_from_bedrock_numeric(id)
            .map(Self::Vanilla)
            .unwrap_or(Self::CustomNumeric(CustomDimensionNumber(id as i32)))
    }

    #[inline]
    pub fn to_java_numeric(self) -> i32 {
        match self {
            Self::Vanilla(v)            => v.to_java_numeric(),
            Self::CustomNumeric(custom) => custom.0,
        }
    }

    #[inline]
    pub fn to_bedrock_numeric(self) -> u32 {
        match self {
            Self::Vanilla(v)            => v.to_bedrock_numeric(),
            Self::CustomNumeric(custom) => custom.0 as u32,
        }
    }
}

/// A dimension of a Minecraft world with a string name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NamedDimension {
    Vanilla(VanillaDimension),
    CustomNamed(CustomDimensionName),
}

impl NamedDimension {
    pub const OVERWORLD: Self = Self::Vanilla(VanillaDimension::Overworld);
    pub const NETHER: Self    = Self::Vanilla(VanillaDimension::Nether);
    pub const END: Self       = Self::Vanilla(VanillaDimension::End);

    pub fn from_java_name(name: &str) -> Self {
        VanillaDimension::try_from_java_name(name)
            .map(Self::Vanilla)
            .unwrap_or(Self::CustomNamed(CustomDimensionName(name.into())))
    }

    pub fn from_bedrock_name(name: &str) -> Self {
        VanillaDimension::try_from_bedrock_name(name)
            .map(Self::Vanilla)
            .unwrap_or(Self::CustomNamed(CustomDimensionName(name.into())))
    }

    #[inline]
    pub fn into_java_name(self) -> Box<str> {
        match self {
            Self::Vanilla(v)          => v.to_java_name().into(),
            Self::CustomNamed(custom) => custom.0,
        }
    }

    #[inline]
    pub fn into_bedrock_name(self) -> Box<str> {
        match self {
            Self::Vanilla(v)          => v.to_bedrock_name().into(),
            Self::CustomNamed(custom) => custom.0,
        }
    }

    #[inline]
    pub fn as_java_name(&self) -> &str {
        match self {
            Self::Vanilla(v)          => v.to_java_name(),
            Self::CustomNamed(custom) => custom.0.as_ref(),
        }
    }

    #[inline]
    pub fn as_bedrock_name(&self) -> &str {
        match self {
            Self::Vanilla(v)          => v.to_bedrock_name(),
            Self::CustomNamed(custom) => custom.0.as_ref(),
        }
    }
}

/// One of Minecraft's three vanilla dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VanillaDimension {
    Overworld,
    Nether,
    End,
}

impl VanillaDimension {
    #[inline]
    pub fn try_from_java_name(name: &str) -> Option<Self> {
        match name {
            "overworld"  => Some(Self::Overworld),
            "the_nether" => Some(Self::Nether),
            "the_end"    => Some(Self::End),
            _ => None,
        }
    }

    #[inline]
    pub fn try_from_java_numeric(id: i32) -> Option<Self> {
        match id {
            0  => Some(Self::Overworld),
            -1 => Some(Self::Nether),
            1  => Some(Self::End),
            _ => None,
        }
    }

    #[inline]
    pub fn try_from_bedrock_name(name: &str) -> Option<Self> {
        match name {
            "Overworld" => Some(Self::Overworld),
            "Nether"    => Some(Self::Nether),
            "TheEnd"    => Some(Self::End),
            _ => None,
        }
    }

    #[inline]
    pub fn try_from_bedrock_numeric(id: u32) -> Option<Self> {
        match id {
            0 => Some(Self::Overworld),
            1 => Some(Self::Nether),
            2 => Some(Self::End),
            _ => None,
        }
    }

    #[inline]
    pub fn to_java_name(self) -> &'static str {
        match self {
            Self::Overworld => "overworld",
            Self::Nether    => "the_nether",
            Self::End       => "the_end",
        }
    }

    #[inline]
    pub fn to_java_numeric(self) -> i32 {
        match self {
            Self::Overworld => 0,
            Self::Nether    => -1,
            Self::End       => 1,
        }
    }

    #[inline]
    pub fn to_bedrock_name(self) -> &'static str {
        match self {
            Self::Overworld => "Overworld",
            Self::Nether    => "Nether",
            Self::End       => "TheEnd",
        }
    }

    #[inline]
    pub fn to_bedrock_numeric(self) -> u32 {
        match self {
            Self::Overworld => 0,
            Self::Nether    => 1,
            Self::End       => 2,
        }
    }
}

/// The numeric ID of a custom dimension; used in Bedrock's LevelDB keys, and can sometimes
/// be important in Java.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CustomDimensionNumber(pub i32);

/// The name of a custom dimension; used as the ID for custom Java dimensions.
// Box<str> because it shouldn't be changed much if ever.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CustomDimensionName(pub Box<str>);

/// The location of a chunk in the world.
///
/// Note that this is not the block position;
/// multiply this position by 16 to find the positions of its blocks. For example
/// `ChunkPosition { x: 1, z: 2 }` refers to the chunk from `(16, 32)` to `(31, 47)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChunkPosition {
    pub x: i32,
    pub y: i32,
}

/// A UUID in the 8-4-4-12 format, such as `002494ea-22dc-4fec-b590-4ea523338c20`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UUID(pub [u32; 4]);

impl UUID {
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

            let mut sum = nibbles[0]?;
            for i in 1..8 {
                sum = (sum << 4) +  nibbles[i]?;
            }

            Some(sum)
        }

        fn pair_to_u32(chars: ([char; 4], [char; 4])) -> Option<u32> {
            let upper = chars.0.map(|c| c.to_digit(16));
            let lower = chars.1.map(|c| c.to_digit(16));

            let mut sum = upper[0]?;

            for i in 1..4 {
                sum = (sum << 4) +  upper[i]?;
            }
            for i in 0..4 {
                sum = (sum << 4) +  lower[i]?;
            }

            Some(sum)
        }

        // Split the UUID into its parts
        let first:       [char; 8] = uuid_chars[ 0 .. 8].try_into().unwrap();
        let second:      [char; 4] = uuid_chars[ 9 ..13].try_into().unwrap();
        let third:       [char; 4] = uuid_chars[14 ..18].try_into().unwrap();
        let fourth:      [char; 4] = uuid_chars[19 ..23].try_into().unwrap();
        let fifth_start: [char; 4] = uuid_chars[24 ..28].try_into().unwrap();
        let fifth_end:   [char; 8] = uuid_chars[28 ..36].try_into().unwrap();

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
        let mut bytes = Vec::new();
        value.extend_serialized(&mut bytes);
        bytes
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
