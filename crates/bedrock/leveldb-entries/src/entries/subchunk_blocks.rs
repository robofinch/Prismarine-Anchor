#![allow(clippy::len_zero)]

use std::io::Cursor;

use subslice_to_array::SubsliceToArray as _;
use thiserror::Error;

use prismarine_anchor_nbt::{NbtCompound, settings::IoOptions};
use prismarine_anchor_nbt::io::{NbtIoError, read_compound, write_compound};
use prismarine_anchor_util::u64_equals_usize;

use super::helpers::NibbleArray;
use super::helpers::palettized_storage::{
    PaletteHeader, PaletteHeaderParseError, PaletteType,
    PalettizedStorage, PalettizedStorageParseError,
};


#[derive(Debug, Clone)]
pub enum SubchunkBlocks {
    Legacy(Box<LegacySubchunkBlocks>),
    V1(SubchunkBlocksV1),
    V8(SubchunkBlocksV8),
    V9(SubchunkBlocksV9),
}

impl SubchunkBlocks {
    pub fn parse(value: &[u8]) -> Result<Self, SubchunkBlocksParseError> {
        if value.len() < 1 {
            return Err(SubchunkBlocksParseError::NoHeader);
        }
        let version = value[0];

        Ok(match version {
            0 | 2..=7 => Self::Legacy(Box::new(LegacySubchunkBlocks::parse(value)?)),
            1 => Self::V1(SubchunkBlocksV1::parse(value)?),
            8 => Self::V8(SubchunkBlocksV8::parse(value)?),
            9 => Self::V9(SubchunkBlocksV9::parse(value)?),
            _ => return Err(SubchunkBlocksParseError::UnknownVersion(version)),
        })
    }

    #[inline]
    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) -> Result<(), NbtIoError> {
        match self {
            Self::Legacy(blocks) => blocks.extend_serialized(bytes),
            Self::V1(blocks) => blocks.extend_serialized(bytes)?,
            Self::V8(blocks) => blocks.extend_serialized(bytes)?,
            Self::V9(blocks) => blocks.extend_serialized(bytes)?,
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

#[derive(Debug, Clone)]
pub struct LegacySubchunkBlocks {
    // TODO: make this easier to use, with unflattened and unpacked data.
    // Not a priority, though, since this data isn't normally used anymore.

    /// Version of the chunk, which in this case is either `0` or in `2..=7`.
    pub version:           u8,
    /// All block IDs in this subchunk, in YZX order (Y increments first).
    pub block_ids:         [u8; 4096],
    /// All block data for this subchunk, with 4 bits per block,
    /// in YZX order (Y increments first).
    ///
    /// Note that there are 4096 values, in 2048 bytes.
    pub packed_block_data: NibbleArray<2048>,
    /// All skylight values for this subchunk, with 4 bits per block,
    /// in YZX order (Y increments first). Optional.
    ///
    /// Note that there are 4096 values, in 2048 bytes.
    pub skylight:          Option<NibbleArray<2048>>,
    /// All blocklight values for this subchunk, with 4 bits per block,
    /// in YZX order (Y increments first). Optional.
    ///
    /// Note that there are 4096 values, in 2048 bytes.
    pub blocklight:        Option<NibbleArray<2048>>,
}

impl LegacySubchunkBlocks {
    pub fn parse(value: &[u8]) -> Result<Self, LegacyParseError> {
        // There must be version and block IDs and block data,
        // and optionally 2048 or 4096 additional bytes for skylight and blocklight.

        let (skylight, blocklight) = if value.len() == 1 + 4096 + 2048 {
            (false, false)
        } else if value.len() == 1 + 4096 + 2048 + 2048 {
            (true, false)
        } else if value.len() == 1 + 4096 + 2048 + 4096 {
            (true, true)
        } else {
            return Err(LegacyParseError::InvalidLength(value.len()));
        };

        // Parse version
        let version = value[0];
        if !matches!(version, 0 | 2..=7) {
            return Err(LegacyParseError::InvalidVersion(version));
        }
        let value = &value[1..];

        let block_ids: [u8; 4096] = value.subslice_to_array::<0, 4096>();
        let value = &value[4096..];

        let packed_block_data: [u8; 2048] = value.subslice_to_array::<0, 2048>();
        let packed_block_data = NibbleArray(packed_block_data);
        let value = &value[2048..];

        if !skylight {
            return Ok(Self {
                version,
                block_ids,
                packed_block_data,
                skylight:   None,
                blocklight: None,
            });
        }

        let skylight: Option<[u8; 2048]> = Some(value.subslice_to_array::<0, 2048>());
        let skylight = skylight.map(NibbleArray);
        let value = &value[2048..];

        if !blocklight {
            return Ok(Self {
                version,
                block_ids,
                packed_block_data,
                skylight,
                blocklight: None,
            });
        }

        let blocklight: Option<[u8; 2048]> = Some(value.subslice_to_array::<0, 2048>());
        let blocklight = blocklight.map(NibbleArray);

        Ok(Self {
            version,
            block_ids,
            packed_block_data,
            skylight,
            blocklight,
        })
    }

    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) {
        let mut needed_space = 1 + 4096 + 2048;
        if self.blocklight.is_some() {
            needed_space += 4096;
        } else if self.skylight.is_some() {
            needed_space += 2048;
        }

        bytes.reserve(needed_space);

        bytes.push(self.version);
        bytes.extend(&self.block_ids);
        bytes.extend(&self.packed_block_data.0);

        if let Some(blocklight) = &self.blocklight {
            bytes.extend(&self.skylight.map(|arr| arr.0).unwrap_or([0; 2048]));
            bytes.extend(blocklight.0);

        } else if let Some(skylight) = &self.skylight {
            bytes.extend(skylight.0);
        }
    }

    #[inline]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes);
        bytes
    }
}

