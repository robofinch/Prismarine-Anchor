use prismarine_anchor_nbt::io::NbtIoError;

use crate::{
    concatenated_nbt_compounds::ConcatenatedNbtCompounds,
    named_compound::NamedCompound,
    ValueParseOptions,
    ValueToBytesOptions,
};


#[cfg_attr(feature = "derive_standard", derive(PartialEq))]
#[derive(Debug, Clone)]
pub enum Actor {
    /// This is what the game uses in normal, non-buggy situations.
    Normal(NamedCompound),
    /// You should *really* not use this yourself, as different entities should have different
    /// `ActorID`s.
    ///
    /// However, at least in 1.18.30 and 1.18.31, I have observed worlds with multiple entities
    /// with the same `ActorID` (and `UniqueID` in their NBT). Presumably, this is not normally
    /// supposed to happen.
    // TODO: how does Minecraft handle this? Is one or both entity deleted? Is it perhaps
    // consistent in preserving only the first, or only the last entity?
    // Is behavior different depending on whether the entities are in the same chunk?
    // If in different chunks, does it depend on which order the chunks are loaded?
    Multiple(Vec<NamedCompound>),
}

impl Actor {
    #[inline]
    pub fn parse(value: &[u8], opts: ValueParseOptions) -> Result<Self, NbtIoError> {
        let nbts = ConcatenatedNbtCompounds::parse(value, opts)?;

        if nbts.0.len() == 1 {
            #[expect(clippy::unwrap_used, reason = "we checked the length")]
            Ok(Self::Normal(nbts.0.into_iter().next().unwrap()))
        } else {
            Ok(Self::Multiple(nbts.0))
        }
    }

    pub fn extend_serialized(
        &self,
        bytes: &mut Vec<u8>,
        opts: ValueToBytesOptions,
    ) -> Result<(), NbtIoError> {
        match self {
            Self::Normal(nbt) => {
                nbt.extend_serialized(bytes, opts)
            }
            Self::Multiple(nbts) => {
                for nbt in nbts {
                    nbt.extend_serialized(bytes, opts)?;
                }

                Ok(())
            }
        }
    }

    #[inline]
    pub fn to_bytes(&self, opts: ValueToBytesOptions) -> Result<Vec<u8>, NbtIoError> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes, opts)?;
        Ok(bytes)
    }
}
