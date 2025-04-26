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
