use subslice_to_array::SubsliceToArray as _;
use vecmap::VecMap;

use crate::ValueToBytesOptions;


// Thanks to rbedrock, I didn't have to do as much work determining the binary format here
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq))]
#[derive(Debug, Clone)]
pub struct Checksums(pub VecMap<ChecksumType, u64>);

impl Checksums {
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() < 4 {
            return None;
        }

        let num_entries = u32::from_le_bytes(value.subslice_to_array::<0, 4>());
        let num_entries = usize::try_from(num_entries).ok()?;

        if value.len() != 4 + num_entries * 11 {
            return None;
        }

        // We can process value in chunks of 11 bytes
        let mut value = &value[4..];
        let mut checksums = VecMap::with_capacity(num_entries);
        for _ in 0..num_entries {
            let tag           = value.subslice_to_array::<0, 2>();
            let subtag        = value[2] as i8;
            let checksum_hash = value.subslice_to_array::<3, 11>();
            value = &value[11..];

            checksums.insert(
                ChecksumType::parse(u16::from_le_bytes(tag), subtag)?,
                u64::from_le_bytes(checksum_hash),
            );
        }

        Some(Self(checksums))
    }

    // TODO: compute checksum for provided value
    // compute checksum for provided DBEntry <- needs to be done on DBEntry side
    // add checksum for provided value
    // add checksum for provided DBEntry <- needs to be done on DBEntry side
    // get checksum for specified Key (if there is any) <- needs to be done on DBKey side

    pub fn extend_serialized(
        &self,
        bytes: &mut Vec<u8>,
        opts: ValueToBytesOptions,
    ) -> Result<(), ChecksumsToBytesError> {
        let (len_u32, num_entries) = opts
            .handle_excessive_length
            .length_to_u32(self.0.len())
            .ok_or(ChecksumsToBytesError::ExcessiveLength)?;

        bytes.reserve(4 + num_entries * 11);
        bytes.extend(len_u32.to_le_bytes());
        for (&checksum_type, &checksum) in self.0.iter().take(num_entries) {
            let (tag, subtag) = checksum_type.to_tag_and_subtag();
            bytes.extend(tag.to_le_bytes());
            bytes.push(subtag as u8);
            bytes.extend(checksum.to_le_bytes());
        }

        Ok(())
    }

    #[inline]
    pub fn to_bytes(
        &self,
        opts: ValueToBytesOptions,
    ) -> Result<Vec<u8>, ChecksumsToBytesError> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes, opts)?;
        Ok(bytes)
    }
}

#[cfg_attr(feature = "derive_standard", derive(PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumType {
    Data2D,
    SubchunkBlocks(i8),
    BlockEntities,
    Entities,
}

impl ChecksumType {
    #[inline]
    pub fn parse(tag: u16, y_index: i8) -> Option<Self> {
        match (tag, y_index) {
            (45, 0)       => Some(Self::Data2D),
            (47, y_index) => Some(Self::SubchunkBlocks(y_index)),
            (49, 0)       => Some(Self::BlockEntities),
            (50, 0)       => Some(Self::Entities),
            _             => None,
        }
    }

    #[inline]
    pub fn to_tag_and_subtag(self) -> (u16, i8) {
        match self {
            Self::Data2D                  => (45, 0),
            Self::SubchunkBlocks(y_index) => (47, y_index),
            Self::BlockEntities           => (49, 0),
            Self::Entities                => (50, 0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ChecksumsToBytesError {
    ExcessiveLength,
}
