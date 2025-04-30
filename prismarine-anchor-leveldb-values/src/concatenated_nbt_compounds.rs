use std::io::Cursor;

use prismarine_anchor_nbt::{NbtCompound, settings::IoOptions};
use prismarine_anchor_nbt::io::{NbtIoError, read_compound, write_compound};

use crate::all_read;


#[derive(Debug, Clone)]
pub struct ConcatenatedNbtCompounds(pub Vec<NbtCompound>);

impl ConcatenatedNbtCompounds {
    pub fn parse(input: &[u8]) -> Result<Self, NbtIoError> {
        let mut compounds = Vec::new();

        let input_len = input.len();
        let mut reader = Cursor::new(input);

        while !all_read(reader.position(), input_len) {
            let (nbt, _) = read_compound(
                &mut reader,
                IoOptions::bedrock_uncompressed(),
            )?;
            compounds.push(nbt);
        }

        Ok(Self(compounds))
    }

    pub fn extend_serialized(
        &self,
        bytes:                 &mut Vec<u8>,
        allow_invalid_strings: bool,
    ) -> Result<(), NbtIoError> {
        let mut writer = Cursor::new(bytes);

        let io_options = IoOptions {
            enable_byte_strings: allow_invalid_strings,
            ..IoOptions::bedrock_uncompressed()
        };

        for compound in &self.0 {
            write_compound(&mut writer, io_options, None, compound)?;
        }

        Ok(())
    }

    pub fn to_bytes(&self, allow_invalid_strings: bool) -> Result<Vec<u8>, NbtIoError> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes, allow_invalid_strings)?;
        Ok(bytes)
    }
}