#[derive(Debug, Clone)]
pub struct SubchunkBlocksV1(pub PalettizedStorage<NbtCompound>);

impl SubchunkBlocksV1 {
    pub fn parse(value: &[u8]) -> Result<Self, V1ParseError> {
        if value.len() < 1 {
            return Err(V1ParseError::HeaderTooShort);
        }

        let version = value[0];
        if version != 1 {
            return Err(V1ParseError::WrongVersion(version));
        }

        let mut reader = Cursor::new(&value[1..]);
        let header = PaletteHeader::parse_header(&mut reader)?;

        let block_layer = match header.palette_type {
            PaletteType::Runtime => {
                // Unlike with Data3D, only Persistent is usually used,
                // and we only support that.
                return Err(V1ParseError::RuntimePalette);
            }
            PaletteType::Persistent => {
                let block_layer = PalettizedStorage::parse(
                    &mut reader,
                    header.bits_per_index,
                    |reader, palette_len| {
                        let mut compounds = Vec::new();
                        let opts = IoOptions::bedrock_uncompressed();

                        for _ in 0..palette_len {
                            let (compound, _) = read_compound(reader, opts)?;
                            compounds.push(compound);
                        }

                        Ok(compounds)
                    },
                )?;

                match &block_layer {
                    PalettizedStorage::Empty => {
                        return Err(V1ParseError::EmptyPalette)
                    }
                    PalettizedStorage::Uniform(_) => {
                        return Err(V1ParseError::UniformPalette)
                    }
                    PalettizedStorage::Palettized(_) => {
                        block_layer
                    }
                }
            }
        };

        if u64_equals_usize(reader.position(), reader.into_inner().len()) {
            Ok(Self(block_layer))
        } else {
            Err(V1ParseError::NotAllRead)
        }
    }

    /// The contents of `bytes` is unspecified if an error is returned.
    #[inline]
    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) -> Result<(), NbtIoError> {
        bytes.push(1);

        self.0.extend_serialized(
            bytes,
            PaletteType::Persistent,
            true,
            write_block_layers,
        )
    }

    #[inline]
    pub fn to_bytes(&self) -> Result<Vec<u8>, NbtIoError> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes)?;
        Ok(bytes)
    }
}

#[derive(Debug, Clone)]
pub struct SubchunkBlocksV8 {
    pub block_layers: Vec<PalettizedStorage<NbtCompound>>,
}

