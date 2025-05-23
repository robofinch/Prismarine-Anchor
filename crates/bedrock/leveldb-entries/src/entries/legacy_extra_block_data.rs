use std::fmt::{Display, Formatter, Result as FmtResult};

use subslice_to_array::SubsliceToArray as _;
use vecmap::VecMap;

use prismarine_anchor_mc_datatypes::{ChunkColumn, BlockPosInSubchunk};

use crate::interface::ValueToBytesOptions;


pub type SubchunkExtraBlockData = VecMap<SubchunkExtraBlockKey, ExtraBlockValue>;
pub type TerrainExtraBlockData  = VecMap<TerrainExtraBlockKey,  ExtraBlockValue>;


/// The data stored in the `LegacyExtraBlockData` entry of a LevelDB has used at least three
/// different formats, two of which can be handled well.
///
/// There is no version number in the data.
/// Therefore, data is stored in a format convenient to convert into any of them,
/// and you can convert this struct into a more appropriate variant
/// (probably [`SubchunkExtraBlockData`] or [`TerrainExtraBlockData`], and almost never
/// [`NbtPieces`]) when needed.
///
/// If the chunk containing this `LegacyExtraBlockData` is using [`SubchunkBlocks`] data,
/// then this data should be interpreted as [`SubchunkExtraBlockData`]. Otherwise,
/// check the value of [`LegacyExtraBlockData::likely_nbt_pieces`] to hint whether [`NbtPieces`]
/// or [`TerrainExtraBlockData`] should be used.
///
/// Note: `LegacyExtraBlockData` does not perfectly round-trip into and from its variants,
/// though converting from a specific variant into `LegacyExtraBlockData` and back is bit-perfect.
/// Translating into `SubchunkExtraBlockData` or `TerrainExtraBlockData` disregards information
/// which is semantically unimportant for that variant, but may be significant for other variants of
/// `LegacyExtraBlockData`.
///
/// [`SubchunkBlocks`]: crate::subchunk_blocks::SubchunkBlocks
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone)]
pub struct LegacyExtraBlockData(pub Vec<ExtraBlockEntry>);

impl LegacyExtraBlockData {
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() < 4 {
            return None;
        }

        let num_entries = u32::from_le_bytes(value.subslice_to_array::<0, 4>());
        let num_entries = usize::try_from(num_entries).ok()?;

        if value.len() != 4 + num_entries * 6 {
            log::warn!(
                "LegacyExtraBlockData with {} entries (according to header) was expected \
                 to have length {}, but had length {}",
                num_entries,
                4 + num_entries * 6,
                value.len(),
            );
            return None;
        }

        // We can process value in chunks of 6 bytes
        let extra_blocks = value[4..]
            .chunks_exact(6)
            .map(|extra_block| {
                ExtraBlockEntry(extra_block.subslice_to_array::<0, 6>())
            })
            .collect();

        Some(Self(extra_blocks))
    }

    /// Check whether the middle two bytes of each 6-byte entry are ever nonzero.
    ///
    /// `SubchunkExtraBlockData` and `TerrainExtraBlockData` each have padding in those
    /// bytes, which is all zeroes in observed data. Conversely, `NbtPieces` should not be able
    /// to avoid having some nonzero bytes in those locations.
    pub fn likely_nbt_pieces(&self) -> bool {
        self.0
            .iter()
            .any(|&ExtraBlockEntry([_, _, byte_2, byte_3, _, _])| {
                byte_2 != 0 || byte_3 != 0
            })
    }

    pub fn extend_serialized(
        &self,
        bytes: &mut Vec<u8>,
        opts: ValueToBytesOptions,
    ) -> Result<(), ExtraBlocksToBytesError> {
        let (len_u32, num_entries) = opts
            .handle_excessive_length
            .length_to_u32(self.0.len())
            .ok_or(ExtraBlocksToBytesError::ExcessiveLength)?;

        let extra_block_iter = self.0
            .iter()
            .flat_map(|extra_block| extra_block.0);

        bytes.reserve(4 + num_entries * 6);
        bytes.extend(len_u32.to_le_bytes());
        bytes.extend(extra_block_iter);

        Ok(())
    }

    #[inline]
    pub fn to_bytes(
        &self,
        opts: ValueToBytesOptions,
    ) -> Result<Vec<u8>, ExtraBlocksToBytesError> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes, opts)?;
        Ok(bytes)
    }
}

