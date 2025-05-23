use std::io::{Cursor, Read};

use thiserror::Error;

use prismarine_anchor_nbt::{NbtCompound, settings::IoOptions};
use prismarine_anchor_nbt::io::{NbtIoError, read_compound, write_compound};
use prismarine_anchor_util::u64_equals_usize;

use crate::interface::{DataFidelity, ValueParseOptions, ValueToBytesOptions};


#[cfg_attr(feature = "derive_standard", derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct NamedCompound {
    pub compound:  NbtCompound,
    /// If using `DataFidelity::Semantic`, the empty string is used in place of the compound's
    /// root name.
    pub root_name: String,
}

impl NamedCompound {
    pub fn parse(value: &[u8], opts: ValueParseOptions) -> Result<Self, NamedCompoundParseError> {
        let mut reader = Cursor::new(value);
        let nbt = Self::read(&mut reader, opts)?;

        if u64_equals_usize(reader.position(), value.len()) {
            Ok(nbt)
        } else {
            Err(NamedCompoundParseError::ExcessData)
        }
    }

    pub fn read<R: Read>(reader: &mut R, opts: ValueParseOptions) -> Result<Self, NbtIoError> {
        let (nbt, name) = read_compound(
            reader,
            IoOptions::bedrock_uncompressed(),
        )?;

        let root_name = if matches!(opts.data_fidelity, DataFidelity::BitPerfect) {
            name
        } else {
            String::new()
        };

        Ok(Self {
            compound: nbt,
            root_name,
        })
    }

    #[inline]
    pub fn extend_serialized(
        &self,
        bytes: &mut Vec<u8>,
        opts: ValueToBytesOptions,
    ) -> Result<(), NbtIoError> {
        let root_name = if matches!(opts.data_fidelity, DataFidelity::BitPerfect) {
            Some(self.root_name.as_str())
        } else {
            None
        };

        write_compound(
            bytes,
            IoOptions::bedrock_uncompressed(),
            root_name,
            &self.compound,
        )
    }

    #[inline]
    pub fn to_bytes(&self, opts: ValueToBytesOptions) -> Result<Vec<u8>, NbtIoError> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes, opts)?;
        Ok(bytes)
    }
}

#[derive(Error, Debug)]
pub enum NamedCompoundParseError {
    #[error("error while parsing NamedCompound: {0}")]
    NbtError(#[from] NbtIoError),
    #[error("a NamedCompound was parsed, but excess data was provided after it")]
    ExcessData,
}