impl SubchunkBlocksV8 {
    pub fn parse(value: &[u8]) -> Result<Self, V8ParseError> {
        if value.len() < 2 {
            return Err(V8ParseError::HeaderTooShort);
        }

        let version = value[0];
        if version != 8 {
            return Err(V8ParseError::WrongVersion(version));
        }

        let num_layers = usize::from(value[1]);
        let layer_bytes = &value[2..];

        let mut reader = Cursor::new(layer_bytes);
        let total_len = layer_bytes.len();

        let mut block_layers = Vec::with_capacity(num_layers);
        for _ in 0..num_layers {
            let header = PaletteHeader::parse_header(&mut reader)?;

            match header.palette_type {
                PaletteType::Runtime => {
                    // Unlike with Data3D, only Persistent is usually used,
                    // and we only support that.
                    return Err(V8ParseError::RuntimePalette);
                }
                PaletteType::Persistent => {
                    let block_layer = PalettizedStorage::parse(
                        &mut reader,
                        header.bits_per_index,
                        |reader, palette_len| {
                            let mut compounds = Vec::new();
                            let opts = IoOptions::bedrock_uncompressed();

                            for _ in 0..palette_len {
                                let (compound, _) = read_compound(reader, opts)?;
                                compounds.push(compound);
                            }

                            Ok(compounds)
                        },
                    )?;

                    match &block_layer {
                        PalettizedStorage::Empty => {
                            return Err(V8ParseError::EmptyPalette)
                        }
                        PalettizedStorage::Uniform(_) => {
                            return Err(V8ParseError::UniformPalette)
                        }
                        PalettizedStorage::Palettized(_) => {
                            block_layers.push(block_layer);
                        }
                    }
                }
            }
        }

        if u64_equals_usize(reader.position(), total_len) {
            Ok(Self { block_layers })
        } else {
            Err(V8ParseError::NotAllRead)
        }
    }

