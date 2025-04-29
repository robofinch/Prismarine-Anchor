use std::io::Cursor;

use prismarine_anchor_nbt::{settings::IoOptions, NbtCompound};
use prismarine_anchor_nbt::io::{NbtIoError, read_compound, write_compound};


pub trait NbtCompoundConversion {
    fn parse(value: &[u8]) -> Option<NbtCompound>;
    fn extend_serialized(&self, bytes: &mut Vec<u8>) -> Result<(), NbtIoError>;
    fn to_bytes(&self) -> Result<Vec<u8>, NbtIoError>;
}

impl NbtCompoundConversion for NbtCompound {
    fn parse(value: &[u8]) -> Option<Self> {
        read_compound(
            &mut Cursor::new(value),
            IoOptions::bedrock_uncompressed(),
        )
        .ok()
        .map(|(nbt, _)| nbt)
    }

    fn extend_serialized(&self, mut bytes: &mut Vec<u8>) -> Result<(), NbtIoError> {
        write_compound(
            &mut bytes,
            IoOptions::bedrock_uncompressed(),
            None,
            self,
        )
    }

    fn to_bytes(&self) -> Result<Vec<u8>, NbtIoError> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes)?;
        Ok(bytes)
    }
}