/// A wrapper for `[u8; 6]`
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct ExtraBlockEntry(pub [u8; 6]);

impl ExtraBlockEntry {
    #[inline]
    pub fn key(self) -> [u8; 4] {
        self.0.subslice_to_array::<0, 4>()
    }

    #[inline]
    pub fn value(self) -> [u8; 2] {
        self.0.subslice_to_array::<4, 6>()
    }
}

#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub struct ExtraBlockValue {
    pub block_id:   u8,
    pub block_data: u8,
}

#[cfg_attr(feature = "derive_standard", derive(PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubchunkExtraBlockKey {
    pub column_pos: ChunkColumn,
    pub y_pos:      u8,
}

impl SubchunkExtraBlockKey {
    /// Returns the subchunk this key is in. Always in `0..=15`.
    #[inline]
    pub fn subchunk_y(self) -> i8 {
        (self.y_pos >> 4) as i8
    }

    #[inline]
    pub fn pos_in_subchunk(self) -> BlockPosInSubchunk {
        #[expect(
            clippy::unwrap_used,
            reason = "we force the y value to be strictly less than 16, so Some is returned",
        )]
        BlockPosInSubchunk::from_column(self.y_pos & 0b1111, self.column_pos).unwrap()
    }
}

#[expect(
    clippy::fallible_impl_from,
    reason = "the conditions on x, y, and z are enforced with bit manipulation",
)]
impl From<ExtraBlockEntry> for (SubchunkExtraBlockKey, ExtraBlockValue) {
    fn from(entry: ExtraBlockEntry) -> Self {
        let pos: [u8; 2] = entry.0.subslice_to_array::<0, 2>();

        let y_pos = pos[0];

        let x = pos[1] >> 4;
        let z = pos[1] & 0b1111;

        #[expect(
            clippy::unwrap_used,
            reason = "x and z are strictly less than 16, so Some is returned",
        )]
        let column_pos = ChunkColumn::new(x, z).unwrap();

        let value = ExtraBlockValue {
            block_id:   entry.0[4],
            block_data: entry.0[5],
        };

        (SubchunkExtraBlockKey { column_pos, y_pos }, value)
    }
}

impl From<(SubchunkExtraBlockKey, ExtraBlockValue)> for ExtraBlockEntry {
    #[inline]
    fn from(entry: (SubchunkExtraBlockKey, ExtraBlockValue)) -> Self {
        let (x, z) = entry.0.column_pos.xz();

        // The first byte is `y`, the second is `x` and `z`,
        // then there's padding,
        // then block_id, then block_data.
        Self([
            entry.0.y_pos, (x << 4) | z,
            0, 0,
            entry.1.block_id, entry.1.block_data,
        ])
    }
}

impl From<LegacyExtraBlockData> for SubchunkExtraBlockData {
    fn from(entries: LegacyExtraBlockData) -> Self {
        entries.0
            .into_iter()
            .map(<(SubchunkExtraBlockKey, ExtraBlockValue)>::from)
            .collect()
    }
}

impl From<SubchunkExtraBlockData> for LegacyExtraBlockData {
    fn from(entries: SubchunkExtraBlockData) -> Self {
        Self(entries
            .into_iter()
            .map(ExtraBlockEntry::from)
            .collect())
    }
}

impl From<&LegacyExtraBlockData> for SubchunkExtraBlockData {
    fn from(entries: &LegacyExtraBlockData) -> Self {
        entries.0
            .iter()
            .copied()
            .map(<(SubchunkExtraBlockKey, ExtraBlockValue)>::from)
            .collect()
    }
}