    /// The contents of `bytes` is unspecified if an error is returned.
    /// If there are more than 255 block layers, later block layers (starting at 256)
    /// are ignored.
    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) -> Result<(), NbtIoError> {
        let layer_len = self.block_layers.len();
        let layer_len = u8::try_from(layer_len)
            .inspect_err(|_| log::warn!("Too many block layers ({layer_len}) to fit in a u8"))
            .unwrap_or(u8::MAX);

        bytes.push(8);
        bytes.push(layer_len);

        for layer in &self.block_layers {

            // match layer {
            //     PalettizedStorage::Empty => Err()
            // }

            layer.extend_serialized(bytes, PaletteType::Persistent, true, write_block_layers)?;
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

#[derive(Debug, Clone)]
pub struct SubchunkBlocksV9 {
    /// The Y-position of the subchunk, from -4 to 19.
    pub y_index:      i8,
    pub block_layers: Vec<PalettizedStorage<NbtCompound>>,
}

impl SubchunkBlocksV9 {
    pub fn parse(value: &[u8]) -> Result<Self, V9ParseError> {
        if value.len() < 3 {
            return Err(V9ParseError::HeaderTooShort);
        }

        let version = value[0];
        if version != 9 {
            return Err(V9ParseError::WrongVersion(version));
        }

        let num_layers = usize::from(value[1]);
        let y_index = value[2] as i8;
        let layer_bytes = &value[3..];

        let mut reader = Cursor::new(layer_bytes);
        let total_len = layer_bytes.len();

        let mut block_layers = Vec::with_capacity(num_layers);
        for _ in 0..num_layers {
            let header = PaletteHeader::parse_header(&mut reader)?;

            match header.palette_type {
                PaletteType::Runtime => {
                    // Unlike with Data3D, only Persistent is usually used,
                    // and we only support that.
                    return Err(V9ParseError::RuntimePalette);
                }
                PaletteType::Persistent => {
                    block_layers.push(PalettizedStorage::parse(
                        &mut reader,
                        header.bits_per_index,
                        |reader, palette_len| {
                            let mut compounds = Vec::new();
                            let opts = IoOptions::bedrock_uncompressed();

                            for _ in 0..palette_len {
                                let (compound, _) = read_compound(reader, opts)?;
                                compounds.push(compound);
                            }

                            Ok(compounds)
                        },
                    )?);
                }
            }
        }

        if u64_equals_usize(reader.position(), total_len) {
            Ok(Self {
                y_index,
                block_layers,
            })
        } else {
            Err(V9ParseError::NotAllRead)
        }
    }

    /// If there are more than 255 block layers, later block layers (starting at 256)
    /// are ignored.
    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) -> Result<(), NbtIoError> {
        let layer_len = self.block_layers.len();
        let layer_len = u8::try_from(layer_len)
            .inspect_err(|_| log::warn!("Too many block layers ({layer_len}) to fit in a u8"))
            .unwrap_or(u8::MAX);

        bytes.push(9);
        bytes.push(layer_len);
        bytes.push(self.y_index as u8);

        for layer in &self.block_layers {
            layer.extend_serialized(bytes, PaletteType::Persistent, true, write_block_layers)?;
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

#[inline]
fn write_block_layers(
    compounds: &[NbtCompound],
    bytes:     &mut Vec<u8>,
) -> Result<(), NbtIoError> {
    for compound in compounds {
        write_compound(bytes, IoOptions::bedrock_uncompressed(), None, compound)?;
    }

    Ok(())
}

#[derive(Error, Debug)]
pub enum SubchunkBlocksParseError {
    #[error("SubchunkBlocks data without a version header was encountered")]
    NoHeader,
    #[error("SubchunkBlocks data with an unknown version {0} was encountered")]
    UnknownVersion(u8),
    #[error("error while parsing SubchunkBlocks: {0}")]
    Legacy(#[from] LegacyParseError),
    #[error("error while parsing SubchunkBlocks: {0}")]
    V1(#[from] V1ParseError),
    #[error("error while parsing SubchunkBlocks: {0}")]
    V8(#[from] V8ParseError),
    #[error("error while parsing SubchunkBlocks: {0}")]
    V9(#[from] V9ParseError),
}

#[derive(Error, Debug)]
pub enum LegacyParseError {
    #[error("version {0} is not a valid version for LegacySubchunkBlocks data")]
    InvalidVersion(u8),
    #[error("LegacySubchunkBlocks data can never have length {0}")]
    InvalidLength(usize),
}

#[derive(Error, Debug)]
pub enum V1ParseError {
    #[error("there was no version header for SubchunkBlocksV1")]
    HeaderTooShort,
    #[error("expected version 1 in the SubchunkBlocksV1 header, but read version {0}")]
    WrongVersion(u8),
    #[error(transparent)]
    PaletteHeaderError(#[from] PaletteHeaderParseError),
    #[error(transparent)]
    PalettizedStorageError(#[from] PalettizedStorageParseError<NbtIoError>),
    #[error("runtime palettes are not supported for SubchunkBlocksV1 data")]
    RuntimePalette,
    #[error("empty palettes are not supported for SubchunkBlocksV1 data")]
    EmptyPalette,
    #[error("uniform palettes are not supported for SubchunkBlocksV1 data")]
    UniformPalette,
    #[error("bytes were left over after parsing SubchunkBlocksV1 data")]
    NotAllRead,
}

#[derive(Error, Debug)]
pub enum V8ParseError {
    #[error("the header for SubchunkBlocksV8 is 2 bytes, but fewer than 2 bytes were received")]
    HeaderTooShort,
    #[error("expected version 8 in the SubchunkBlocksV8 header, but read version {0}")]
    WrongVersion(u8),
    #[error(transparent)]
    PaletteHeaderError(#[from] PaletteHeaderParseError),
    #[error(transparent)]
    PalettizedStorageError(#[from] PalettizedStorageParseError<NbtIoError>),
    #[error("runtime palettes are not supported for SubchunkBlocksV8 data")]
    RuntimePalette,
    #[error("empty palettes are not supported for SubchunkBlocksV8 data")]
    EmptyPalette,
    #[error("uniform palettes are not supported for SubchunkBlocksV8 data")]
    UniformPalette,
    #[error("bytes were left over after parsing SubchunkBlocksV8 data")]
    NotAllRead,
}

#[derive(Error, Debug)]
pub enum V9ParseError {
    #[error("the header for SubchunkBlocksV9 is 3 bytes, but fewer than 3 bytes were received")]
    HeaderTooShort,
    #[error("expected version 9 in the SubchunkBlocksV9 header, but read version {0}")]
    WrongVersion(u8),
    #[error(transparent)]
    PaletteHeaderError(#[from] PaletteHeaderParseError),
    #[error(transparent)]
    PalettizedStorageError(#[from] PalettizedStorageParseError<NbtIoError>),
    #[error("runtime palettes are not supported for SubchunkBlocksV9 data")]
    RuntimePalette,
    #[error("bytes were left over after parsing SubchunkBlocksV9 data")]
    NotAllRead,
}
