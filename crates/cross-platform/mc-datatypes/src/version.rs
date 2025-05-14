use std::{array, fmt};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};


/// Indicates a version of the game, which may have a different encoding for game data.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameVersion {
    Universal,
    Bedrock(VersionName),
    Java(VersionName),
    // TODO: add more versions
    Other(String, VersionName),
}

impl PartialOrd for GameVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        #[expect(clippy::match_same_arms, reason = "clarity")]
        match (self, other) {
            (Self::Universal,  Self::Universal)        => Some(Ordering::Equal),
            (Self::Bedrock(v), Self::Bedrock(other_v)) => v.partial_cmp(other_v),
            (Self::Java(v),    Self::Java(other_v))    => v.partial_cmp(other_v),
            (Self::Other(name, v), Self::Other(other_name, other_v)) if name == other_name => {
                v.partial_cmp(other_v)
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VersionName {
    Numeric(NumericVersion),
    String(String),
}

impl VersionName {
    #[inline]
    pub fn parse_numeric(version: &str) -> Option<Self> {
        NumericVersion::parse(version).map(Self::Numeric)
    }

    #[inline]
    pub fn numeric(major: u32, minor: u32, patch: u32) -> Self {
        Self::Numeric(NumericVersion(major, minor, patch, 0, 0))
    }
}

impl PartialOrd for VersionName {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if let &Self::Numeric(version) = self {
            if let &Self::Numeric(other_version) = other {
                return Some(version.cmp(&other_version));
            }
        }
        None
    }
}

impl From<String> for VersionName {
    #[inline]
    fn from(version: String) -> Self {
        Self::parse_numeric(&version).unwrap_or(Self::String(version))
    }
}

impl Display for VersionName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            &Self::Numeric(numeric) => Display::fmt(&numeric, f),
            Self::String(string)    => Display::fmt(&string, f),
        }
    }
}

/// A type that should be able to describe numeric versions of Minecraft, such as
/// 1.21.0 (stored here as 1.21.0.0.0), as well as edge cases like 1.16.100.56 in Bedrock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NumericVersion(pub u32, pub u32, pub u32, pub u32, pub u32);

impl NumericVersion {
    /// Parse a string into a numeric version.
    ///
    /// Note that the first component is always the major
    /// version, so "1.0" is parsed the same as "1.0.0" and not "0.1.0".
    /// A single component, such as "1", is assumed to be a mistake
    /// and returns `None`. Additionally, parsing `"1."` will return `None`,
    /// as it is split into `"1"` and `""`, and the latter cannot be parsed into a number.
    pub fn parse(version: &str) -> Option<Self> {
        let mut components = version.split('.');

        let nums: [Option<u32>; 5] = array::from_fn(|idx| {
            if let Some(next_component) = components.next() {
                u32::from_str_radix(next_component, 10).ok()
            } else if idx <= 1 {
                // This is the either the first or second component, so we either got
                // "" or something like "1", which is not allowed.
                None
            } else {
                Some(0)
            }
        });

        if components.next().is_some() {
            // There were more than 5 version components
            return None;
        }

        Some(Self(nums[0]?, nums[1]?, nums[2]?, nums[3]?, nums[4]?))
    }
}

impl Display for NumericVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Only show the fourth or fifth components if necessary.
        match (self.3 == 0, self.4 == 0) {
            (true, true)  => write!(f, "{}.{}.{}",       self.0, self.1, self.2),
            (false, true) => write!(f, "{}.{}.{}.{}",    self.0, self.1, self.2, self.3),
            (_, false)    => write!(f, "{}.{}.{}.{}.{}", self.0, self.1, self.2, self.3, self.4),
        }
    }
}

impl From<[u32; 5]> for NumericVersion {
    #[inline]
    fn from(value: [u32; 5]) -> Self {
        Self(value[0], value[1], value[2], value[3], value[4])
    }
}

impl From<(u32, u32, u32, u32, u32)> for NumericVersion {
    #[inline]
    fn from(value: (u32, u32, u32, u32, u32)) -> Self {
        Self(value.0, value.1, value.2, value.3, value.4)
    }
}

impl From<[u32; 3]> for NumericVersion {
    #[inline]
    fn from(value: [u32; 3]) -> Self {
        Self(value[0], value[1], value[2], 0, 0)
    }
}

impl From<(u32, u32, u32)> for NumericVersion {
    #[inline]
    fn from(value: (u32, u32, u32)) -> Self {
        Self(value.0, value.1, value.2, 0, 0)
    }
}

impl From<NumericVersion> for [u32; 5] {
    #[inline]
    fn from(value: NumericVersion) -> Self {
        [value.0, value.1, value.2, value.3, value.4]
    }
}

impl From<NumericVersion> for (u32, u32, u32, u32, u32) {
    #[inline]
    fn from(value: NumericVersion) -> Self {
        (value.0, value.1, value.2, value.3, value.4)
    }
}

impl TryFrom<&str> for NumericVersion {
    type Error = ();

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value).ok_or(())
    }
}
