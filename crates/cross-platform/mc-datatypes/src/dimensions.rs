#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};


/// A dimension of a Minecraft world, such as the Overworld.
#[cfg_attr(feature = "derive_serde",    derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone)]
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
            Self::CustomNamed(_)   => None,
        }
    }

    /// Get a `NamedDimension` if this dimension's string representation is known.
    #[inline]
    pub fn named(self) -> Option<NamedDimension> {
        match self {
            Self::Vanilla(v)       => Some(NamedDimension::Vanilla(v)),
            Self::CustomNumeric(_) => None,
            Self::CustomNamed(n)   => Some(NamedDimension::CustomNamed(n)),
        }
    }
}

/// A dimension of a Minecraft world with a numeric identifier.
#[cfg_attr(feature = "derive_serde",    derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
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
#[cfg_attr(feature = "derive_serde",    derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone)]
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
#[cfg_attr(feature = "derive_serde",    derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
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
            _  => None,
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
#[cfg_attr(feature = "derive_serde",    derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct CustomDimensionNumber(pub i32);

/// The name of a custom dimension; used as the ID for custom Java dimensions.
// Box<str> because it shouldn't be changed much if ever.
#[cfg_attr(feature = "derive_serde",    derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone)]
pub struct CustomDimensionName(pub Box<str>);

/// Some versions of Bedrock elide the numeric ID or name of the Overworld,
/// and only serialize the IDs or names of non-Overworld dimensions.
///
/// Dimension IDs and names are read as `Option<NumericDimension>` or `Option<NamedDimension>`,
/// with `None` indicating an implicit Overworld value.
///
/// These options indicate how a `Option<NumericDimension>` or `Option<NamedDimension>`
/// should be serialized: either
/// - never elide the value and always write it,
/// - always elide the Overworld value and only write the ID or name of a non-Overworld
///   dimension, or
/// - elide the Overworld value if the option is `None`.
///
/// The best choices (aside from testing, where `MatchElision` may be useful) are
/// - numeric dimension IDs for all current versions (up to at least 1.21.51): `AlwaysElide`
/// - dimension names for any version below 1.20.40: `AlwaysElide`
/// - dimension names for any version at or above 1.20.40: `AlwaysWrite`
#[cfg_attr(feature = "derive_serde",    derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub enum OverworldElision {
    /// Write the IDs and names of all dimensions.
    AlwaysWrite,
    /// Always write the ID or name of the Overworld, and only write the IDs and names of all
    /// non-Overworld dimensions.
    AlwaysElide,
    /// Elide the ID or name of the Overworld if and only if a `Option<NumericDimension>`
    /// or `Option<NamedDimension>` is `None`. The IDs and names of all non-Overworld dimensions
    /// are always written.
    MatchElision,
}

impl OverworldElision {
    pub fn maybe_elide_id(
        self,
        dimension_id: Option<NumericDimension>,
    ) -> Option<NumericDimension> {
        match self {
            Self::AlwaysElide => match dimension_id {
                // Can't do `Some(NumericDimension::OVERWORLD)` in a pattern without `PartialEq`
                None | Some(NumericDimension::Vanilla(VanillaDimension::Overworld)) => None,
                Some(other_dimension) => Some(other_dimension),
            }
            Self::AlwaysWrite => {
                Some(dimension_id.unwrap_or(NumericDimension::OVERWORLD))
            }
            Self::MatchElision => dimension_id,
        }
    }

    pub fn maybe_elide_name(
        self,
        dimension_name: Option<&NamedDimension>,
    ) -> Option<&NamedDimension> {
        match self {
            Self::AlwaysElide => match dimension_name {
                // Can't do `Some(&NamedDimension::OVERWORLD)` in a pattern without `PartialEq`
                None | Some(&NamedDimension::Vanilla(VanillaDimension::Overworld)) => None,
                Some(other_dimension) => Some(other_dimension),
            }
            Self::AlwaysWrite => {
                Some(dimension_name.unwrap_or(&NamedDimension::OVERWORLD))
            }
            Self::MatchElision => dimension_name,
        }
    }
}