impl From<&SubchunkExtraBlockData> for LegacyExtraBlockData {
    fn from(entries: &SubchunkExtraBlockData) -> Self {
        Self(entries
            .iter()
            .map(|(key, val)| (*key, *val))
            .map(ExtraBlockEntry::from)
            .collect())
    }
}

#[cfg_attr(feature = "derive_standard", derive(PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainExtraBlockKey {
    pub column_pos: ChunkColumn,
    /// Should be in the range `0..=127`.
    /// The most-signficant bit is ignored or zeroed by functions in this module.
    pub y_pos:      u8,
}

impl TerrainExtraBlockKey {
    /// Returns the lower seven bits of `self.subchunk_y`
    #[inline]
    pub fn masked_y_pos(self) -> u8 {
        self.y_pos & 0b0111_1111
    }
}

#[expect(
    clippy::fallible_impl_from,
    reason = "the conditions on x and z are enforced with bit manipulation",
)]
impl From<ExtraBlockEntry> for (TerrainExtraBlockKey, ExtraBlockValue) {
    fn from(entry: ExtraBlockEntry) -> Self {
        let pos = u16::from_le_bytes(entry.0.subslice_to_array::<0, 2>());

        let y_pos = (pos & 0b0111_1111) as u8;
        let x = ((pos >>  7) & 0b1111) as u8;
        let z = ((pos >> 11) & 0b1111) as u8;

        #[expect(
            clippy::unwrap_used,
            reason = "x and z are strictly less than 16, so Some is returned",
        )]
        let column_pos = ChunkColumn::new(x, z).unwrap();

        let value = ExtraBlockValue {
            block_id:   entry.0[4],
            block_data: entry.0[5],
        };

        (TerrainExtraBlockKey { column_pos, y_pos }, value)
    }
}

impl From<(TerrainExtraBlockKey, ExtraBlockValue)> for ExtraBlockEntry {
    fn from(entry: (TerrainExtraBlockKey, ExtraBlockValue)) -> Self {
        let (x, z) = entry.0.column_pos.xz();
        let y = entry.0.masked_y_pos();

        let pos = (u16::from(z) << 11) | (u16::from(x) << 7) | u16::from(y);
        let [pos_0, pos_1] = pos.to_le_bytes();

        Self([
            pos_0, pos_1,
            0, 0,
            entry.1.block_id, entry.1.block_data,
        ])
    }
}

impl From<LegacyExtraBlockData> for TerrainExtraBlockData {
    fn from(entries: LegacyExtraBlockData) -> Self {
        entries.0
            .into_iter()
            .map(<(TerrainExtraBlockKey, ExtraBlockValue)>::from)
            .collect()
    }
}

impl From<TerrainExtraBlockData> for LegacyExtraBlockData {
    fn from(entries: TerrainExtraBlockData) -> Self {
        Self(entries
            .into_iter()
            .map(ExtraBlockEntry::from)
            .collect())
    }
}

impl From<&LegacyExtraBlockData> for TerrainExtraBlockData {
    fn from(entries: &LegacyExtraBlockData) -> Self {
        entries.0
            .iter()
            .copied()
            .map(<(TerrainExtraBlockKey, ExtraBlockValue)>::from)
            .collect()
    }
}

impl From<&TerrainExtraBlockData> for LegacyExtraBlockData {
    fn from(entries: &TerrainExtraBlockData) -> Self {
        Self(entries
            .iter()
            .map(|(key, val)| (*key, *val))
            .map(ExtraBlockEntry::from)
            .collect())
    }
}

