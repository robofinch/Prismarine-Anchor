use std::io::Cursor;

use prismarine_anchor_nbt::{NbtCompound, settings::IoOptions};
use prismarine_anchor_nbt::io::{NbtIoError, read_compound, write_compound};
use prismarine_anchor_util::u64_equals_usize;


#[derive(Debug, Clone)]
pub struct ConcatenatedNbtCompounds(pub Vec<NbtCompound>);

impl ConcatenatedNbtCompounds {
    pub fn parse(input: &[u8]) -> Result<Self, NbtIoError> {
        let mut compounds = Vec::new();

        let input_len = input.len();
        let mut reader = Cursor::new(input);

        while !u64_equals_usize(reader.position(), input_len) {
            let (nbt, _) = read_compound(
                &mut reader,
                IoOptions::bedrock_uncompressed(),
            )?;
            compounds.push(nbt);
        }

        Ok(Self(compounds))
    }

    #[inline]
    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) -> Result<(), NbtIoError> {
        let mut writer = Cursor::new(bytes);

        for compound in &self.0 {
            write_compound(&mut writer, IoOptions::bedrock_uncompressed(), None, compound)?;
        }

        Ok(())
    }

    #[inline]
    pub fn to_bytes(&self) -> Result<Vec<u8>, NbtIoError> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes)?;
        Ok(bytes)
    }
}
