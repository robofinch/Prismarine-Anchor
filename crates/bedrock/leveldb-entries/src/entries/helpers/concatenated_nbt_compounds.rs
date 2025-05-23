use std::io::Cursor;

use prismarine_anchor_nbt::io::NbtIoError;
use prismarine_anchor_util::u64_equals_usize;

use crate::interface::{ValueParseOptions, ValueToBytesOptions};
use super::NamedCompound;


#[derive(Debug, Clone)]
pub struct ConcatenatedNbtCompounds(pub Vec<NamedCompound>);

impl ConcatenatedNbtCompounds {
    pub fn parse(input: &[u8], opts: ValueParseOptions) -> Result<Self, NbtIoError> {
        let mut compounds = Vec::new();

        let input_len = input.len();
        let mut reader = Cursor::new(input);

        while !u64_equals_usize(reader.position(), input_len) {
            let nbt = NamedCompound::read(&mut reader, opts)?;
            compounds.push(nbt);
        }

        Ok(Self(compounds))
    }

    #[inline]
    pub fn extend_serialized(
        &self,
        bytes: &mut Vec<u8>,
        opts: ValueToBytesOptions,
    ) -> Result<(), NbtIoError> {
        for nbt in &self.0 {
            nbt.extend_serialized(bytes, opts)?;
        }

        Ok(())
    }

    #[inline]
    pub fn to_bytes(&self, opts: ValueToBytesOptions) -> Result<Vec<u8>, NbtIoError> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes, opts)?;
        Ok(bytes)
    }
}
