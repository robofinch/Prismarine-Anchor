#![allow(clippy::len_zero)]

use std::io::Cursor;

use prismarine_anchor_nbt::{NbtCompound, settings::IoOptions};
use prismarine_anchor_nbt::io::{NbtIoError, read_compound, write_compound};

use crate::{all_read, slice_to_array};
use crate::palettized_storage::{PaletteHeader, PaletteType, PalettizedStorage};


#[derive(Debug, Clone)]
pub enum SubchunkBlocks {
    Legacy(Box<LegacySubchunkBlocks>),
    V8(SubchunkBlocksV8),
    V9(SubchunkBlocksV9),
}

impl SubchunkBlocks {
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() < 1 {
            return None;
        }
        let version = value[0];

        Some(match version {
            0 | 2..=7 => Self::Legacy(Box::new(LegacySubchunkBlocks::parse(value)?)),
            8 => Self::V8(SubchunkBlocksV8::parse(value)?),
            9 => Self::V9(SubchunkBlocksV9::parse(value)?),
            _ => return None,
        })
    }

    #[inline]
    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) -> Result<(), NbtIoError> {
        match self {
            Self::Legacy(blocks) => blocks.extend_serialized(bytes),
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
    // TODO: is less significant nibble before the more significant nibble?
    pub packed_block_data: [u8; 2048],
    /// All skylight values for this subchunk, with 4 bits per block,
    /// in YZX order (Y increments first). Optional.
    pub skylight:          Option<[u8; 2048]>,
    /// All blocklight values for this subchunk, with 4 bits per block,
    /// in YZX order (Y increments first). Optional.
    pub blocklight:        Option<[u8; 2048]>,
}

impl LegacySubchunkBlocks {
    pub fn parse(value: &[u8]) -> Option<Self> {
        // There must be version and block IDs and block data,
        // and optionally 2048 or 4096 additional bytes for skylight and blocklight.

        let (skylight, blocklight) = if value.len() == 1 + 4096 + 2048 {
            (false, false)
        } else if value.len() == 1 + 4096 + 2048 + 2048 {
            (true, false)
        } else if value.len() == 1 + 4096 + 2048 + 4096 {
            (true, true)
        } else {
            return None;
        };

        // Parse version
        let version = value[0];
        if !matches!(version, 0 | 2..=7) {
            return None;
        }
        let value = &value[1..];

        let block_ids: [u8; 4096] = slice_to_array::<0, 4096, _, 4096>(value);
        let value = &value[4096..];

        let packed_block_data: [u8; 2048] = slice_to_array::<0, 2048, _, 2048>(value);
        let value = &value[2048..];

        if !skylight {
            return Some(Self {
                version,
                block_ids,
                packed_block_data,
                skylight: None,
                blocklight: None,
            });
        }

        let skylight: Option<[u8; 2048]> = Some(slice_to_array::<0, 2048, _, 2048>(value));
        let value = &value[2048..];

        if !blocklight {
            return Some(Self {
                version,
                block_ids,
                packed_block_data,
                skylight,
                blocklight: None,
            });
        }

        let blocklight: Option<[u8; 2048]> = Some(slice_to_array::<0, 2048, _, 2048>(value));

        Some(Self {
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
        bytes.extend(&self.packed_block_data);

        if let Some(blocklight) = &self.blocklight {
            bytes.extend(&self.skylight.unwrap_or([0; 2048]));
            bytes.extend(blocklight);

        } else if let Some(skylight) = &self.skylight {
            bytes.extend(skylight);
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
pub struct SubchunkBlocksV8 {
    pub block_layers: Vec<PalettizedStorage<NbtCompound>>,
}

impl SubchunkBlocksV8 {
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() < 2 {
            return None;
        }

        let version = value[0];
        if version != 8 {
            return None;
        }
        let num_block_layers = usize::from(value[1]);

        let block_layers = parse_block_layers(&value[2..], num_block_layers)?;

        Some(Self { block_layers })
    }

    // TODO: explicitly say that the contents of `bytes` is unspecified if an error is returned.
    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) -> Result<(), NbtIoError> {
        // TODO: log error if there's too many block layers
        let layer_len = u8::try_from(self.block_layers.len()).unwrap_or(u8::MAX);

        bytes.push(8);
        bytes.push(layer_len);

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

#[derive(Debug, Clone)]
pub struct SubchunkBlocksV9 {
    /// The Y-position of the subchunk, from -4 to 19.
    pub y_index:      i8,
    pub block_layers: Vec<PalettizedStorage<NbtCompound>>,
}

impl SubchunkBlocksV9 {
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() < 3 {
            return None;
        }

        let version = value[0];
        if version != 9 {
            return None;
        }
        let num_block_layers = usize::from(value[1]);
        let y_index = value[2] as i8;

        let block_layers = parse_block_layers(&value[3..], num_block_layers)?;

        Some(Self {
            y_index,
            block_layers,
        })
    }

    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) -> Result<(), NbtIoError> {
        // TODO: log error if there's too many block layers
        let layer_len = u8::try_from(self.block_layers.len()).unwrap_or(u8::MAX);

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

fn parse_block_layers(
    layer_bytes: &[u8],
    num_layers:  usize,
) -> Option<Vec<PalettizedStorage<NbtCompound>>> {
    let mut reader = Cursor::new(layer_bytes);
    let total_len = layer_bytes.len();

    let mut block_layers = Vec::with_capacity(num_layers);
    for _ in 0..num_layers {
        let header = PaletteHeader::parse_header(&mut reader)?;

        match header.palette_type {
            PaletteType::Runtime => {
                // Unlike with Data3D, only Persistent is usually used, and we only support that.
                return None;
            }
            PaletteType::Persistent => {
                block_layers.push(PalettizedStorage::parse(
                    &mut reader,
                    header.bits_per_index,
                    |reader, palette_len| {
                        let mut compounds = Vec::new();

                        let opts = IoOptions {
                            allow_invalid_strings: true,
                            ..IoOptions::bedrock_uncompressed()
                        };

                        for _ in 0..palette_len {
                            let (compound, _) = read_compound(reader, opts).ok()?;
                            compounds.push(compound);
                        }

                        Some(compounds)
                    },
                )?);
            }
        }
    }

    if !all_read(reader.position(), total_len) {
        return None;
    }

    Some(block_layers)
}

fn write_block_layers(
    compounds: &[NbtCompound],
    bytes:     &mut Vec<u8>,
) -> Result<(), NbtIoError> {
    let opts = IoOptions {
        allow_invalid_strings: true,
        ..IoOptions::bedrock_uncompressed()
    };

    for compound in compounds {
        write_compound(bytes, opts, None, compound)?;
    }

    Ok(())
}