/// Inexplicably, in some old versions of MCBE, the NBT data of block entities (chests, furnaces,
/// and signs) is broken into pieces in an odd way.
///
/// It seems to be stored in the LevelDB as `LegacyExtraBlockData` by eliding the first four bytes
/// (that is, in the case of chests, eliding the bytes `[10, 0, 0, 9]` at the start and beginning
/// with `[5, 0, b'I', b't', b'e', b'm', b's']`), splitting the remainder of the NBT
/// bytes into 6-byte pieces, and then storing these pieces in a deduplicated list without a
/// consistent order.
///
/// This process does not seem reversible. Manual inspection was sufficient for me to recognize
/// the data as described in [minecraft.wiki], and then to determine the number of filled slots
/// in a chest, a list of the different types of items in the chest, a list of the different stack
/// sizes of chest entries, and a list of the `Damage` values of the items.
///
/// However, associating a slot in the chest, and item type, a stack size, and a `Damage` value
/// with each other appears to be impossible due to great ambiguity caused by the lack of order
/// in the data.
///
/// In the event you do need to recover this data for some reason, `Display` is implemented;
/// its implementation attempts to interpret each byte as a graphic ASCII character, if possible,
/// which helps with reading binary NBT data. If you are familiar with NBT data, and use the
/// [Block ID table] and [Legacy Numeric ID] in the wiki, it should be possibly to determine
/// roughly what was in the chest.
///
/// As this is an unusual edge case, further support for recovery is not provided.
///
/// Note: this problematic data was observed at least in multiple worlds using protocol version 81,
/// which was used in variants of MCBE v0.15 (which were released in the middle of 2016).
/// It may also occur in other versions. I only encountered Chest data, but assume that Furnace and
/// Sign data could have the same problem.
///
/// [minecraft.wiki]:
///     https://minecraft.wiki/w/Bedrock_Edition_level_format/History#Tile_Entity_Format
/// [Block ID table]:
///     https://minecraft.wiki/w/Bedrock_Edition_data_values#Block_IDs
/// [Legacy Numeric ID]:
///     https://minecraft.wiki/w/Bedrock_Edition_data_values#Item_Table_with_Legacy_Numeric_ID
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone)]
pub struct NbtPieces(pub Vec<[u8; 6]>);

impl Display for NbtPieces {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "NbtPieces (length {}):", self.0.len())?;

        fn byte_to_str(byte: u8) -> String {
            if let Some(ch) = char::from_u32(u32::from(byte)) {
                if ch.is_ascii_graphic() {
                    return format!("'{ch}'");
                }
            }
            format!("{byte}")
        }

        let piece_iter = self.0
            .iter()
            .map(|bytes| {
                let mut original_bytes = bytes
                    .map(|byte| format!("{byte}"))
                    .join(",");
                original_bytes.push(']');

                let mut parsed_ascii = bytes
                    .map(byte_to_str)
                    .join(",");
                parsed_ascii.push_str("],");

                (original_bytes, parsed_ascii)
            });

        for (idx, (original_bytes, parsed_ascii)) in piece_iter.enumerate() {
            writeln!(f, "entry {idx:3}: [{parsed_ascii:25} originally [{original_bytes:23}")?;
        }

        Ok(())
    }
}

impl From<LegacyExtraBlockData> for NbtPieces {
    #[inline]
    fn from(entries: LegacyExtraBlockData) -> Self {
        let pieces = entries.0
            .into_iter()
            .map(|entry| entry.0)
            .collect();
        Self(pieces)
    }
}

impl From<NbtPieces> for LegacyExtraBlockData {
    #[inline]
    fn from(pieces: NbtPieces) -> Self {
        let entries = pieces.0
            .into_iter()
            .map(ExtraBlockEntry)
            .collect();
        Self(entries)
    }
}

impl From<&LegacyExtraBlockData> for NbtPieces {
    #[inline]
    fn from(entries: &LegacyExtraBlockData) -> Self {
        let pieces = entries.0
            .iter()
            .map(|entry| entry.0)
            .collect();
        Self(pieces)
    }
}

impl From<&NbtPieces> for LegacyExtraBlockData {
    #[inline]
    fn from(pieces: &NbtPieces) -> Self {
        let entries = pieces.0
            .iter()
            .copied()
            .map(ExtraBlockEntry)
            .collect();
        Self(entries)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ExtraBlocksToBytesError {
    ExcessiveLength,
}
